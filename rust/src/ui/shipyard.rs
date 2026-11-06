#![cfg(not(test))]

//! Shipyard menu, crew transactions and hangar animation, ported from shipyard.c.

use crate::ui::rgb15;
mod catalog;
use super::shipyard_hangar as hangar;
use super::sis::game_string;
use catalog::{get_available_race_count, get_available_race_from_index};

use crate::engine::battle::link_queue::LinkId;
use crate::engine::world::queues::{FleetInfo, Fragment};
use crate::engine::{input::InputSession, Kernel, StoryKey, World, WorldData};
use crate::navigation::abi::*;
use crate::starbase::cffi::{avail_race_queue, built_ship_queue};

use std::{mem, ptr};

use crate::front_end::fmv::runtime::DoShipSpin;

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

unsafe fn animate_power_lines(menu: *mut MENU_STATE) {
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

unsafe fn on_input_frame(
    _checkpoint: &mut dyn FnMut(crate::engine::observe::CheckpointView<'_>),
    _input: &crate::engine::input::InputControls,
) {
    let old = SetContext(Kernel::current().read(|owner| owner.space_context));
    animate_power_lines(ptr::null_mut());
    SetContext(old);
}

unsafe fn draw_race_strings(menu: *mut MENU_STATE, mut race: BYTE) {
    let old = SetContext(Kernel::current().read(|owner| owner.status_context));
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
        let Some((index, ship)) = get_available_race_from_index(race) else {
            panic!("the selected hangar race must be allied");
        };
        race = index as BYTE;
        stamp.frame = SetAbsFrameIndex((*menu).ModuleFrame, 3 + race as COUNT);
        DrawStamp(&mut stamp);
        stamp.frame = ship.melee_icon;
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
        SetContextFont(Kernel::current().read(|owner| owner.tiny_font));
        SetContextForeGroundColor(rgb15(0, 0x1f, 0));
        font_DrawText(&mut text);
    }
    UnbatchGraphics();
    SetContext(old);
}

unsafe fn show_ship_crew(id: LinkId<Fragment>, rect: *const RECT) {
    let ship = built_ship_queue(|queue| *queue.get(id));
    let max_crew = avail_race_queue(|queue| {
        let Some(template) = queue.at(usize::from(ship.race_id)) else {
            panic!("a hangar ship must have a fleet template");
        };
        queue.get(template).crew_level
    });
    let buffer = if ship.crew_level >= max_crew {
        format!("{}\0", ship.crew_level)
    } else if ship.crew_level == 0 {
        String::from("SCRAP\0")
    } else {
        format!("{}/{}\0", ship.crew_level, max_crew)
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
    SetContextForeGroundColor(if ship.crew_level != 0 {
        rgb15(0, 0x14, 0)
    } else {
        rgb15(0x12, 0, 0)
    });
    font_DrawText(&mut text);
}

unsafe fn show_combat_ship(
    input: &InputSession,
    menu: *mut MENU_STATE,
    window: COUNT,
    yanked: Option<LinkId<Fragment>>,
) {
    let (mut windows, num_ships) =
        built_ship_queue(|queue| hangar::prepare_windows(queue, &*menu, window, yanked));
    if num_ships != 0 {
        let mut all_finished = false;
        let r = RECT {
            corner: POINT { x: 0, y: 0 },
            extent: EXTENT {
                width: SHIP_WIN_WIDTH,
                height: SHIP_WIN_HEIGHT,
            },
        };
        FlushInput(input);
        let mut time_in = GetTimeCounter();
        let mut j = 0;
        while j < SHIP_WIN_FRAMES && !all_finished {
            SleepThreadUntil(time_in.wrapping_add(ONE_SECOND / 24));
            time_in = GetTimeCounter();
            if AnyButtonPress(input, 0) != 0 {
                if yanked.is_some() {
                    windows[0].left_door.origin.x = 0;
                    windows[0].right_door.origin.x = 0;
                }
                all_finished = true;
            }
            let old = SetContext(Kernel::current().read(|owner| owner.space_context));
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
                let Some(ship) = info.ship else {
                    panic!("a prepared hangar window must have a ship");
                };
                show_ship_crew(ship, &r);
                if !all_finished || yanked.is_some() {
                    DrawStamp(&mut info.left_door);
                    DrawStamp(&mut info.right_door);
                    if yanked.is_some() {
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
                animate_power_lines(ptr::null_mut());
            }
            UnbatchGraphics();
            SetContext(old);
            j += 1;
        }
    }
}

fn purchased_crew() -> SIZE {
    u16::from_le_bytes([
        World::current().read(|world| world.story_state(StoryKey::CREW_PURCHASED0)) as BYTE,
        World::current().read(|world| world.story_state(StoryKey::CREW_PURCHASED1)) as BYTE,
    ]) as SIZE
}

unsafe fn crew_transaction(delta: SIZE) {
    if delta != 0 {
        let mut bought = purchased_crew().wrapping_add(delta);
        if bought < 0 {
            bought = if delta < 0 { 0 } else { 0x7fff };
        } else if delta > 0 {
            if bought as i32 >= CREW_EXPENSE_THRESHOLD as i32
                && (bought as i32 - delta as i32) < CREW_EXPENSE_THRESHOLD as i32
            {
                (**WorldData::current()).game.crew_cost += 2;
                DrawMenuStateStrings(PM_CREW as BYTE, SHIPYARD_CREW as i16);
            }
        } else if (bought as i32) < CREW_EXPENSE_THRESHOLD as i32
            && bought as i32 - delta as i32 >= CREW_EXPENSE_THRESHOLD as i32
        {
            (**WorldData::current()).game.crew_cost -= 2;
            DrawMenuStateStrings(PM_CREW as BYTE, SHIPYARD_CREW as i16);
        }
        World::current().write(|world| {
            if crate::engine::world::fleet::check_alliance(world, SHOFIXTI_SHIP as COUNT)
                != GOOD_GUY as COUNT
            {
                world.set_story_state(StoryKey::CREW_PURCHASED0, bought as BYTE as u32);
                world.set_story_state(StoryKey::CREW_PURCHASED1, ((bought as u16) >> 8) as u32);
            }
        });
    }
}

unsafe fn dms_flash_flag_ship() {
    let r = RECT {
        corner: POINT { x: 0, y: 0 },
        extent: EXTENT {
            width: (ScreenWidth - STATUS_WIDTH as i32 - SAFE_X as i32 * 2 - 14) as i16,
            height: 61,
        },
    };
    SetFlashRect(&r);
}

unsafe fn dms_flash_escort_ship(slot: BYTE) {
    SetFlashRect(&DMS_GetEscortShipRect(slot));
}

unsafe fn dms_flash_flag_ship_crew_count() {
    let mut r: RECT = mem::zeroed();
    SetContext(Kernel::current().read(|owner| owner.status_context));
    GetGaugeRect(&mut r, 1);
    SetFlashRect(&r);
    SetContext(Kernel::current().read(|owner| owner.space_context));
}

unsafe fn dms_flash_escort_ship_crew_count(slot: BYTE) {
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
    SetContext(Kernel::current().read(|owner| owner.space_context));
    SetFlashRect(&r);
}

unsafe fn dms_flash_active_ship(menu: *mut MENU_STATE) {
    if (*menu).CurState >> 4 != 0 {
        dms_flash_flag_ship();
    } else {
        dms_flash_escort_ship((*menu).CurState);
    }
}

unsafe fn dms_set_mode(input: &InputSession, menu: *mut MENU_STATE, mode: ModifyMode) {
    match mode {
        ModifyMode::Navigate => {
            SetMenuSounds(input, SOUND_ARROWS, SOUND_SELECT);
            dms_flash_active_ship(menu);
        }
        ModifyMode::AddEscort => {
            SetMenuSounds(input, SOUND_ARROWS, SOUND_SELECT);
            SetFlashRect(MENU_ANY);
        }
        ModifyMode::EditCrew => {
            SetMenuSounds(input, SOUND_UP | SOUND_DOWN, SOUND_SELECT | SOUND_CANCEL);
            if (*menu).CurState >> 4 != 0 {
                dms_flash_flag_ship_crew_count();
            } else {
                dms_flash_escort_ship_crew_count((*menu).CurState);
            }
        }
        ModifyMode::Exit => {
            SetMenuSounds(input, SOUND_ARROWS, SOUND_SELECT);
            SetFlashRect(MENU_3DO);
        }
    }
}

unsafe fn dms_spin_ship(menu: *mut MENU_STATE, ship: Option<LinkId<Fragment>>) -> bool {
    if (*menu).CurState >> 4 != 0 {
        return false;
    }
    if ship.is_none() == ((*menu).delta_item & MODIFY_CREW_FLAG == 0) {
        return false;
    }
    let spin_ship = if let Some(id) = ship {
        let race = built_ship_queue(|queue| queue.get(id).race_id);
        avail_race_queue(|queue| queue.at(usize::from(race)).map(|id| *queue.get(id)))
    } else {
        get_available_race_from_index((*menu).delta_item as BYTE).map(|(_, fleet)| fleet)
    };
    let Some(spin_ship) = spin_ship else {
        return false;
    };
    SetFlashRect(ptr::null());
    use crate::ships::master_native::find;
    let species = spin_ship.species_id;
    let (context, index) =
        Kernel::current().read(|owner| (owner.screen_context, find(&owner.master_queue, species)));
    let old = SetContext(context);
    let mut old_clip: RECT = mem::zeroed();
    GetContextClipRect(&mut old_clip);
    if index >= 0 && index < (MeleeShip_MELEE_ZOQFOTPIK + 1) as i32 {
        DoShipSpin(index as COUNT, (*menu).hMusic);
    }
    SetContextClipRect(&mut old_clip);
    SetContext(old);
    true
}

unsafe fn dms_hire_escort_ship_crew(id: LinkId<Fragment>) -> SIZE {
    let world = WorldData::current().read(|world| *world);
    let ship = built_ship_queue(|queue| *queue.get(id));
    let max_crew = avail_race_queue(|queue| {
        let Some(template) = queue.at(usize::from(ship.race_id)) else {
            panic!("a hangar ship must have a fleet template");
        };
        queue.get(template).crew_level
    });
    if world.flagship.res_units < world.game.crew_cost as DWORD {
        return 0;
    }
    if ship.crew_level >= ship.max_crew || ship.crew_level >= max_crew {
        return 0;
    }
    if ship.crew_level > 0 {
        DeltaSISGauges(0, 0, -(world.game.crew_cost as i32));
    } else {
        DeltaSISGauges(0, 0, -(SHIP_COST[ship.race_id as usize] as i32));
    }
    built_ship_queue(|queue| queue.get_mut(id).crew_level += 1);
    pre_update_flash_rect();
    let slot = built_ship_queue(|queue| queue.get(id).index);
    let r = DMS_GetEscortShipRect(slot);
    show_ship_crew(id, &r);
    post_update_flash_rect();
    1
}

unsafe fn dms_dismiss_escort_ship_crew(id: LinkId<Fragment>) -> SIZE {
    let ship = built_ship_queue(|queue| *queue.get(id));
    let mut delta = 0;
    if ship.crew_level > 0 {
        if ship.crew_level > 1 {
            let bought = purchased_crew();
            DeltaSISGauges(
                0,
                0,
                WorldData::current().read(|owner| owner.game.crew_cost) as i32
                    - if bought == CREW_EXPENSE_THRESHOLD as SIZE {
                        2
                    } else {
                        0
                    },
            );
        } else {
            DeltaSISGauges(0, 0, SHIP_COST[ship.race_id as usize] as i32);
        }
        delta = -1;
        built_ship_queue(|queue| queue.get_mut(id).crew_level -= 1);
    } else {
        PlayMenuSound(MENU_SOUND_EFFECT_MENU_SOUND_FAILURE);
    }
    pre_update_flash_rect();
    let slot = built_ship_queue(|queue| queue.get(id).index);
    let r = DMS_GetEscortShipRect(slot);
    show_ship_crew(id, &r);
    post_update_flash_rect();
    delta
}

unsafe fn dms_modify_crew(
    input: &InputSession,
    menu: *mut MENU_STATE,
    handle: Option<LinkId<Fragment>>,
    dy: SBYTE,
) {
    let delta = match handle {
        None => {
            let delta = if dy < 0 {
                dms_hire_flag_ship_crew(input)
            } else {
                dms_dismiss_flag_ship_crew(input)
            };
            if delta != 0 {
                dms_flash_flag_ship_crew_count();
            }
            delta
        }
        Some(ship) => {
            let delta = if dy < 0 {
                dms_hire_escort_ship_crew(ship)
            } else {
                dms_dismiss_escort_ship_crew(ship)
            };
            if delta != 0 {
                dms_flash_escort_ship_crew_count(built_ship_queue(|queue| queue.get(ship).index));
            }
            delta
        }
    };
    if delta == 0 {
        PlayMenuSound(MENU_SOUND_EFFECT_MENU_SOUND_FAILURE);
    }
    if handle.is_some() {
        (*menu).delta_item &= MODIFY_CREW_FLAG;
    }
    crew_transaction(delta);
}

unsafe fn dms_try_add_escort_ship(input: &InputSession, menu: *mut MENU_STATE) {
    let Some((index, template)) = get_available_race_from_index((*menu).delta_item as BYTE) else {
        panic!("the purchased hangar race must be allied");
    };
    let added = WorldData::current().read(|world| world.flagship.res_units)
        >= SHIP_COST[index as usize] as DWORD
        && built_ship_queue(|queue| {
            queue
                .clone_fragment(
                    &template,
                    index as u8,
                    1,
                    crate::comm::dispatch::production_captain_name,
                )
                .is_some()
        });
    if added {
        show_combat_ship(input, menu, (*menu).CurState as COUNT, None);
        DrawMenuStateStrings(PM_CREW as BYTE, SHIPYARD_CREW as i16);
        DeltaSISGauges(
            UNDEFINED_DELTA as SIZE,
            UNDEFINED_DELTA as SIZE,
            -(SHIP_COST[index as usize] as i32),
        );
        dms_set_mode(input, menu, ModifyMode::EditCrew);
    } else {
        PlayMenuSound(MENU_SOUND_EFFECT_MENU_SOUND_FAILURE);
    }
}

unsafe fn dms_add_escort_ship(
    input: &InputSession,
    menu: *mut MENU_STATE,
    special: bool,
    select: bool,
    cancel: bool,
    dx: SBYTE,
    dy: SBYTE,
) {
    assert!((*menu).delta_item & MODIFY_CREW_FLAG != 0);
    if special {
        let ship = built_ship_queue(|queue| {
            queue.find(|ship| COUNT::from(ship.index) == (*menu).delta_item as COUNT)
        });
        if dms_spin_ship(menu, ship) {
            dms_set_mode(input, menu, ModifyMode::AddEscort);
        }
        return;
    }
    if cancel {
        (*menu).delta_item &= !MODIFY_CREW_FLAG;
        SetFlashRect(ptr::null());
        DrawMenuStateStrings(PM_CREW as BYTE, SHIPYARD_CREW as i16);
        dms_set_mode(input, menu, ModifyMode::Navigate);
    } else if select {
        dms_try_add_escort_ship(input, menu);
    } else if dx != 0 || dy != 0 {
        let available = get_available_race_count();
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
            pre_update_flash_rect();
            draw_race_strings(menu, current);
            post_update_flash_rect();
            (*menu).delta_item = current as SIZE | MODIFY_CREW_FLAG;
        }
    }
}

unsafe fn dms_scrap_escort_ship(
    input: &InputSession,
    menu: *mut MENU_STATE,
    handle: LinkId<Fragment>,
) {
    SetFlashRect(ptr::null());
    show_combat_ship(input, menu, (*menu).CurState as COUNT, Some(handle));
    built_ship_queue(|queue| {
        queue.remove(handle);
        queue.free(handle);
    });
    DeltaSISGauges(
        UNDEFINED_DELTA as SIZE,
        UNDEFINED_DELTA as SIZE,
        UNDEFINED_DELTA as i32,
    );
    SetContext(Kernel::current().read(|owner| owner.space_context));
    dms_set_mode(input, menu, ModifyMode::Navigate);
}

unsafe fn dms_edit_crew_mode(
    input: &InputSession,
    menu: *mut MENU_STATE,
    ship: Option<LinkId<Fragment>>,
    select: bool,
    cancel: bool,
    dy: SBYTE,
) {
    if select || cancel {
        if let Some(ship) = ship {
            let crew = built_ship_queue(|queue| queue.get(ship).crew_level);
            if crew == 0 {
                dms_scrap_escort_ship(input, menu, ship);
            }
        }
        (*menu).delta_item &= !MODIFY_CREW_FLAG;
        dms_set_mode(input, menu, ModifyMode::Navigate);
    } else if dy != 0 {
        dms_modify_crew(input, menu, ship, dy);
    }
}

unsafe fn dms_navigate_ship_slots(
    input: &InputSession,
    menu: *mut MENU_STATE,
    special: bool,
    select: bool,
    cancel: bool,
    dx: SBYTE,
    dy: SBYTE,
) {
    let ship = built_ship_queue(|queue| {
        queue.find(|ship| COUNT::from(ship.index) == (*menu).CurState as COUNT)
    });
    if dx != 0 || dy != 0 {
        let new_state = DMS_MoveCursor((*menu).CurState, dx, dy);
        if new_state != (*menu).CurState {
            (*menu).CurState = new_state;
            dms_flash_active_ship(menu);
        }
    }
    if special {
        if dms_spin_ship(menu, ship) {
            dms_set_mode(input, menu, ModifyMode::Navigate);
        }
    } else if select {
        if ship.is_none() && (*menu).CurState >> 4 == 0 {
            (*menu).delta_item = MODIFY_CREW_FLAG;
            draw_race_strings(menu, 0);
            dms_set_mode(input, menu, ModifyMode::AddEscort);
        } else {
            (*menu).delta_item |= MODIFY_CREW_FLAG;
            dms_set_mode(input, menu, ModifyMode::EditCrew);
        }
    } else if cancel {
        (*menu).InputFunc = Some(do_shipyard);
        (*menu).CurState = SHIPYARD_CREW;
        DrawMenuStateStrings(PM_CREW as BYTE, (*menu).CurState as i16);
        dms_set_mode(input, menu, ModifyMode::Exit);
    }
}

unsafe fn do_modify_ships(menu: *mut MENU_STATE, input: &InputSession) -> BOOLEAN {
    if crate::engine::World::current().read(|world| world.activity()) & CHECK_ABORT as u16 != 0 {
        (*menu).InputFunc = Some(do_shipyard);
        return 1;
    }
    if (*menu).Initialized == 0 {
        (*menu).InputFunc = Some(do_modify_ships);
        (*menu).Initialized = 1;
        (*menu).CurState = 0xf0;
        (*menu).delta_item = 0;
        SetContext(Kernel::current().read(|owner| owner.space_context));
        dms_set_mode(input, menu, ModifyMode::Navigate);
    } else {
        let special = input.pulsed().menu[KEY_MENU_SPECIAL as usize] != 0;
        let select = input.pulsed().menu[KEY_MENU_SELECT as usize] != 0;
        let cancel = input.pulsed().menu[KEY_MENU_CANCEL as usize] != 0;
        let mut dx = 0;
        let mut dy = 0;
        if input.pulsed().menu[KEY_MENU_RIGHT as usize] != 0 {
            dx = 1;
        }
        if input.pulsed().menu[KEY_MENU_LEFT as usize] != 0 {
            dx = -1;
        }
        if input.pulsed().menu[KEY_MENU_UP as usize] != 0 {
            dy = -1;
        }
        if input.pulsed().menu[KEY_MENU_DOWN as usize] != 0 {
            dy = 1;
        }
        if (*menu).delta_item & MODIFY_CREW_FLAG == 0 {
            dms_navigate_ship_slots(input, menu, special, select, cancel, dx, dy);
        } else {
            let ship = built_ship_queue(|queue| {
                queue.find(|ship| COUNT::from(ship.index) == (*menu).CurState as COUNT)
            });
            if ship.is_none() && (*menu).CurState >> 4 == 0 {
                dms_add_escort_ship(input, menu, special, select, cancel, dx, dy);
            } else {
                dms_edit_crew_mode(input, menu, ship, select, cancel, dy);
            }
        }
    }
    SleepThread(ONE_SECOND / 30);
    1
}

unsafe fn draw_blue_print(input: &InputSession, menu: *mut MENU_STATE) {
    let frame = CaptureDrawable(LoadGraphicInstance(c"graphics.blueprints".as_ptr()).cast());
    let mut stamp = STAMP {
        origin: POINT { x: 0, y: 0 },
        frame: DecFrameIndex(frame),
    };
    SetContextForeGroundColor(rgb15(0, 0, 0x16));
    DrawFilledStamp(&mut stamp);
    for slot in 0..NUM_DRIVE_SLOTS {
        let piece = WorldData::current().read(|owner| owner.flagship.drive_slots[slot as usize]);
        DrawShipPiece(frame, piece as COUNT, slot as COUNT, 1);
    }
    for slot in 0..NUM_JET_SLOTS {
        let piece = WorldData::current().read(|owner| owner.flagship.jet_slots[slot as usize]);
        DrawShipPiece(frame, piece as COUNT, slot as COUNT, 1);
    }
    for slot in 0..NUM_MODULE_SLOTS {
        let piece = WorldData::current().read(|owner| owner.flagship.module_slots[slot as usize]);
        if !((*menu).CurState as u32 == SHIPYARD && piece as u32 == CREW_POD) {
            DrawShipPiece(frame, piece as COUNT, slot as COUNT, 1);
        }
    }
    SetContextForeGroundColor(rgb15(0x0a, 0x0a, 0x1f));
    for slot in 0..NUM_MODULE_SLOTS {
        let piece = WorldData::current().read(|owner| owner.flagship.module_slots[slot as usize]);
        if (*menu).CurState as u32 == SHIPYARD && piece as u32 == CREW_POD {
            DrawShipPiece(frame, piece as COUNT, slot as COUNT, 1);
        }
    }
    let mut frames = WorldData::current().read(|owner| owner.flagship.crew_enlisted);
    (**WorldData::current()).flagship.crew_enlisted = 0;
    loop {
        let old = frames;
        frames = frames.wrapping_sub(1);
        if old == 0 {
            break;
        }
        let mut point: POINT = mem::zeroed();
        get_c_pod_capacity(input.preferences().fonts == OPT_PC as i32, &mut point);
        DrawPoint(&mut point);
        (**WorldData::current()).flagship.crew_enlisted += 1;
    }
    frames = WorldData::current().read(|owner| owner.flagship.total_element_mass);
    (**WorldData::current()).flagship.total_element_mass = 0;
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
        (**WorldData::current()).flagship.total_element_mass += mass;
        GetSBayCapacity(&mut r.corner);
        DrawFilledRectangle(&mut r);
        frames -= mass;
    }
    if WorldData::current().read(|owner| owner.flagship.fuel_on_board) > FUEL_RESERVE {
        let mut volume =
            WorldData::current().read(|owner| owner.flagship.fuel_on_board) - FUEL_RESERVE;
        (**WorldData::current()).flagship.fuel_on_board = FUEL_RESERVE;
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
            (**WorldData::current()).flagship.fuel_on_board += mass as DWORD;
            volume -= mass as DWORD;
        }
    }
    DestroyDrawable(ReleaseDrawable(frame));
}

