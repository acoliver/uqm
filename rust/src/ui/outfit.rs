//! Outfit installation, fuel transactions and menu callbacks, ported from outfit.c.
#![cfg(not(test))]

use crate::engine::StoryKey;
use crate::engine::World;
use crate::engine::{Kernel, WorldData};
use crate::navigation::abi::*;

use crate::state::game_state_keys::get_game_state;
use std::{mem, ptr};

const OUTFIT_FUEL: BYTE = 0;
const OUTFIT_MODULES: BYTE = 1;
const OUTFIT_SAVELOAD: BYTE = 2;
const OUTFIT_EXIT: BYTE = 3;
const OUTFIT_DOFUEL: BYTE = 4;
use super::outfit_state::{apply_piece, can_install, EMPTY_SLOT};
const LANDER_X: i16 = 24;
const LANDER_Y: i16 = 67;
const LANDER_WIDTH: i16 = 15;
const LANDER_DISASTERS: [u32; 4] = [
    EARTHQUAKE_DISASTER,
    BIOLOGICAL_DISASTER,
    LIGHTNING_DISASTER,
    LAVASPOT_DISASTER,
];
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
    GetStringAddress(SetAbsStringTableIndex(
        Kernel::current().read(|owner| owner.game_strings),
        index as COUNT,
    ))
    .cast()
}

unsafe fn draw_module_strings(menu: &mut MENU_STATE, module: BYTE) {
    let old = SetContext(Kernel::current().read(|owner| owner.status_context));
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
    } else if !menu.CurFrame.is_null() {
        s.frame = SetAbsFrameIndex(menu.CurFrame, module as COUNT);
        DrawStamp(&mut s);
        let buffer = format!(
            "{}\0",
            (**WorldData::current()).game.module_cost[module as usize] as u32 * MODULE_COST_SCALE
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
        SetContextFont(Kernel::current().read(|owner| owner.tiny_font));
        SetContextForeGroundColor(rgb15(0, 0x1f, 0));
        font_DrawText(&mut text);
    }
    UnbatchGraphics();
    SetContext(old);
}

