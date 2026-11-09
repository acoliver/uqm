//! Shipyard menu, crew transactions and hangar animation, ported from shipyard.c.
use crate::mainloop::types::activity_flags;
use crate::starbase::ids;
use crate::starbase::sis_modules as equipment;
use crate::starbase::sis_modules::ModuleType;
use crate::ui::sis;
use uqm_engine::time::counter_abi::ONE_SECOND;

use super::menu_state::PM_CREW;
use super::session::{draw_piece, MenuAssets};
use super::shipyard_catalog as catalog;
use super::shipyard_crew as crew;
use super::shipyard_selection::{
    dms_flash_active_ship, dms_flash_escort_ship_crew_count, dms_flash_flag_ship_crew_count,
    dms_set_mode, ModifyMode, SOUND_ARROWS, SOUND_SELECT,
};
use super::{shipyard_frame as frame, shipyard_hangar as hangar};
use crate::engine::observe::Io;
use crate::graphics::{
    context_abi::ContextFrame, drawable_abi::LoadedDrawable, menu as draw,
    palette_cycle::PaletteCycle,
};
use crate::ui::{rgb15, settings::play_menu_sound};
use catalog::{get_available_race_count, get_available_race_from_index};
use crew::{
    crew_transaction, dms_dismiss_escort_ship_crew, dms_dismiss_flag_ship_crew,
    dms_hire_escort_ship_crew, dms_hire_flag_ship_crew, show_ship_crew,
};
use std::{cell::RefCell, rc::Rc};

use crate::engine::battle::{link_queue::LinkId, Cx};
use crate::engine::input::frames::*;
use crate::engine::world::queues::Fragment;
use crate::engine::{input::InputSession, World};
use crate::graphics::native_abi::*;

use crate::ui::menu::draw_campaign_menu;
use sis::{
    clear_flash, clear_sis_rect, delta_sis_gauges, draw_campaign_message, draw_title,
    set_menu_flash,
};

use std::ptr;

use super::shipyard_layout::{
    DMS_GetEscortShipRect, DMS_MoveCursor, SHIP_WIN_HEIGHT, SHIP_WIN_WIDTH, USE_3DO_HANGAR,
};
const HANGAR_ANIM_RATE: u32 = 15;
const SHIPYARD: u32 = 2;
const SHIPYARD_CREW: u8 = 0;
const SHIPYARD_SAVELOAD: u8 = 1;
const SHIPYARD_EXIT: u8 = 2;
const SHIP_WIN_FRAMES: i32 = ((SHIP_WIN_WIDTH >> 1) + 1) as i32;
const MODIFY_CREW_FLAG: SIZE = 1 << 8;
use crate::starbase::shipyard::SHIP_COST;

const BLACK: Color = rgb15(0, 0, 0);

struct PowerLines {
    frame: ContextFrame,
    palette: PaletteCycle,
    next_time: u32,
}
impl PowerLines {
    fn draw(&mut self, force: bool) {
        let now = uqm_engine::time::source::now_ticks();
        if now >= self.next_time || force {
            self.next_time = now.wrapping_add(ONE_SECOND / HANGAR_ANIM_RATE);
            self.palette.apply();
            draw::stamp(self.frame.as_raw(), POINT { x: 0, y: 0 });
            self.palette.advance();
        }
    }
}
fn animate_power_lines(menu: &ShipyardSession, force: bool) {
    menu.power
        .as_ref()
        .unwrap_or_else(|| panic!("hangar animation not loaded"))
        .borrow_mut()
        .draw(force);
}

