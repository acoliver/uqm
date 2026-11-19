use super::{landing_fuel, move_cursor};
use crate::battle::element::ElementFlags;
use crate::comm::locdata::{CColor, CPoint, CRect};
use crate::engine::battle::views::FrameSelection;
use crate::engine::graphics::context_abi::{ContextFrame, ContextLease};
use crate::engine::world::activity::activity_flags::{CHECK_ABORT, CHECK_LOAD, START_ENCOUNTER};
use crate::engine::{
    battle::Cx,
    input::InputSession,
    observe::{CheckpointView, Io},
    StoryKey,
};
use crate::graphics::menu as drawing;
use crate::input::frames::{
    KEY_MENU_CANCEL, KEY_MENU_DOWN, KEY_MENU_LEFT, KEY_MENU_RIGHT, KEY_MENU_SELECT, KEY_MENU_UP,
};
use crate::planet_side::ffi::{run_session, TripError, TripInput};
use crate::planets::orbit::OrbitServices;
use crate::planets::solar_system::SolarSystem;
use crate::starbase::gfx::{Gfx, Rgba};
use crate::starbase::graphics::Graphics;
use std::ffi::CString;

const TEXT_COLORS: [Rgba; 3] = [
    Rgba::rgb15(19, 0, 0),
    Rgba::rgb15(12, 12, 12),
    Rgba::rgb15(0, 14, 0),
];
const TINT_COLORS: [Rgba; 3] = [
    Rgba::rgb15(31, 0, 0),
    Rgba::rgb15(31, 31, 31),
    Rgba::rgb15(0, 31, 0),
];

mod display;

use uqm_engine::time::counter_abi::GetTimeCounter;

use super::state::ScanState;
use crate::engine::graphics::drawable_abi::LoadedDrawable;

use crate::planets::render::draw_star_background;

use crate::planets::sphere::live::draw_default_planet_sphere;

use crate::planets::sphere::live::draw_planet;

use crate::planets::orbit::live::draw_scanned_objects;

use crate::planets::orbit::live::redraw_planet_orbit_display;

use crate::ui::settings::play_menu_sound;

use crate::planet_side::init_lander::{draw_lander, LanderStatus};

use crate::ui::util::wait_for_any_button_until;

use crate::graphics::native_font::FontLease;

impl ScanState {
    pub(crate) fn draw_surface(&self, tint_y: i32, color: CColor) {
        draw_planet(
            &self.offscreen,
            &mut self.sphere.borrow_mut(),
            tint_y,
            color,
        );
    }
}

fn color(rgb: Rgba) -> CColor {
    CColor {
        r: rgb.r,
        g: rgb.g,
        b: rgb.b,
        a: rgb.a,
    }
}
fn black() -> CColor {
    color(Rgba::BLACK)
}
fn width(screen_width: i32) -> i16 {
    (screen_width - 78) as i16
}
fn draw(frame: FrameSelection, point: CPoint, filled: bool) {
    let point = crate::graphics::native_abi::POINT {
        x: point.x,
        y: point.y,
    };
    if filled {
        drawing::filled_stamp(frame.raw().cast(), point);
    } else {
        drawing::stamp(frame.raw().cast(), point);
    }
}

fn cursor_frame(scan: &ScanState) -> ContextFrame {
    scan.cursor_image.indexed_frame(105)
}

use super::repair_back_rect;

fn erase_coarse(planet: &ContextLease, scan: &ScanState) {
    planet.select();
    drawing::batch();
    draw_star_background(
        planet,
        scan.background
            .as_ref()
            .unwrap_or_else(|| panic!("scan background is not supplied")),
    );
    draw_default_planet_sphere(planet, &scan.sphere.borrow());
    drawing::unbatch();
}

