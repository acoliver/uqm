#![cfg(any(not(test), feature = "linked_c_archive"))]
#![allow(non_snake_case)]

//! Shipyard menu, crew transactions and hangar animation, ported from shipyard.c.

use crate::navigation::abi::*;
use crate::navigation::globals::GlobData;
use crate::state::game_state_keys::{get_game_state, set_game_state};
use std::{mem, ptr};
extern "C" {
    fn DoShipSpin(index: COUNT, music: MUSIC_REF);
}

use super::shipyard_layout::{
    DMS_GetEscortShipRect, DMS_MoveCursor, HANGAR_DY, HANGAR_SHIPS_ROW, HANGAR_X_COORDS, HANGAR_Y,
    SHIP_WIN_HEIGHT, SHIP_WIN_WIDTH, USE_3DO_HANGAR,
};
const HANGAR_ANIM_RATE: u32 = 15;
const SHIPYARD: u32 = 2;
const SHIPYARD_CREW: u8 = 0;
const SHIPYARD_SAVELOAD: u8 = 1;
const SHIPYARD_EXIT: u8 = 2;
const SHIP_WIN_FRAMES: i32 = ((SHIP_WIN_WIDTH >> 1) + 1) as i32;
const MODIFY_CREW_FLAG: SIZE = 1 << 8;
use crate::starbase::shipyard::SHIP_COST;

#[derive(Clone, Copy)]
enum ModifyMode {
    Navigate,
    AddEscort,
    EditCrew,
    Exit,
}

const fn rgb15(r: u8, g: u8, b: u8) -> Color {
    Color {
        r: (r << 3) | (r >> 2),
        g: (g << 3) | (g >> 2),
        b: (b << 3) | (b >> 2),
        a: 255,
    }
}

const BLACK: Color = rgb15(0, 0, 0);
const MENU_3DO: *const RECT = usize::MAX as *const RECT;
const MENU_ANY: *const RECT = (usize::MAX - 1) as *const RECT;
const SOUND_UP: MENU_SOUND_FLAGS = 1 << 0;
const SOUND_DOWN: MENU_SOUND_FLAGS = 1 << 1;
const SOUND_ARROWS: MENU_SOUND_FLAGS = 0x0f;
const SOUND_SELECT: MENU_SOUND_FLAGS = 1 << 4;
const SOUND_CANCEL: MENU_SOUND_FLAGS = 1 << 5;

static mut POWER_STAMP: STAMP = STAMP {
    origin: POINT { x: 0, y: 0 },
    frame: ptr::null_mut(),
};
static mut POWER_COLORMAP: COLORMAP = ptr::null_mut();
static mut POWER_NEXT_TIME: TimeCount = 0;

unsafe fn animatePowerLines(menu: *mut MENU_STATE) {
    let now = GetTimeCounter();
    if !menu.is_null() {
        POWER_STAMP.origin.x = 0;
        POWER_STAMP.origin.y = 0;
        POWER_STAMP.frame = SetAbsFrameIndex((*menu).ModuleFrame, 24);
        POWER_COLORMAP = SetAbsStringTableIndex((*menu).CurString, 0);
    }
    if now >= POWER_NEXT_TIME || !menu.is_null() {
        POWER_NEXT_TIME = now.wrapping_add(ONE_SECOND / HANGAR_ANIM_RATE);
        SetColorMap(GetColorMapAddress(POWER_COLORMAP));
        DrawStamp(ptr::addr_of_mut!(POWER_STAMP));
        POWER_COLORMAP = SetRelStringTableIndex(POWER_COLORMAP, 1);
    }
}

unsafe extern "C" fn on_input_frame() {
    let old = SetContext(SpaceContext);
    animatePowerLines(ptr::null_mut());
    SetContext(old);
}

unsafe fn SpinStarShip(menu: *mut MENU_STATE, ship: HFLEETINFO) {
    let fleet = ship.cast::<FLEET_INFO>();
    let index = FindMasterShipIndex((*fleet).SpeciesID);
    if index >= 0 && index < (MeleeShip_MELEE_ZOQFOTPIK + 1) as i32 {
        DoShipSpin(index as COUNT, (*menu).hMusic);
    }
}

unsafe fn GetAvailableRaceCount() -> COUNT {
    let mut index: COUNT = 0;
    let mut ship = GlobData.Game_state.avail_race_q.head;
    while !ship.is_null() {
        let fleet = ship.cast::<FLEET_INFO>();
        if (*fleet).allied_state == GOOD_GUY as u16 {
            index += 1;
        }
        ship = (*fleet).succ;
    }
    index
}

unsafe fn GetAvailableRaceFromIndex(mut index: BYTE) -> HFLEETINFO {
    let mut ship = GlobData.Game_state.avail_race_q.head;
    while !ship.is_null() {
        let fleet = ship.cast::<FLEET_INFO>();
        if (*fleet).allied_state == GOOD_GUY as u16 {
            let old_index = index;
            index = index.wrapping_sub(1);
            if old_index == 0 {
                return ship;
            }
        }
        ship = (*fleet).succ;
    }
    ptr::null_mut()
}

