//! Outfit installation, fuel transactions and menu callbacks, ported from outfit.c.
#![cfg(not(test))]
#![allow(non_snake_case)]

use crate::navigation::abi::*;
use crate::navigation::globals::GlobData;
use crate::state::game_state_keys::get_game_state;
use std::{mem, ptr};

const OUTFIT_FUEL: BYTE = 0;
const OUTFIT_MODULES: BYTE = 1;
const OUTFIT_SAVELOAD: BYTE = 2;
const OUTFIT_EXIT: BYTE = 3;
const OUTFIT_DOFUEL: BYTE = 4;
const EMPTY_SLOT: BYTE = NUM_MODULES as BYTE;
const LANDER_X: i16 = 24;
const LANDER_Y: i16 = 67;
const LANDER_WIDTH: i16 = 15;
const MENU_3DO: *const RECT = usize::MAX as *const RECT;
const MENU_ANY: *const RECT = (usize::MAX - 1) as *const RECT;
const SOUND_ARROWS: MENU_SOUND_FLAGS = 0x0f;
const SOUND_SELECT: MENU_SOUND_FLAGS = 1 << 4;
const SOUND_CANCEL: MENU_SOUND_FLAGS = 1 << 5;
const SOUND_FUEL: MENU_SOUND_FLAGS = (1 << 0) | (1 << 1) | (1 << 7) | (1 << 8);

const fn rgb15(r: u8, g: u8, b: u8) -> Color {
    Color {
        r: (r << 3) | (r >> 2),
        g: (g << 3) | (g >> 2),
        b: (b << 3) | (b >> 2),
        a: 255,
    }
}
const BLACK: Color = rgb15(0, 0, 0);

unsafe fn game_string(index: u32) -> *mut UNICODE {
    GetStringAddress(SetAbsStringTableIndex(GameStrings, index as COUNT)).cast()
}

unsafe fn DrawModuleStrings(menu: *mut MENU_STATE, module: BYTE) {
    let old = SetContext(StatusContext);
    let mut r: RECT = mem::zeroed();
    GetContextClipRect(&mut r);
    let mut s = STAMP {
        origin: POINT {
            x: (4 + ScreenWidth - STATUS_WIDTH as i32 - SAFE_X as i32) as i16 - r.corner.x,
            y: (SIS_ORG_Y as i32 + ScreenHeight - SAFE_Y as i32 * 2 - 13 - RADAR_HEIGHT as i32)
                as i16
                - r.corner.y,
        },
        frame: ptr::null_mut(),
    };
    r.corner.x = s.origin.x - 1;
    r.corner.y = s.origin.y - 11;
    r.extent.width = RADAR_WIDTH as i16 + 2;
    r.extent.height = 11;
    BatchGraphics();
    SetContextForeGroundColor(rgb15(0x0a, 0x0a, 0x0a));
    DrawFilledRectangle(&mut r);
    if module >= EMPTY_SLOT {
        r.corner = s.origin;
        r.extent.width = RADAR_WIDTH as i16;
        r.extent.height = RADAR_HEIGHT as i16;
        SetContextForeGroundColor(BLACK);
        DrawFilledRectangle(&mut r);
    } else if !(*menu).CurFrame.is_null() {
        s.frame = SetAbsFrameIndex((*menu).CurFrame, module as COUNT);
        DrawStamp(&mut s);
        let buffer = format!(
            "{}\0",
            GlobData.Game_state.ModuleCost[module as usize] as u32 * MODULE_COST_SCALE
        );
        let mut text = TEXT {
            baseline: POINT {
                x: s.origin.x + RADAR_WIDTH as i16 - 2,
                y: s.origin.y + RADAR_HEIGHT as i16 - 2,
            },
            align: TEXT_ALIGN_ALIGN_RIGHT,
            CharCount: COUNT::MAX,
            pStr: buffer.as_ptr().cast(),
        };
        SetContextFont(TinyFont);
        SetContextForeGroundColor(rgb15(0, 0x1f, 0));
        font_DrawText(&mut text);
    }
    UnbatchGraphics();
    SetContext(old);
}