fn draw_race_strings(
    cx: &mut crate::engine::battle::Cx<'_>,
    menu: &mut ShipyardSession,
    mut race: BYTE,
) {
    let old = draw::select(cx.kernel.status_context);
    let mut r = draw::clip();
    let mut stamp = STAMP {
        origin: POINT {
            x: (4 + cx.screen_dimensions().0 - ids::STATUS_WIDTH as i32 - ids::SAFE_X as i32)
                as i16
                - r.corner.x,
            y: (ids::SIS_ORG_Y as i32 + cx.screen_dimensions().1
                - ids::SAFE_Y as i32 * 2
                - 13
                - ids::RADAR_HEIGHT as i32) as i16
                - r.corner.y,
        },
        frame: ptr::null_mut(),
    };
    r.corner.x = stamp.origin.x - 1;
    r.corner.y = stamp.origin.y - 11;
    r.extent.width = ids::RADAR_WIDTH as i16 + 2;
    r.extent.height = 11;
    draw::batch();
    {
        let dimensions = cx.screen_dimensions();
        clear_sis_rect(cx.kernel, cx.world, dimensions, sis::CLEAR_RADAR as BYTE)
    };
    draw::foreground(rgb15(0x0a, 0x0a, 0x0a));
    draw::fill(r);
    r.corner = stamp.origin;
    r.extent.width = ids::RADAR_WIDTH as i16;
    r.extent.height = ids::RADAR_HEIGHT as i16;
    draw::foreground(BLACK);
    draw::fill(r);
    if race != BYTE::MAX {
        let Some((index, ship)) = get_available_race_from_index(cx.world, race) else {
            panic!("the selected hangar race must be allied");
        };
        race = index as BYTE;
        stamp.frame = draw::frame(menu.assets.modules(), 3 + race as COUNT);
        draw::stamp(stamp.frame, stamp.origin);
        stamp.frame = ship.melee_icon;
        let baseline = POINT {
            x: stamp.origin.x + ids::RADAR_WIDTH as i16 - 2,
            y: stamp.origin.y + ids::RADAR_HEIGHT as i16 - 2,
        };
        stamp.origin.x += (ids::RADAR_WIDTH >> 1) as i16;
        stamp.origin.y += (ids::RADAR_HEIGHT >> 1) as i16;
        draw::stamp(stamp.frame, stamp.origin);
        let cost = format!("{}\0", SHIP_COST[race as usize]);
        draw::font(cx.kernel.tiny_font);
        draw::foreground(rgb15(0, 0x1f, 0));
        super::session::text(&cost, baseline, TEXT_ALIGN_ALIGN_RIGHT);
    }
    draw::unbatch();
    draw::select(old);
}

fn show_combat_ship(
    cx: &mut Cx<'_>,
    input: &InputSession,
    menu: &mut ShipyardSession,
    window: COUNT,
    yanked: Option<LinkId<Fragment>>,
) {
    let (mut windows, num_ships) = {
        with_built_ships(cx.world, |queue| {
            hangar::prepare_windows(queue, menu.assets.modules(), window, yanked)
        })
    };
    if num_ships != 0 {
        let mut all_finished = false;
        let r = RECT {
            corner: POINT { x: 0, y: 0 },
            extent: EXTENT {
                width: SHIP_WIN_WIDTH,
                height: SHIP_WIN_HEIGHT,
            },
        };
        cx.kernel.input_runtime().flush();
        let mut time_in = uqm_engine::time::source::now_ticks();
        let mut j = 0;
        while j < SHIP_WIN_FRAMES && !all_finished {
            cx.kernel
                .input_runtime()
                .sleep_until(cx, time_in.wrapping_add(ONE_SECOND / 24));
            time_in = uqm_engine::time::source::now_ticks();
            cx.kernel.input_runtime().update(cx);
            if input.current().any_button_pressed() {
                if yanked.is_some() {
                    windows[0].left_door.origin.x = 0;
                    windows[0].right_door.origin.x = 0;
                }
                all_finished = true;
            }
            let old = draw::select(cx.kernel.space_context);
            let old_clip = draw::clip();
            draw::background(BLACK);
            draw::batch();
            for i in 0..num_ships {
                let info = &mut windows[i as usize];
                let clip = RECT {
                    corner: POINT {
                        x: ids::SIS_ORG_X as i16 + info.finished.x,
                        y: ids::SIS_ORG_Y as i16 + info.finished.y,
                    },
                    extent: EXTENT {
                        width: SHIP_WIN_WIDTH,
                        height: SHIP_WIN_HEIGHT,
                    },
                };
                draw::set_clip(clip);
                draw::clear();
                draw::stamp(info.image.frame, info.image.origin);
                let Some(ship) = info.ship else {
                    panic!("a prepared hangar window must have a ship");
                };
                show_ship_crew(cx.world, ship, &r);
                if !all_finished || yanked.is_some() {
                    draw::stamp(info.left_door.frame, info.left_door.origin);
                    draw::stamp(info.right_door.frame, info.right_door.origin);
                    if yanked.is_some() {
                        info.left_door.origin.x += 1;
                        info.right_door.origin.x -= 1;
                    } else {
                        info.left_door.origin.x -= 1;
                        info.right_door.origin.x += 1;
                    }
                }
            }
            draw::set_clip(old_clip);
            if !USE_3DO_HANGAR {
                animate_power_lines(menu, false);
            }
            draw::unbatch();
            draw::select(old);
            j += 1;
        }
    }
}

