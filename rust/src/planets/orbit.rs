//! Orbit display, planet resource ownership, and menu from `planets.c`.

pub mod presentation;

pub enum OrbitSubmenu {
    Devices,
    Cargo,
    Roster,
    Options,
}

/// Synchronous mode and persistence effects supplied by the exploration owner.
pub struct OrbitServices {
    pub submenu: fn(
        &mut crate::engine::battle::Cx<'_>,
        &mut crate::engine::observe::Io<'_>,
        &mut crate::planets::solar_system::SolarSystem,
        OrbitSubmenu,
    ) -> bool,
    pub save_location: fn(
        &mut crate::engine::World,
        &crate::engine::Kernel,
        &crate::planets::solar_system::SolarSystem,
    ),
    pub store_groups: fn(&mut crate::engine::World),
}

fn submenu_selection(current: u8, selected: bool, pc_menu: bool) -> u8 {
    if selected && !pc_menu {
        6
    } else {
        current
    }
}

pub(crate) mod live {
    use super::{presentation::OrbitPresentation, submenu_selection};
    use crate::comm::locdata::CRect;
    use crate::engine::graphics::context_abi::ContextLease;
    use crate::engine::{
        battle::Cx,
        observe::{CheckpointView, Io},
    };
    use crate::mainloop::types::activity_flags::{CHECK_ABORT, CHECK_LOAD, START_ENCOUNTER};
    use crate::planets::solar_system::SolarSystem;
    use crate::starbase::gfx::{ContextId, Gfx, Point, Rect, Rgba, Stamp};
    use crate::starbase::graphics::Graphics;

    use crate::planets::scan::state::ScanState;

    use crate::planets::render::draw_star_background;

    use crate::planets::sphere::live::draw_default_planet_sphere;

    use crate::planets::sphere::live::zoom_in_planet_sphere;

    use crate::planets::sphere::live::rotate_planet_sphere;

    use crate::planets::plangen::live::generate_planet_surface;

    use crate::planets::scan::live::scan_system;

    use crate::planets::pstarmap::star_map;

    fn black() -> crate::comm::locdata::CColor {
        crate::comm::locdata::CColor {
            r: 0,
            g: 0,
            b: 0,
            a: 255,
        }
    }

    pub fn draw_scanned_objects(
        cx: &mut Cx<'_>,
        reversed: bool,
        visible: impl Fn(u16, u16) -> bool,
    ) {
        let mut handle = if reversed {
            cx.elements().tail()
        } else {
            cx.elements().head()
        };
        while let Some(element) = handle {
            handle = if reversed {
                cx.elements().pred(element)
            } else {
                cx.elements().succ(element)
            };
            let state = cx.element(element);
            if visible(state.flags, state.life) {
                let stamp = Stamp {
                    origin: Point {
                        x: state.current.location.x,
                        y: state.current.location.y,
                    },
                    frame: state.next.image.frame.raw() as usize,
                };
                crate::graphics::menu::stamp(
                    stamp.frame as _,
                    crate::graphics::native_abi::POINT {
                        x: stamp.origin.x,
                        y: stamp.origin.y,
                    },
                );
            }
        }
    }

    fn draw_planet_surface_border(g: &Graphics, space: &ContextLease, screen_width: i16) {
        space.with_selected(|| {
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
            r.corner.x -= screen_width - 78 + 1;
            g.fill_rect(r);
            r.width = screen_width - 78 + 2;
            r.height = 1;
            g.fill_rect(r);
            g.unbatch();
            g.set_context_clip_rect(old_clip);
        });
    }

    const FULL: u8 = 0;
    const WAIT: u8 = 1;
    const UPDATE: u8 = 2;

