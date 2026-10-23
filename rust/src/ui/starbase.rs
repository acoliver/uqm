//! Starbase encounters, menus and flagship-piece rendering, ported from starbase.c.
#![cfg(any(not(test), feature = "linked_c_archive"))]
#![allow(non_snake_case)]

use super::{outfit::DoOutfit, shipyard::DoShipyard};
use crate::comm::comm_abi::{InitCommunication, SetCommIntroMode};
use crate::navigation::abi::*;
use crate::navigation::globals::GlobData;
use crate::state::game_state_keys::{get_game_state, set_game_state};
use std::{mem, ptr};
extern "C" {
    static mut NextActivity: ACTIVITY;
}
const CROSSFADE_SCREEN: u32 = 2;
const FADE_IN_SCREEN: u32 = 3;

const TALK_COMMANDER: BYTE = 0;
const OUTFIT_STARSHIP: BYTE = 1;
const SHIPYARD: BYTE = 2;
const DEPART_BASE: BYTE = 3;
const EMPTY_SLOT: COUNT = NUM_MODULES as COUNT;
const SOUND_ARROWS: MENU_SOUND_FLAGS = 0x0f;
const SOUND_SELECT: MENU_SOUND_FLAGS = 1 << 4;
static mut NEXT_ANIMATION_TIME: TimeCount = 0;

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

unsafe fn DrawBaseStateStrings(mut old: BYTE, new: BYTE) {
    SetContext(ScreenContext);
    SetContextFont(StarConFont);
    SetContextForeGroundColor(BLACK);
    let mut text = TEXT {
        baseline: POINT {
            x: 73 - 4 + SAFE_X as i16,
            y: 0,
        },
        align: TEXT_ALIGN_ALIGN_CENTER,
        pStr: ptr::null(),
        CharCount: 0,
    };
    if old == BYTE::MAX {
        text.baseline.y = 106 + 28 + SAFE_Y as i16 + 4;
        old = TALK_COMMANDER;
        while old < DEPART_BASE {
            if old != new {
                text.pStr = game_string(STARBASE_STRING_BASE + 1 + old as u32);
                text.CharCount = COUNT::MAX;
                font_DrawText(&mut text);
            }
            text.baseline.y += 23 - 4;
            old += 1;
        }
    }
    text.baseline.y = 106 + 28 + SAFE_Y as i16 + 4 + (23 - 4) * old as i16;
    text.pStr = game_string(STARBASE_STRING_BASE + 1 + old as u32);
    text.CharCount = COUNT::MAX;
    font_DrawText(&mut text);
    SetContextForeGroundColor(rgb15(0x1f, 0x1f, 0x0a));
    text.baseline.y = 106 + 28 + SAFE_Y as i16 + 4 + (23 - 4) * new as i16;
    text.pStr = game_string(STARBASE_STRING_BASE + 1 + new as u32);
    text.CharCount = COUNT::MAX;
    font_DrawText(&mut text);
}