fn dms_spin_ship(
    cx: &mut crate::engine::battle::Cx<'_>,
    menu: &mut ShipyardSession,
    ship: Option<LinkId<Fragment>>,
    io: &mut Io<'_>,
    services: &super::MenuServices,
) -> bool {
    if menu.state >> 4 != 0 {
        return false;
    }
    if ship.is_none() == (menu.delta_item & MODIFY_CREW_FLAG == 0) {
        return false;
    }
    let spin_ship = if let Some(id) = ship {
        let race = with_built_ships(cx.world, |queue| queue.get(id).race_id);
        {
            with_avail_races(cx.world, |queue| {
                queue.at(usize::from(race)).map(|id| *queue.get(id))
            })
        }
    } else {
        get_available_race_from_index(cx.world, menu.delta_item as BYTE).map(|(_, fleet)| fleet)
    };
    let Some(spin_ship) = spin_ship else {
        return false;
    };
    clear_flash(cx.kernel);
    use crate::ships::master_native::find;
    let species = spin_ship.species_id;
    let (context, index) = (
        cx.kernel.screen_context,
        find(&cx.kernel.master_queue, species),
    );
    let old = draw::select(context);
    let old_clip = draw::clip();
    if index >= 0 && index < crate::supermelee::types::NUM_MELEE_SHIPS as i32 {
        (services.ship_spin)(cx, io, index as COUNT, menu.assets.music.as_ref());
    }
    draw::set_clip(old_clip);
    draw::select(old);
    true
}

fn dms_modify_crew(
    cx: &mut Cx<'_>,
    input: &InputSession,
    menu: &mut ShipyardSession,
    handle: Option<LinkId<Fragment>>,
    dy: i8,
) {
    let delta = match handle {
        None => {
            let delta = if dy < 0 {
                dms_hire_flag_ship_crew(cx, input)
            } else {
                dms_dismiss_flag_ship_crew(cx, input)
            };
            if delta != 0 {
                dms_flash_flag_ship_crew_count(cx);
            }
            delta
        }
        Some(ship) => {
            let delta = if dy < 0 {
                dms_hire_escort_ship_crew(cx, ship)
            } else {
                dms_dismiss_escort_ship_crew(cx, ship)
            };
            if delta != 0 {
                let slot = cx.world.built_ships().get(ship).index;
                dms_flash_escort_ship_crew_count(cx, slot);
            }
            delta
        }
    };
    if delta == 0 {
        play_menu_sound(
            cx.kernel,
            cx.world,
            crate::ui::settings::MENU_SOUND_FAILURE as u16,
        );
    }
    if handle.is_some() {
        menu.delta_item &= MODIFY_CREW_FLAG;
    }
    crew_transaction(cx, delta, |cx| {
        draw_campaign_menu(cx, PM_CREW as BYTE, SHIPYARD_CREW as i16);
    });
}

