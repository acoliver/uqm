//! Starbase encounters, menus and flagship-piece rendering, ported from starbase.c.
#![cfg(not(test))]

use super::{outfit::do_outfit, shipyard::do_shipyard};
use crate::engine::{
    battle::Cx,
    observe::{CheckpointView, Io},
    Kernel, StoryKey, World, WorldData,
};
use crate::navigation::abi::*;

use std::{mem, ptr};
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
    GetStringAddress(SetAbsStringTableIndex(
        Kernel::current().read(|owner| owner.game_strings),
        index as COUNT,
    ))
    .cast()
}

unsafe fn draw_base_state_strings(mut old: BYTE, new: BYTE) {
    SetContext(Kernel::current().read(|owner| owner.screen_context));
    SetContextFont(Kernel::current().read(|owner| owner.star_con_font));
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
        let slots = WorldData::current().read(|world| world.flagship.module_slots);
        if which_piece < EMPTY_SLOT + 2 {
            repair = 1;
            if which_piece < EMPTY_SLOT
                && (which_slot == 0 || slots[(which_slot - 1) as usize] < EMPTY_SLOT as BYTE)
            {
                repair += 1;
            }
        } else if blueprint == 0 {
            if which_slot == 0 || which_slot >= NUM_MODULE_SLOTS as COUNT - 3 {
                which_piece += 1;
            }
            if which_slot < NUM_MODULE_SLOTS as COUNT - 1
                && slots[(which_slot + 1) as usize] < EMPTY_SLOT as BYTE
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
            let slot = which_slot as usize;
            which_piece = WorldData::current().read(|w| w.flagship.module_slots[slot]) as COUNT;
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

unsafe fn rotate_starbase(menu: *mut MENU_STATE, frame: FRAME) {
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
        if GetFrameIndex(stamp.frame.cast()) == 0 {
            stamp.frame = IncFrameIndex(stamp.frame);
        }
        (*menu).flash_frame0 = stamp.frame;
    }
}

unsafe fn finish_starbase_screen(menu: *mut MENU_STATE) -> BOOLEAN {
    DestroyDrawable(ReleaseDrawable((*menu).CurFrame));
    (*menu).CurFrame = ptr::null_mut();
    stop_music();
    if !(*menu).hMusic.is_null() {
        DestroyMusic((*menu).hMusic);
        (*menu).hMusic = ptr::null_mut();
    }
    if (*menu).CurState == DEPART_BASE {
        if crate::engine::World::current().read(|world| world.activity())
            & (CHECK_ABORT | CHECK_LOAD) as u16
            == 0
        {
            World::current().write(|world| world.set_story_state(StoryKey::STARBASE_VISITED, 0));
        }
        return 0;
    }
    (*menu).Initialized = 0;
    1
}

unsafe fn activate_starbase_selection(
    input: &crate::engine::input::InputSession,
    cx: &mut Cx<'_>,
    io: &mut Io<'_>,
    menu: &mut MENU_STATE,
) -> BOOLEAN {
    if cx.with_legacy(|| finish_starbase_screen(menu)) == 0 {
        return 0;
    }
    if menu.CurState == TALK_COMMANDER {
        io.observer.checkpoint(&CheckpointView::StarbaseMenu(false));
        // SAFETY: the starbase session owns initialized game-thread input.
        cx.with_legacy(|| unsafe { FlushInput(input) });
        crate::comm::dispatch::init_communication(cx, io, CONVERSATION_COMMANDER_CONVERSATION);
        io.observer.checkpoint(&CheckpointView::StarbaseMenu(true));
        cx.world
            .set_story_state(StoryKey::GLOBAL_FLAGS_AND_DATA, BYTE::MAX as u32);
    } else {
        let old = menu.CurState;
        menu.InputFunc = Some(match old {
            OUTFIT_STARSHIP => do_outfit,
            SHIPYARD => do_shipyard,
            _ => unreachable!("invalid Starbase submenu"),
        });
        cx.with_legacy(|| SetMenuSounds(input, SOUND_ARROWS, SOUND_SELECT));
        cx.run_input(io, true, |cx, _| {
            let callback = menu
                .InputFunc
                .unwrap_or_else(|| panic!("missing Starbase submenu callback"));
            callback(cx, menu, input) != 0
        });
        menu.Initialized = 0;
        menu.CurState = old;
        menu.InputFunc = None;
    }
    1
}

unsafe fn update_starbase_screen(
    input: &crate::engine::input::InputSession,
    menu: *mut MENU_STATE,
    music_enabled: bool,
) -> bool {
    if crate::engine::World::current().read(|world| world.activity())
        & (CHECK_ABORT | CHECK_LOAD) as u16
        != 0
    {
        (*menu).CurState = DEPART_BASE;
        return false;
    }
    SetMenuSounds(input, SOUND_ARROWS, SOUND_SELECT);
    if (*menu).Initialized == 0 {
        Kernel::current().write(|kernel| {
            kernel.last_activity &= !(CHECK_LOAD as u16);
            crate::ui::sis::clear_flash(kernel);
        });
        if !(*menu).hMusic.is_null() {
            stop_music();
            DestroyMusic((*menu).hMusic);
            (*menu).hMusic = ptr::null_mut();
        }
        (*menu).Initialized = 1;
        (*menu).CurFrame =
            CaptureDrawable(LoadGraphicInstance(c"graphics.starbase".as_ptr()).cast());
        (*menu).hMusic = (LoadMusicInstance(c"music.starbase".as_ptr())).cast();
        SetContext(Kernel::current().read(|owner| owner.screen_context));
        SetTransitionSource(ptr::null());
        BatchGraphics();
        SetContextBackGroundColor(BLACK);
        ClearDrawable();
        rotate_starbase(menu, (*menu).CurFrame);
        draw_base_state_strings(BYTE::MAX, (*menu).CurState);
        ScreenTransition(3, ptr::null());
        crate::sound::music::remember_native_music(
            std::ptr::NonNull::new((*menu).hMusic.cast()),
            true,
            1,
            music_enabled,
        );
        UnbatchGraphics();
    } else if input.pulsed().menu[KEY_MENU_SELECT as usize] != 0 {
        return false;
    } else {
        let mut state = (*menu).CurState;
        if input.pulsed().menu[KEY_MENU_LEFT as usize] != 0
            || input.pulsed().menu[KEY_MENU_UP as usize] != 0
        {
            let old = state;
            state = state.wrapping_sub(1);
            if old == TALK_COMMANDER {
                state = DEPART_BASE;
            }
        } else if input.pulsed().menu[KEY_MENU_RIGHT as usize] != 0
            || input.pulsed().menu[KEY_MENU_DOWN as usize] != 0
        {
            let old = state;
            state = state.wrapping_add(1);
            if old == DEPART_BASE {
                state = TALK_COMMANDER;
            }
        }
        BatchGraphics();
        SetContext(Kernel::current().read(|owner| owner.screen_context));
        if state != (*menu).CurState {
            draw_base_state_strings((*menu).CurState, state);
            (*menu).CurState = state;
        }
        rotate_starbase(menu, ptr::null_mut());
        UnbatchGraphics();
        SleepThread(ONE_SECOND / 30);
    }
    true
}

unsafe fn do_time_passage(cx: &mut Cx<'_>) {
    let interval = (ONE_SECOND * 2) as SIZE;
    cx.with_legacy(|| SleepThreadUntil(FadeScreen(ScreenFadeType_FadeAllToBlack, interval)));
    crate::time::clock_abi::advance_days(cx, 14);
}

fn cleanup_after_star_base(world: &mut World) {
    if world.activity() & (CHECK_LOAD | CHECK_ABORT) as u16 == 0 {
        world.set_story_state(StoryKey::GLOBAL_FLAGS_AND_DATA, 0);
        world.set_activity(CHECK_LOAD as u16);
        world.next_activity = IN_INTERPLANETARY as u16 | START_INTERPLANETARY as u16;
    }
}

unsafe fn prepare_starbase_visit(cx: &mut Cx<'_>, io: &mut Io<'_>) -> Option<u32> {
    if cx.world.story_state(StoryKey::CHMMR_BOMB_STATE) == 2 {
        cx.world.select_star(None);
        cx.world
            .set_story_state(StoryKey::GLOBAL_FLAGS_AND_DATA, BYTE::MAX as u32);
    }
    if cx.world.story_state(StoryKey::STARBASE_AVAILABLE) == 0 {
        io.observer.checkpoint(&CheckpointView::EncounterDispatch(
            CONVERSATION_COMMANDER_CONVERSATION,
        ));
        crate::comm::dispatch::init_communication_with_options(
            cx,
            io,
            CONVERSATION_COMMANDER_CONVERSATION,
            crate::comm::CommOptions {
                intro: (CROSSFADE_SCREEN, 0),
                ..Default::default()
            },
        );
        if cx.world.story_state(StoryKey::PROBE_ILWRATH_ENCOUNTER) == 0
            || cx.world.activity() & CHECK_ABORT as u16 != 0
        {
            cleanup_after_star_base(cx.world);
            return None;
        }
        let ship = crate::comm::dispatch::clone_npc_fragment(cx.world, ILWRATH_SHIP as u8, 7)
            .unwrap_or_else(|| panic!("starbase Ilwrath fragment capacity"));
        cx.world.npc_built_ships().get_mut(ship).race_id = BYTE::MAX;
        crate::comm::dispatch::init_communication(cx, io, CONVERSATION_ILWRATH_CONVERSATION);
        if cx.world.data.flagship.crew_enlisted == COUNT::MAX
            || cx.world.activity() & CHECK_ABORT as u16 != 0
        {
            return None;
        }
        crate::comm::dispatch::init_communication_with_options(
            cx,
            io,
            CONVERSATION_COMMANDER_CONVERSATION,
            crate::comm::CommOptions {
                intro: (CROSSFADE_SCREEN, 0),
                ..Default::default()
            },
        );
        if cx.world.activity() & CHECK_ABORT as u16 != 0 {
            return None;
        }
        cx.world
            .set_story_state(StoryKey::GLOBAL_FLAGS_AND_DATA, BYTE::MAX as u32);
    }
    let previous = cx.kernel.set_status_message_mode(StatMsgMode_SMM_RES_UNITS);
    if cx.world.story_state(StoryKey::MOONBASE_ON_SHIP) != 0
        || cx.world.story_state(StoryKey::CHMMR_BOMB_STATE) == 2
    {
        do_time_passage(cx);
        if cx.world.data.flagship.crew_enlisted == COUNT::MAX {
            return None;
        }
        io.observer.checkpoint(&CheckpointView::EncounterDispatch(
            CONVERSATION_COMMANDER_CONVERSATION,
        ));
        crate::comm::dispatch::init_communication_with_options(
            cx,
            io,
            CONVERSATION_COMMANDER_CONVERSATION,
            crate::comm::CommOptions {
                intro: (FADE_IN_SCREEN, ONE_SECOND * 2),
                ..Default::default()
            },
        );
        if cx.world.activity() & CHECK_ABORT as u16 != 0 {
            return None;
        }
        cx.world
            .set_story_state(StoryKey::GLOBAL_FLAGS_AND_DATA, BYTE::MAX as u32);
    }
    Some(previous)
}

/// Run the unallied/Ilwrath encounters, delivery dialogue and Starbase menus.
///
/// # Safety
/// World queues, game clock, communication, input, graphics and audio must be
/// initialized and exclusively accessed by the calling game thread.
pub unsafe fn run_starbase_visit(cx: &mut Cx<'_>, io: &mut Io<'_>) {
    let Some(previous) = prepare_starbase_visit(cx, io) else {
        return;
    };
    let runtime = cx.kernel.input_runtime();
    let input = runtime.input();
    let mut menu: MENU_STATE = mem::zeroed();
    let context = cx.kernel.screen_context;
    let old = cx.with_legacy(|| SetContext(context));
    io.observer.checkpoint(&CheckpointView::StarbaseMenu(true));
    cx.run_input(io, true, |cx, io| {
        let music_enabled = cx.world.data.game.glob_flags & 8 == 0;
        if cx.with_legacy(|| update_starbase_screen(input, &mut menu, music_enabled)) {
            true
        } else {
            activate_starbase_selection(input, cx, io, &mut menu) != 0
        }
    });
    io.observer.checkpoint(&CheckpointView::StarbaseMenu(false));
    cx.with_legacy(|| {
        SetContext(old);
    });
    cx.kernel.set_status_message_mode(previous);
    cleanup_after_star_base(cx.world);
}

/// Advance fourteen days, finish bomb installation and force a SolarSys reload.
///
/// # Safety
/// The game clock, live world and graphics must be exclusively accessed on
/// the initialized game thread.
pub unsafe fn install_bomb_at_earth(cx: &mut Cx<'_>) {
    let screen_context = cx.kernel.screen_context;
    do_time_passage(cx);
    cx.with_legacy(|| {
        SetContext(screen_context);
        SetTransitionSource(ptr::null());
        SetContextBackGroundColor(BLACK);
        ClearDrawable();
        SleepThreadUntil(FadeScreen(ScreenFadeType_FadeAllToColor, 0));
    });
    cx.world.set_story_state(StoryKey::CHMMR_BOMB_STATE, 3);
    cx.world.set_activity(CHECK_LOAD as u16);
    cx.world.next_activity = IN_INTERPLANETARY as u16 | START_INTERPLANETARY as u16;
    cx.world.select_star(None);
}
