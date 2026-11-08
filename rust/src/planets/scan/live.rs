use super::{landing_fuel, move_cursor};
use crate::battle::element::ElementFlags;
use crate::c_bindings::{controller_input::*, solar_system};
use crate::comm::locdata::{CColor, CPoint, CRect};
use crate::engine::graphics::context_abi::{ContextFrame, ContextLease};
use crate::engine::{
    battle::Cx,
    input::InputSession,
    observe::{CheckpointView, Io},
    Kernel, StoryKey,
};
use crate::mainloop::restart_menu::c_extern::{
    self as base, uqm_sleep_thread_until as SleepThreadUntil,
};
use crate::mainloop::types::activity_flags::{CHECK_ABORT, CHECK_LOAD, START_ENCOUNTER};
use crate::planet_side::ffi::{run_session, PlanetSideRunContext, STATUS_OK};
use crate::ships::native::abi::MENU_STATE;
use crate::starbase::cffi::CffiPlatform;
use crate::starbase::gfx::{FlashTarget, Gfx, Point, Rgba, Stamp};
use std::ffi::{c_void, CString};

mod display;

use crate::time::counter_abi::GetTimeCounter;

use super::state::ScanState;
use crate::engine::graphics::drawable_abi::LoadedDrawable;

use crate::graphics::gfx_common_abi::DrawFromExtraScreen;

use crate::planets::solarsys::drawing::DrawStarBackGround;

use crate::planets::sphere::live::draw_default_planet_sphere;

use crate::planets::sphere::live::DrawPlanet;

use crate::planets::orbit::live::DrawScannedObjects;

use crate::planets::orbit::live::redraw_planet_orbit_display;

use crate::ui::sis::DeltaSISGauges;

use crate::ui::sis::DrawStatusMessage;

use crate::ui::settings::play_menu_sound;

use crate::mainloop::input_abi::live::SetInputCallback;

use crate::navigation::groups::live::put_group_info;

use crate::planets::solarsys::lifecycle::SaveSolarSysLocation;

use crate::planet_side::init_lander::InitLander;

use crate::ui::util::live::wait_for_any_button_until;

use crate::mainloop::input_abi::live::FlushInput;

use crate::ui::menu::do_menu_chooser;

use crate::ui::menu::DrawMenuStateStrings;

use crate::graphics::font_abi::SetContextFont;