unsafe fn RedistributeFuel() {
    let volume = GlobData.SIS_state.FuelOnBoard;
    let old = SetContext(SpaceContext);
    let mut r: RECT = mem::zeroed();
    r.extent.height = 1;
    BatchGraphics();
    GlobData.SIS_state.FuelOnBoard = FUEL_RESERVE;
    while GlobData.SIS_state.FuelOnBoard < GetFTankCapacity(&mut r.corner) {
        if GlobData.SIS_state.FuelOnBoard < volume {
            r.extent.width = 3;
            DrawPoint(&mut r.corner);
            r.corner.x += r.extent.width + 1;
            DrawPoint(&mut r.corner);
            r.corner.x -= r.extent.width;
            let background = SetContextBackGroundColor(BLACK);
            SetContextForeGroundColor(background);
        } else {
            r.extent.width = 5;
            SetContextForeGroundColor(rgb15(0x0b, 0, 0));
        }
        DrawFilledRectangle(&mut r);
        GlobData.SIS_state.FuelOnBoard += FUEL_VOLUME_PER_ROW;
    }
    UnbatchGraphics();
    SetContext(old);
    GlobData.SIS_state.FuelOnBoard = volume;
}

unsafe fn DisplayLanders(menu: *mut MENU_STATE) {
    let mut s = STAMP {
        origin: POINT { x: 0, y: 0 },
        frame: (*menu).ModuleFrame,
    };
    if get_game_state("CHMMR_BOMB_STATE") == 3 {
        s.frame = DecFrameIndex(s.frame);
        DrawStamp(&mut s);
    } else {
        s.origin = POINT {
            x: LANDER_X,
            y: LANDER_Y,
        };
        let mut i = 0;
        while i < GlobData.SIS_state.NumLanders as COUNT {
            DrawStamp(&mut s);
            s.origin.x += LANDER_WIDTH;
            i += 1;
        }
        SetContextForeGroundColor(BLACK);
        while i < MAX_LANDERS as COUNT {
            DrawFilledStamp(&mut s);
            s.origin.x += LANDER_WIDTH;
            i += 1;
        }
    }
}

unsafe fn SlotPiece(state: BYTE, item: SIZE) -> BYTE {
    if state == PLANET_LANDER as BYTE || state == EMPTY_SLOT + 3 {
        if item < GlobData.SIS_state.NumLanders as SIZE {
            PLANET_LANDER as BYTE
        } else {
            EMPTY_SLOT + 3
        }
    } else if state == FUSION_THRUSTER as BYTE || state == EMPTY_SLOT {
        GlobData.SIS_state.DriveSlots[item as usize]
    } else if state == TURNING_JETS as BYTE || state == EMPTY_SLOT + 1 {
        GlobData.SIS_state.JetSlots[item as usize]
    } else {
        GlobData.SIS_state.ModuleSlots[item as usize]
    }
}

// The C InitFlash label is entered both after a transaction and after moving
// between slot classes. Horizontal movement instead offsets the existing rect.
unsafe fn InitFlash(menu: *mut MENU_STATE, piece: BYTE) {
    let state = (*menu).CurState;
    let mut width = SHIP_PIECE_OFFSET as SIZE;
    let rect = &mut (*menu).flash_rect0;
    if state == PLANET_LANDER as BYTE || state == EMPTY_SLOT + 3 {
        *rect = RECT {
            corner: POINT {
                x: LANDER_X - 1,
                y: LANDER_Y - 1,
            },
            extent: EXTENT {
                width: 13,
                height: 15,
            },
        };
        width = LANDER_WIDTH;
    } else if state == FUSION_THRUSTER as BYTE || state == EMPTY_SLOT {
        *rect = RECT {
            corner: POINT {
                x: DRIVE_TOP_X as i16 - 1,
                y: DRIVE_TOP_Y as i16 - 1,
            },
            extent: EXTENT {
                width: 8,
                height: 6,
            },
        };
    } else if state == TURNING_JETS as BYTE || state == EMPTY_SLOT + 1 {
        *rect = RECT {
            corner: POINT {
                x: JET_TOP_X as i16 - 1,
                y: JET_TOP_Y as i16 - 1,
            },
            extent: EXTENT {
                width: 9,
                height: 10,
            },
        };
    } else {
        *rect = RECT {
            corner: POINT {
                x: MODULE_TOP_X as i16 - 1,
                y: MODULE_TOP_Y as i16 - 1,
            },
            extent: EXTENT {
                width: SHIP_PIECE_OFFSET as i16 + 2,
                height: 34,
            },
        };
    }
    rect.corner.x += width * (*menu).delta_item;
    DrawModuleStrings(menu, piece);
    if state < EMPTY_SLOT {
        SetFlashRect(MENU_ANY);
    } else {
        SetFlashRect(ptr::addr_of!((*menu).flash_rect0));
    }
}

