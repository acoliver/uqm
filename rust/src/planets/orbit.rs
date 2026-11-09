//! Orbit display, planet resource ownership, and menu from `planets.c`.

pub mod presentation;

fn submenu_selection(current: u8, selected: bool, pc_menu: bool) -> u8 {
    if selected && !pc_menu {
        6
    } else {
        current
    }
}

#[cfg(not(test))]
pub(crate) mod live {
    use super::{presentation::OrbitPresentation, submenu_selection};
    use crate::c_bindings::{global_state::MENU_STATE, solar_system};
    use crate::comm::locdata::CRect;
    use crate::engine::graphics::context_abi::ContextLease;
    use crate::engine::Kernel as KernelOwner;
    use crate::engine::{
        battle::Cx,
        observe::{CheckpointView, Io},
    };
    use crate::mainloop::restart_menu::c_extern::{self as base, SetContext};
    use crate::mainloop::types::activity_flags::{CHECK_ABORT, CHECK_LOAD, START_ENCOUNTER};
    use crate::planet_side::orbit_scan;
    use crate::starbase::cffi::CffiPlatform;
    use crate::starbase::gfx::{ContextId, FlashTarget, Gfx, Point, Rect, Rgba, Stamp};
    use std::ffi::c_void;

    use crate::planets::scan::state::ScanState;

    use crate::planets::solarsys::drawing::DrawStarBackGround;

    use crate::graphics::gfx_common_abi::LoadIntoExtraScreen;

    use crate::planets::sphere::live::draw_default_planet_sphere;

    use crate::planets::sphere::live::draw_planet;

    use crate::planets::sphere::live::zoom_in_planet_sphere;

    use crate::planets::sphere::live::rotate_planet_sphere;

    use crate::planets::plangen::live::generate_planet_surface;

    use crate::sound::heart_ffi::PLRPlaying;

    use crate::strings::string_table::ReleaseStringTable;

    use crate::strings::string_table::DestroyStringTable;

    use crate::planets::scan::live::scan_system;

    use crate::planets::devices::devices_menu;

    use crate::planets::cargo::live::cargo_menu;

    use crate::planets::roster::live::roster_menu;

    use crate::front_end::gameopt::runtime::game_options;

    use crate::planets::pstarmap::star_map;

    use crate::mainloop::input_abi::live::SetInputCallback;

    fn black() -> crate::comm::locdata::CColor {
        crate::comm::locdata::CColor {
            r: 0,
            g: 0,
            b: 0,
            a: 255,
        }
    }

    #[no_mangle]
    pub unsafe extern "C" fn DrawScannedObjects(reversed: u32) {
        crate::app::bridge::enter(|cx| {
            let mut handle = if reversed != 0 {
                cx.elements().tail()
            } else {
                cx.elements().head()
            };
            while let Some(element) = handle {
                handle = if reversed != 0 {
                    cx.elements().pred(element)
                } else {
                    cx.elements().succ(element)
                };
                let state = cx.element(element);
                if crate::planet_side::orbit_scan::node_visible(
                    state.flags,
                    state.life,
                    &(*solar_system::current()).SysInfo.planet_info,
                ) {
                    CffiPlatform.draw_stamp(Stamp {
                        origin: Point {
                            x: state.current.location.x,
                            y: state.current.location.y,
                        },
                        frame: state.next.image.frame.raw() as usize,
                    });
                }
            }
        });
    }