/// Draw one flagship piece, including adjoining module end-cap repairs.
///
/// # Safety
/// The drawable, graphics context and global SIS state must be live on the
/// game thread. Piece and slot indices must be valid for the supplied drawable.
#[no_mangle]
pub unsafe extern "C" fn DrawShipPiece(
    module_frame: FRAME,
    mut which_piece: COUNT,
    mut which_slot: COUNT,
    blueprint: BOOLEAN,
) {
    let mut old_color = BLACK;
    let mut side: STAMP = mem::zeroed();
    let mut top: STAMP = mem::zeroed();
    let mut repair: SBYTE = 0;
    if which_piece == FUSION_THRUSTER as COUNT || which_piece == EMPTY_SLOT {
        side.origin = POINT {
            x: DRIVE_SIDE_X as i16,
            y: DRIVE_SIDE_Y as i16,
        };
        top.origin = POINT {
            x: DRIVE_TOP_X as i16,
            y: DRIVE_TOP_Y as i16,
        };
    } else if which_piece == TURNING_JETS as COUNT || which_piece == EMPTY_SLOT + 1 {
        side.origin = POINT {
            x: JET_SIDE_X as i16,
            y: JET_SIDE_Y as i16,
        };
        top.origin = POINT {
            x: JET_TOP_X as i16,
            y: JET_TOP_Y as i16,
        };
    } else {
        if which_piece < EMPTY_SLOT + 2 {
            repair = 1;
            if which_piece < EMPTY_SLOT
                && (which_slot == 0
                    || GlobData.SIS_state.ModuleSlots[(which_slot - 1) as usize]
                        < EMPTY_SLOT as BYTE)
            {
                repair += 1;
            }
        } else if blueprint == 0 {
            if which_slot == 0 || which_slot >= NUM_MODULE_SLOTS as COUNT - 3 {
                which_piece += 1;
            }
            if which_slot < NUM_MODULE_SLOTS as COUNT - 1
                && GlobData.SIS_state.ModuleSlots[(which_slot + 1) as usize] < EMPTY_SLOT as BYTE
            {
                repair = -1;
                if which_piece == EMPTY_SLOT + 3 || which_slot + 1 == NUM_MODULE_SLOTS as COUNT - 3
                {
                    repair -= 1;
                }
            }
        }
        side.origin = POINT {
            x: MODULE_SIDE_X as i16,
            y: MODULE_SIDE_Y as i16,
        };
        top.origin = POINT {
            x: MODULE_TOP_X as i16,
            y: MODULE_TOP_Y as i16,
        };
    }
    side.origin.x += (which_slot as i32 * SHIP_PIECE_OFFSET as i32) as i16;
    if repair < 0 {
        side.frame = SetAbsFrameIndex(
            module_frame,
            ((NUM_MODULES as i32 - 1 + (6 - 2)) + (NUM_MODULES as i32 + 6) - (repair as i32 + 1))
                as COUNT,
        );
        DrawStamp(&mut side);
    } else if repair != 0 {
        let mut r = RECT {
            corner: side.origin,
            extent: EXTENT {
                width: SHIP_PIECE_OFFSET as i16,
                height: 1,
            },
        };
        old_color = SetContextForeGroundColor(BLACK);
        DrawFilledRectangle(&mut r);
        r.corner.y += 23 - 1;
        DrawFilledRectangle(&mut r);
        r.extent.width = 1;
        r.extent.height = 8;
        if repair == 2 {
            r.corner = side.origin;
            DrawFilledRectangle(&mut r);
            r.corner.y += 15;
            DrawFilledRectangle(&mut r);
        }
        if which_slot < NUM_MODULE_SLOTS as COUNT - 1 {
            r.corner = side.origin;
            r.corner.x += SHIP_PIECE_OFFSET as i16;
            DrawFilledRectangle(&mut r);
            r.corner.y += 15;
            DrawFilledRectangle(&mut r);
        }
    }
    if blueprint != 0 {
        if repair != 0 {
            SetContextForeGroundColor(old_color);
        }
        side.frame = SetAbsFrameIndex(module_frame, which_piece.wrapping_sub(1));
        DrawFilledStamp(&mut side);
    } else {
        top.origin.x += (which_slot as i32 * SHIP_PIECE_OFFSET as i32) as i16;
        if repair < 0 {
            top.frame = SetRelFrameIndex(side.frame, -((NUM_MODULES - 1 + 6) as SIZE));
            DrawStamp(&mut top);
        } else if repair != 0 {
            let mut r = RECT {
                corner: top.origin,
                extent: EXTENT {
                    width: SHIP_PIECE_OFFSET as i16,
                    height: 1,
                },
            };
            DrawFilledRectangle(&mut r);
            r.corner.y += 32 - 1;
            DrawFilledRectangle(&mut r);
            r.extent.width = 1;
            r.extent.height = 12;
            if repair == 2 {
                r.corner = top.origin;
                DrawFilledRectangle(&mut r);
                r.corner.y += 20;
                DrawFilledRectangle(&mut r);
            }
            repair = (which_slot < NUM_MODULE_SLOTS as COUNT - 1) as SBYTE;
            if repair != 0 {
                r.corner = top.origin;
                r.corner.x += SHIP_PIECE_OFFSET as i16;
                DrawFilledRectangle(&mut r);
                r.corner.y += 20;
                DrawFilledRectangle(&mut r);
            }
        }
        top.frame = SetAbsFrameIndex(module_frame, which_piece);
        DrawStamp(&mut top);
        side.frame = SetRelFrameIndex(top.frame, (NUM_MODULES - 1 + 6) as SIZE);
        DrawStamp(&mut side);
        if which_slot == 1 && which_piece == EMPTY_SLOT + 2 {
            let mut stamp = STAMP {
                origin: top.origin,
                frame: SetAbsFrameIndex(module_frame, (NUM_MODULES + 5) as COUNT),
            };
            stamp.origin.x -= SHIP_PIECE_OFFSET as i16;
            DrawStamp(&mut stamp);
            stamp.origin = side.origin;
            stamp.origin.x -= SHIP_PIECE_OFFSET as i16;
            stamp.frame = SetRelFrameIndex(stamp.frame, (NUM_MODULES - 1 + 6) as SIZE);
            DrawStamp(&mut stamp);
        }
        if repair != 0 {
            top.origin.x += SHIP_PIECE_OFFSET as i16;
            side.origin.x += SHIP_PIECE_OFFSET as i16;
            which_slot += 1;
            which_piece = GlobData.SIS_state.ModuleSlots[which_slot as usize] as COUNT;
            if which_piece == EMPTY_SLOT + 2 && which_slot >= NUM_MODULE_SLOTS as COUNT - 3 {
                which_piece += 1;
            }
            top.frame = SetAbsFrameIndex(module_frame, which_piece);
            DrawStamp(&mut top);
            side.frame = SetRelFrameIndex(top.frame, (NUM_MODULES - 1 + 6) as SIZE);
            DrawStamp(&mut side);
        }
    }
}