fn dms_try_add_escort_ship(cx: &mut Cx<'_>, input: &InputSession, menu: &mut ShipyardSession) {
    let (index, added) = {
        let Some((index, template)) =
            get_available_race_from_index(cx.world, menu.delta_item as BYTE)
        else {
            panic!("the purchased hangar race must be allied");
        };
        let added = cx.world.data.flagship.res_units >= SHIP_COST[index as usize] as DWORD && {
            with_built_ships(cx.world, |queue| {
                queue
                    .clone_fragment(
                        &template,
                        index as u8,
                        1,
                        crate::comm::dispatch::production_captain_name,
                    )
                    .is_some()
            })
        };
        (index, added)
    };
    if added {
        show_combat_ship(cx, input, menu, menu.state as COUNT, None);
        {
            draw_campaign_menu(cx, PM_CREW as BYTE, SHIPYARD_CREW as i16);
            delta_sis_gauges(
                cx.kernel,
                cx.world,
                sis::UNDEFINED_DELTA as SIZE,
                sis::UNDEFINED_DELTA as SIZE,
                -(SHIP_COST[index as usize] as i32),
            );
            dms_set_mode(cx, input, menu.state, ModifyMode::EditCrew);
        };
    } else {
        play_menu_sound(
            cx.kernel,
            cx.world,
            crate::ui::settings::MENU_SOUND_FAILURE as u16,
        );
    }
}

struct ModifyKeys {
    special: bool,
    select: bool,
    cancel: bool,
    direction: (i8, i8),
}

fn dms_add_escort_ship(
    cx: &mut Cx<'_>,
    input: &InputSession,
    menu: &mut ShipyardSession,
    keys: ModifyKeys,
    io: &mut Io<'_>,
    services: &super::MenuServices,
) {
    let highlight = cx.kernel.sis_highlight.clone();
    let ModifyKeys {
        special,
        select,
        cancel,
        direction: (dx, dy),
    } = keys;
    assert!(menu.delta_item & MODIFY_CREW_FLAG != 0);
    if special {
        {
            let ship = {
                with_built_ships(cx.world, |queue| {
                    queue.find(|ship| COUNT::from(ship.index) == menu.delta_item as COUNT)
                })
            };
            if dms_spin_ship(cx, menu, ship, io, services) {
                dms_set_mode(cx, input, menu.state, ModifyMode::AddEscort);
            }
        };
        return;
    }
    if cancel {
        {
            menu.delta_item &= !MODIFY_CREW_FLAG;
            clear_flash(cx.kernel);
            draw_campaign_menu(cx, PM_CREW as BYTE, SHIPYARD_CREW as i16);
            dms_set_mode(cx, input, menu.state, ModifyMode::Navigate);
        };
    } else if select {
        dms_try_add_escort_ship(cx, input, menu);
    } else if dx != 0 || dy != 0 {
        {
            let available = get_available_race_count(cx.world);
            let mut current = menu.delta_item as BYTE;
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
            if current != menu.delta_item as BYTE {
                highlight.pre_update_flash_rect();
                draw_race_strings(cx, menu, current);
                highlight.post_update_flash_rect();
                menu.delta_item = current as SIZE | MODIFY_CREW_FLAG;
            }
        };
    }
}

fn dms_scrap_escort_ship(
    cx: &mut Cx<'_>,
    input: &InputSession,
    menu: &mut ShipyardSession,
    handle: LinkId<Fragment>,
) {
    clear_flash(cx.kernel);
    show_combat_ship(cx, input, menu, menu.state as COUNT, Some(handle));
    {
        with_built_ships(cx.world, |queue| {
            queue.remove(handle);
            queue.free(handle);
        });
        delta_sis_gauges(
            cx.kernel,
            cx.world,
            sis::UNDEFINED_DELTA as SIZE,
            sis::UNDEFINED_DELTA as SIZE,
            sis::UNDEFINED_DELTA as i32,
        );
        draw::select(cx.kernel.space_context);
        dms_set_mode(cx, input, menu.state, ModifyMode::Navigate);
    };
}

