//! Outfit installation, fuel transactions and menu callbacks, ported from outfit.c.

use super::menu_state::PM_FUEL;
use super::session::{draw_piece, MenuAssets};
use crate::engine::input::frames::*;
use crate::engine::observe::Io;
use crate::engine::world::campaign::*;
use crate::engine::World;
use crate::engine::{battle::Cx, input::InputSession, StoryKey};
use crate::graphics::native_abi::*;
use crate::graphics::{drawable_abi::LoadedDrawable, menu as draw};
use crate::ui::menu::draw_campaign_menu;
use crate::ui::sis::{
    clear_flash, clear_sis_rect, delta_sis_gauges, draw_campaign_message, draw_flagship_name,
    draw_flagship_stats, draw_title, highlight_rect, set_menu_flash,
};

use crate::state::game_state_keys::get_game_state;
use std::ptr;

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
const SOUND_ARROWS: u16 = 0x0f;
const SOUND_SELECT: u16 = 1 << 4;
const SOUND_CANCEL: u16 = 1 << 5;
const SOUND_FUEL: u16 = (1 << 0) | (1 << 1) | (1 << 7) | (1 << 8);

const fn rgb15(r: u8, g: u8, b: u8) -> Color {
    Color {
        r: (r << 3) | (r >> 2),
        g: (g << 3) | (g >> 2),
        b: (b << 3) | (b >> 2),
        a: 255,
    }
}
const BLACK: Color = rgb15(0, 0, 0);

fn draw_module_strings(cx: &mut Cx<'_>, menu: &mut OutfitSession, module: BYTE) {
    let old = draw::select(cx.kernel.status_context);
    let mut r = draw::clip();
    let mut s = STAMP {
        origin: POINT {
            x: (4 + cx.screen_dimensions().0 - STATUS_WIDTH as i32 - SAFE_X as i32) as i16
                - r.corner.x,
            y: (SIS_ORG_Y as i32 + cx.screen_dimensions().1
                - SAFE_Y as i32 * 2
                - 13
                - RADAR_HEIGHT as i32) as i16
                - r.corner.y,
        },
        frame: ptr::null_mut(),
    };
    r.corner.x = s.origin.x - 1;
    r.corner.y = s.origin.y - 11;
    r.extent.width = RADAR_WIDTH as i16 + 2;
    r.extent.height = 11;
    draw::batch();
    draw::foreground(rgb15(0x0a, 0x0a, 0x0a));
    draw::fill(r);
    if module >= EMPTY_SLOT {
        r.corner = s.origin;
        r.extent.width = RADAR_WIDTH as i16;
        r.extent.height = RADAR_HEIGHT as i16;
        draw::foreground(BLACK);
        draw::fill(r);
    } else if !menu.assets.art().is_null() {
        s.frame = draw::frame(menu.assets.art(), module as COUNT);
        draw::stamp(s.frame, s.origin);
        let buffer = format!(
            "{}\0",
            cx.world.data.game.module_cost[module as usize] as u32 * MODULE_COST_SCALE
        );
        draw::font(cx.kernel.tiny_font);
        draw::foreground(rgb15(0, 0x1f, 0));
        super::session::text(
            &buffer,
            POINT {
                x: s.origin.x + RADAR_WIDTH as i16 - 2,
                y: s.origin.y + RADAR_HEIGHT as i16 - 2,
            },
            TEXT_ALIGN_ALIGN_RIGHT,
        );
    }
    draw::unbatch();
    draw::select(old);
}