unsafe fn DrawRaceStrings(menu: *mut MENU_STATE, mut race: BYTE) {
    let old = SetContext(StatusContext);
    let mut r: RECT = mem::zeroed();
    GetContextClipRect(&mut r);
    let mut stamp = STAMP {
        origin: POINT {
            x: (4 + ScreenWidth - STATUS_WIDTH as i32 - SAFE_X as i32) as i16 - r.corner.x,
            y: (SIS_ORG_Y as i32 + ScreenHeight - SAFE_Y as i32 * 2 - 13 - RADAR_HEIGHT as i32)
                as i16
                - r.corner.y,
        },
        frame: ptr::null_mut(),
    };
    r.corner.x = stamp.origin.x - 1;
    r.corner.y = stamp.origin.y - 11;
    r.extent.width = RADAR_WIDTH as i16 + 2;
    r.extent.height = 11;
    BatchGraphics();
    ClearSISRect(CLEAR_SIS_RADAR as BYTE);
    SetContextForeGroundColor(rgb15(0x0a, 0x0a, 0x0a));
    DrawFilledRectangle(&mut r);
    r.corner = stamp.origin;
    r.extent.width = RADAR_WIDTH as i16;
    r.extent.height = RADAR_HEIGHT as i16;
    SetContextForeGroundColor(BLACK);
    DrawFilledRectangle(&mut r);
    if race != BYTE::MAX {
        let ship = GetAvailableRaceFromIndex(race);
        race =
            GetIndexFromStarShip(ptr::addr_of_mut!(GlobData.Game_state.avail_race_q), ship) as BYTE;
        stamp.frame = SetAbsFrameIndex((*menu).ModuleFrame, 3 + race as COUNT);
        DrawStamp(&mut stamp);
        stamp.frame = (*ship.cast::<FLEET_INFO>()).melee_icon;
        let mut text = TEXT {
            baseline: POINT {
                x: stamp.origin.x + RADAR_WIDTH as i16 - 2,
                y: stamp.origin.y + RADAR_HEIGHT as i16 - 2,
            },
            align: TEXT_ALIGN_ALIGN_RIGHT,
            CharCount: COUNT::MAX,
            pStr: ptr::null(),
        };
        stamp.origin.x += (RADAR_WIDTH >> 1) as i16;
        stamp.origin.y += (RADAR_HEIGHT >> 1) as i16;
        DrawStamp(&mut stamp);
        let cost = format!("{}\0", SHIP_COST[race as usize]);
        text.pStr = cost.as_ptr().cast();
        SetContextFont(TinyFont);
        SetContextForeGroundColor(rgb15(0, 0x1f, 0));
        font_DrawText(&mut text);
    }
    UnbatchGraphics();
    SetContext(old);
}

unsafe fn ShowShipCrew(ship: *mut SHIP_FRAGMENT, rect: *const RECT) {
    let template = GetStarShipFromIndex(
        ptr::addr_of_mut!(GlobData.Game_state.avail_race_q),
        (*ship).race_id as COUNT,
    );
    let max_crew = (*template.cast::<FLEET_INFO>()).crew_level;
    let buffer = if (*ship).crew_level >= max_crew {
        format!("{}\0", (*ship).crew_level)
    } else if (*ship).crew_level == 0 {
        String::from("SCRAP\0")
    } else {
        format!("{}/{}\0", (*ship).crew_level, max_crew)
    };
    let mut r = *rect;
    let mut text = TEXT {
        baseline: POINT {
            x: r.corner.x + (r.extent.width >> 1),
            y: r.corner.y + r.extent.height - 1,
        },
        align: TEXT_ALIGN_ALIGN_CENTER,
        pStr: buffer.as_ptr().cast(),
        CharCount: COUNT::MAX,
    };
    if r.corner.y != 0 {
        r.corner.y = text.baseline.y - 6;
        r.extent.width = SHIP_WIN_WIDTH;
        r.extent.height = 6;
        SetContextForeGroundColor(BLACK);
        DrawFilledRectangle(&mut r);
    }
    SetContextForeGroundColor(if (*ship).crew_level != 0 {
        rgb15(0, 0x14, 0)
    } else {
        rgb15(0x12, 0, 0)
    });
    font_DrawText(&mut text);
}

#[derive(Clone, Copy)]
struct ShipWindow {
    ship: *mut SHIP_FRAGMENT,
    finished: POINT,
    image: STAMP,
    left_door: STAMP,
    right_door: STAMP,
}