    fn draw_display(cx: &mut Cx<'_>, planet: &ContextLease, scan: &ScanState, mode: u8) {
        let g = Graphics::new(cx.kernel, cx.screen_dimensions());
        let r = {
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
        };
        if mode != UPDATE {
            planet.transition_source(None);
            let dimensions = cx.screen_dimensions();
            crate::ui::border::draw_sis_frame(cx.kernel, cx.world, dimensions);
            crate::ui::sis::draw_campaign_message(cx.kernel, cx.world, dimensions.0, None);
            let name = std::ffi::CStr::from_bytes_until_nul(&cx.world.data.flagship.planet_name)
                .unwrap_or_else(|error| panic!("invalid orbital planet name: {error}"));
            crate::ui::sis::draw_title(cx.kernel, dimensions.0, name);
            let space = ContextLease::require(cx.kernel.space_context);
            let screen_width = cx.screen_dimensions().0 as i16;
            {
                draw_star_background(
                    &space,
                    scan.background
                        .as_ref()
                        .unwrap_or_else(|| panic!("orbit background is not supplied")),
                );
                draw_planet_surface_border(&g, &space, screen_width);
            };
        }
        {
            if mode == WAIT {
                scan.context().select();
                let frame = crate::engine::graphics::drawable_abi::LoadedDrawable::load(
                    c"graphics.orbitenter",
                )
                .unwrap_or_else(|| panic!("missing orbit entry graphic"));
                scan.context().draw_stamp(
                    &frame.lease().context_frame(0),
                    crate::graphics::native_abi::POINT { x: 0, y: 0 },
                );
            } else if mode == FULL {
                draw_default_planet_sphere(planet, &scan.sphere.borrow());
            }
            if mode != WAIT {
                scan.context().select();
                scan.draw_surface(0, black());
            }
            let region = crate::graphics::native_abi::RECT {
                corner: crate::graphics::native_abi::POINT {
                    x: r.corner.x,
                    y: r.corner.y,
                },
                extent: crate::graphics::native_abi::EXTENT {
                    width: r.width,
                    height: r.height,
                },
            };
            if mode != UPDATE {
                scan.context().transition(Some(region));
            }
            g.unbatch();
            scan.context().save_screen_region(region);
        };
    }

    pub fn redraw_planet_orbit_display(cx: &mut Cx<'_>, planet: &ContextLease, scan: &ScanState) {
        draw_display(cx, planet, scan, FULL);
    }

    /// Build an orbit presentation from the supplied generation records.
    ///
    /// Interactive loads require initialized retained graphics.
    /// The supplied system must retain disjoint records and RNG; callbacks may
    /// access them only between its bounded record borrows.
    pub fn load_planet(
        game: &std::cell::RefCell<crate::engine::battle::Cx<'_>>,
        system: &mut crate::engine::world::generation::SolarSys<'_>,
        body: crate::engine::world::generation::Body,
        in_orbit: bool,
        surface: Option<crate::engine::graphics::drawable_abi::LoadedDrawable>,
        resources: (
            crate::planets::report::typography::ReportResources,
            Option<crate::graphics::context_abi::ContextFrame>,
        ),
        populate: impl FnOnce(
            &mut crate::engine::world::generation::SolarSys<'_>,
            crate::planets::report::typography::ReportContext,
        ),
    ) -> Option<OrbitPresentation> {
        let (reports, background) = resources;
        #[cfg(feature = "debug-process")]
        if !game.borrow().kernel.diagnostics.interactive() {
            return None;
        }
        let width = game.borrow().screen_dimensions().0 as i16 - 78;
        let presentation = OrbitPresentation::new(game.borrow().kernel, width, reports, background);
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
        let planet_type = {
            crate::ui::settings::stop_music();
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
        };
        populate(system, presentation.scan().report_context());
        play_planet_music(&game.borrow(), planet_type);
        if wait {
            zoom_in_planet_sphere(&context, &sphere);
        };
        draw_display(
            &mut game.borrow_mut(),
            &context,
            presentation.scan(),
            if wait { UPDATE } else { FULL },
        );
        Some(presentation)
    }