fn save_location_image(scan: &ScanState) {
    let corner = cursor_frame(scan).rect().corner;
    let hot = crate::graphics::native_abi::HOT_SPOT {
        x: corner.x.wrapping_neg(),
        y: corner.y.wrapping_neg(),
    };
    let mut cursor = scan.cursor.borrow_mut();
    cursor.erase = None;
    let mut rect = cursor.rect;
    rect.corner.x -= hot.x;
    rect.corner.y -= hot.y;
    let saved = LoadedDrawable::capture(
        &scan.context(),
        Some(crate::graphics::native_abi::RECT {
            corner: crate::graphics::native_abi::POINT {
                x: rect.corner.x,
                y: rect.corner.y,
            },
            extent: crate::graphics::native_abi::EXTENT {
                width: rect.width,
                height: rect.height,
            },
        }),
    )
    .unwrap_or_else(|| panic!("landing cursor background intersects scan context"));
    saved.lease().set_hotspot(0, hot);
    cursor.erase = Some(saved);
}
fn restore_location_image(scan: &ScanState) {
    let cursor = scan.cursor.borrow();
    let saved = cursor
        .erase
        .as_ref()
        .unwrap_or_else(|| panic!("saved landing cursor background"));
    scan.context().draw_stamp(
        &saved.lease().context_frame(0),
        crate::graphics::native_abi::POINT {
            x: cursor.rect.corner.x,
            y: cursor.rect.corner.y,
        },
    );
}
fn draw_cursor(scan: &ScanState, filled: bool) {
    let context = scan.context();
    let point = scan.cursor.borrow().rect.corner;
    let point = crate::graphics::native_abi::POINT {
        x: point.x,
        y: point.y,
    };
    if filled {
        context.draw_filled_stamp(&cursor_frame(scan), point, context.foreground());
    } else {
        context.draw_stamp(&cursor_frame(scan), point);
    }
}
fn set_location(scan: &ScanState, point: CPoint, restore_old: bool) {
    scan.context().select();
    if restore_old {
        restore_location_image(scan);
    }
    scan.cursor.borrow_mut().move_to(point);
    save_location_image(scan);
}

fn flash_location(
    scan: &ScanState,
    animation: &crate::engine::kernel::planet_animation::PlanetAnimation,
) {
    let now = GetTimeCounter();
    let position = scan.cursor.borrow().rect.corner;
    let Some(shade) = animation.cursor_shade((position.x, position.y), now) else {
        return;
    };
    scan.context().select();
    scan.context()
        .set_foreground(crate::ui::rgb15(shade, shade, shade));
    draw_cursor(scan, true);
}

pub fn redraw_surface_scan(
    cx: &mut Cx<'_>,
    scan: &ScanState,
    location: Option<CPoint>,
    visible: impl Fn(u16, u16) -> bool,
) {
    scan.context().with_selected(|| {
        {
            drawing::batch();
            scan.draw_surface(0, black());
        };
        draw_scanned_objects(cx, true, visible);
        {
            if let Some(location) = location {
                set_location(scan, location, false);
                draw_cursor(scan, false);
            }
            drawing::unbatch();
        };
    });
}

fn draw_scanned(cx: &mut Cx<'_>, solar: &SolarSystem, reversed: bool) {
    draw_scanned_objects(cx, reversed, |flags, life| {
        crate::planet_side::orbit_scan::node_visible(flags, life, &solar.sys.info.planet_info)
    });
}

fn fuel(solar: &SolarSystem) -> u16 {
    landing_fuel(solar.sys.info.planet_info.gravity)
}

fn spawn_fwiffo(world: &mut crate::engine::World, services: &OrbitServices) {
    world.encounter_group = 0;
    (services.store_groups)(world);
    world.ip_groups().reinit();
    assert_eq!(world.npc_built_ships_ref().count(), 0);
    let fleets = world.avail_races_ref();
    let template = *fleets.get(
        fleets
            .at(crate::engine::species::ship::SPATHI_SHIP as usize)
            .unwrap_or_else(|| panic!("Spathi fleet")),
    );
    if let Some(id) = world.npc_built_ships().clone_fragment(
        &template,
        crate::engine::species::ship::SPATHI_SHIP,
        1,
        crate::comm::dispatch::production_captain_name,
    ) {
        world.npc_built_ships().get_mut(id).captains_name_index = 5 + 16;
    }
}