unsafe fn rotateStarbase(menu: *mut MENU_STATE, frame: FRAME) {
    let now = GetTimeCounter();
    if !frame.is_null() {
        (*menu).flash_frame0 = frame;
        (*menu).flash_rect0.corner.x = SAFE_X as i16;
        (*menu).flash_rect0.corner.y = SAFE_Y as i16 + 4;
    }
    if now >= NEXT_ANIMATION_TIME || !frame.is_null() {
        NEXT_ANIMATION_TIME = now.wrapping_add(ONE_SECOND / 20);
        let mut stamp = STAMP {
            origin: (*menu).flash_rect0.corner,
            frame: (*menu).flash_frame0,
        };
        DrawStamp(&mut stamp);
        stamp.frame = IncFrameIndex(stamp.frame);
        if GetFrameIndex(stamp.frame) == 0 {
            stamp.frame = IncFrameIndex(stamp.frame);
        }
        (*menu).flash_frame0 = stamp.frame;
    }
}

unsafe fn ExitStarBase(menu: *mut MENU_STATE) -> BOOLEAN {
    DestroyDrawable(ReleaseDrawable((*menu).CurFrame));
    (*menu).CurFrame = ptr::null_mut();
    StopMusic();
    if !(*menu).hMusic.is_null() {
        DestroyMusic((*menu).hMusic);
        (*menu).hMusic = ptr::null_mut();
    }
    if (*menu).CurState == DEPART_BASE {
        if GlobData.Game_state.CurrentActivity & (CHECK_ABORT | CHECK_LOAD) as u16 == 0 {
            set_game_state("STARBASE_VISITED", 0);
        }
        return 0;
    }
    (*menu).Initialized = 0;
    if (*menu).CurState == TALK_COMMANDER {
        crate::automation::ui_observation::observe_starbase_menu(false);
        FlushInput();
        InitCommunication(CONVERSATION_COMMANDER_CONVERSATION);
        crate::automation::ui_observation::observe_starbase_menu(true);
        set_game_state("GLOBAL_FLAGS_AND_DATA", BYTE::MAX as u32);
    } else {
        let old = (*menu).CurState;
        match old {
            OUTFIT_STARSHIP => (*menu).InputFunc = Some(DoOutfit),
            SHIPYARD => (*menu).InputFunc = Some(DoShipyard),
            _ => {}
        }
        SetMenuSounds(SOUND_ARROWS, SOUND_SELECT);
        DoInput(menu.cast(), 1);
        (*menu).Initialized = 0;
        (*menu).CurState = old;
        (*menu).InputFunc = Some(DoStarBase);
    }
    1
}