extern "C" {

    static mut SpaceJunkFrame: *mut crate::graphics::native_abi::frame_desc;

    static mut ScreenWidth: i32;
    static mut ScreenHeight: i32;
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
unsafe fn width() -> i16 {
    (ScreenWidth - 78) as i16
}
fn draw(frame: *mut c_void, point: CPoint, filled: bool) {
    let stamp = Stamp {
        origin: Point {
            x: point.x,
            y: point.y,
        },
        frame: frame as usize,
    };
    if filled {
        CffiPlatform.draw_filled_stamp(stamp);
    } else {
        CffiPlatform.draw_stamp(stamp);
    }
}
fn cursor_frame() -> ContextFrame {
    ContextFrame::retain(std::ptr::NonNull::new(
        Kernel::current().read(|owner| owner.misc_data_frame),
    ))
    .indexed_frame(105)
}

#[no_mangle]
pub unsafe extern "C" fn RepairBackRect(rect: *mut CRect) {
    let old = CffiPlatform.context_clip_rect();
    let mut new = *rect;
    new.corner.x += old.corner.x;
    new.corner.y += old.corner.y;
    new.height += new.corner.y & 1;
    new.corner.y &= !1;
    DrawFromExtraScreen(std::ptr::from_mut(&mut new).cast());
}

unsafe fn erase_coarse(planet: &ContextLease) {
    planet.select();
    CffiPlatform.batch();
    DrawStarBackGround();
    draw_default_planet_sphere(planet);
    CffiPlatform.unbatch();
}

fn save_location_image(scan: &ScanState) {
    let corner = cursor_frame().rect().corner;
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
        context.draw_filled_stamp(&cursor_frame(), point, context.foreground());
    } else {
        context.draw_stamp(&cursor_frame(), point);
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
    CffiPlatform.set_fg_color(Rgba::rgb15(shade, shade, shade));
    draw_cursor(scan, true);
}

pub unsafe fn redraw_surface_scan(scan: &ScanState, location: Option<CPoint>) {
    scan.context().with_selected(|| {
        CffiPlatform.batch();
        DrawPlanet(0, black());
        DrawScannedObjects(1);
        if let Some(location) = location {
            set_location(scan, location, false);
            draw_cursor(scan, false);
        }
        CffiPlatform.unbatch();
    });
}

unsafe fn fuel() -> u16 {
    landing_fuel((*solar_system::current()).SysInfo.planet_info.gravity)
}

unsafe fn spawn_fwiffo() {
    crate::planets::c_api::set_encounter_group(0);
    crate::engine::World::current().write(|world| {
        put_group_info(world, 0, u8::MAX);
        world.ip_groups().reinit();
        assert_eq!(world.npc_built_ships_ref().count(), 0);
        let fleets = world.avail_races_ref();
        let template = *fleets.get(
            fleets
                .at(crate::comm::dispatch::ship::SPATHI_SHIP as usize)
                .unwrap_or_else(|| panic!("Spathi fleet")),
        );
        if let Some(id) = world.npc_built_ships().clone_fragment(
            &template,
            crate::comm::dispatch::ship::SPATHI_SHIP,
            1,
            crate::comm::dispatch::production_captain_name,
        ) {
            world.npc_built_ships().get_mut(id).captains_name_index = 5 + 16;
        }
    });
}

unsafe fn dispatch_lander(
    planet: &ContextLease,
    scan: &ScanState,
    cx: &mut Cx<'_>,
    io: &mut Io<'_>,
) -> bool {
    let runtime = cx.kernel.input_runtime();
    let misc_data = cx.kernel.misc_data_frame;
    let (mut context, old_callback) = cx.with_legacy(|| {
        let landing_fuel = fuel() as i16;
        erase_coarse(planet);
        let old_callback = SetInputCallback(runtime.input(), None);
        (
            prepare_lander(
                scan,
                landing_fuel,
                misc_data.cast(),
                solar_system::current(),
            ),
            old_callback,
        )
    });
    let reply = run_session(
        crate::planet_side::graphics_adapter::SurfaceDrawing {
            planet: planet.clone(),
            scan,
        },
        runtime.input(),
        cx.world.activity_control(),
        cx,
        io,
        &mut context,
    );
    let system = context
        .solar_system
        .cast::<crate::ships::native::abi::SOLARSYS_STATE>();
    if reply.status != STATUS_OK {
        crate::logging::log_add(
            crate::logging::LogLevel::Fatal,
            &format!("Rust PlanetSide failed with status {}", reply.status),
        );
        cx.world.set_activity(cx.world.activity() | CHECK_ABORT);
    } else {
        (*system).SysInfo.planet_info.scan_retrieve_mask = context.retrieval_masks;
        if reply.detail == 4 {
            crate::logging::log_add(
                crate::logging::LogLevel::Error,
                &format!(
                    "Rust PlanetSide adapter failed (operation code {}); returning to orbit",
                    reply.value0 as u64
                ),
            );
        } else if reply.detail == 5 {
            crate::logging::log_add(
                crate::logging::LogLevel::Error,
                "Rust PlanetSide frame budget exhausted; returning to orbit",
            );
        }
    }
    if cx.world.activity() & CHECK_ABORT != 0 {
        return false;
    }
    if cx.world.story_state(StoryKey::FOUND_PLUTO_SPATHI) == 1 {
        cx.with_legacy(|| {
            spawn_fwiffo();
        });
        cx.world.next_activity |= CHECK_LOAD;
        cx.world.set_activity(cx.world.activity() | START_ENCOUNTER);
        cx.with_legacy(|| SaveSolarSysLocation());
        return false;
    }
    cx.with_legacy(|| {
        redraw_planet_orbit_display(planet, scan);
        display::coarse(planet, runtime.input().preferences().coarse_scan);
        SetInputCallback(runtime.input(), old_callback);
    });
    true
}

unsafe fn prepare_lander(
    scan: &ScanState,
    landing_fuel: i16,
    misc_data: *mut c_void,
    system: *mut crate::ships::native::abi::SOLARSYS_STATE,
) -> PlanetSideRunContext {
    DeltaSISGauges(0, -landing_fuel, 0);
    scan.context().select();
    draw_cursor(scan, false);
    PlanetSideRunContext {
        solar_system: system.cast(),
        world: (*system).pOrbitalDesc.cast(),
        misc_data_frame: misc_data,
        energy_frame: (*system).PlanetSideFrame[1].cast(),
        life_frames: std::array::from_fn(|i| (*system).PlanetSideFrame[i + 3].cast()),
        landing_x: i32::from(scan.cursor.borrow().location.x),
        landing_y: i32::from(scan.cursor.borrow().location.y),
        facing: crate::math::random() as u8,
        retrieval_masks: (*system).SysInfo.planet_info.scan_retrieve_mask,
        tick_period: 840 / 35,
        frame_budget: u32::MAX,
        tectonics_rating: (*system).SysInfo.planet_info.tectonics as u8,
        weather_rating: (*system).SysInfo.planet_info.weather as u8,
        temperature: i32::from((*system).SysInfo.planet_info.temperature),
        ..PlanetSideRunContext::default()
    }
}

struct PickState {
    success: bool,
}

unsafe fn pick_input(
    scan: &ScanState,
    input: &InputSession,
    animation: &crate::engine::kernel::planet_animation::PlanetAnimation,
    state: &mut PickState,
) -> u32 {
    let time_in = GetTimeCounter();
    let pulse = input.pulsed();
    if crate::engine::World::current().read(|world| world.activity()) & CHECK_ABORT != 0
        || pulse.menu[KEY_MENU_CANCEL as usize] != 0
    {
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
    CffiPlatform.batch();
    let point = scan.cursor.borrow().location;
    let next = move_cursor((point.x, point.y), dx, dy, width());
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
    CffiPlatform.unbatch();
    SleepThreadUntil(time_in.wrapping_add(840 / 40));
    1
}

unsafe fn pick_side(
    planet: &ContextLease,
    scan: &ScanState,
    cx: &mut Cx<'_>,
    io: &mut Io<'_>,
) -> bool {
    let runtime = cx.kernel.input_runtime();
    let input = runtime.input();
    let mut state = PickState { success: false };
    let animation = cx.kernel.planet_animation.clone();
    cx.with_legacy(|| prepare_pick_side(scan, input));
    io.observer.checkpoint(&CheckpointView::PlanetMenu(4));
    cx.run_input(io, true, |cx, _| {
        cx.with_legacy(|| pick_input(scan, input, &animation, &mut state) != 0)
    });
    io.observer.checkpoint(&CheckpointView::PlanetMenu(3));
    cx.with_legacy(|| DrawStatusMessage(std::ptr::null()));
    let result = if state.success {
        dispatch_lander(planet, scan, cx, io)
    } else {
        cx.with_legacy(|| restore_location_image(scan));
        true
    };
    cx.with_legacy(|| base::SetMenuSounds(input, 15, 16));
    result
}

unsafe fn prepare_pick_side(scan: &ScanState, input: &InputSession) {
    let fuel = fuel();

    use crate::ui::sis::ClearSISRect;
    ClearSISRect(1 << 2);
    scan.context().select();
    CffiPlatform.batch();
    DrawPlanet(0, black());
    DrawScannedObjects(0);
    CffiPlatform.unbatch();
    let message = CString::new(format!(
        "{}{:.1}",
        CffiPlatform.game_string(435 + 5),
        f32::from(fuel) / 100.0
    ))
    .unwrap();
    DrawStatusMessage(message.as_ptr());
    let location = scan.cursor.borrow().location;
    scan.cursor.borrow_mut().move_to(location);
    save_location_image(scan);
    InitLander(0);
    base::SetMenuSounds(input, 0, 16);
}

unsafe fn scanned_stuff(cx: &mut crate::engine::battle::Cx<'_>, y: u16, scan: u16) {
    let g = CffiPlatform;
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
                    &(*solar_system::current()).SysInfo.planet_info,
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
                draw(
                    cx.element(element).next.image.frame.raw().cast(),
                    point,
                    false,
                );
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
                let frame = g.rel_frame(cx.element(element).current.image.frame.raw() as usize, dy)
                    as *mut c_void;
                draw(frame, point, true);
            }
        }
    }
    g.set_fg_color(old_color);
}