unsafe fn ShowCombatShip(menu: *mut MENU_STATE, mut window: COUNT, yanked: *mut SHIP_FRAGMENT) {
    let mut windows: [ShipWindow; MAX_BUILT_SHIPS as usize] = mem::zeroed();
    let mut num_ships: COUNT = 1;
    if !yanked.is_null() {
        let info = &mut windows[0];
        info.ship = yanked;
        info.left_door.origin = POINT {
            x: -(SHIP_WIN_WIDTH >> 1),
            y: 0,
        };
        info.right_door.origin = POINT {
            x: SHIP_WIN_WIDTH >> 1,
            y: 0,
        };
        info.left_door.frame = IncFrameIndex((*menu).ModuleFrame);
        info.right_door.frame = IncFrameIndex(info.left_door.frame);
        info.image.origin = POINT {
            x: (SHIP_WIN_WIDTH >> 1) + 1,
            y: SHIP_WIN_WIDTH >> 1,
        };
        info.image.frame = (*yanked).melee_icon;
        info.finished.x = HANGAR_X_COORDS[window as usize % HANGAR_SHIPS_ROW as usize];
        info.finished.y = HANGAR_Y + HANGAR_DY * (window / HANGAR_SHIPS_ROW as COUNT) as i16;
    } else {
        let queue = ptr::addr_of_mut!(GlobData.Game_state.built_ship_q);
        let mut handle;
        if window == COUNT::MAX {
            handle = (*queue).head;
            num_ships = CountLinks(queue);
        } else {
            let tail = (*queue).tail;
            RemoveQueue(queue, tail);
            handle = (*queue).head;
            while !handle.is_null() {
                let ship = handle.cast::<SHIP_FRAGMENT>();
                if (*ship).index as COUNT > window {
                    break;
                }
                handle = (*ship).succ;
            }
            InsertQueue(queue, tail, handle);
            handle = tail;
            (*handle.cast::<SHIP_FRAGMENT>()).index = window as BYTE;
        }
        for i in 0..num_ships {
            let ship = handle.cast::<SHIP_FRAGMENT>();
            let next = (*ship).succ;
            let info = &mut windows[i as usize];
            info.ship = ship;
            info.left_door.origin = POINT { x: -1, y: 0 };
            info.right_door.origin = POINT { x: 1, y: 0 };
            info.left_door.frame = IncFrameIndex((*menu).ModuleFrame);
            info.right_door.frame = IncFrameIndex(info.left_door.frame);
            info.image.origin = POINT {
                x: (SHIP_WIN_WIDTH >> 1) + 1,
                y: SHIP_WIN_WIDTH >> 1,
            };
            info.image.frame = (*ship).melee_icon;
            window = (*ship).index as COUNT;
            info.finished.x = HANGAR_X_COORDS[window as usize % HANGAR_SHIPS_ROW as usize];
            info.finished.y = HANGAR_Y + HANGAR_DY * (window / HANGAR_SHIPS_ROW as COUNT) as i16;
            handle = next;
        }
    }
    if num_ships != 0 {
        let mut all_finished = false;
        let r = RECT {
            corner: POINT { x: 0, y: 0 },
            extent: EXTENT {
                width: SHIP_WIN_WIDTH,
                height: SHIP_WIN_HEIGHT,
            },
        };
        FlushInput();
        let mut time_in = GetTimeCounter();
        let mut j = 0;
        while j < SHIP_WIN_FRAMES && !all_finished {
            SleepThreadUntil(time_in.wrapping_add(ONE_SECOND / 24));
            time_in = GetTimeCounter();
            if AnyButtonPress(0) != 0 {
                if !yanked.is_null() {
                    windows[0].left_door.origin.x = 0;
                    windows[0].right_door.origin.x = 0;
                }
                all_finished = true;
            }
            let old = SetContext(SpaceContext);
            let mut old_clip: RECT = mem::zeroed();
            GetContextClipRect(&mut old_clip);
            SetContextBackGroundColor(BLACK);
            BatchGraphics();
            for i in 0..num_ships {
                let info = &mut windows[i as usize];
                let mut clip = RECT {
                    corner: POINT {
                        x: SIS_ORG_X as i16 + info.finished.x,
                        y: SIS_ORG_Y as i16 + info.finished.y,
                    },
                    extent: EXTENT {
                        width: SHIP_WIN_WIDTH,
                        height: SHIP_WIN_HEIGHT,
                    },
                };
                SetContextClipRect(&mut clip);
                ClearDrawable();
                DrawStamp(&mut info.image);
                ShowShipCrew(info.ship, &r);
                if !all_finished || !yanked.is_null() {
                    DrawStamp(&mut info.left_door);
                    DrawStamp(&mut info.right_door);
                    if !yanked.is_null() {
                        info.left_door.origin.x += 1;
                        info.right_door.origin.x -= 1;
                    } else {
                        info.left_door.origin.x -= 1;
                        info.right_door.origin.x += 1;
                    }
                }
            }
            SetContextClipRect(&mut old_clip);
            if !USE_3DO_HANGAR {
                animatePowerLines(ptr::null_mut());
            }
            UnbatchGraphics();
            SetContext(old);
            j += 1;
        }
    }
}

fn purchased_crew() -> SIZE {
    u16::from_le_bytes([
        get_game_state("CREW_PURCHASED0") as BYTE,
        get_game_state("CREW_PURCHASED1") as BYTE,
    ]) as SIZE
}