    #[no_mangle]
    unsafe extern "C" fn DrawPlanetSurfaceBorder() {
        let g = CffiPlatform;
        let old = g.set_context(ContextId::Space);
        let old_clip = g.context_clip_rect();
        let mut clip = old_clip;
        clip.corner.x -= 1;
        clip.width += 2;
        clip.height += 1;
        g.set_context_clip_rect(clip);
        g.batch();
        g.set_fg_color(Rgba::rgb15(10, 10, 10));
        let mut r = Rect {
            corner: Point {
                x: 0,
                y: clip.height - 75 - 5,
            },
            width: clip.width,
            height: 3,
        };
        g.fill_rect(r);
        g.set_fg_color(Rgba::rgb15(8, 8, 8));
        r.width -= 1;
        r.height = 1;
        r.corner.x = 1;
        r.corner.y -= 1;
        g.fill_rect(r);
        r.width = 1;
        r.height = 77;
        r.corner.y += 4;
        r.corner.x = clip.width - 1;
        g.fill_rect(r);
        g.set_fg_color(Rgba::rgb15(16, 16, 16));
        r.corner.x -= g.screen_size().0 as i16 - 78 + 1;
        g.fill_rect(r);
        r.width = g.screen_size().0 as i16 - 78 + 2;
        r.height = 1;
        g.fill_rect(r);
        g.unbatch();
        g.set_context_clip_rect(old_clip);
        g.restore_context(old);
    }

    const FULL: u8 = 0;
    const WAIT: u8 = 1;
    const UPDATE: u8 = 2;

    unsafe fn draw_display(cx: &mut Cx<'_>, planet: &ContextLease, scan: &ScanState, mode: u8) {
        let g = CffiPlatform;
        let mut r = cx.with_legacy(|| {
            g.set_context(ContextId::Space);
            let clip = g.context_clip_rect();
            g.batch();
            CRect {
                corner: crate::comm::locdata::CPoint {
                    x: clip.corner.x,
                    y: clip.corner.y,
                },
                width: clip.width,
                height: clip.height,
            }
        });
        if mode != UPDATE {
            base::SetTransitionSource(std::ptr::null_mut());
            let dimensions = cx.screen_dimensions();
            crate::ui::border::draw_sis_frame(cx.kernel, cx.world, dimensions);
            crate::ui::sis::draw_campaign_message(cx.kernel, cx.world, dimensions.0, None);
            let name = std::ffi::CStr::from_bytes_until_nul(&cx.world.data.flagship.planet_name)
                .unwrap_or_else(|error| panic!("invalid orbital planet name: {error}"));
            crate::ui::sis::draw_title(cx.kernel, dimensions.0, name);
            cx.with_legacy(|| {
                DrawStarBackGround();
                DrawPlanetSurfaceBorder();
            });
        }
        cx.with_legacy(|| {
            if mode == WAIT {
                scan.context().select();
                let frame = base::CaptureDrawable(
                    (base::LoadGraphicInstance(c"graphics.orbitenter".as_ptr())).cast(),
                );
                g.draw_stamp(Stamp {
                    origin: Point { x: 0, y: 0 },
                    frame: frame as usize,
                });
                base::DestroyDrawable(base::ReleaseDrawable(frame));
            } else if mode == FULL {
                draw_default_planet_sphere(planet, &scan.sphere.borrow());
            }
            if mode != WAIT {
                scan.context().select();
                draw_planet(&mut scan.sphere.borrow_mut(), 0, black());
            }
            if mode != UPDATE {
                base::ScreenTransition(3, (&mut r as *mut CRect).cast());
            }
            g.unbatch();
            LoadIntoExtraScreen(std::ptr::from_mut(&mut r).cast());
        });
    }

    pub unsafe fn redraw_planet_orbit_display(
        cx: &mut Cx<'_>,
        planet: &ContextLease,
        scan: &ScanState,
    ) {
        draw_display(cx, planet, scan, FULL);
    }