unsafe fn scan_planet(
    scan_state: &ScanState,
    planet: &ContextLease,
    input: &InputSession,
    cx: &mut Cx<'_>,
    io: &mut Io<'_>,
    scan_type: u16,
) {
    let (start, end) = if scan_type == 4 {
        (0, 2)
    } else {
        (scan_type, scan_type)
    };
    let text_colors = [
        Rgba::rgb15(19, 0, 0),
        Rgba::rgb15(12, 12, 12),
        Rgba::rgb15(0, 14, 0),
    ];
    let tint_colors = [
        Rgba::rgb15(31, 0, 0),
        Rgba::rgb15(31, 31, 31),
        Rgba::rgb15(0, 31, 0),
    ];
    let mut repair = CRect::default();
    for scan in start..=end {
        let (mut timeout, tint) = cx.with_legacy(|| {
            let baseline_y = ScreenHeight - 13 - 75 - 7;
            planet.select();
            repair = CRect {
                corner: CPoint {
                    x: 0,
                    y: (baseline_y - 10) as i16,
                },
                width: width(),
                height: 11,
            };
            RepairBackRect(&mut repair);
            SetContextFont((Kernel::current().read(|owner| owner.micro_font)).cast());
            CffiPlatform.set_fg_color(text_colors[usize::from(scan)]);
            display::text(
                width() >> 1,
                baseline_y as i16,
                crate::starbase::gfx::Align::Center,
                CffiPlatform.game_string(305 + i32::from(scan)),
            );
            scan_state.context().select();
            CffiPlatform.batch();
            DrawPlanet(0, black());
            CffiPlatform.unbatch();
            (GetTimeCounter(), color(tint_colors[usize::from(scan)]))
        });
        let mut line = 0;
        while line < 84 {
            timeout = timeout.wrapping_add((840 * 7 / 4) / 84);
            if wait_for_any_button_until(cx, io, 1, timeout, 0) != 0 {
                break;
            }
            cx.with_legacy(|| {
                CffiPlatform.batch();
                DrawPlanet(line, tint);
            });
            scanned_stuff(cx, line as u16, scan);
            CffiPlatform.unbatch();
            line += 1;
        }
        if line < 84 {
            cx.with_legacy(|| {
                CffiPlatform.batch();
                DrawPlanet(83, tint);
            });
            scanned_stuff(cx, 83, scan);
            CffiPlatform.unbatch();
        }
    }
    cx.with_legacy(|| {
        planet.select();
        RepairBackRect(&mut repair);
        scan_state.context().select();
    });
    if scan_type == 4 {
        cx.with_legacy(|| {
            DrawPlanet(0, black());
            DrawScannedObjects(0);
        });
    }
    FlushInput(input);
}