unsafe fn exit_shipyard(input: &InputSession, menu: *mut MENU_STATE) -> BOOLEAN {
    SetInputCallback(input, None);
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
pub unsafe fn do_shipyard(menu: *mut MENU_STATE, input: &InputSession) -> BOOLEAN {
    if crate::engine::World::current().read(|world| world.activity()) & CHECK_ABORT as u16 != 0 {
        return exit_shipyard(input, menu);
    }
    let select = input.pulsed().menu[KEY_MENU_SELECT as usize] != 0;
    let cancel = input.pulsed().menu[KEY_MENU_CANCEL as usize] != 0;
    SetMenuSounds(input, SOUND_ARROWS, SOUND_SELECT);
    if (*menu).Initialized == 0 {
        (*menu).InputFunc = Some(do_shipyard);
        (*menu).ModuleFrame =
            CaptureDrawable(LoadGraphicInstance(c"graphics.shipyard".as_ptr()).cast());
        (*menu).CurString =
            CaptureStringTable(LoadStringTableInstance(c"colortable.hangar".as_ptr()));
        (*menu).hMusic = (LoadMusicInstance(c"music.shipyard".as_ptr())).cast();
        SetTransitionSource(ptr::null());
        BatchGraphics();
        draw_sisframe();
        DrawSISMessage(game_string(STARBASE_STRING_BASE + 3));
        DrawSISTitle(game_string(STARBASE_STRING_BASE));
        SetContext(Kernel::current().read(|owner| owner.space_context));
        draw_blue_print(input, menu);
        (*menu).CurState = SHIPYARD_CREW;
        DrawMenuStateStrings(PM_CREW as BYTE, (*menu).CurState as i16);
        SetContext(Kernel::current().read(|owner| owner.space_context));
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
            animate_power_lines(menu);
        }
        SetContextFont(Kernel::current().read(|owner| owner.tiny_font));
        ScreenTransition(3, ptr::null());
        UnbatchGraphics();
        PlayMusic((*menu).hMusic, 1, 1);
        show_combat_ship(input, menu, COUNT::MAX, None);
        SetInputCallback(input, Some(on_input_frame));
        SetFlashRect(MENU_3DO);
        (*menu).Initialized = 1;
    } else if cancel || (select && (*menu).CurState == SHIPYARD_EXIT) {
        return exit_shipyard(input, menu);
    } else if select {
        if (*menu).CurState != SHIPYARD_SAVELOAD {
            (*menu).Initialized = 0;
            do_modify_ships(menu, input);
        } else {
            if GameOptions() == 0 {
                return exit_shipyard(input, menu);
            }
            DrawMenuStateStrings(PM_CREW as BYTE, (*menu).CurState as i16);
            SetFlashRect(MENU_3DO);
        }
    } else {
        do_menu_chooser(input, menu, PM_CREW as BYTE);
    }
    1
}