unsafe extern "C" fn DoInstallModule(menu: *mut MENU_STATE) -> BOOLEAN {
    if GlobData.Game_state.CurrentActivity & CHECK_ABORT as u16 != 0 {
        (*menu).InputFunc = Some(DoOutfit);
        return 1;
    }
    let select = PulsedInputState.menu[KEY_MENU_SELECT as usize] != 0;
    let mut cancel = PulsedInputState.menu[KEY_MENU_CANCEL as usize] != 0;
    let left = PulsedInputState.menu[KEY_MENU_LEFT as usize] != 0;
    let right = PulsedInputState.menu[KEY_MENU_RIGHT as usize] != 0;
    let up = PulsedInputState.menu[KEY_MENU_UP as usize] != 0;
    let down = PulsedInputState.menu[KEY_MENU_DOWN as usize] != 0;
    SetMenuSounds(SOUND_ARROWS, SOUND_SELECT);
    let mut first: SIZE = 0;
    let mut state = (*menu).CurState;
    let old_piece = SlotPiece(state, (*menu).delta_item);
    let mut last = if state == PLANET_LANDER as BYTE || state == EMPTY_SLOT + 3 {
        MAX_LANDERS as SIZE - 1
    } else if state == FUSION_THRUSTER as BYTE || state == EMPTY_SLOT {
        NUM_DRIVE_SLOTS as SIZE - 1
    } else if state == TURNING_JETS as BYTE || state == EMPTY_SLOT + 1 {
        NUM_JET_SLOTS as SIZE - 1
    } else {
        if get_game_state("CHMMR_BOMB_STATE") == 3 {
            first = NUM_BOMB_MODULES as SIZE;
        }
        NUM_MODULE_SLOTS as SIZE - 1
    };
    if state < CREW_POD as BYTE {
        first = state as SIZE;
        last = state as SIZE;
    } else if state < EMPTY_SLOT {
        first = CREW_POD as SIZE;
        last = NUM_PURCHASE_MODULES as SIZE - 1;
    }
    if (*menu).Initialized == 0 {
        (*menu).Initialized = 1;
        (*menu).InputFunc = Some(DoInstallModule);
        SetContext(SpaceContext);
        ClearSISRect(CLEAR_SIS_RADAR as BYTE);
        SetFlashRect(ptr::null());
        InitFlash(menu, old_piece);
    } else if select || cancel {
        let mut new_piece = (*menu).CurState;
        if select {
            if new_piece < EMPTY_SLOT {
                if GlobData.SIS_state.ResUnits
                    < GlobData.Game_state.ModuleCost[new_piece as usize] as u32 * MODULE_COST_SCALE
                {
                    PlayMenuSound(MENU_SOUND_EFFECT_MENU_SOUND_FAILURE);
                    return 1;
                }
            } else if new_piece == EMPTY_SLOT + 2 {
                if old_piece == CREW_POD as BYTE {
                    if GlobData.SIS_state.CrewEnlisted as i32
                        > CREW_POD_CAPACITY as i32 * (CountSISPieces(CREW_POD as BYTE) as i32 - 1)
                    {
                        PlayMenuSound(MENU_SOUND_EFFECT_MENU_SOUND_FAILURE);
                        return 1;
                    }
                } else if old_piece == FUEL_TANK as BYTE || old_piece == HIGHEFF_FUELSYS as BYTE {
                    let mut volume = CountSISPieces(FUEL_TANK as BYTE) as DWORD
                        * FUEL_TANK_CAPACITY
                        + CountSISPieces(HIGHEFF_FUELSYS as BYTE) as DWORD * HEFUEL_TANK_CAPACITY;
                    volume -= if old_piece == FUEL_TANK as BYTE {
                        FUEL_TANK_CAPACITY
                    } else {
                        HEFUEL_TANK_CAPACITY
                    };
                    if GlobData.SIS_state.FuelOnBoard > volume + FUEL_RESERVE {
                        PlayMenuSound(MENU_SOUND_EFFECT_MENU_SOUND_FAILURE);
                        return 1;
                    }
                } else if old_piece == STORAGE_BAY as BYTE
                    && GlobData.SIS_state.TotalElementMass as i32
                        > STORAGE_BAY_CAPACITY as i32
                            * (CountSISPieces(STORAGE_BAY as BYTE) as i32 - 1)
                {
                    PlayMenuSound(MENU_SOUND_EFFECT_MENU_SOUND_FAILURE);
                    return 1;
                }
            }
        }
        SetContext(SpaceContext);
        SetFlashRect(ptr::null());
        if select {
            if new_piece >= EMPTY_SLOT && old_piece >= EMPTY_SLOT {
                new_piece -= EMPTY_SLOT - 1;
                if new_piece > CREW_POD as BYTE {
                    new_piece = PLANET_LANDER as BYTE;
                }
            } else {
                match (*menu).CurState as u32 {
                    PLANET_LANDER => GlobData.SIS_state.NumLanders += 1,
                    s if s == (EMPTY_SLOT + 3) as u32 => GlobData.SIS_state.NumLanders -= 1,
                    FUSION_THRUSTER => {
                        GlobData.SIS_state.DriveSlots[(*menu).delta_item as usize] = new_piece
                    }
                    s if s == EMPTY_SLOT as u32 => {
                        GlobData.SIS_state.DriveSlots[(*menu).delta_item as usize] = new_piece
                    }
                    TURNING_JETS => {
                        GlobData.SIS_state.JetSlots[(*menu).delta_item as usize] = new_piece
                    }
                    s if s == (EMPTY_SLOT + 1) as u32 => {
                        GlobData.SIS_state.JetSlots[(*menu).delta_item as usize] = new_piece
                    }
                    _ => GlobData.SIS_state.ModuleSlots[(*menu).delta_item as usize] = new_piece,
                }
                let cost = if new_piece < EMPTY_SLOT {
                    -(GlobData.Game_state.ModuleCost[new_piece as usize] as i32
                        * MODULE_COST_SCALE as i32)
                } else {
                    GlobData.Game_state.ModuleCost[old_piece as usize] as i32
                        * MODULE_COST_SCALE as i32
                };
                DeltaSISGauges(UNDEFINED_DELTA as SIZE, UNDEFINED_DELTA as SIZE, cost);
                if (*menu).CurState == PLANET_LANDER as BYTE || (*menu).CurState == EMPTY_SLOT + 3 {
                    DisplayLanders(menu);
                } else {
                    DrawShipPiece(
                        (*menu).ModuleFrame,
                        new_piece as COUNT,
                        (*menu).delta_item as COUNT,
                        0,
                    );
                    if new_piece > TURNING_JETS as BYTE && old_piece > TURNING_JETS as BYTE {
                        RedistributeFuel();
                    }
                    if optWhichFonts == OPT_PC as i32 {
                        DrawFlagshipStats();
                    }
                }
            }
            cancel = false;
        }
        if (*menu).CurState < EMPTY_SLOT {
            (*menu).CurState += EMPTY_SLOT - 1;
            if (*menu).CurState < EMPTY_SLOT {
                (*menu).CurState = EMPTY_SLOT + 3;
            } else if (*menu).CurState > EMPTY_SLOT + 2 {
                (*menu).CurState = EMPTY_SLOT + 2;
            }
            if cancel {
                new_piece = (*menu).CurState;
            }
            InitFlash(menu, new_piece);
        } else if !cancel {
            (*menu).CurState = new_piece;
            InitFlash(menu, new_piece);
        } else {
            SetContext(StatusContext);
            (*menu).CurState = OUTFIT_MODULES;
            DrawMenuStateStrings(PM_FUEL as BYTE, (*menu).CurState as i16);
            SetFlashRect(MENU_3DO);
            (*menu).InputFunc = Some(DoOutfit);
            ClearSISRect(DRAW_SIS_DISPLAY as BYTE);
        }
    } else if left || right || up || down {
        let mut item = if state < EMPTY_SLOT {
            (*menu).CurState as SIZE
        } else {
            (*menu).delta_item
        };
        loop {
            if state >= EMPTY_SLOT && (up || down) {
                if up {
                    let old = state;
                    state = state.wrapping_sub(1);
                    if old == EMPTY_SLOT {
                        state = EMPTY_SLOT + 3;
                    }
                } else {
                    let old = state;
                    state = state.wrapping_add(1);
                    if old == EMPTY_SLOT + 3 {
                        state = EMPTY_SLOT;
                    }
                }
                item = 0;
                if get_game_state("CHMMR_BOMB_STATE") == 3 {
                    if state == EMPTY_SLOT + 3 {
                        state = if up { EMPTY_SLOT + 2 } else { EMPTY_SLOT };
                    }
                    if state == EMPTY_SLOT + 2 {
                        item = NUM_BOMB_MODULES as SIZE;
                    }
                }
                (*menu).delta_item = item;
            } else if left || up {
                let old = item;
                item = item.wrapping_sub(1);
                if old == first {
                    item = last;
                }
            } else if right || down {
                let old = item;
                item = item.wrapping_add(1);
                if old == last {
                    item = first;
                }
            }
            if !(state < EMPTY_SLOT
                && (GlobData.Game_state.ModuleCost[item as usize] == 0
                    || (item >= GUN_WEAPON as SIZE
                        && item <= CANNON_WEAPON as SIZE
                        && (*menu).delta_item > 0
                        && (*menu).delta_item < 13)))
            {
                break;
            }
        }
        if state < EMPTY_SLOT {
            if item != (*menu).CurState as SIZE {
                (*menu).CurState = item as BYTE;
                PreUpdateFlashRect();
                DrawModuleStrings(menu, item as BYTE);
                PostUpdateFlashRect();
            }
        } else if item != (*menu).delta_item || state != (*menu).CurState {
            let piece = SlotPiece(state, item);
            SetContext(SpaceContext);
            if state == (*menu).CurState {
                let width = if state == PLANET_LANDER as BYTE || state == EMPTY_SLOT + 3 {
                    LANDER_WIDTH
                } else {
                    SHIP_PIECE_OFFSET as SIZE
                };
                (*menu).flash_rect0.corner.x += width * (item - (*menu).delta_item);
                (*menu).delta_item = item;
                DrawModuleStrings(menu, piece);
                SetFlashRect(ptr::addr_of!((*menu).flash_rect0));
            } else {
                (*menu).CurState = state;
                InitFlash(menu, piece);
            }
        }
    }
    1
}