fn dms_edit_crew_mode(
    cx: &mut Cx<'_>,
    input: &InputSession,
    menu: &mut ShipyardSession,
    ship: Option<LinkId<Fragment>>,
    select: bool,
    cancel: bool,
    dy: i8,
) {
    if select || cancel {
        if let Some(ship) = ship {
            let crew = with_built_ships(cx.world, |queue| queue.get(ship).crew_level);
            if crew == 0 {
                dms_scrap_escort_ship(cx, input, menu, ship);
            }
        }
        menu.delta_item &= !MODIFY_CREW_FLAG;
        dms_set_mode(cx, input, menu.state, ModifyMode::Navigate);
    } else if dy != 0 {
        dms_modify_crew(cx, input, menu, ship, dy);
    }
}

fn dms_navigate_ship_slots(
    cx: &mut Cx<'_>,
    input: &InputSession,
    menu: &mut ShipyardSession,
    keys: ModifyKeys,
    io: &mut Io<'_>,
    services: &super::MenuServices,
) {
    let ModifyKeys {
        special,
        select,
        cancel,
        direction: (dx, dy),
    } = keys;
    let ship = {
        with_built_ships(cx.world, |queue| {
            queue.find(|ship| COUNT::from(ship.index) == menu.state as COUNT)
        })
    };
    if dx != 0 || dy != 0 {
        let new_state = DMS_MoveCursor(menu.state, dx, dy);
        if new_state != menu.state {
            menu.state = new_state;
            dms_flash_active_ship(cx, menu.state);
        }
    }
    if special {
        if dms_spin_ship(cx, menu, ship, io, services) {
            dms_set_mode(cx, input, menu.state, ModifyMode::Navigate);
        }
    } else if select {
        if ship.is_none() && menu.state >> 4 == 0 {
            menu.delta_item = MODIFY_CREW_FLAG;
            draw_race_strings(cx, menu, 0);
            dms_set_mode(cx, input, menu.state, ModifyMode::AddEscort);
        } else {
            menu.delta_item |= MODIFY_CREW_FLAG;
            dms_set_mode(cx, input, menu.state, ModifyMode::EditCrew);
        }
    } else if cancel {
        menu.page = ShipyardPage::Main;
        menu.state = SHIPYARD_CREW;
        draw_campaign_menu(cx, PM_CREW as BYTE, menu.state as i16);
        dms_set_mode(cx, input, menu.state, ModifyMode::Exit);
    }
}

fn do_modify_ships(
    cx: &mut Cx<'_>,
    menu: &mut ShipyardSession,
    input: &InputSession,
    io: &mut Io<'_>,
    services: &super::MenuServices,
) -> BOOLEAN {
    if cx.world.activity() & activity_flags::CHECK_ABORT as u16 != 0 {
        menu.page = ShipyardPage::Main;
        return 1;
    }
    if !menu.initialized {
        {
            menu.page = ShipyardPage::Modify;
            menu.initialized = true;
            menu.state = 0xf0;
            menu.delta_item = 0;
            draw::select(cx.kernel.space_context);
            dms_set_mode(cx, input, menu.state, ModifyMode::Navigate);
        };
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
        if menu.delta_item & MODIFY_CREW_FLAG == 0 {
            dms_navigate_ship_slots(
                cx,
                input,
                menu,
                ModifyKeys {
                    special,
                    select,
                    cancel,
                    direction: (dx, dy),
                },
                io,
                services,
            );
        } else {
            let ship = {
                with_built_ships(cx.world, |queue| {
                    queue.find(|ship| COUNT::from(ship.index) == menu.state as COUNT)
                })
            };
            if ship.is_none() && menu.state >> 4 == 0 {
                dms_add_escort_ship(
                    cx,
                    input,
                    menu,
                    ModifyKeys {
                        special,
                        select,
                        cancel,
                        direction: (dx, dy),
                    },
                    io,
                    services,
                );
            } else {
                dms_edit_crew_mode(cx, input, menu, ship, select, cancel, dy);
            }
        }
    }
    cx.kernel.input_runtime().sleep(cx, ONE_SECOND / 30);
    1
}