unsafe fn redistribute_fuel() {
    let volume = WorldData::current().read(|owner| owner.flagship.fuel_on_board);
    let old = SetContext(Kernel::current().read(|owner| owner.space_context));
    let mut r: RECT = mem::zeroed();
    r.extent.height = 1;
    BatchGraphics();
    (**WorldData::current()).flagship.fuel_on_board = FUEL_RESERVE;
    while WorldData::current().read(|owner| owner.flagship.fuel_on_board)
        < GetFTankCapacity(&mut r.corner)
    {
        if WorldData::current().read(|owner| owner.flagship.fuel_on_board) < volume {
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
        (**WorldData::current()).flagship.fuel_on_board += FUEL_VOLUME_PER_ROW;
    }
    UnbatchGraphics();
    SetContext(old);
    (**WorldData::current()).flagship.fuel_on_board = volume;
}

unsafe fn display_landers(menu: &mut MENU_STATE) {
    let mut s = STAMP {
        origin: POINT { x: 0, y: 0 },
        frame: menu.ModuleFrame,
    };
    if World::current().read(|world| get_game_state(world, StoryKey::CHMMR_BOMB_STATE)) == 3 {
        s.frame = DecFrameIndex(s.frame);
        DrawStamp(&mut s);
    } else {
        s.origin = POINT {
            x: LANDER_X,
            y: LANDER_Y,
        };
        let mut i = 0;
        while i < WorldData::current().read(|owner| owner.flagship.num_landers) as COUNT {
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

fn slot_piece(world: &World, state: BYTE, item: SIZE) -> BYTE {
    if state == PLANET_LANDER as BYTE || state == EMPTY_SLOT + 3 {
        if item < world.data.flagship.num_landers as SIZE {
            PLANET_LANDER as BYTE
        } else {
            EMPTY_SLOT + 3
        }
    } else if state == FUSION_THRUSTER as BYTE || state == EMPTY_SLOT {
        world.data.flagship.drive_slots[item as usize]
    } else if state == TURNING_JETS as BYTE || state == EMPTY_SLOT + 1 {
        world.data.flagship.jet_slots[item as usize]
    } else {
        world.data.flagship.module_slots[item as usize]
    }
}

// The C InitFlash label is entered both after a transaction and after moving
// between slot classes. Horizontal movement instead offsets the existing rect.
unsafe fn init_flash(menu: &mut MENU_STATE, piece: BYTE) {
    let state = menu.CurState;
    let mut width = SHIP_PIECE_OFFSET as SIZE;
    let rect = &mut menu.flash_rect0;
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
    rect.corner.x += width * menu.delta_item;
    draw_module_strings(menu, piece);
    if state < EMPTY_SLOT {
        SetFlashRect(MENU_ANY);
    } else {
        SetFlashRect(ptr::addr_of!(menu.flash_rect0));
    }
}

fn module_selection_range(world: &World, state: BYTE) -> (SIZE, SIZE) {
    let mut first: SIZE = 0;
    let mut last = if state == PLANET_LANDER as BYTE || state == EMPTY_SLOT + 3 {
        MAX_LANDERS as SIZE - 1
    } else if state == FUSION_THRUSTER as BYTE || state == EMPTY_SLOT {
        NUM_DRIVE_SLOTS as SIZE - 1
    } else if state == TURNING_JETS as BYTE || state == EMPTY_SLOT + 1 {
        NUM_JET_SLOTS as SIZE - 1
    } else {
        if world.story_state(StoryKey::CHMMR_BOMB_STATE) == 3 {
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
    (first, last)
}

unsafe fn finish_module_transaction(
    menu: &mut MENU_STATE,
    input: &crate::engine::input::InputSession,
    new_piece: BYTE,
    old_piece: BYTE,
    cost: i32,
) {
    DeltaSISGauges(UNDEFINED_DELTA as SIZE, UNDEFINED_DELTA as SIZE, cost);
    if menu.CurState == PLANET_LANDER as BYTE || menu.CurState == EMPTY_SLOT + 3 {
        display_landers(menu);
    } else {
        DrawShipPiece(
            menu.ModuleFrame,
            new_piece as COUNT,
            menu.delta_item as COUNT,
            0,
        );
        if new_piece > TURNING_JETS as BYTE && old_piece > TURNING_JETS as BYTE {
            redistribute_fuel();
        }
        if input.preferences().fonts == OPT_PC as i32 {
            draw_flagship_stats();
        }
    }
}

unsafe fn do_install_module(
    cx: &mut crate::engine::battle::Cx<'_>,
    menu: &mut MENU_STATE,
    input: &crate::engine::input::InputSession,
) -> BOOLEAN {
    if cx.world.activity() & CHECK_ABORT as u16 != 0 {
        menu.InputFunc = Some(do_outfit);
        return 1;
    }
    let select = input.pulsed().menu[KEY_MENU_SELECT as usize] != 0;
    let mut cancel = input.pulsed().menu[KEY_MENU_CANCEL as usize] != 0;
    let left = input.pulsed().menu[KEY_MENU_LEFT as usize] != 0;
    let right = input.pulsed().menu[KEY_MENU_RIGHT as usize] != 0;
    let up = input.pulsed().menu[KEY_MENU_UP as usize] != 0;
    let down = input.pulsed().menu[KEY_MENU_DOWN as usize] != 0;
    SetMenuSounds(input, SOUND_ARROWS, SOUND_SELECT);
    let mut state = menu.CurState;
    let old_piece = slot_piece(cx.world, state, menu.delta_item);
    let (first, last) = module_selection_range(cx.world, state);
    if menu.Initialized == 0 {
        menu.Initialized = 1;
        menu.InputFunc = Some(do_install_module);
        SetContext(cx.kernel.space_context);
        cx.with_legacy(|| ClearSISRect(CLEAR_SIS_RADAR as BYTE));
        cx.with_legacy(|| SetFlashRect(ptr::null()));
        cx.with_legacy(|| init_flash(menu, old_piece));
    } else if select || cancel {
        let mut new_piece = menu.CurState;
        if select && !can_install(cx.world, new_piece, old_piece) {
            crate::ui::settings::play_menu_sound(
                cx.kernel,
                cx.world,
                MENU_SOUND_EFFECT_MENU_SOUND_FAILURE as u16,
            );
            return 1;
        }
        SetContext(cx.kernel.space_context);
        cx.with_legacy(|| SetFlashRect(ptr::null()));
        if select {
            if new_piece >= EMPTY_SLOT && old_piece >= EMPTY_SLOT {
                new_piece -= EMPTY_SLOT - 1;
                if new_piece > CREW_POD as BYTE {
                    new_piece = PLANET_LANDER as BYTE;
                }
            } else {
                let cost = apply_piece(
                    cx.world,
                    menu.CurState,
                    menu.delta_item,
                    new_piece,
                    old_piece,
                );
                cx.with_legacy(|| {
                    finish_module_transaction(menu, input, new_piece, old_piece, cost)
                });
            }
            cancel = false;
        }
        if menu.CurState < EMPTY_SLOT {
            menu.CurState += EMPTY_SLOT - 1;
            if menu.CurState < EMPTY_SLOT {
                menu.CurState = EMPTY_SLOT + 3;
            } else if menu.CurState > EMPTY_SLOT + 2 {
                menu.CurState = EMPTY_SLOT + 2;
            }
            if cancel {
                new_piece = menu.CurState;
            }
            cx.with_legacy(|| init_flash(menu, new_piece));
        } else if !cancel {
            menu.CurState = new_piece;
            cx.with_legacy(|| init_flash(menu, new_piece));
        } else {
            SetContext(cx.kernel.status_context);
            menu.CurState = OUTFIT_MODULES;
            cx.with_legacy(|| DrawMenuStateStrings(PM_FUEL as BYTE, menu.CurState as i16));
            cx.with_legacy(|| SetFlashRect(MENU_3DO));
            menu.InputFunc = Some(do_outfit);
            cx.with_legacy(|| ClearSISRect(DRAW_SIS_DISPLAY as BYTE));
        }
    } else if left || right || up || down {
        let mut item = if state < EMPTY_SLOT {
            menu.CurState as SIZE
        } else {
            menu.delta_item
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
                if cx.world.story_state(StoryKey::CHMMR_BOMB_STATE) == 3 {
                    if state == EMPTY_SLOT + 3 {
                        state = if up { EMPTY_SLOT + 2 } else { EMPTY_SLOT };
                    }
                    if state == EMPTY_SLOT + 2 {
                        item = NUM_BOMB_MODULES as SIZE;
                    }
                }
                menu.delta_item = item;
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
                && (cx.world.data.game.module_cost[item as usize] == 0
                    || (item >= GUN_WEAPON as SIZE
                        && item <= CANNON_WEAPON as SIZE
                        && menu.delta_item > 0
                        && menu.delta_item < 13)))
            {
                break;
            }
        }
        if state < EMPTY_SLOT {
            if item != menu.CurState as SIZE {
                menu.CurState = item as BYTE;
                cx.kernel.sis_highlight.pre_update_flash_rect();
                cx.with_legacy(|| draw_module_strings(menu, item as BYTE));
                cx.kernel.sis_highlight.post_update_flash_rect();
            }
        } else if item != menu.delta_item || state != menu.CurState {
            let piece = slot_piece(cx.world, state, item);
            SetContext(cx.kernel.space_context);
            if state == menu.CurState {
                let width = if state == PLANET_LANDER as BYTE || state == EMPTY_SLOT + 3 {
                    LANDER_WIDTH
                } else {
                    SHIP_PIECE_OFFSET as SIZE
                };
                menu.flash_rect0.corner.x += width * (item - menu.delta_item);
                menu.delta_item = item;
                cx.with_legacy(|| draw_module_strings(menu, piece));
                cx.with_legacy(|| SetFlashRect(ptr::addr_of!(menu.flash_rect0)));
            } else {
                menu.CurState = state;
                cx.with_legacy(|| init_flash(menu, piece));
            }
        }
    }
    1
}
unsafe fn change_fuel_quantity(
    cx: &mut crate::engine::battle::Cx<'_>,
    input: &crate::engine::input::InputSession,
) {
    let highlight = cx.kernel.sis_highlight.clone();
    let mut incr: i32 = if input.pulsed().menu[KEY_MENU_UP as usize] != 0 {
        FUEL_TANK_SCALE as i32
    } else if input.pulsed().menu[KEY_MENU_DOWN as usize] != 0 {
        -(FUEL_TANK_SCALE as i32)
    } else if input.pulsed().menu[KEY_MENU_PAGE_UP as usize] != 0 {
        FUEL_VOLUME_PER_ROW as i32
    } else if input.pulsed().menu[KEY_MENU_PAGE_DOWN as usize] != 0 {
        -(FUEL_VOLUME_PER_ROW as i32)
    } else {
        return;
    };
    let max_fit = cx
        .with_legacy(|| GetFuelTankCapacity())
        .wrapping_sub(cx.world.data.flagship.fuel_on_board) as i32;
    let max_afford =
        (cx.world.data.flagship.res_units / cx.world.data.game.fuel_cost as DWORD) as i32;
    let min_fit = -(cx.world.data.flagship.fuel_on_board as i32);
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
        crate::ui::settings::play_menu_sound(
            cx.kernel,
            cx.world,
            MENU_SOUND_EFFECT_MENU_SOUND_FAILURE as u16,
        );
    } else {
        let cost = incr / FUEL_TANK_SCALE as i32 * cx.world.data.game.fuel_cost as i32;
        cx.with_legacy(|| {
            highlight.pre_update_flash_rect();
            DeltaSISGauges(0, incr as SIZE, -cost);
            highlight.post_update_flash_rect();
            redistribute_fuel();
        });
    }
    let context = cx.kernel.status_context;
    cx.with_legacy(|| {
        let mut r: RECT = mem::zeroed();
        let old = SetContext(context);
        GetGaugeRect(&mut r, 0);
        SetFlashRect(&r);
        SetContext(old);
    });
}

unsafe extern "C" fn on_naming_done() {
    DrawFlagshipName(0);
}

unsafe fn exit_outfit(menu: &mut MENU_STATE) -> BOOLEAN {
    DestroyDrawable(ReleaseDrawable(menu.CurFrame));
    menu.CurFrame = ptr::null_mut();
    DestroyDrawable(ReleaseDrawable(menu.ModuleFrame));
    menu.ModuleFrame = ptr::null_mut();
    SetNamingCallback(None);
    0
}

/// Process an Outfit input frame, retaining installation state in `MENU_STATE`.
///
/// # Safety
/// `menu` must be the live game-thread menu. Game, resources, input, graphics
/// and audio must be initialized and exclusively accessed on that thread.
unsafe fn initialize_outfit(
    cx: &mut crate::engine::battle::Cx<'_>,
    menu: &mut MENU_STATE,
    input: &crate::engine::input::InputSession,
) {
    menu.InputFunc = Some(do_outfit);
    menu.Initialized = 1;
    SetNamingCallback(Some(on_naming_done));
    menu.CurFrame = CaptureDrawable(LoadGraphicInstance(c"graphics.modulesmenu".as_ptr()).cast());
    menu.hMusic = (LoadMusicInstance(c"music.outfit".as_ptr())).cast();
    menu.CurState = OUTFIT_FUEL;
    menu.ModuleFrame =
        CaptureDrawable(LoadGraphicInstance(c"graphics.outfitmodules".as_ptr()).cast());
    let mut s = STAMP {
        origin: POINT { x: 0, y: 0 },
        frame: CaptureDrawable(LoadGraphicInstance(c"graphics.outfit".as_ptr()).cast()),
    };
    SetTransitionSource(ptr::null());
    BatchGraphics();
    cx.with_legacy(|| {
        draw_sisframe();
        DrawSISMessage(game_string(STARBASE_STRING_BASE + 2));
        DrawSISTitle(game_string(STARBASE_STRING_BASE));
    });
    SetContext(cx.kernel.space_context);
    DrawStamp(&mut s);
    DestroyDrawable(ReleaseDrawable(s.frame));
    let slots = cx.world.data.flagship;
    for pieces in [
        slots.drive_slots.as_slice(),
        slots.jet_slots.as_slice(),
        slots.module_slots.as_slice(),
    ] {
        for (slot, &piece) in pieces.iter().enumerate() {
            if piece < EMPTY_SLOT {
                cx.with_legacy(|| {
                    DrawShipPiece(menu.ModuleFrame, piece as COUNT, slot as COUNT, 0)
                });
            }
        }
    }
    cx.with_legacy(|| {
        redistribute_fuel();
        display_landers(menu);
    });
    if cx.world.story_state(StoryKey::CHMMR_BOMB_STATE) < 3 {
        let shields = cx.world.story_state(StoryKey::LANDER_SHIELDS);
        s.frame = SetAbsFrameIndex(menu.ModuleFrame, GetFrameCount(menu.ModuleFrame) - 5);
        for (index, disaster) in LANDER_DISASTERS.into_iter().enumerate() {
            if index != 0 {
                s.frame = IncFrameIndex(s.frame);
            }
            if shields & (1 << disaster) != 0 {
                DrawStamp(&mut s);
            }
        }
    }
    cx.with_legacy(|| {
        DrawMenuStateStrings(PM_FUEL as BYTE, menu.CurState as i16);
        DrawFlagshipName(0);
    });
    if input.preferences().fonts == OPT_PC as i32 {
        cx.with_legacy(|| draw_flagship_stats());
    }
    cx.with_legacy(|| {
        ScreenTransition(3, ptr::null());
        PlayMusic(menu.hMusic, 1, 1);
        UnbatchGraphics();
        SetFlashRect(MENU_3DO);
    });
    cx.world.data.flagship.fuel_on_board = cx
        .world
        .data
        .flagship
        .fuel_on_board
        .wrapping_add(FUEL_TANK_SCALE >> 1)
        / FUEL_TANK_SCALE;
    cx.world.data.flagship.fuel_on_board *= FUEL_TANK_SCALE;
    SetContext(cx.kernel.status_context);
}

/// # Safety
/// The menu's native graphics resources must remain initialized during this frame.
pub unsafe fn do_outfit(
    cx: &mut crate::engine::battle::Cx<'_>,
    menu: &mut MENU_STATE,
    input: &crate::engine::input::InputSession,
) -> BOOLEAN {
    if cx.world.activity() & CHECK_ABORT as u16 != 0 {
        return cx.with_legacy(|| exit_outfit(menu));
    }
    if menu.Initialized == 0 {
        initialize_outfit(cx, menu, input);
    } else if input.pulsed().menu[KEY_MENU_CANCEL as usize] != 0
        || (input.pulsed().menu[KEY_MENU_SELECT as usize] != 0 && menu.CurState == OUTFIT_EXIT)
    {
        if menu.CurState == OUTFIT_DOFUEL {
            menu.CurState = OUTFIT_FUEL;
            cx.with_legacy(|| SetFlashRect(MENU_3DO));
        } else {
            return cx.with_legacy(|| exit_outfit(menu));
        }
    } else if input.pulsed().menu[KEY_MENU_SELECT as usize] != 0 {
        match menu.CurState {
            OUTFIT_FUEL => {
                menu.CurState = OUTFIT_DOFUEL;
                let context = cx.kernel.status_context;
                cx.with_legacy(|| {
                    SetContext(context);
                    let mut r: RECT = mem::zeroed();
                    GetGaugeRect(&mut r, 0);
                    SetFlashRect(&r);
                });
            }
            OUTFIT_DOFUEL => {
                menu.CurState = OUTFIT_FUEL;
                cx.with_legacy(|| SetFlashRect(MENU_3DO));
            }
            OUTFIT_MODULES => {
                menu.CurState = EMPTY_SLOT + 2;
                menu.delta_item = if cx.world.story_state(StoryKey::CHMMR_BOMB_STATE) != 3 {
                    0
                } else {
                    NUM_BOMB_MODULES as SIZE
                };
                menu.first_item.y = 0;
                menu.Initialized = 0;
                do_install_module(cx, menu, input);
            }
            OUTFIT_SAVELOAD => {
                if cx.with_legacy(|| GameOptions()) == 0 {
                    return cx.with_legacy(|| exit_outfit(menu));
                }
                cx.with_legacy(|| {
                    DrawMenuStateStrings(PM_FUEL as BYTE, menu.CurState as i16);
                    SetFlashRect(MENU_3DO);
                });
            }
            _ => {}
        }
    } else {
        if menu.CurState == OUTFIT_DOFUEL {
            SetMenuSounds(input, SOUND_FUEL, SOUND_SELECT | SOUND_CANCEL);
        } else {
            SetMenuSounds(input, SOUND_ARROWS, SOUND_SELECT);
        }
        if menu.CurState == OUTFIT_DOFUEL {
            change_fuel_quantity(cx, input);
            cx.with_legacy(|| SleepThread(ONE_SECOND / 30));
        } else {
            cx.with_legacy(|| do_menu_chooser(input, menu, PM_FUEL as BYTE));
        }
    }
    1
}