unsafe fn ChangeFuelQuantity() {
    let mut incr: i32 = if PulsedInputState.menu[KEY_MENU_UP as usize] != 0 {
        FUEL_TANK_SCALE as i32
    } else if PulsedInputState.menu[KEY_MENU_DOWN as usize] != 0 {
        -(FUEL_TANK_SCALE as i32)
    } else if PulsedInputState.menu[KEY_MENU_PAGE_UP as usize] != 0 {
        FUEL_VOLUME_PER_ROW as i32
    } else if PulsedInputState.menu[KEY_MENU_PAGE_DOWN as usize] != 0 {
        -(FUEL_VOLUME_PER_ROW as i32)
    } else {
        return;
    };
    let max_fit = GetFuelTankCapacity().wrapping_sub(GlobData.SIS_state.FuelOnBoard) as i32;
    let max_afford = (GlobData.SIS_state.ResUnits / GlobData.Game_state.FuelCost as DWORD) as i32;
    let min_fit = -(GlobData.SIS_state.FuelOnBoard as i32);
    if incr > max_fit {
        incr = max_fit;
    }
    if incr > max_afford * FUEL_TANK_SCALE as i32 {
        incr = max_afford * FUEL_TANK_SCALE as i32;
    }
    if incr < min_fit {
        incr = min_fit;
    }
    if incr == 0 {
        PlayMenuSound(MENU_SOUND_EFFECT_MENU_SOUND_FAILURE);
    } else {
        let cost = incr / FUEL_TANK_SCALE as i32 * GlobData.Game_state.FuelCost as i32;
        PreUpdateFlashRect();
        DeltaSISGauges(0, incr as SIZE, -cost);
        PostUpdateFlashRect();
        RedistributeFuel();
    }
    let mut r: RECT = mem::zeroed();
    let old = SetContext(StatusContext);
    GetGaugeRect(&mut r, 0);
    SetFlashRect(&r);
    SetContext(old);
}