fn display_landers(cx: &mut Cx<'_>, menu: &mut OutfitSession) {
    let mut s = STAMP {
        origin: POINT { x: 0, y: 0 },
        frame: menu.assets.modules(),
    };
    if get_game_state(cx.world, StoryKey::CHMMR_BOMB_STATE) == 3 {
        s.frame = draw::relative_frame(s.frame, -1);
        draw::stamp(s.frame, s.origin);
    } else {
        s.origin = POINT {
            x: LANDER_X,
            y: LANDER_Y,
        };
        let mut i = 0;
        while i < cx.world.data.flagship.num_landers as COUNT {
            draw::stamp(s.frame, s.origin);
            s.origin.x += LANDER_WIDTH;
            i += 1;
        }
        draw::foreground(BLACK);
        while i < MAX_LANDERS as COUNT {
            draw::filled_stamp(s.frame, s.origin);
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
fn init_flash(cx: &mut Cx<'_>, menu: &mut OutfitSession, piece: BYTE) {
    let state = menu.state;
    let mut width = SHIP_PIECE_OFFSET as SIZE;
    let rect = &mut menu.flash_rect;
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
    draw_module_strings(cx, menu, piece);
    if state < EMPTY_SLOT {
        set_menu_flash(cx.kernel, cx.screen_dimensions(), true);
    } else {
        highlight_rect(cx.kernel, menu.flash_rect);
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

fn finish_module_transaction(
    cx: &mut Cx<'_>,
    menu: &mut OutfitSession,
    input: &InputSession,
    new_piece: BYTE,
    old_piece: BYTE,
    cost: i32,
) {
    delta_sis_gauges(
        cx.kernel,
        cx.world,
        UNDEFINED_DELTA as SIZE,
        UNDEFINED_DELTA as SIZE,
        cost,
    );
    if menu.state == PLANET_LANDER as BYTE || menu.state == EMPTY_SLOT + 3 {
        display_landers(cx, menu);
    } else {
        draw_piece(
            cx.kernel,
            &cx.world.data.flagship,
            cx.screen_dimensions(),
            menu.assets.modules(),
            new_piece as COUNT,
            menu.delta_item as COUNT,
            false,
        );
        if new_piece > TURNING_JETS as BYTE && old_piece > TURNING_JETS as BYTE {
            crate::ui::sis::capacity::draw_fuel_bars(
                &crate::graphics::context_abi::ContextLease::require(cx.kernel.space_context),
                &cx.world.data.flagship,
            );
        }
        if input.preferences().fonts == OPT_PC as i32 {
            draw_flagship_stats(cx.kernel, cx.world, cx.screen_dimensions());
        }
    }
}

fn do_install_module(cx: &mut Cx<'_>, menu: &mut OutfitSession, input: &InputSession) -> u32 {
    let size = cx.screen_dimensions();
    if cx.world.activity() & CHECK_ABORT as u16 != 0 {
        menu.page = OutfitPage::Main;
        return 1;
    }
    let select = input.pulsed().menu[KEY_MENU_SELECT as usize] != 0;
    let mut cancel = input.pulsed().menu[KEY_MENU_CANCEL as usize] != 0;
    let left = input.pulsed().menu[KEY_MENU_LEFT as usize] != 0;
    let right = input.pulsed().menu[KEY_MENU_RIGHT as usize] != 0;
    let up = input.pulsed().menu[KEY_MENU_UP as usize] != 0;
    let down = input.pulsed().menu[KEY_MENU_DOWN as usize] != 0;
    input.set_menu_sounds(SOUND_ARROWS, SOUND_SELECT);
    let mut state = menu.state;
    let old_piece = slot_piece(cx.world, state, menu.delta_item);
    let (first, last) = module_selection_range(cx.world, state);
    if !menu.initialized {
        menu.initialized = true;
        menu.page = OutfitPage::Install;
        draw::select(cx.kernel.space_context);
        clear_sis_rect(cx.kernel, cx.world, size, CLEAR_SIS_RADAR as BYTE);
        clear_flash(cx.kernel);
        init_flash(cx, menu, old_piece);
    } else if select || cancel {
        let mut new_piece = menu.state;
        if select && !can_install(cx.world, new_piece, old_piece) {
            crate::ui::settings::play_menu_sound(
                cx.kernel,
                cx.world,
                MENU_SOUND_EFFECT_MENU_SOUND_FAILURE as u16,
            );
            return 1;
        }
        draw::select(cx.kernel.space_context);
        clear_flash(cx.kernel);
        if select {
            if new_piece >= EMPTY_SLOT && old_piece >= EMPTY_SLOT {
                new_piece -= EMPTY_SLOT - 1;
                if new_piece > CREW_POD as BYTE {
                    new_piece = PLANET_LANDER as BYTE;
                }
            } else {
                let slot = menu.delta_item;
                let cost = apply_piece(cx.world, menu.state, slot, new_piece, old_piece);
                finish_module_transaction(cx, menu, input, new_piece, old_piece, cost);
            }
            cancel = false;
        }
        if menu.state < EMPTY_SLOT {
            menu.state += EMPTY_SLOT - 1;
            if menu.state < EMPTY_SLOT {
                menu.state = EMPTY_SLOT + 3;
            } else if menu.state > EMPTY_SLOT + 2 {
                menu.state = EMPTY_SLOT + 2;
            }
            if cancel {
                new_piece = menu.state;
            }
            init_flash(cx, menu, new_piece);
        } else if !cancel {
            menu.state = new_piece;
            init_flash(cx, menu, new_piece);
        } else {
            draw::select(cx.kernel.status_context);
            menu.state = OUTFIT_MODULES;
            draw_campaign_menu(cx, PM_FUEL as BYTE, menu.state as i16);
            set_menu_flash(cx.kernel, size, false);
            menu.page = OutfitPage::Main;
            clear_sis_rect(cx.kernel, cx.world, size, DRAW_SIS_DISPLAY as BYTE);
        }
    } else if left || right || up || down {
        let mut item = if state < EMPTY_SLOT {
            menu.state as SIZE
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
            if item != menu.state as SIZE {
                menu.state = item as BYTE;
                cx.kernel.sis_highlight.pre_update_flash_rect();
                draw_module_strings(cx, menu, item as BYTE);
                cx.kernel.sis_highlight.post_update_flash_rect();
            }
        } else if item != menu.delta_item || state != menu.state {
            let piece = slot_piece(cx.world, state, item);
            draw::select(cx.kernel.space_context);
            if state == menu.state {
                let width = if state == PLANET_LANDER as BYTE || state == EMPTY_SLOT + 3 {
                    LANDER_WIDTH
                } else {
                    SHIP_PIECE_OFFSET as SIZE
                };
                menu.flash_rect.corner.x += width * (item - menu.delta_item);
                menu.delta_item = item;
                draw_module_strings(cx, menu, piece);
                highlight_rect(cx.kernel, menu.flash_rect);
            } else {
                menu.state = state;
                init_flash(cx, menu, piece);
            }
        }
    }
    1
}
fn change_fuel_quantity(cx: &mut Cx<'_>, input: &InputSession) {
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
    let max_fit = crate::ui::sis::capacity::fuel_capacity(&cx.world.data.flagship)
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
        {
            highlight.pre_update_flash_rect();
            delta_sis_gauges(cx.kernel, cx.world, 0, incr as SIZE, -cost);
            highlight.post_update_flash_rect();
        };
        crate::ui::sis::capacity::draw_fuel_bars(
            &crate::graphics::context_abi::ContextLease::require(cx.kernel.space_context),
            &cx.world.data.flagship,
        );
    }
    let context = cx.kernel.status_context;
    {
        let old = draw::select(context);
        let r = crate::ui::sis::gauges::gauge_rect(false);
        highlight_rect(cx.kernel, r);
        draw::select(old);
    };
}

fn on_naming_done(kernel: &crate::engine::Kernel, world: &crate::engine::World, width: i32) {
    crate::ui::sis::gauges::draw_flagship_name(kernel, world, width, false);
}

fn exit_outfit(kernel: &mut crate::engine::Kernel, menu: &mut OutfitSession) -> BOOLEAN {
    menu.assets.art = None;
    menu.assets.modules = None;
    kernel.naming_callback = None;
    0
}

/// Process an Outfit input frame, retaining installation state in `OutfitSession`.
///
fn initialize_outfit(cx: &mut Cx<'_>, menu: &mut OutfitSession, input: &InputSession) {
    let size = cx.screen_dimensions();
    menu.page = OutfitPage::Main;
    menu.initialized = true;
    cx.kernel.naming_callback = Some(on_naming_done);
    menu.assets.art = LoadedDrawable::load(c"graphics.modulesmenu");
    menu.assets.music = crate::sound::resource_types::LoadedMusic::load(c"music.outfit");
    menu.state = OUTFIT_FUEL;
    menu.assets.modules = LoadedDrawable::load(c"graphics.outfitmodules");
    let background = LoadedDrawable::load(c"graphics.outfit")
        .unwrap_or_else(|| panic!("outfit background missing"));
    let mut s = STAMP {
        origin: POINT { x: 0, y: 0 },
        frame: background.lease().frame(0).unwrap(),
    };
    draw::transition_source();
    draw::batch();

    crate::ui::border::draw_sis_frame(cx.kernel, cx.world, size);
    let message = cx.kernel.game_text((STARBASE_STRING_BASE + 2) as usize);
    let title = cx.kernel.game_text(STARBASE_STRING_BASE as usize);
    draw_campaign_message(cx.kernel, cx.world, size.0, Some(message));
    draw_title(cx.kernel, size.0, title);
    draw::select(cx.kernel.space_context);
    draw::stamp(s.frame, s.origin);
    drop(background);
    let slots = cx.world.data.flagship;
    for pieces in [
        slots.drive_slots.as_slice(),
        slots.jet_slots.as_slice(),
        slots.module_slots.as_slice(),
    ] {
        for (slot, &piece) in pieces.iter().enumerate() {
            if piece < EMPTY_SLOT {
                draw_piece(
                    cx.kernel,
                    &cx.world.data.flagship,
                    cx.screen_dimensions(),
                    menu.assets.modules(),
                    piece as COUNT,
                    slot as COUNT,
                    false,
                );
            }
        }
    }
    crate::ui::sis::capacity::draw_fuel_bars(
        &crate::graphics::context_abi::ContextLease::require(cx.kernel.space_context),
        &cx.world.data.flagship,
    );
    display_landers(cx, menu);
    if cx.world.story_state(StoryKey::CHMMR_BOMB_STATE) < 3 {
        let shields = cx.world.story_state(StoryKey::LANDER_SHIELDS);
        s.frame = draw::frame(
            menu.assets.modules(),
            draw::frame_count(menu.assets.modules()) - 5,
        );
        for (index, disaster) in LANDER_DISASTERS.into_iter().enumerate() {
            if index != 0 {
                s.frame = draw::relative_frame(s.frame, 1);
            }
            if shields & (1 << disaster) != 0 {
                draw::stamp(s.frame, s.origin);
            }
        }
    }
    {
        draw_campaign_menu(cx, PM_FUEL as BYTE, menu.state as i16);
        draw_flagship_name(cx.kernel, cx.world, size.0, false);
    };
    if input.preferences().fonts == OPT_PC as i32 {
        draw_flagship_stats(cx.kernel, cx.world, size);
    }
    {
        draw::transition(3);
        crate::ui::settings::play_loaded_music(cx.world, menu.assets.music.as_ref(), 1, 1);
        draw::unbatch();
        set_menu_flash(cx.kernel, size, false);
    };
    cx.world.data.flagship.fuel_on_board = cx
        .world
        .data
        .flagship
        .fuel_on_board
        .wrapping_add(FUEL_TANK_SCALE >> 1)
        / FUEL_TANK_SCALE;
    cx.world.data.flagship.fuel_on_board *= FUEL_TANK_SCALE;
    draw::select(cx.kernel.status_context);
}

fn do_outfit(
    cx: &mut Cx<'_>,
    menu: &mut OutfitSession,
    input: &InputSession,
    io: &mut Io<'_>,
    services: &super::MenuServices,
) -> BOOLEAN {
    let size = cx.screen_dimensions();
    if cx.world.activity() & CHECK_ABORT as u16 != 0 {
        return exit_outfit(cx.kernel, menu);
    }
    if !menu.initialized {
        initialize_outfit(cx, menu, input);
    } else if input.pulsed().menu[KEY_MENU_CANCEL as usize] != 0
        || (input.pulsed().menu[KEY_MENU_SELECT as usize] != 0 && menu.state == OUTFIT_EXIT)
    {
        if menu.state == OUTFIT_DOFUEL {
            menu.state = OUTFIT_FUEL;
            set_menu_flash(cx.kernel, size, false);
        } else {
            return exit_outfit(cx.kernel, menu);
        }
    } else if input.pulsed().menu[KEY_MENU_SELECT as usize] != 0 {
        match menu.state {
            OUTFIT_FUEL => {
                menu.state = OUTFIT_DOFUEL;
                let context = cx.kernel.status_context;
                {
                    draw::select(context);

                    let r = crate::ui::sis::gauges::gauge_rect(false);
                    highlight_rect(cx.kernel, r);
                };
            }
            OUTFIT_DOFUEL => {
                menu.state = OUTFIT_FUEL;
                set_menu_flash(cx.kernel, size, false);
            }
            OUTFIT_MODULES => {
                menu.state = EMPTY_SLOT + 2;
                menu.delta_item = if cx.world.story_state(StoryKey::CHMMR_BOMB_STATE) != 3 {
                    0
                } else {
                    NUM_BOMB_MODULES as SIZE
                };
                menu.initialized = false;
                do_install_module(cx, menu, input);
            }
            OUTFIT_SAVELOAD => {
                if (services.game_options)(cx, io) == 0 {
                    return exit_outfit(cx.kernel, menu);
                }
                {
                    draw_campaign_menu(cx, PM_FUEL as BYTE, menu.state as i16);
                    set_menu_flash(cx.kernel, size, false);
                };
            }
            _ => {}
        }
    } else {
        if menu.state == OUTFIT_DOFUEL {
            input.set_menu_sounds(SOUND_FUEL, SOUND_SELECT | SOUND_CANCEL);
        } else {
            input.set_menu_sounds(SOUND_ARROWS, SOUND_SELECT);
        }
        if menu.state == OUTFIT_DOFUEL {
            change_fuel_quantity(cx, input);
            cx.kernel.input_runtime().sleep(cx, ONE_SECOND / 30);
        } else {
            crate::ui::menu::choose_menu_state(cx, input, &mut menu.state, PM_FUEL as BYTE);
        }
    }
    1
}

#[derive(Default)]
enum OutfitPage {
    #[default]
    Main,
    Install,
}
struct OutfitSession {
    initialized: bool,
    state: u8,
    delta_item: i16,
    flash_rect: RECT,
    assets: MenuAssets,
    page: OutfitPage,
}
impl Default for OutfitSession {
    fn default() -> Self {
        Self {
            initialized: false,
            state: 0,
            delta_item: 0,
            flash_rect: RECT {
                corner: POINT { x: 0, y: 0 },
                extent: EXTENT {
                    width: 0,
                    height: 0,
                },
            },
            assets: MenuAssets::default(),
            page: OutfitPage::Main,
        }
    }
}
pub fn run(
    cx: &mut Cx<'_>,
    io: &mut Io<'_>,
    services: &super::MenuServices,
) -> Option<crate::sound::resource_types::LoadedMusic> {
    let runtime = cx.kernel.input_runtime();
    let input = runtime.input();
    let mut menu = OutfitSession::default();
    cx.run_input(io, true, |cx, io| match menu.page {
        OutfitPage::Main => do_outfit(cx, &mut menu, input, io, services) != 0,
        OutfitPage::Install => do_install_module(cx, &mut menu, input) != 0,
    });
    cx.kernel.naming_callback = None;
    menu.assets.music.take()
}