fn dispatch_lander(
    solar: &mut SolarSystem,
    planet: &ContextLease,
    scan: &ScanState,
    cx: &mut Cx<'_>,
    io: &mut Io<'_>,
    services: &OrbitServices,
) -> bool {
    let runtime = cx.kernel.input_runtime();
    let misc_data = ContextFrame::retain(std::ptr::NonNull::new(cx.kernel.misc_data_frame));
    let (landing_fuel, old_callback) = {
        let landing_fuel = fuel(solar) as i16;
        erase_coarse(planet, scan);
        let old_callback = runtime.input().replace_callback(None);
        (landing_fuel, old_callback)
    };
    crate::ui::sis::delta_sis_gauges(cx.kernel, cx.world, 0, -landing_fuel, 0);
    let context = prepare_lander(scan, misc_data, solar);
    let reply = run_session(
        crate::planet_side::graphics_adapter::SurfaceDrawing {
            planet: planet.clone(),
            scan,
            refresh: refresh_lander_scan,
            radar: ContextLease::require(cx.kernel.radar_context),
        },
        runtime.input(),
        cx.world.activity_control(),
        cx,
        io,
        solar,
        context,
    );
    match reply {
        Ok(_) => {}
        Err(TripError::Adapter(error)) => crate::log_abi::log_message(
            crate::logging::LogLevel::Error.as_i32(),
            &format!(
                "Rust PlanetSide adapter failed ({}); returning to orbit",
                error.operation
            ),
        ),
        Err(TripError::FrameBudgetExceeded) => crate::log_abi::log_message(
            crate::logging::LogLevel::Error.as_i32(),
            "Rust PlanetSide frame budget exhausted; returning to orbit",
        ),
    }
    if cx.world.activity() & CHECK_ABORT != 0 {
        return false;
    }
    if cx.world.story_state(StoryKey::FOUND_PLUTO_SPATHI) == 1 {
        spawn_fwiffo(cx.world, services);
        cx.world.next_activity |= CHECK_LOAD;
        cx.world.set_activity(cx.world.activity() | START_ENCOUNTER);
        (services.save_location)(cx.world, cx.kernel, solar);
        return false;
    }
    redraw_planet_orbit_display(cx, planet, scan);
    display::coarse(cx, solar, planet, runtime.input().preferences().coarse_scan);
    runtime.input().replace_callback(old_callback);
    true
}

fn prepare_lander(scan: &ScanState, misc_data: ContextFrame, solar: &SolarSystem) -> TripInput {
    scan.context().select();
    draw_cursor(scan, false);
    solar.require_body();
    TripInput {
        misc_data_frame: misc_data,
        energy_frame: solar.sys.frames[1]
            .as_ref()
            .map(|frame| frame.lease().context_frame(0)),
        life_frames: std::array::from_fn(|i| {
            solar.sys.frames[i + 3]
                .as_ref()
                .map(|frame| frame.lease().context_frame(0))
        }),
        landing: crate::planet_side::model::SurfacePoint {
            x: i32::from(scan.cursor.borrow().location.x),
            y: i32::from(scan.cursor.borrow().location.y),
        },
        facing: crate::math::random() as u8,
        tick_period: std::num::NonZeroU32::new(840 / 35).unwrap_or_else(|| unreachable!()),
        frame_budget: std::num::NonZeroU32::MAX,
        tectonics_rating: solar.sys.info.planet_info.tectonics as u8,
        weather_rating: solar.sys.info.planet_info.weather as u8,
        temperature: i32::from(solar.sys.info.planet_info.temperature),
    }
}

struct PickState {
    success: bool,
}

fn pick_input(
    cx: &mut Cx<'_>,
    scan: &ScanState,
    input: &InputSession,
    animation: &crate::engine::kernel::planet_animation::PlanetAnimation,
    state: &mut PickState,
) -> u32 {
    let time_in = GetTimeCounter();
    let pulse = input.pulsed();
    if cx.world.activity() & CHECK_ABORT != 0 || pulse.menu[KEY_MENU_CANCEL as usize] != 0 {
        state.success = false;
        return 0;
    } else if pulse.menu[KEY_MENU_SELECT as usize] != 0 {
        state.success = true;
        return 0;
    }
    let current = input.current();
    let mut dx = 0;
    let mut dy = 0;
    if current.menu[KEY_MENU_LEFT as usize] != 0 {
        dx = -1;
    }
    if current.menu[KEY_MENU_RIGHT as usize] != 0 {
        dx = 1;
    }
    if current.menu[KEY_MENU_UP as usize] != 0 {
        dy = -1;
    }
    if current.menu[KEY_MENU_DOWN as usize] != 0 {
        dy = 1;
    }
    drawing::batch();
    let point = scan.cursor.borrow().location;
    let next = move_cursor((point.x, point.y), dx, dy, width(cx.screen_dimensions().0));
    if next != (point.x, point.y) {
        set_location(
            scan,
            CPoint {
                x: next.0,
                y: next.1,
            },
            true,
        );
    }
    flash_location(scan, animation);
    drawing::unbatch();
    cx.kernel
        .input_runtime()
        .sleep_until(cx, time_in.wrapping_add(840 / 40));
    1
}