    /// Build an orbit presentation from the supplied generation records.
    ///
    /// # Safety
    /// Game-thread call. Interactive loads require initialized native graphics.
    /// The supplied system must retain disjoint records and RNG; callbacks may
    /// access them only between its bounded record borrows.
    pub unsafe fn load_planet(
        game: &std::cell::RefCell<crate::engine::battle::Cx<'_>>,
        io: &std::cell::RefCell<crate::engine::observe::Io<'_>>,
        system: &mut crate::engine::world::generation::SolarSys<'_>,
        body: crate::engine::world::generation::Body,
        in_orbit: bool,
        surface: Option<crate::engine::graphics::drawable_abi::LoadedDrawable>,
        reports: crate::planets::report::typography::ReportResources,
    ) -> Option<OrbitPresentation> {
        #[cfg(feature = "debug-process")]
        if crate::front_end::uqmdebug::DISABLE_INTERACTIVITY != 0 {
            return None;
        }
        let width = CffiPlatform.screen_size().0 as i16 - 78;
        let presentation = OrbitPresentation::new(game.borrow().kernel, width, reports);
        let context = presentation.context();
        let sphere = presentation.sphere();
        let (animation, last_activity, pulsing_shield) = {
            let game = game.borrow();
            (
                game.kernel.planet_animation.clone(),
                game.kernel.last_activity,
                game.kernel.configuration.preferences().shield == 1,
            )
        };
        let wait = last_activity & CHECK_LOAD == 0;
        assert!(in_orbit);
        if wait {
            draw_display(&mut game.borrow_mut(), &context, presentation.scan(), WAIT);
        }
        let planet_type = game.borrow_mut().with_legacy(|| {
            base::stop_music();
            let location = system.orbital_location(body);
            system.orbital(body, |data| {
                generate_planet_surface(
                    &mut sphere.borrow_mut(),
                    &animation,
                    pulsing_shield,
                    crate::planets::plangen::live::SurfaceInputs {
                        rng: data.rng,
                        planet: data.world,
                        info: &data.info.planet_info,
                        location: crate::graphics::drawable::Point::new(location.x, location.y),
                        width: width as usize,
                    },
                    surface,
                );
            });
            system.body_data_index(body) & !0x80
        });
        orbit_scan::generate_planet_side(game, io, presentation.scan().report_context());
        {
            let game = game.borrow();
            if PLRPlaying(usize::MAX as *mut c_void) == 0 {
                let music = game
                    .kernel
                    .lander_resources
                    .as_ref()
                    .and_then(|resources| resources.music(planet_type));
                crate::sound::music::remember_music(
                    music.map_or(
                        crate::sound::music::MusicRequest::Absent,
                        crate::sound::music::MusicRequest::Track,
                    ),
                    true,
                    1,
                    game.world.data.game.glob_flags & crate::navigation::abi::MUSIC_DISABLED as u8
                        == 0,
                );
            }
        }
        game.borrow_mut().with_legacy(|| {
            if wait {
                zoom_in_planet_sphere(&context, &sphere);
            }
        });
        draw_display(
            &mut game.borrow_mut(),
            &context,
            presentation.scan(),
            if wait { UPDATE } else { FULL },
        );
        Some(presentation)
    }

    #[no_mangle]
    /// # Safety
    /// Call on the game thread with a loaded solar system. No other code may retain
    /// references to the planet resources released by this call.
    pub unsafe extern "C" fn FreePlanet() {
        let system = solar_system::current();
        base::stop_music();
        for i in 0..(*system).PlanetSideFrame.len() {
            base::DestroyDrawable(base::ReleaseDrawable((*system).PlanetSideFrame[i].cast()));
            (*system).PlanetSideFrame[i] = std::ptr::null_mut();
        }
        DestroyStringTable(ReleaseStringTable(
            (*system).SysInfo.planet_info.discovery_string.cast(),
        ));
        (*system).SysInfo.planet_info.discovery_string = std::ptr::null_mut();
        SetContext(KernelOwner::current().read(|owner| owner.space_context));
    }