fn draw_blue_print(
    cx: &mut Cx<'_>,
    input: &InputSession,
    menu: &mut ShipyardSession,
    flagship: &crate::engine::world::SisState,
    context: &crate::graphics::context_abi::ContextLease,
) {
    let blueprints = LoadedDrawable::load(c"graphics.blueprints")
        .unwrap_or_else(|| panic!("blueprints missing"));
    let frame = blueprints
        .lease()
        .frame(0)
        .unwrap_or_else(|| panic!("blueprints have no frame"));
    let stamp = STAMP {
        origin: POINT { x: 0, y: 0 },
        frame: draw::relative_frame(frame, -1),
    };
    draw::foreground(rgb15(0, 0, 0x16));
    draw::filled_stamp(stamp.frame, stamp.origin);
    for slot in 0..ids::NUM_DRIVE_SLOTS {
        let piece = flagship.drive_slots[slot as usize];
        draw_piece(
            cx.kernel,
            flagship,
            cx.screen_dimensions(),
            frame,
            piece as COUNT,
            slot as COUNT,
            true,
        );
    }
    for slot in 0..ids::NUM_JET_SLOTS {
        let piece = flagship.jet_slots[slot as usize];
        draw_piece(
            cx.kernel,
            flagship,
            cx.screen_dimensions(),
            frame,
            piece as COUNT,
            slot as COUNT,
            true,
        );
    }
    for slot in 0..equipment::NUM_MODULE_SLOTS {
        let piece = flagship.module_slots[slot as usize];
        if !(menu.state as u32 == SHIPYARD && piece as u32 == ModuleType::CrewPod as u32) {
            draw_piece(
                cx.kernel,
                flagship,
                cx.screen_dimensions(),
                frame,
                piece as COUNT,
                slot as COUNT,
                true,
            );
        }
    }
    draw::foreground(rgb15(0x0a, 0x0a, 0x1f));
    for slot in 0..equipment::NUM_MODULE_SLOTS {
        let piece = flagship.module_slots[slot as usize];
        if menu.state as u32 == SHIPYARD && piece as u32 == ModuleType::CrewPod as u32 {
            draw_piece(
                cx.kernel,
                flagship,
                cx.screen_dimensions(),
                frame,
                piece as COUNT,
                slot as COUNT,
                true,
            );
        }
    }
    super::sis::capacity::draw_blueprint_cells(
        context,
        flagship,
        input.preferences().fonts == crate::ui::settings::OPT_PC as i32,
    );
    drop(blueprints);
}

fn exit_shipyard(input: &InputSession, menu: &mut ShipyardSession) -> BOOLEAN {
    input.replace_callback(None);
    menu.assets.modules = None;
    menu.power = None;
    0
}