unsafe fn CrewTransaction(delta: SIZE) {
    if delta != 0 {
        let mut bought = purchased_crew().wrapping_add(delta);
        if bought < 0 {
            bought = if delta < 0 { 0 } else { 0x7fff };
        } else if delta > 0 {
            if bought as i32 >= CREW_EXPENSE_THRESHOLD as i32
                && (bought as i32 - delta as i32) < CREW_EXPENSE_THRESHOLD as i32
            {
                GlobData.Game_state.CrewCost += 2;
                DrawMenuStateStrings(PM_CREW as BYTE, SHIPYARD_CREW as i16);
            }
        } else if (bought as i32) < CREW_EXPENSE_THRESHOLD as i32
            && bought as i32 - delta as i32 >= CREW_EXPENSE_THRESHOLD as i32
        {
            GlobData.Game_state.CrewCost -= 2;
            DrawMenuStateStrings(PM_CREW as BYTE, SHIPYARD_CREW as i16);
        }
        if CheckAlliance(SHOFIXTI_SHIP as COUNT) != GOOD_GUY as COUNT {
            set_game_state("CREW_PURCHASED0", bought as BYTE as u32);
            set_game_state("CREW_PURCHASED1", ((bought as u16) >> 8) as u32);
        }
    }
}

unsafe fn DMS_FlashFlagShip() {
    let r = RECT {
        corner: POINT { x: 0, y: 0 },
        extent: EXTENT {
            width: (ScreenWidth - STATUS_WIDTH as i32 - SAFE_X as i32 * 2 - 14) as i16,
            height: 61,
        },
    };
    SetFlashRect(&r);
}

unsafe fn DMS_FlashEscortShip(slot: BYTE) {
    SetFlashRect(&DMS_GetEscortShipRect(slot));
}

unsafe fn DMS_FlashFlagShipCrewCount() {
    let mut r: RECT = mem::zeroed();
    SetContext(StatusContext);
    GetGaugeRect(&mut r, 1);
    SetFlashRect(&r);
    SetContext(SpaceContext);
}

unsafe fn DMS_FlashEscortShipCrewCount(slot: BYTE) {
    let row = slot / HANGAR_SHIPS_ROW;
    let col = slot % HANGAR_SHIPS_ROW;
    let r = RECT {
        corner: POINT {
            x: HANGAR_X_COORDS[col as usize],
            y: HANGAR_Y + HANGAR_DY * row as i16 + SHIP_WIN_HEIGHT - 6,
        },
        extent: EXTENT {
            width: SHIP_WIN_WIDTH,
            height: 5,
        },
    };
    SetContext(SpaceContext);
    SetFlashRect(&r);
}

unsafe fn DMS_FlashActiveShip(menu: *mut MENU_STATE) {
    if (*menu).CurState >> 4 != 0 {
        DMS_FlashFlagShip();
    } else {
        DMS_FlashEscortShip((*menu).CurState);
    }
}

unsafe fn DMS_SetMode(menu: *mut MENU_STATE, mode: ModifyMode) {
    match mode {
        ModifyMode::Navigate => {
            SetMenuSounds(SOUND_ARROWS, SOUND_SELECT);
            DMS_FlashActiveShip(menu);
        }
        ModifyMode::AddEscort => {
            SetMenuSounds(SOUND_ARROWS, SOUND_SELECT);
            SetFlashRect(MENU_ANY);
        }
        ModifyMode::EditCrew => {
            SetMenuSounds(SOUND_UP | SOUND_DOWN, SOUND_SELECT | SOUND_CANCEL);
            if (*menu).CurState >> 4 != 0 {
                DMS_FlashFlagShipCrewCount();
            } else {
                DMS_FlashEscortShipCrewCount((*menu).CurState);
            }
        }
        ModifyMode::Exit => {
            SetMenuSounds(SOUND_ARROWS, SOUND_SELECT);
            SetFlashRect(MENU_3DO);
        }
    }
}

unsafe fn DMS_SpinShip(menu: *mut MENU_STATE, ship: HSHIPFRAG) -> bool {
    if (*menu).CurState >> 4 != 0 {
        return false;
    }
    if ship.is_null() == ((*menu).delta_item & MODIFY_CREW_FLAG == 0) {
        return false;
    }
    let spin_ship = if ship.is_null() {
        let spin_ship = GetAvailableRaceFromIndex((*menu).delta_item as BYTE);
        if spin_ship.is_null() {
            return false;
        }
        spin_ship
    } else {
        let fragment = ship.cast::<SHIP_FRAGMENT>();
        GetStarShipFromIndex(
            ptr::addr_of_mut!(GlobData.Game_state.avail_race_q),
            (*fragment).race_id as COUNT,
        )
    };
    SetFlashRect(ptr::null());
    let old = SetContext(ScreenContext);
    let mut old_clip: RECT = mem::zeroed();
    GetContextClipRect(&mut old_clip);
    SpinStarShip(menu, spin_ship);
    SetContextClipRect(&mut old_clip);
    SetContext(old);
    true
}