unsafe extern "C" fn onNamingDone() {
    DrawFlagshipName(0);
}

unsafe fn ExitOutfit(menu: *mut MENU_STATE) -> BOOLEAN {
    DestroyDrawable(ReleaseDrawable((*menu).CurFrame));
    (*menu).CurFrame = ptr::null_mut();
    DestroyDrawable(ReleaseDrawable((*menu).ModuleFrame));
    (*menu).ModuleFrame = ptr::null_mut();
    SetNamingCallback(None);
    0
}

/// Process an Outfit input frame, retaining installation state in `MENU_STATE`.
///
/// # Safety
/// `menu` must be the live game-thread menu. Game, resources, input, graphics
/// and audio must be initialized and exclusively accessed on that thread.
#[no_mangle]
pub unsafe extern "C" fn DoOutfit(menu: *mut MENU_STATE) -> BOOLEAN {
    if GlobData.Game_state.CurrentActivity & CHECK_ABORT as u16 != 0 {
        return ExitOutfit(menu);
    }
    if (*menu).Initialized == 0 {
        (*menu).InputFunc = Some(DoOutfit);
        (*menu).Initialized = 1;
        SetNamingCallback(Some(onNamingDone));
        (*menu).CurFrame =
            CaptureDrawable(LoadGraphicInstance(c"graphics.modulesmenu".as_ptr()).cast());
        (*menu).hMusic = LoadMusicInstance(c"music.outfit".as_ptr());
        (*menu).CurState = OUTFIT_FUEL;
        (*menu).ModuleFrame =
            CaptureDrawable(LoadGraphicInstance(c"graphics.outfitmodules".as_ptr()).cast());
        let mut s = STAMP {
            origin: POINT { x: 0, y: 0 },
            frame: CaptureDrawable(LoadGraphicInstance(c"graphics.outfit".as_ptr()).cast()),
        };
        SetTransitionSource(ptr::null());
        BatchGraphics();
        DrawSISFrame();
        DrawSISMessage(game_string(STARBASE_STRING_BASE + 2));
        DrawSISTitle(game_string(STARBASE_STRING_BASE));
        SetContext(SpaceContext);
        DrawStamp(&mut s);
        DestroyDrawable(ReleaseDrawable(s.frame));
        for slot in 0..NUM_DRIVE_SLOTS {
            let piece = GlobData.SIS_state.DriveSlots[slot as usize];
            if piece < EMPTY_SLOT {
                DrawShipPiece((*menu).ModuleFrame, piece as COUNT, slot as COUNT, 0);
            }
        }
        for slot in 0..NUM_JET_SLOTS {
            let piece = GlobData.SIS_state.JetSlots[slot as usize];
            if piece < EMPTY_SLOT {
                DrawShipPiece((*menu).ModuleFrame, piece as COUNT, slot as COUNT, 0);
            }
        }
        for slot in 0..NUM_MODULE_SLOTS {
            let piece = GlobData.SIS_state.ModuleSlots[slot as usize];
            if piece < EMPTY_SLOT {
                DrawShipPiece((*menu).ModuleFrame, piece as COUNT, slot as COUNT, 0);
            }
        }
        RedistributeFuel();
        DisplayLanders(menu);
        if get_game_state("CHMMR_BOMB_STATE") < 3 {
            let shields = get_game_state("LANDER_SHIELDS");
            s.frame = SetAbsFrameIndex((*menu).ModuleFrame, GetFrameCount((*menu).ModuleFrame) - 5);
            if shields & (1 << EARTHQUAKE_DISASTER) != 0 {
                DrawStamp(&mut s);
            }
            s.frame = IncFrameIndex(s.frame);
            if shields & (1 << BIOLOGICAL_DISASTER) != 0 {
                DrawStamp(&mut s);
            }
            s.frame = IncFrameIndex(s.frame);
            if shields & (1 << LIGHTNING_DISASTER) != 0 {
                DrawStamp(&mut s);
            }
            s.frame = IncFrameIndex(s.frame);
            if shields & (1 << LAVASPOT_DISASTER) != 0 {
                DrawStamp(&mut s);
            }
        }
        DrawMenuStateStrings(PM_FUEL as BYTE, (*menu).CurState as i16);
        DrawFlagshipName(0);
        if optWhichFonts == OPT_PC as i32 {
            DrawFlagshipStats();
        }
        ScreenTransition(3, ptr::null());
        PlayMusic((*menu).hMusic, 1, 1);
        UnbatchGraphics();
        SetFlashRect(MENU_3DO);
        GlobData.SIS_state.FuelOnBoard = GlobData
            .SIS_state
            .FuelOnBoard
            .wrapping_add(FUEL_TANK_SCALE >> 1)
            / FUEL_TANK_SCALE;
        GlobData.SIS_state.FuelOnBoard *= FUEL_TANK_SCALE;
        SetContext(StatusContext);
    } else if PulsedInputState.menu[KEY_MENU_CANCEL as usize] != 0
        || (PulsedInputState.menu[KEY_MENU_SELECT as usize] != 0 && (*menu).CurState == OUTFIT_EXIT)
    {
        if (*menu).CurState == OUTFIT_DOFUEL {
            (*menu).CurState = OUTFIT_FUEL;
            SetFlashRect(MENU_3DO);
        } else {
            return ExitOutfit(menu);
        }
    } else if PulsedInputState.menu[KEY_MENU_SELECT as usize] != 0 {
        match (*menu).CurState {
            OUTFIT_FUEL => {
                (*menu).CurState = OUTFIT_DOFUEL;
                SetContext(StatusContext);
                let mut r: RECT = mem::zeroed();
                GetGaugeRect(&mut r, 0);
                SetFlashRect(&r);
            }
            OUTFIT_DOFUEL => {
                (*menu).CurState = OUTFIT_FUEL;
                SetFlashRect(MENU_3DO);
            }
            OUTFIT_MODULES => {
                (*menu).CurState = EMPTY_SLOT + 2;
                (*menu).delta_item = if get_game_state("CHMMR_BOMB_STATE") != 3 {
                    0
                } else {
                    NUM_BOMB_MODULES as SIZE
                };
                (*menu).first_item.y = 0;
                (*menu).Initialized = 0;
                DoInstallModule(menu);
            }
            OUTFIT_SAVELOAD => {
                if GameOptions() == 0 {
                    return ExitOutfit(menu);
                }
                DrawMenuStateStrings(PM_FUEL as BYTE, (*menu).CurState as i16);
                SetFlashRect(MENU_3DO);
            }
            _ => {}
        }
    } else {
        if (*menu).CurState == OUTFIT_DOFUEL {
            SetMenuSounds(SOUND_FUEL, SOUND_SELECT | SOUND_CANCEL);
        } else {
            SetMenuSounds(SOUND_ARROWS, SOUND_SELECT);
        }
        if (*menu).CurState == OUTFIT_DOFUEL {
            ChangeFuelQuantity();
            SleepThread(ONE_SECOND / 30);
        } else {
            DoMenuChooser(menu, PM_FUEL as BYTE);
        }
    }
    1
}