/// Processes one shipyard input frame, including entry and exit resource handling.
///
fn do_shipyard(
    cx: &mut Cx<'_>,
    menu: &mut ShipyardSession,
    input: &InputSession,
    io: &mut Io<'_>,
    services: &super::MenuServices,
) -> BOOLEAN {
    let size = cx.screen_dimensions();
    if cx.world.activity() & activity_flags::CHECK_ABORT as u16 != 0 {
        return exit_shipyard(input, menu);
    }
    let select = input.pulsed().menu[KEY_MENU_SELECT as usize] != 0;
    let cancel = input.pulsed().menu[KEY_MENU_CANCEL as usize] != 0;
    input.set_menu_sounds(SOUND_ARROWS, SOUND_SELECT);
    if !menu.initialized {
        let flagship = cx.world.data.flagship;
        let context = crate::graphics::context_abi::ContextLease::require(cx.kernel.space_context);
        {
            menu.page = ShipyardPage::Main;
            menu.assets.modules = LoadedDrawable::load(c"graphics.shipyard");
            menu.power = Some(Rc::new(RefCell::new(PowerLines {
                frame: ContextFrame::retain(std::ptr::NonNull::new(draw::frame(
                    menu.assets.modules(),
                    24,
                ))),
                palette: PaletteCycle::load(c"colortable.hangar")
                    .unwrap_or_else(|| panic!("hangar palette missing")),
                next_time: 0,
            })));
            menu.assets.music = crate::sound::resource_types::LoadedMusic::load(c"music.shipyard");
            draw::transition_source();
            draw::batch();
            crate::ui::border::draw_sis_frame(cx.kernel, cx.world, size);
            let message = cx
                .kernel
                .game_text((ids::STARBASE_STRING_BASE + 3) as usize);
            let title = cx.kernel.game_text(ids::STARBASE_STRING_BASE as usize);
            draw_campaign_message(cx.kernel, cx.world, size.0, Some(message));
            draw_title(cx.kernel, size.0, title);
            draw::select(cx.kernel.space_context);
            draw_blue_print(cx, input, menu, &flagship, &context);
            menu.state = SHIPYARD_CREW;
            draw_campaign_menu(cx, PM_CREW as BYTE, menu.state as i16);
            let stamp = STAMP {
                origin: POINT { x: 0, y: 0 },
                frame: draw::frame(menu.assets.modules(), 0),
            };
            if USE_3DO_HANGAR {
                draw::stamp(stamp.frame, stamp.origin);
            } else {
                let old_r = draw::clip();
                let mut r = old_r;
                r.corner.x -= 1;
                r.extent.width += 2;
                r.extent.height += 1;
                draw::set_clip(r);
                draw::stamp(stamp.frame, stamp.origin);
                draw::set_clip(old_r);
                animate_power_lines(menu, true);
            }
            draw::font(cx.kernel.tiny_font);
            draw::transition(3);
            draw::unbatch();
            crate::ui::settings::play_loaded_music(cx.world, menu.assets.music.as_ref(), 1, 1);
        };
        show_combat_ship(cx, input, menu, COUNT::MAX, None);
        let power = menu
            .power
            .as_ref()
            .unwrap_or_else(|| panic!("hangar power lines not loaded"))
            .clone();
        frame::install(input, context, move || power.borrow_mut().draw(false));
        set_menu_flash(cx.kernel, size, false);
        menu.initialized = true;
    } else if cancel || (select && menu.state == SHIPYARD_EXIT) {
        return exit_shipyard(input, menu);
    } else if select && menu.state != SHIPYARD_SAVELOAD {
        menu.initialized = false;
        do_modify_ships(cx, menu, input, io, services);
    } else if select {
        if (services.game_options)(cx, io) == 0 {
            return exit_shipyard(input, menu);
        }
        draw_campaign_menu(cx, PM_CREW as BYTE, menu.state as i16);
        set_menu_flash(cx.kernel, size, false);
    } else {
        crate::ui::menu::choose_menu_state(cx, input, &mut menu.state, PM_CREW as BYTE);
    }
    1
}

#[derive(Clone, Copy)]
enum ShipyardPage {
    Main,
    Modify,
}
struct ShipyardSession {
    initialized: bool,
    state: u8,
    delta_item: i16,
    assets: MenuAssets,
    page: ShipyardPage,
    power: Option<Rc<RefCell<PowerLines>>>,
}
fn with_built_ships<R>(
    world: &mut World,
    operation: impl FnOnce(&mut crate::engine::battle::link_queue::LinkQueue<'_, Fragment, 12>) -> R,
) -> R {
    operation(&mut world.built_ships())
}
fn with_avail_races<R>(
    world: &World,
    operation: impl FnOnce(
        &crate::engine::battle::link_queue::LinkQueue<'_, crate::engine::world::queues::Fleet, 24>,
    ) -> R,
) -> R {
    operation(&world.avail_races_ref())
}
pub fn run(
    cx: &mut Cx<'_>,
    io: &mut Io<'_>,
    services: &super::MenuServices,
) -> Option<crate::sound::resource_types::LoadedMusic> {
    let runtime = cx.kernel.input_runtime();
    let input = runtime.input();
    let mut menu = ShipyardSession {
        initialized: false,
        state: 0,
        delta_item: 0,
        assets: MenuAssets::default(),
        page: ShipyardPage::Main,
        power: None,
    };
    cx.run_input(io, true, |cx, io| match menu.page {
        ShipyardPage::Main => do_shipyard(cx, &mut menu, input, io, services) != 0,
        ShipyardPage::Modify => do_modify_ships(cx, &mut menu, input, io, services) != 0,
    });
    input.replace_callback(None);
    menu.assets.music.take()
}