fn pick_side(
    solar: &mut SolarSystem,
    planet: &ContextLease,
    scan: &ScanState,
    cx: &mut Cx<'_>,
    io: &mut Io<'_>,
    services: &OrbitServices,
) -> bool {
    let runtime = cx.kernel.input_runtime();
    let input = runtime.input();
    let mut state = PickState { success: false };
    let animation = cx.kernel.planet_animation.clone();
    let assets = crate::planet_side::resources::PlanetSideAssets::retain(
        cx.kernel
            .lander_resources
            .as_ref()
            .unwrap_or_else(|| panic!("lander resources not loaded")),
    );
    let status = LanderStatus::campaign(cx.world);
    let radar = ContextLease::require(cx.kernel.radar_context);
    prepare_pick_side(solar, cx, scan, &radar, &assets, status);
    io.observer.checkpoint(&CheckpointView::PlanetMenu(4));
    solar.run_input(cx, io, true, |cx, _, _| {
        pick_input(cx, scan, input, &animation, &mut state) != 0
    });
    io.observer.checkpoint(&CheckpointView::PlanetMenu(3));
    crate::ui::sis::draw_status_message(cx.kernel, cx.world, None);
    let result = if state.success {
        dispatch_lander(solar, planet, scan, cx, io, services)
    } else {
        restore_location_image(scan);
        true
    };
    input.set_menu_sounds(15, 16);
    result
}

fn prepare_pick_side(
    solar: &mut SolarSystem,
    cx: &mut Cx<'_>,
    scan: &ScanState,
    radar: &ContextLease,
    assets: &crate::planet_side::resources::PlanetSideAssets,
    status: LanderStatus,
) {
    let runtime = cx.kernel.input_runtime();
    let input = runtime.input();
    let g = Graphics::new(cx.kernel, cx.screen_dimensions());
    let fuel = fuel(solar);
    let dimensions = cx.screen_dimensions();
    crate::ui::sis::clear_sis_rect(cx.kernel, cx.world, dimensions, 1 << 2);
    {
        scan.context().select();
        drawing::batch();
        scan.draw_surface(0, black());
    };
    draw_scanned(cx, solar, false);
    let message = {
        drawing::unbatch();
        CString::new(format!(
            "{}{:.1}",
            g.game_string(435 + 5),
            f32::from(fuel) / 100.0
        ))
        .unwrap()
    };
    crate::ui::sis::draw_status_message(cx.kernel, cx.world, Some(&message));
    {
        let location = scan.cursor.borrow().location;
        scan.cursor.borrow_mut().move_to(location);
        save_location_image(scan);
        draw_lander(radar, assets, status);
        input.set_menu_sounds(0, 16);
    };
}

fn scanned_stuff(solar: &SolarSystem, cx: &mut crate::engine::battle::Cx<'_>, y: u16, scan: u16) {
    let g = Graphics::new(cx.kernel, cx.screen_dimensions());
    let old_color = g.set_fg_color(Rgba::BLACK);
    {
        let mut handle = cx.elements().head();
        while let Some(element) = handle {
            handle = cx.elements().succ(element);
            let mut dy = (i32::from(y) - i32::from(cx.element(element).current.location.y)) as i16;
            if cx.element(element).life as u8 != scan as u8
                || dy < 0
                || crate::planet_side::orbit_scan::node_retrieved(
                    cx.element(element).life,
                    &solar.sys.info.planet_info,
                )
            {
                continue;
            }
            cx.element_mut(element).flags |= ElementFlags::APPEARING.bits();
            let point = CPoint {
                x: cx.element(element).current.location.x,
                y: cx.element(element).current.location.y,
            };
            if dy >= 8 {
                draw(cx.element(element).next.image.frame.clone(), point, false);
            } else {
                let gradient = (255 - 255 * dy / 7) as u8;
                let mut c = Rgba::rgb15(31, 31, 31);
                match scan {
                    0 => c.r = gradient,
                    1 => {
                        c.g = gradient;
                        c.b = gradient;
                    }
                    2 => c.g = gradient,
                    _ => unreachable!("scan type"),
                }
                g.set_fg_color(c);
                let size = g
                    .frame_index(cx.element(element).next.image.frame.raw() as usize)
                    .wrapping_sub(
                        g.frame_index(cx.element(element).current.image.frame.raw() as usize),
                    );
                if dy as u16 > size {
                    dy = size as i16;
                }
                let frame = g.rel_frame(cx.element(element).current.image.frame.raw() as usize, dy);
                draw(FrameSelection::retain(frame as _), point, true);
            }
        }
    }
    g.set_fg_color(old_color);
}