unsafe fn DMS_HireFlagShipCrew() -> SIZE {
    let mut point: POINT = mem::zeroed();
    if GetCPodCapacity(&mut point) <= GetCrewCount() {
        return 0;
    }
    if GlobData.SIS_state.ResUnits < GlobData.Game_state.CrewCost as DWORD {
        return 0;
    }
    DrawPoint(&mut point);
    PreUpdateFlashRect();
    DeltaSISGauges(1, 0, -(GlobData.Game_state.CrewCost as i32));
    PostUpdateFlashRect();
    1
}

unsafe fn DMS_DismissFlagShipCrew() -> SIZE {
    if GetCrewCount() == 0 {
        return 0;
    }
    let bought = purchased_crew();
    PreUpdateFlashRect();
    DeltaSISGauges(
        -1,
        0,
        GlobData.Game_state.CrewCost as i32
            - if bought == CREW_EXPENSE_THRESHOLD as SIZE {
                2
            } else {
                0
            },
    );
    PostUpdateFlashRect();
    let mut point: POINT = mem::zeroed();
    GetCPodCapacity(&mut point);
    SetContextForeGroundColor(BLACK);
    DrawPoint(&mut point);
    -1
}

unsafe fn DMS_HireEscortShipCrew(ship: *mut SHIP_FRAGMENT) -> SIZE {
    let template = GetStarShipFromIndex(
        ptr::addr_of_mut!(GlobData.Game_state.avail_race_q),
        (*ship).race_id as COUNT,
    );
    let max_crew = (*template.cast::<FLEET_INFO>()).crew_level;
    if GlobData.SIS_state.ResUnits < GlobData.Game_state.CrewCost as DWORD {
        return 0;
    }
    if (*ship).crew_level >= (*ship).max_crew {
        return 0;
    }
    if (*ship).crew_level >= max_crew {
        return 0;
    }
    if (*ship).crew_level > 0 {
        DeltaSISGauges(0, 0, -(GlobData.Game_state.CrewCost as i32));
    } else {
        DeltaSISGauges(0, 0, -(SHIP_COST[(*ship).race_id as usize] as i32));
    }
    (*ship).crew_level += 1;
    PreUpdateFlashRect();
    let r = DMS_GetEscortShipRect((*ship).index);
    ShowShipCrew(ship, &r);
    PostUpdateFlashRect();
    1
}

unsafe fn DMS_DismissEscortShipCrew(ship: *mut SHIP_FRAGMENT) -> SIZE {
    let mut delta = 0;
    if (*ship).crew_level > 0 {
        if (*ship).crew_level > 1 {
            let bought = purchased_crew();
            DeltaSISGauges(
                0,
                0,
                GlobData.Game_state.CrewCost as i32
                    - if bought == CREW_EXPENSE_THRESHOLD as SIZE {
                        2
                    } else {
                        0
                    },
            );
        } else {
            DeltaSISGauges(0, 0, SHIP_COST[(*ship).race_id as usize] as i32);
        }
        delta = -1;
        (*ship).crew_level -= 1;
    } else {
        PlayMenuSound(MENU_SOUND_EFFECT_MENU_SOUND_FAILURE);
    }
    PreUpdateFlashRect();
    let r = DMS_GetEscortShipRect((*ship).index);
    ShowShipCrew(ship, &r);
    PostUpdateFlashRect();
    delta
}

unsafe fn DMS_ModifyCrew(menu: *mut MENU_STATE, handle: HSHIPFRAG, dy: SBYTE) {
    let delta;
    let ship = handle.cast::<SHIP_FRAGMENT>();
    if handle.is_null() {
        delta = if dy < 0 {
            DMS_HireFlagShipCrew()
        } else {
            DMS_DismissFlagShipCrew()
        };
        if delta != 0 {
            DMS_FlashFlagShipCrewCount();
        }
    } else {
        delta = if dy < 0 {
            DMS_HireEscortShipCrew(ship)
        } else {
            DMS_DismissEscortShipCrew(ship)
        };
        if delta != 0 {
            DMS_FlashEscortShipCrewCount((*ship).index);
        }
    }
    if delta == 0 {
        PlayMenuSound(MENU_SOUND_EFFECT_MENU_SOUND_FAILURE);
    }
    if !handle.is_null() {
        (*menu).delta_item &= MODIFY_CREW_FLAG;
    }
    CrewTransaction(delta);
}

unsafe fn DMS_TryAddEscortShip(menu: *mut MENU_STATE) {
    let info = GetAvailableRaceFromIndex((*menu).delta_item as BYTE);
    let index = GetIndexFromStarShip(ptr::addr_of_mut!(GlobData.Game_state.avail_race_q), info);
    if GlobData.SIS_state.ResUnits >= SHIP_COST[index as usize] as DWORD
        && !CloneShipFragment(
            index,
            ptr::addr_of_mut!(GlobData.Game_state.built_ship_q),
            1,
        )
        .is_null()
    {
        ShowCombatShip(menu, (*menu).CurState as COUNT, ptr::null_mut());
        DrawMenuStateStrings(PM_CREW as BYTE, SHIPYARD_CREW as i16);
        DeltaSISGauges(
            UNDEFINED_DELTA as SIZE,
            UNDEFINED_DELTA as SIZE,
            -(SHIP_COST[index as usize] as i32),
        );
        DMS_SetMode(menu, ModifyMode::EditCrew);
    } else {
        PlayMenuSound(MENU_SOUND_EFFECT_MENU_SOUND_FAILURE);
    }
}