    unsafe fn do_orbit(
        cx: &mut Cx<'_>,
        io: &mut Io<'_>,
        planet: &ContextLease,
        scan: &ScanState,
        menu: &mut MENU_STATE,
    ) -> u32 {
        let runtime = cx.kernel.input_runtime();
        let input = runtime.input();
        use crate::c_bindings::controller_input::KEY_MENU_SELECT;
        let mut selected = input.pulsed().menu[KEY_MENU_SELECT as usize] != 0;
        if cx.world.activity() & (CHECK_ABORT | CHECK_LOAD) != 0
            || cx.world.data.flagship.crew_enlisted == u16::MAX
        {
            return 0;
        }
        if crate::ui::menu::choose_menu_state(cx, input, &mut menu.CurState, 0) != 0 || !selected {
            return 1;
        }
        cx.with_legacy(|| CffiPlatform.set_flash(FlashTarget::Off));
        match menu.CurState {
            0 => {
                scan_system(scan, planet, cx, io);
                if cx.world.activity() & START_ENCOUNTER != 0 {
                    return 0;
                }
            }
            2 => {
                selected = devices_menu(cx, io) != 0;
                if cx.world.activity() & START_ENCOUNTER != 0 {
                    return 0;
                }
            }
            3 => cargo_menu(cx, io),
            4 => selected = roster_menu(cx, io) != 0,
            5 => {
                if game_options(cx, io) == 0 {
                    return 0;
                }
            }
            1 => {
                let old = cx.with_legacy(|| SetInputCallback(runtime.input(), None));
                let dimensions = cx.screen_dimensions();
                crate::ui::sis::repair_sis_border(cx.kernel, dimensions);
                let autopilot = star_map(cx, io);
                if cx.world.activity() & CHECK_ABORT != 0 {
                    return 0;
                }
                cx.with_legacy(|| SetInputCallback(runtime.input(), old));
                if autopilot == 0 {
                    draw_display(cx, planet, scan, FULL);
                } else {
                    return 0;
                }
            }
            6 => return 0,
            _ => {}
        }
        if cx.world.activity() & CHECK_ABORT == 0 {
            if selected {
                menu.CurState =
                    submenu_selection(menu.CurState, selected, input.preferences().menu == 2);
                crate::ui::menu::draw_menu_state(
                    cx.kernel,
                    cx.world,
                    (cx.screen_dimensions().0, cx.screen_dimensions().1),
                    0,
                    i16::from(menu.CurState),
                );
            }
            cx.with_legacy(|| CffiPlatform.set_flash(FlashTarget::Menu3do));
        }
        1
    }

    fn orbit_frame_hook(
        planet: ContextLease,
        sphere: std::rc::Rc<std::cell::RefCell<crate::planets::sphere::SphereState>>,
        animation: std::rc::Rc<crate::engine::kernel::planet_animation::PlanetAnimation>,
    ) -> crate::engine::input::InputFrameHook {
        // SAFETY: The orbit menu removes this hook before freeing its drawing resources.
        crate::engine::input::InputFrameHook::new(move |_, _| unsafe {
            rotate_planet_sphere(&planet, &sphere, &animation, true);
        })
    }

    pub unsafe fn planet_orbit_menu(
        cx: &mut Cx<'_>,
        io: &mut Io<'_>,
        presentation: &OrbitPresentation,
    ) {
        let runtime = cx.kernel.input_runtime();
        let input = runtime.input();
        let mut menu: MENU_STATE = std::mem::zeroed();
        let planet = presentation.context();
        let hook = orbit_frame_hook(
            planet.clone(),
            presentation.sphere(),
            cx.kernel.planet_animation.clone(),
        );
        let dimensions = cx.screen_dimensions();
        crate::ui::menu::draw_menu_state(cx.kernel, cx.world, dimensions, 0, 0);
        crate::ui::sis::set_menu_flash(cx.kernel, dimensions, false);
        base::SetMenuSounds(input, 15, 16);
        let old = SetInputCallback(runtime.input(), Some(hook));
        io.observer.checkpoint(&CheckpointView::PlanetMenu(1));
        cx.run_input(io, true, |cx, io| {
            do_orbit(cx, io, &planet, presentation.scan(), &mut menu) != 0
        });
        io.observer.checkpoint(&CheckpointView::PlanetMenu(0));
        SetInputCallback(runtime.input(), old);
        crate::ui::sis::clear_flash(cx.kernel);
        crate::ui::menu::draw_menu_state(cx.kernel, cx.world, dimensions, 1, -6);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn successful_3do_submenus_jump_to_navigation_but_pc_and_cancel_stay() {
        for current in 0..7 {
            assert_eq!(submenu_selection(current, true, false), 6);
            assert_eq!(submenu_selection(current, false, false), current);
            assert_eq!(submenu_selection(current, true, true), current);
        }
    }
}