/// Process one Starbase input frame and run the selected nested menu.
///
/// # Safety
/// `menu` must be the live game-thread menu. Graphics, audio, communication
/// and input systems must be initialized and accessed only on that thread.
#[no_mangle]
pub unsafe extern "C" fn DoStarBase(menu: *mut MENU_STATE) -> BOOLEAN {
    if GlobData.Game_state.CurrentActivity & (CHECK_ABORT | CHECK_LOAD) as u16 != 0 {
        (*menu).CurState = DEPART_BASE;
        return ExitStarBase(menu);
    }
    SetMenuSounds(SOUND_ARROWS, SOUND_SELECT);
    if (*menu).Initialized == 0 {
        LastActivity &= !(CHECK_LOAD as u16);
        (*menu).InputFunc = Some(DoStarBase);
        SetFlashRect(ptr::null());
        if !(*menu).hMusic.is_null() {
            StopMusic();
            DestroyMusic((*menu).hMusic);
            (*menu).hMusic = ptr::null_mut();
        }
        (*menu).Initialized = 1;
        (*menu).CurFrame =
            CaptureDrawable(LoadGraphicInstance(c"graphics.starbase".as_ptr()).cast());
        (*menu).hMusic = LoadMusicInstance(c"music.starbase".as_ptr());
        SetContext(ScreenContext);
        SetTransitionSource(ptr::null());
        BatchGraphics();
        SetContextBackGroundColor(BLACK);
        ClearDrawable();
        rotateStarbase(menu, (*menu).CurFrame);
        DrawBaseStateStrings(BYTE::MAX, (*menu).CurState);
        ScreenTransition(3, ptr::null());
        PlayMusic((*menu).hMusic, 1, 1);
        UnbatchGraphics();
    } else if PulsedInputState.menu[KEY_MENU_SELECT as usize] != 0 {
        return ExitStarBase(menu);
    } else {
        let mut state = (*menu).CurState;
        if PulsedInputState.menu[KEY_MENU_LEFT as usize] != 0
            || PulsedInputState.menu[KEY_MENU_UP as usize] != 0
        {
            let old = state;
            state = state.wrapping_sub(1);
            if old == TALK_COMMANDER {
                state = DEPART_BASE;
            }
        } else if PulsedInputState.menu[KEY_MENU_RIGHT as usize] != 0
            || PulsedInputState.menu[KEY_MENU_DOWN as usize] != 0
        {
            let old = state;
            state = state.wrapping_add(1);
            if old == DEPART_BASE {
                state = TALK_COMMANDER;
            }
        }
        BatchGraphics();
        SetContext(ScreenContext);
        if state != (*menu).CurState {
            DrawBaseStateStrings((*menu).CurState, state);
            (*menu).CurState = state;
        }
        rotateStarbase(menu, ptr::null_mut());
        UnbatchGraphics();
        SleepThread(ONE_SECOND / 30);
    }
    1
}

unsafe fn DoTimePassage() {
    SleepThreadUntil(FadeScreen(
        ScreenFadeType_FadeAllToBlack,
        (ONE_SECOND * 2) as SIZE,
    ));
    MoveGameClockDays(14);
}

unsafe fn CleanupAfterStarBase() {
    if GlobData.Game_state.CurrentActivity & (CHECK_LOAD | CHECK_ABORT) as u16 == 0 {
        set_game_state("GLOBAL_FLAGS_AND_DATA", 0);
        GlobData.Game_state.CurrentActivity = CHECK_LOAD as u16;
        NextActivity = IN_INTERPLANETARY as u16 | START_INTERPLANETARY as u16;
    }
}

/// Run the unallied/Ilwrath encounters, delivery dialogue and Starbase menus.
///
/// # Safety
/// World queues, game clock, communication, input, graphics and audio must be
/// initialized and exclusively accessed by the calling game thread.
#[no_mangle]
pub unsafe extern "C" fn VisitStarBase() {
    if get_game_state("CHMMR_BOMB_STATE") == 2 {
        CurStarDescPtr = ptr::null_mut();
        set_game_state("GLOBAL_FLAGS_AND_DATA", BYTE::MAX as u32);
    }
    if get_game_state("STARBASE_AVAILABLE") == 0 {
        crate::automation::scenario::observe_encounter_dispatch(
            CONVERSATION_COMMANDER_CONVERSATION,
        );
        SetCommIntroMode(CROSSFADE_SCREEN, 0);
        InitCommunication(CONVERSATION_COMMANDER_CONVERSATION);
        if get_game_state("PROBE_ILWRATH_ENCOUNTER") == 0
            || GlobData.Game_state.CurrentActivity & CHECK_ABORT as u16 != 0
        {
            CleanupAfterStarBase();
            return;
        }
        let ship = CloneShipFragment(
            ILWRATH_SHIP as COUNT,
            ptr::addr_of_mut!(GlobData.Game_state.npc_built_ship_q),
            7,
        );
        (*ship.cast::<SHIP_FRAGMENT>()).race_id = BYTE::MAX;
        InitCommunication(CONVERSATION_ILWRATH_CONVERSATION);
        if GlobData.SIS_state.CrewEnlisted == COUNT::MAX
            || GlobData.Game_state.CurrentActivity & CHECK_ABORT as u16 != 0
        {
            return;
        }
        SetCommIntroMode(CROSSFADE_SCREEN, 0);
        InitCommunication(CONVERSATION_COMMANDER_CONVERSATION);
        if GlobData.Game_state.CurrentActivity & CHECK_ABORT as u16 != 0 {
            return;
        }
        set_game_state("GLOBAL_FLAGS_AND_DATA", BYTE::MAX as u32);
    }
    let previous = SetStatusMessageMode(StatMsgMode_SMM_RES_UNITS);
    if get_game_state("MOONBASE_ON_SHIP") != 0 || get_game_state("CHMMR_BOMB_STATE") == 2 {
        DoTimePassage();
        if GlobData.SIS_state.CrewEnlisted == COUNT::MAX {
            return;
        }
        crate::automation::scenario::observe_encounter_dispatch(
            CONVERSATION_COMMANDER_CONVERSATION,
        );
        SetCommIntroMode(FADE_IN_SCREEN, ONE_SECOND * 2);
        InitCommunication(CONVERSATION_COMMANDER_CONVERSATION);
        if GlobData.Game_state.CurrentActivity & CHECK_ABORT as u16 != 0 {
            return;
        }
        set_game_state("GLOBAL_FLAGS_AND_DATA", BYTE::MAX as u32);
    }
    let mut menu: MENU_STATE = mem::zeroed();
    menu.InputFunc = Some(DoStarBase);
    let old = SetContext(ScreenContext);
    crate::automation::ui_observation::observe_starbase_menu(true);
    DoInput(ptr::addr_of_mut!(menu).cast(), 1);
    crate::automation::ui_observation::observe_starbase_menu(false);
    SetContext(old);
    SetStatusMessageMode(previous);
    CleanupAfterStarBase();
}