unsafe fn DMS_AddEscortShip(
    menu: *mut MENU_STATE,
    special: bool,
    select: bool,
    cancel: bool,
    dx: SBYTE,
    dy: SBYTE,
) {
    assert!((*menu).delta_item & MODIFY_CREW_FLAG != 0);
    if special {
        let ship = GetEscortByStarShipIndex((*menu).delta_item as COUNT);
        if DMS_SpinShip(menu, ship) {
            DMS_SetMode(menu, ModifyMode::AddEscort);
        }
        return;
    }
    if cancel {
        (*menu).delta_item &= !MODIFY_CREW_FLAG;
        SetFlashRect(ptr::null());
        DrawMenuStateStrings(PM_CREW as BYTE, SHIPYARD_CREW as i16);
        DMS_SetMode(menu, ModifyMode::Navigate);
    } else if select {
        DMS_TryAddEscortShip(menu);
    } else if dx != 0 || dy != 0 {
        let available = GetAvailableRaceCount();
        let mut current = (*menu).delta_item as BYTE;
        if dx < 0 || dy < 0 {
            let old = current;
            current = current.wrapping_sub(1);
            if old == 0 {
                current = available.wrapping_sub(1) as BYTE;
            }
        } else if dx > 0 || dy > 0 {
            current = current.wrapping_add(1);
            if current as COUNT == available {
                current = 0;
            }
        }
        if current != (*menu).delta_item as BYTE {
            PreUpdateFlashRect();
            DrawRaceStrings(menu, current);
            PostUpdateFlashRect();
            (*menu).delta_item = current as SIZE | MODIFY_CREW_FLAG;
        }
    }
}

unsafe fn DMS_ScrapEscortShip(menu: *mut MENU_STATE, handle: HSHIPFRAG) {
    let ship = handle.cast::<SHIP_FRAGMENT>();
    SetFlashRect(ptr::null());
    ShowCombatShip(menu, (*menu).CurState as COUNT, ship);
    let queue = ptr::addr_of_mut!(GlobData.Game_state.built_ship_q);
    RemoveQueue(queue, handle);
    FreeLink(queue, handle);
    DeltaSISGauges(
        UNDEFINED_DELTA as SIZE,
        UNDEFINED_DELTA as SIZE,
        UNDEFINED_DELTA as i32,
    );
    SetContext(SpaceContext);
    DMS_SetMode(menu, ModifyMode::Navigate);
}

unsafe fn DMS_EditCrewMode(
    menu: *mut MENU_STATE,
    ship: HSHIPFRAG,
    select: bool,
    cancel: bool,
    dy: SBYTE,
) {
    if select || cancel {
        if !ship.is_null() {
            let crew = (*ship.cast::<SHIP_FRAGMENT>()).crew_level;
            if crew == 0 {
                DMS_ScrapEscortShip(menu, ship);
            }
        }
        (*menu).delta_item &= !MODIFY_CREW_FLAG;
        DMS_SetMode(menu, ModifyMode::Navigate);
    } else if dy != 0 {
        DMS_ModifyCrew(menu, ship, dy);
    }
}

unsafe fn DMS_NavigateShipSlots(
    menu: *mut MENU_STATE,
    special: bool,
    select: bool,
    cancel: bool,
    dx: SBYTE,
    dy: SBYTE,
) {
    let ship = GetEscortByStarShipIndex((*menu).CurState as COUNT);
    if dx != 0 || dy != 0 {
        let new_state = DMS_MoveCursor((*menu).CurState, dx, dy);
        if new_state != (*menu).CurState {
            (*menu).CurState = new_state;
            DMS_FlashActiveShip(menu);
        }
    }
    if special {
        if DMS_SpinShip(menu, ship) {
            DMS_SetMode(menu, ModifyMode::Navigate);
        }
    } else if select {
        if ship.is_null() && (*menu).CurState >> 4 == 0 {
            (*menu).delta_item = MODIFY_CREW_FLAG;
            DrawRaceStrings(menu, 0);
            DMS_SetMode(menu, ModifyMode::AddEscort);
        } else {
            (*menu).delta_item |= MODIFY_CREW_FLAG;
            DMS_SetMode(menu, ModifyMode::EditCrew);
        }
    } else if cancel {
        (*menu).InputFunc = Some(DoShipyard);
        (*menu).CurState = SHIPYARD_CREW;
        DrawMenuStateStrings(PM_CREW as BYTE, (*menu).CurState as i16);
        DMS_SetMode(menu, ModifyMode::Exit);
    }
}