    fn play_planet_music(game: &Cx<'_>, planet_type: u8) {
        if !crate::sound::music::plr_playing_current() {
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
                game.world.data.game.glob_flags & crate::engine::world::MUSIC_DISABLED == 0,
            );
        }
    }

    /// Release the visit's orbit presentation.
    ///
    pub fn free_planet(cx: &mut Cx<'_>, solar: &mut SolarSystem) {
        let space_context = cx.kernel.space_context;
        {
            crate::ui::settings::stop_music();
            solar.sys.frames.fill_with(|| None);
            solar.sys.report.clear();
            crate::graphics::menu::select(space_context);
        };
    }

    fn do_orbit(
        cx: &mut Cx<'_>,
        io: &mut Io<'_>,
        solar: &mut SolarSystem,
        planet: &ContextLease,
        scan: &ScanState,
        menu: &mut u8,
        services: &super::OrbitServices,
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
        if crate::ui::menu::choose_menu_state(cx, input, menu, 0) != 0 || !selected {
            return 1;
        }
        crate::ui::sis::clear_flash(cx.kernel);
        match *menu {
            0 => {
                scan_system(solar, scan, planet, cx, io, services);
                if cx.world.activity() & START_ENCOUNTER != 0 {
                    return 0;
                }
            }
            2 => {
                selected = (services.submenu)(cx, io, solar, super::OrbitSubmenu::Devices);
                if cx.world.activity() & START_ENCOUNTER != 0 {
                    return 0;
                }
            }
            3 => {
                (services.submenu)(cx, io, solar, super::OrbitSubmenu::Cargo);
            }
            4 => selected = (services.submenu)(cx, io, solar, super::OrbitSubmenu::Roster),
            5 => {
                if !(services.submenu)(cx, io, solar, super::OrbitSubmenu::Options) {
                    return 0;
                }
            }
            1 => {
                let old = runtime.input().replace_callback(None);
                let dimensions = cx.screen_dimensions();
                crate::ui::sis::repair_sis_border(cx.kernel, dimensions);
                let autopilot = star_map(cx, io);
                if cx.world.activity() & CHECK_ABORT != 0 {
                    return 0;
                }
                runtime.input().replace_callback(old);
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
                *menu = submenu_selection(*menu, selected, input.preferences().menu == 2);
                crate::ui::menu::draw_menu_state(
                    cx.kernel,
                    cx.world,
                    (cx.screen_dimensions().0, cx.screen_dimensions().1),
                    0,
                    i16::from(*menu),
                );
            }
            crate::ui::sis::set_menu_flash(cx.kernel, cx.screen_dimensions(), false);
        }
        1
    }

    fn orbit_frame_hook(
        planet: ContextLease,
        sphere: std::rc::Rc<std::cell::RefCell<crate::planets::sphere::SphereState>>,
        animation: std::rc::Rc<crate::engine::kernel::planet_animation::PlanetAnimation>,
    ) -> crate::engine::input::InputFrameHook {
        crate::engine::input::InputFrameHook::new(move |_, _| {
            rotate_planet_sphere(&planet, &sphere, &animation, true);
        })
    }

    pub fn planet_orbit_menu(
        cx: &mut Cx<'_>,
        io: &mut Io<'_>,
        solar: &mut SolarSystem,
        presentation: &OrbitPresentation,
        services: &super::OrbitServices,
    ) {
        let runtime = cx.kernel.input_runtime();
        let input = runtime.input();
        let mut menu = 0;
        let planet = presentation.context();
        let hook = orbit_frame_hook(
            planet.clone(),
            presentation.sphere(),
            cx.kernel.planet_animation.clone(),
        );
        let dimensions = cx.screen_dimensions();
        crate::ui::menu::draw_menu_state(cx.kernel, cx.world, dimensions, 0, 0);
        crate::ui::sis::set_menu_flash(cx.kernel, dimensions, false);
        input.set_menu_sounds(15, 16);
        let old = runtime.input().replace_callback(Some(hook));
        io.observer.checkpoint(&CheckpointView::PlanetMenu(1));
        solar.run_input(cx, io, true, |cx, io, solar| {
            do_orbit(
                cx,
                io,
                solar,
                &planet,
                presentation.scan(),
                &mut menu,
                services,
            ) != 0
        });
        io.observer.checkpoint(&CheckpointView::PlanetMenu(0));
        runtime.input().replace_callback(old);
        crate::ui::sis::clear_flash(cx.kernel);
        crate::ui::menu::draw_menu_state(cx.kernel, cx.world, dimensions, 1, -6);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn real_orbit_entry_points_use_supplied_owners_without_unsafe() {
        let _: fn(
            &mut crate::engine::battle::Cx<'_>,
            &mut crate::planets::solar_system::SolarSystem,
        ) = super::live::free_planet;
        type Visible = fn(u16, u16) -> bool;
        let _: fn(&mut crate::engine::battle::Cx<'_>, bool, Visible) =
            super::live::draw_scanned_objects;
        let _ = super::live::planet_orbit_menu;
    }

    #[test]
    #[cfg(feature = "debug-process")]
    fn noninteractive_load_does_not_enter_graphics_or_generate_nodes() {
        let mut game = crate::engine::Game::new();
        let _quiet = game.kernel.diagnostics.suppress_interaction();
        let mut battle = crate::engine::battle::Battle::new();
        let cx = std::cell::RefCell::new(crate::engine::battle::Cx::new(
            &mut battle,
            &mut game.world,
            &mut game.kernel,
            crate::math::random,
        ));
        let mut system = crate::engine::world::generation::SolarSys::new(
            &crate::planets::tables::PLANET_ARRAY,
            0,
            0,
            crate::math::RandomContext::with_seed(12345),
        );
        assert!(super::live::load_planet(
            &cx,
            &mut system,
            crate::engine::world::generation::Body::Planet(0),
            false,
            None,
            (Default::default(), None),
            |_, _| panic!("noninteractive orbit load must not generate nodes"),
        )
        .is_none());
    }

    #[test]
    fn successful_3do_submenus_jump_to_navigation_but_pc_and_cancel_stay() {
        for current in 0..7 {
            assert_eq!(submenu_selection(current, true, false), 6);
            assert_eq!(submenu_selection(current, false, false), current);
            assert_eq!(submenu_selection(current, true, true), current);
        }
    }
}