/// Advance fourteen days, finish bomb installation and force a SolarSys reload.
///
/// # Safety
/// The game clock, live world and graphics must be exclusively accessed on
/// the initialized game thread.
#[no_mangle]
pub unsafe extern "C" fn InstallBombAtEarth() {
    DoTimePassage();
    SetContext(ScreenContext);
    SetTransitionSource(ptr::null());
    SetContextBackGroundColor(BLACK);
    ClearDrawable();
    SleepThreadUntil(FadeScreen(ScreenFadeType_FadeAllToColor, 0));
    set_game_state("CHMMR_BOMB_STATE", 3);
    GlobData.Game_state.CurrentActivity = CHECK_LOAD as u16;
    NextActivity = IN_INTERPLANETARY as u16 | START_INTERPLANETARY as u16;
    CurStarDescPtr = ptr::null_mut();
}

/// Wrap text with the original word scanning and oversized-word splitting.
///
/// # Safety
/// `text` must point to readable UNICODE storage with at least `len + 1` code
/// units, because the original scanner reads the next unit before checking the
/// remaining length. `records` must have space for every output line. The
/// active font must be initialized; `field_width` must fit at least one glyph.
#[no_mangle]
pub unsafe extern "C" fn WrapText(
    mut text: *const UNICODE,
    mut len: COUNT,
    mut records: *mut TEXT,
    field_width: SIZE,
) -> COUNT {
    let mut lines: COUNT = 0;
    loop {
        let mut rect: RECT = mem::zeroed();
        (*records).align = TEXT_ALIGN_ALIGN_LEFT;
        (*records).pStr = text;
        (*records).CharCount = 1;
        lines = lines.wrapping_add(1);
        let mut old_count;
        loop {
            old_count = (*records).CharCount;
            loop {
                text = text.add(1);
                if *text == b' ' as UNICODE || (text.offset_from((*records).pStr) as COUNT) >= len {
                    break;
                }
            }
            (*records).CharCount = text.offset_from((*records).pStr) as COUNT;
            TextRect(records, &mut rect, ptr::null_mut());
            if !((*records).CharCount < len && rect.extent.width < field_width) {
                break;
            }
        }
        if rect.extent.width >= field_width {
            (*records).CharCount = old_count;
            if (*records).CharCount == 1 {
                loop {
                    (*records).CharCount = (*records).CharCount.wrapping_add(1);
                    TextRect(records, &mut rect, ptr::null_mut());
                    if rect.extent.width >= field_width {
                        break;
                    }
                }
                (*records).CharCount = (*records).CharCount.wrapping_sub(1);
            }
        }
        text = (*records).pStr.add((*records).CharCount as usize);
        len = len.wrapping_sub((*records).CharCount);
        records = records.add(1);
        if len != 0 && (rect.extent.width < field_width || old_count > 1) {
            text = text.add(1);
            len -= 1;
        }
        if len == 0 {
            return lines;
        }
    }
}