unsafe extern "C" fn DoModifyShips(menu: *mut MENU_STATE) -> BOOLEAN {
    if GlobData.Game_state.CurrentActivity & CHECK_ABORT as u16 != 0 {
        (*menu).InputFunc = Some(DoShipyard);
        return 1;
    }
    if (*menu).Initialized == 0 {
        (*menu).InputFunc = Some(DoModifyShips);
        (*menu).Initialized = 1;
        (*menu).CurState = 0xf0;
        (*menu).delta_item = 0;
        SetContext(SpaceContext);
        DMS_SetMode(menu, ModifyMode::Navigate);
    } else {
        let special = PulsedInputState.menu[KEY_MENU_SPECIAL as usize] != 0;
        let select = PulsedInputState.menu[KEY_MENU_SELECT as usize] != 0;
        let cancel = PulsedInputState.menu[KEY_MENU_CANCEL as usize] != 0;
        let mut dx = 0;
        let mut dy = 0;
        if PulsedInputState.menu[KEY_MENU_RIGHT as usize] != 0 {
            dx = 1;
        }
        if PulsedInputState.menu[KEY_MENU_LEFT as usize] != 0 {
            dx = -1;
        }
        if PulsedInputState.menu[KEY_MENU_UP as usize] != 0 {
            dy = -1;
        }
        if PulsedInputState.menu[KEY_MENU_DOWN as usize] != 0 {
            dy = 1;
        }
        if (*menu).delta_item & MODIFY_CREW_FLAG == 0 {
            DMS_NavigateShipSlots(menu, special, select, cancel, dx, dy);
        } else {
            let ship = GetEscortByStarShipIndex((*menu).CurState as COUNT);
            if ship.is_null() && (*menu).CurState >> 4 == 0 {
                DMS_AddEscortShip(menu, special, select, cancel, dx, dy);
            } else {
                DMS_EditCrewMode(menu, ship, select, cancel, dy);
            }
        }
    }
    SleepThread(ONE_SECOND / 30);
    1
}

unsafe fn DrawBluePrint(menu: *mut MENU_STATE) {
    let frame = CaptureDrawable(LoadGraphicInstance(c"graphics.blueprints".as_ptr()).cast());
    let mut stamp = STAMP {
        origin: POINT { x: 0, y: 0 },
        frame: DecFrameIndex(frame),
    };
    SetContextForeGroundColor(rgb15(0, 0, 0x16));
    DrawFilledStamp(&mut stamp);
    for slot in 0..NUM_DRIVE_SLOTS {
        DrawShipPiece(
            frame,
            GlobData.SIS_state.DriveSlots[slot as usize] as COUNT,
            slot as COUNT,
            1,
        );
    }
    for slot in 0..NUM_JET_SLOTS {
        DrawShipPiece(
            frame,
            GlobData.SIS_state.JetSlots[slot as usize] as COUNT,
            slot as COUNT,
            1,
        );
    }
    for slot in 0..NUM_MODULE_SLOTS {
        let piece = GlobData.SIS_state.ModuleSlots[slot as usize];
        if !((*menu).CurState as u32 == SHIPYARD && piece as u32 == CREW_POD) {
            DrawShipPiece(frame, piece as COUNT, slot as COUNT, 1);
        }
    }
    SetContextForeGroundColor(rgb15(0x0a, 0x0a, 0x1f));
    for slot in 0..NUM_MODULE_SLOTS {
        let piece = GlobData.SIS_state.ModuleSlots[slot as usize];
        if (*menu).CurState as u32 == SHIPYARD && piece as u32 == CREW_POD {
            DrawShipPiece(frame, piece as COUNT, slot as COUNT, 1);
        }
    }
    let mut frames = GlobData.SIS_state.CrewEnlisted;
    GlobData.SIS_state.CrewEnlisted = 0;
    loop {
        let old = frames;
        frames = frames.wrapping_sub(1);
        if old == 0 {
            break;
        }
        let mut point: POINT = mem::zeroed();
        GetCPodCapacity(&mut point);
        DrawPoint(&mut point);
        GlobData.SIS_state.CrewEnlisted += 1;
    }
    frames = GlobData.SIS_state.TotalElementMass;
    GlobData.SIS_state.TotalElementMass = 0;
    let mut r = RECT {
        corner: POINT { x: 0, y: 0 },
        extent: EXTENT {
            width: 9,
            height: 1,
        },
    };
    while frames != 0 {
        let mass = if frames < SBAY_MASS_PER_ROW as COUNT {
            frames
        } else {
            SBAY_MASS_PER_ROW as COUNT
        };
        GlobData.SIS_state.TotalElementMass += mass;
        GetSBayCapacity(&mut r.corner);
        DrawFilledRectangle(&mut r);
        frames -= mass;
    }
    if GlobData.SIS_state.FuelOnBoard > FUEL_RESERVE {
        let mut volume = GlobData.SIS_state.FuelOnBoard - FUEL_RESERVE;
        GlobData.SIS_state.FuelOnBoard = FUEL_RESERVE;
        r.extent.width = 3;
        r.extent.height = 1;
        while volume != 0 {
            GetFTankCapacity(&mut r.corner);
            DrawPoint(&mut r.corner);
            r.corner.x += r.extent.width + 1;
            DrawPoint(&mut r.corner);
            r.corner.x -= r.extent.width;
            let old_background = SetContextBackGroundColor(BLACK);
            SetContextForeGroundColor(old_background);
            DrawFilledRectangle(&mut r);
            let mass = if volume < FUEL_VOLUME_PER_ROW {
                volume as COUNT
            } else {
                FUEL_VOLUME_PER_ROW as COUNT
            };
            GlobData.SIS_state.FuelOnBoard += mass as DWORD;
            volume -= mass as DWORD;
        }
    }
    DestroyDrawable(ReleaseDrawable(frame));
}