unsafe fn unavailable() -> bool {
    let system = solar_system::current();
    (*(*system).pOrbitalDesc).data_index & 0x80 != 0
        || (*system).SysInfo.planet_info.atmo_density == u16::MAX
}

unsafe fn scan_input(
    scan: &ScanState,
    planet: &ContextLease,
    cx: &mut Cx<'_>,
    io: &mut Io<'_>,
    menu: &mut MENU_STATE,
) -> u32 {
    let runtime = cx.kernel.input_runtime();
    let input = runtime.input();
    let pulse = input.pulsed();
    let selected = pulse.menu[KEY_MENU_SELECT as usize] != 0;
    if cx.world.activity() & CHECK_ABORT != 0 {
        return 0;
    }
    if pulse.menu[KEY_MENU_CANCEL as usize] != 0 || (selected && menu.CurState == 3) {
        return 0;
    } else if selected {
        if menu.CurState == 5 {
            if unavailable() {
                play_menu_sound(cx.kernel, cx.world, 2);
                return 1;
            }
            let required = fuel();
            if cx.world.data.flagship.fuel_on_board < u32::from(required)
                || cx.world.data.flagship.num_landers == 0
                || cx.world.data.flagship.crew_enlisted == 0
            {
                play_menu_sound(cx.kernel, cx.world, 2);
                return 1;
            }
            cx.with_legacy(|| CffiPlatform.set_flash(FlashTarget::Off));
            if !pick_side(planet, scan, cx, io) {
                return 0;
            }
            cx.with_legacy(|| DrawMenuStateStrings(7, i16::from(menu.CurState)));
            cx.with_legacy(|| CffiPlatform.set_flash(FlashTarget::Menu3do));
            return 1;
        }
        if (*(*solar_system::current()).pOrbitalDesc).data_index & 0x80 != 0 {
            play_menu_sound(cx.kernel, cx.world, 2);
            return 1;
        }
        scan_planet(scan, planet, input, cx, io, u16::from(menu.CurState));
        if menu.CurState == 4 {
            menu.CurState = 5;
            io.observer.checkpoint(&CheckpointView::PlanetMenu(3));
            cx.with_legacy(|| DrawMenuStateStrings(7, i16::from(menu.CurState)));
        }
    } else if input.preferences().menu == 2 || !unavailable() {
        cx.with_legacy(|| do_menu_chooser(input, menu, 7));
    }
    1
}