fn scan_planet(
    solar: &mut SolarSystem,
    scan_state: &ScanState,
    planet: &ContextLease,
    cx: &mut Cx<'_>,
    io: &mut Io<'_>,
    scan_type: u16,
) {
    let dimensions = cx.screen_dimensions();
    let g = Graphics::new(cx.kernel, cx.screen_dimensions());
    let (start, end) = if scan_type == 4 {
        (0, 2)
    } else {
        (scan_type, scan_type)
    };
    let mut repair = CRect::default();
    let micro = FontLease::retain(
        std::ptr::NonNull::new(cx.kernel.micro_font)
            .unwrap_or_else(|| panic!("scan micro font is not loaded")),
    );
    for scan in start..=end {
        let (mut timeout, tint) = {
            let baseline_y = dimensions.1 - 13 - 75 - 7;
            planet.select();
            repair = CRect {
                corner: CPoint {
                    x: 0,
                    y: (baseline_y - 10) as i16,
                },
                width: width(dimensions.0),
                height: 11,
            };
            repair_back_rect(planet, &repair);
            planet.set_font(&micro);
            g.set_fg_color(TEXT_COLORS[usize::from(scan)]);
            display::text(
                &g,
                width(dimensions.0) >> 1,
                baseline_y as i16,
                crate::starbase::gfx::Align::Center,
                g.game_string(305 + i32::from(scan)),
            );
            scan_state.context().select();
            drawing::batch();
            scan_state.draw_surface(0, black());
            drawing::unbatch();
            (GetTimeCounter(), color(TINT_COLORS[usize::from(scan)]))
        };
        let mut line = 0;
        while line < 84 {
            timeout = timeout.wrapping_add((840 * 7 / 4) / 84);
            if wait_for_any_button_until(cx, io, 1, timeout, 0) != 0 {
                break;
            }
            {
                drawing::batch();
                scan_state.draw_surface(line, tint);
            };
            scanned_stuff(solar, cx, line as u16, scan);
            drawing::unbatch();
            line += 1;
        }
        if line < 84 {
            {
                drawing::batch();
                scan_state.draw_surface(83, tint);
            };
            scanned_stuff(solar, cx, 83, scan);
            drawing::unbatch();
        }
    }
    {
        planet.select();
        repair_back_rect(planet, &repair);
        scan_state.context().select();
    };
    if scan_type == 4 {
        {
            scan_state.draw_surface(0, black());
        };
        draw_scanned(cx, solar, false);
    }
    cx.kernel.input_runtime().flush();
}

fn unavailable(solar: &SolarSystem) -> bool {
    solar.sys.descriptor(solar.require_body()).data_index & 0x80 != 0
        || solar.sys.info.planet_info.atmo_density == u16::MAX
}

fn scan_input(
    solar: &mut SolarSystem,
    scan: &ScanState,
    planet: &ContextLease,
    cx: &mut Cx<'_>,
    io: &mut Io<'_>,
    menu: &mut u8,
    services: &OrbitServices,
) -> u32 {
    let runtime = cx.kernel.input_runtime();
    let input = runtime.input();
    let pulse = input.pulsed();
    let selected = pulse.menu[KEY_MENU_SELECT as usize] != 0;
    if cx.world.activity() & CHECK_ABORT != 0 {
        return 0;
    }
    if pulse.menu[KEY_MENU_CANCEL as usize] != 0 || (selected && *menu == 3) {
        return 0;
    } else if selected {
        if *menu == 5 {
            if unavailable(solar) {
                play_menu_sound(cx.kernel, cx.world, 2);
                return 1;
            }
            let required = fuel(solar);
            if cx.world.data.flagship.fuel_on_board < u32::from(required)
                || cx.world.data.flagship.num_landers == 0
                || cx.world.data.flagship.crew_enlisted == 0
            {
                play_menu_sound(cx.kernel, cx.world, 2);
                return 1;
            }
            crate::ui::sis::clear_flash(cx.kernel);
            if !pick_side(solar, planet, scan, cx, io, services) {
                return 0;
            }
            crate::ui::menu::draw_menu_state(
                cx.kernel,
                cx.world,
                (cx.screen_dimensions().0, cx.screen_dimensions().1),
                7,
                i16::from(*menu),
            );
            crate::ui::sis::set_menu_flash(cx.kernel, cx.screen_dimensions(), false);
            return 1;
        }
        if solar.sys.descriptor(solar.require_body()).data_index & 0x80 != 0 {
            play_menu_sound(cx.kernel, cx.world, 2);
            return 1;
        }
        scan_planet(solar, scan, planet, cx, io, u16::from(*menu));
        if *menu == 4 {
            *menu = 5;
            io.observer.checkpoint(&CheckpointView::PlanetMenu(3));
            crate::ui::menu::draw_menu_state(
                cx.kernel,
                cx.world,
                (cx.screen_dimensions().0, cx.screen_dimensions().1),
                7,
                i16::from(*menu),
            );
        }
    } else if input.preferences().menu == 2 || !unavailable(solar) {
        crate::ui::menu::choose_menu_state(cx, input, menu, 7);
    }
    1
}