unsafe fn ExitShipyard(menu: *mut MENU_STATE) -> BOOLEAN {
    SetInputCallback(None);
    DestroyDrawable(ReleaseDrawable((*menu).ModuleFrame));
    (*menu).ModuleFrame = ptr::null_mut();
    DestroyStringTable(ReleaseStringTable((*menu).CurString));
    (*menu).CurString = ptr::null_mut();
    0
}

/// Processes one shipyard input frame, including entry and exit resource handling.
///
/// # Safety
/// `menu` must point to the live `MENU_STATE` owned by the game-thread input loop.
/// Global game state, queues, graphics, resource and input systems must be initialized;
/// callbacks and this function must run exclusively on that same thread.
#[no_mangle]
pub unsafe extern "C" fn DoShipyard(menu: *mut MENU_STATE) -> BOOLEAN {
    if GlobData.Game_state.CurrentActivity & CHECK_ABORT as u16 != 0 {
        return ExitShipyard(menu);
    }
    let select = PulsedInputState.menu[KEY_MENU_SELECT as usize] != 0;
    let cancel = PulsedInputState.menu[KEY_MENU_CANCEL as usize] != 0;
    SetMenuSounds(SOUND_ARROWS, SOUND_SELECT);
    if (*menu).Initialized == 0 {
        (*menu).InputFunc = Some(DoShipyard);
        (*menu).ModuleFrame =
            CaptureDrawable(LoadGraphicInstance(c"graphics.shipyard".as_ptr()).cast());
        (*menu).CurString =
            CaptureStringTable(LoadStringTableInstance(c"colortable.hangar".as_ptr()));
        (*menu).hMusic = LoadMusicInstance(c"music.shipyard".as_ptr());
        SetTransitionSource(ptr::null());
        BatchGraphics();
        DrawSISFrame();
        DrawSISMessage(
            GetStringAddress(SetAbsStringTableIndex(
                GameStrings,
                STARBASE_STRING_BASE as COUNT + 3,
            ))
            .cast(),
        );
        DrawSISTitle(
            GetStringAddress(SetAbsStringTableIndex(
                GameStrings,
                STARBASE_STRING_BASE as COUNT,
            ))
            .cast(),
        );
        SetContext(SpaceContext);
        DrawBluePrint(menu);
        (*menu).CurState = SHIPYARD_CREW;
        DrawMenuStateStrings(PM_CREW as BYTE, (*menu).CurState as i16);
        SetContext(SpaceContext);
        let mut stamp = STAMP {
            origin: POINT { x: 0, y: 0 },
            frame: SetAbsFrameIndex((*menu).ModuleFrame, 0),
        };
        if USE_3DO_HANGAR {
            DrawStamp(&mut stamp);
        } else {
            let mut old_r: RECT = mem::zeroed();
            GetContextClipRect(&mut old_r);
            let mut r = old_r;
            r.corner.x -= 1;
            r.extent.width += 2;
            r.extent.height += 1;
            SetContextClipRect(&mut r);
            DrawStamp(&mut stamp);
            SetContextClipRect(&mut old_r);
            animatePowerLines(menu);
        }
        SetContextFont(TinyFont);
        ScreenTransition(3, ptr::null());
        UnbatchGraphics();
        PlayMusic((*menu).hMusic, 1, 1);
        ShowCombatShip(menu, COUNT::MAX, ptr::null_mut());
        SetInputCallback(Some(on_input_frame));
        SetFlashRect(MENU_3DO);
        (*menu).Initialized = 1;
    } else if cancel || (select && (*menu).CurState == SHIPYARD_EXIT) {
        return ExitShipyard(menu);
    } else if select {
        if (*menu).CurState != SHIPYARD_SAVELOAD {
            (*menu).Initialized = 0;
            DoModifyShips(menu);
        } else {
            if GameOptions() == 0 {
                return ExitShipyard(menu);
            }
            DrawMenuStateStrings(PM_CREW as BYTE, (*menu).CurState as i16);
            SetFlashRect(MENU_3DO);
        }
    } else {
        DoMenuChooser(menu, PM_CREW as BYTE);
    }
    1
}