pub unsafe fn scan_system(
    scan: &ScanState,
    planet: &ContextLease,
    cx: &mut Cx<'_>,
    io: &mut Io<'_>,
) {
    let runtime = cx.kernel.input_runtime();
    let input = runtime.input();
    let mut menu: MENU_STATE = std::mem::zeroed();
    cx.with_legacy(|| {
        prepare_scan_system(scan, planet, input, &mut menu);
        base::SetMenuSounds(input, 15, 16);
    });
    io.observer.checkpoint(&CheckpointView::PlanetMenu(2));
    cx.run_input(io, false, |cx, io| {
        scan_input(scan, planet, cx, io, &mut menu) != 0
    });
    io.observer.checkpoint(&CheckpointView::PlanetMenu(0));
    cx.with_legacy(|| finish_scan_system(scan, planet));
}

unsafe fn prepare_scan_system(
    scan: &ScanState,
    planet: &ContextLease,
    input: &InputSession,
    menu: &mut MENU_STATE,
) {
    if input.preferences().menu == 1 && unavailable() {
        menu.CurState = 3;
    } else {
        menu.CurState = 4;
        scan.cursor.borrow_mut().location = CPoint {
            x: (width() >> 1) << 2,
            y: (75 >> 1) << 2,
        };
        let cursor = cursor_frame().rect();
        scan.cursor.borrow_mut().rect.width = cursor.extent.width;
        scan.cursor.borrow_mut().rect.height = cursor.extent.height;
        scan.context().select();
        DrawScannedObjects(0);
    }
    DrawMenuStateStrings(7, i16::from(menu.CurState));
    CffiPlatform.set_flash(FlashTarget::Menu3do);
    display::coarse(planet, input.preferences().coarse_scan);
}

unsafe fn finish_scan_system(scan: &ScanState, planet: &ContextLease) {
    CffiPlatform.set_flash(FlashTarget::Off);
    CffiPlatform.batch();
    scan.context().select();
    DrawPlanet(0, black());
    erase_coarse(planet);
    CffiPlatform.unbatch();
    scan.cursor.borrow_mut().erase = None;
}