pub fn scan_system(
    solar: &mut SolarSystem,
    scan: &ScanState,
    planet: &ContextLease,
    cx: &mut Cx<'_>,
    io: &mut Io<'_>,
    services: &OrbitServices,
) {
    let g = Graphics::new(cx.kernel, cx.screen_dimensions());
    let runtime = cx.kernel.input_runtime();
    let input = runtime.input();
    let mut menu = 0;
    prepare_scan_system(solar, cx, scan, planet, input, &mut menu);
    input.set_menu_sounds(15, 16);
    io.observer.checkpoint(&CheckpointView::PlanetMenu(2));
    solar.run_input(cx, io, false, |cx, io, solar| {
        scan_input(solar, scan, planet, cx, io, &mut menu, services) != 0
    });
    io.observer.checkpoint(&CheckpointView::PlanetMenu(0));
    finish_scan_system(scan, planet, &g);
}

fn prepare_scan_system(
    solar: &mut SolarSystem,
    cx: &mut Cx<'_>,
    scan: &ScanState,
    planet: &ContextLease,
    input: &InputSession,
    menu: &mut u8,
) {
    let dimensions = cx.screen_dimensions();
    if input.preferences().menu == 1 && unavailable(solar) {
        *menu = 3;
    } else {
        *menu = 4;
        scan.cursor.borrow_mut().location = CPoint {
            x: (width(dimensions.0) >> 1) << 2,
            y: (75 >> 1) << 2,
        };
        let cursor = cursor_frame(scan).rect();
        scan.cursor.borrow_mut().rect.width = cursor.extent.width;
        scan.cursor.borrow_mut().rect.height = cursor.extent.height;
        scan.context().select();
    };
    if *menu == 4 {
        draw_scanned(cx, solar, false);
    }
    let dimensions = cx.screen_dimensions();
    crate::ui::menu::draw_menu_state(
        cx.kernel,
        cx.world,
        (dimensions.0, dimensions.1),
        7,
        i16::from(*menu),
    );
    crate::ui::sis::set_menu_flash(cx.kernel, dimensions, false);
    display::coarse(cx, solar, planet, input.preferences().coarse_scan);
}

fn finish_scan_system(scan: &ScanState, planet: &ContextLease, g: &Graphics) {
    g.set_flash(crate::starbase::gfx::FlashTarget::Off);
    drawing::batch();
    scan.context().select();
    scan.draw_surface(0, black());
    erase_coarse(planet, scan);
    drawing::unbatch();
    scan.cursor.borrow_mut().erase = None;
}

fn refresh_lander_scan(
    scan: &ScanState,
    cx: &mut Cx<'_>,
    surface: &crate::planet_side::assembly::SurfaceAssembly,
    point: crate::planet_side::model::SurfacePoint,
) {
    crate::planet_side::orbit_scan::sync_positions(cx, surface);
    redraw_surface_scan(
        cx,
        scan,
        Some(CPoint {
            x: point.x as i16,
            y: point.y as i16,
        }),
        |flags, life| {
            flags & ElementFlags::APPEARING.bits() != 0
                && surface.generated.iter().any(|entry| {
                    life == (entry.scan as u16 | (u16::from(entry.node.get() + 1) << 8))
                })
        },
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scan_width_uses_the_supplied_display_and_surface_drawing_is_safe() {
        assert_eq!(width(320), 242);
        assert_eq!(width(398), 320);
        let _: fn(&ScanState, i32, CColor) = ScanState::draw_surface;
    }
}