unsafe fn dms_hire_flag_ship_crew(input: &InputSession) -> SIZE {
    let mut point: POINT = mem::zeroed();
    if get_c_pod_capacity(input.preferences().fonts == OPT_PC as i32, &mut point) <= GetCrewCount()
    {
        return 0;
    }
    if WorldData::current().read(|owner| owner.flagship.res_units)
        < WorldData::current().read(|owner| owner.game.crew_cost) as DWORD
    {
        return 0;
    }
    DrawPoint(&mut point);
    pre_update_flash_rect();
    DeltaSISGauges(
        1,
        0,
        -(WorldData::current().read(|owner| owner.game.crew_cost) as i32),
    );
    post_update_flash_rect();
    1
}

unsafe fn dms_dismiss_flag_ship_crew(input: &InputSession) -> SIZE {
    if GetCrewCount() == 0 {
        return 0;
    }
    let bought = purchased_crew();
    pre_update_flash_rect();
    DeltaSISGauges(
        -1,
        0,
        WorldData::current().read(|owner| owner.game.crew_cost) as i32
            - if bought == CREW_EXPENSE_THRESHOLD as SIZE {
                2
            } else {
                0
            },
    );
    post_update_flash_rect();
    let mut point: POINT = mem::zeroed();
    get_c_pod_capacity(input.preferences().fonts == OPT_PC as i32, &mut point);
    SetContextForeGroundColor(BLACK);
    DrawPoint(&mut point);
    -1
}
