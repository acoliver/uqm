//! Real packaged resources and cleanup, without a window or gameplay loop.

use super::Harness;
use uqm_rust::engine::graphics::surface::CapturedSurface;
use uqm_rust::engine::test_support::native_probe;
use uqm_rust::planets::c_api;
use uqm_rust::planets::generate::consts::{MAP_HEIGHT, MAP_WIDTH};
use uqm_rust::planets::generate::sol::Generator as GENERATE_SOL_FUNCTIONS;
use uqm_rust::state::game_state_keys::set_game_state;

pub fn check(h: &mut Harness) {
    let content = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../sc2/content")
        .canonicalize()
        .unwrap();
    unsafe {
        c_api::log_init(100);
        let mount = uqm_engine::test_support::native_session::ContentMount::new(&content);
        let Some(dir) = uqm_rust::io::content_dir() else {
            panic!("checked-in content directory must open");
        };
        uqm_rust::resource::init();
        uqm_rust::resource::load_index(&dir, c"uqm.rmp", None);
        names(h);
        binary_palettes();
        scenes(h);
        uqm_rust::graphics::state::flush_ex(true);
        let baseline = native_probe::allocations();
        indexed_rescale();
        let earth = CapturedSurface::load(c"graphics.earthmask");
        assert!(
            !earth.first_frame().is_null(),
            "real Earth definition must load"
        );
        let raw = earth.first_frame();
        let before_tint = frame_rgb(raw);
        let earth = earth
            .into_definition(MAP_WIDTH as i16, MAP_HEIGHT as i16)
            .unwrap();
        assert_eq!(
            (
                native_probe::frame_bounds(earth.first_frame()).0.width,
                native_probe::frame_bounds(earth.first_frame()).0.height
            ),
            (MAP_WIDTH as i16, MAP_HEIGHT as i16)
        );
        println!(
            "real Earth resource: source={raw:?}, normalized={:?}",
            earth.first_frame()
        );
        let system = &raw mut *h.state;
        let world = &raw mut (*system).sys.planets[2];
        (*world).data_index = 42 | 128;
        (*world).rand_seed = 104729;
        (*world).location.x = 100;
        (*world).location.y = 0;
        (*system).sys.sun.num_planets = 9;
        (*system).enter_inner(2);
        (*system).enter_orbit(uqm_rust::engine::world::generation::Body::Planet(2));
        uqm_rust::planets::generate::sol::orbital::physical_overrides(
            &mut (*system).sys.info,
            uqm_rust::planets::generate::special::Body::Planet(2),
        );
        let animation = crate::battle_context::with(|cx| cx.kernel.planet_animation.clone());
        let mut sphere = uqm_rust::planets::sphere::SphereState::default();
        uqm_rust::planets::plangen::live::generate_planet_surface(
            &mut sphere,
            &animation,
            false,
            uqm_rust::planets::plangen::live::SurfaceInputs {
                rng: &mut (*system).sys.rng,
                planet: &*world,
                info: &(*system).sys.info.planet_info,
                location: uqm_rust::graphics::geometry::Point::new(
                    (*world).location.x,
                    (*world).location.y,
                ),
                width: usize::from(MAP_WIDTH),
            },
            earth.into_drawable(),
        );
        assert_eq!(
            (*system).sys.rng.seed(),
            104729,
            "packaged definitions still reseed the supplied system RNG"
        );

        let topography = sphere.topography();
        let topo_frame = topography.frame(0).unwrap();
        assert!(!topo_frame.is_null());
        assert_eq!(
            sphere.elevation().len(),
            MAP_WIDTH as usize * MAP_HEIGHT as usize
        );
        assert_eq!(
            sphere.colors().len(),
            (MAP_WIDTH as usize + 75) * MAP_HEIGHT as usize
        );
        let first_elevation = sphere.elevation()[0];
        assert_eq!(sphere.sphere_frame().rect().extent.width, 75);
        let after_tint = frame_rgb(topo_frame);
        assert_eq!(after_tint.len(), before_tint.len());
        for (index, (before, after)) in before_tint.iter().zip(&after_tint).enumerate() {
            assert_eq!(
                *after,
                [255, before[1], before[2]],
                "Earth shield tint pixel {index}"
            );
        }
        println!(
            "real Earth shield tint: all {} RGB pixels equal [255, source.g, source.b]",
            after_tint.len()
        );
        let planet_system = &mut *system;
        crate::battle_context::with(|cx| c_api::free_planet(cx, planet_system));
        assert_eq!(sphere.elevation()[0], first_elevation);
        drop(topography);
        drop(sphere);
        uqm_rust::graphics::state::flush_ex(true);
        println!("real Earth definition consumed by actual GeneratePlanetSurface, shield and sphere allocated, then FreePlanet plus owned release cleared surface/sphere/topography allocations");
        assert_eq!(
            native_probe::allocations(),
            baseline,
            "real Earth generation/rescale/cleanup must release all graphics allocations"
        );
        uqm_rust::resource::uninit();
        drop(mount);
    }
}

fn frame_rgb(frame: uqm_rust::graphics::drawable::FrameId) -> Vec<[u8; 3]> {
    let (bounds, _) = native_probe::frame_bounds(frame);
    native_probe::read_pixels(frame, i32::from(bounds.width), i32::from(bounds.height))
        .unwrap_or_else(|| panic!("Earth frame"))
        .into_iter()
        .map(|p| [p.r, p.g, p.b])
        .collect()
}

unsafe fn names(h: &mut Harness) {
    // SAFETY: the serialized harness retains its captured strings and initialized system through each name callback.
    unsafe {
        let strings_owner =
            uqm_rust::engine::strings::owned_table::LoadedTextTable::load(c"text.starcon").unwrap();
        crate::battle_context::with(|cx| {
            assert!(cx.kernel.micro_font.is_none());
            cx.kernel.install_game_text(strings_owner);
        });
        let system = &raw mut *h.state;
        for (index, expected) in [(0, "Mercury"), (2, "Earth"), (8, "Pluto")] {
            let parent = &raw mut (*system).sys.planets[index];
            (*parent).data_index = index as u8 + 128;
            assert!(crate::invoke_generation(&mut *system, |view, host| {
                GENERATE_SOL_FUNCTIONS.generate_name(
                    view,
                    uqm_rust::engine::world::generation::Body::Planet(index as u8),
                    host,
                )
            }));
            let name = crate::battle_context::with(|cx| cx.world.data.flagship.planet_name);
            let name = name.as_ptr().cast::<std::ffi::c_char>();
            assert_eq!(std::ffi::CStr::from_ptr(name).to_str().unwrap(), expected);
            assert_eq!(
                crate::battle_context::with(|cx| {
                    uqm_rust::state::game_state_keys::get_game_state(
                        cx.world,
                        uqm_rust::engine::StoryKey::BATTLE_PLANET,
                    )
                }),
                index as u32 + 128
            );
            let moon = &raw mut (*system).sys.moons[1];
            (*moon).data_index = 27;
            assert!(crate::invoke_generation(&mut *system, |view, host| {
                GENERATE_SOL_FUNCTIONS.generate_name(
                    view,
                    uqm_rust::engine::world::generation::Body::Moon {
                        planet: index as u8,
                        moon: 1,
                    },
                    host,
                )
            }));
            let name = crate::battle_context::with(|cx| cx.world.data.flagship.planet_name);
            assert_eq!(
                std::ffi::CStr::from_ptr(name.as_ptr().cast())
                    .to_str()
                    .unwrap(),
                expected
            );
            assert_eq!(
                crate::battle_context::with(|cx| {
                    uqm_rust::state::game_state_keys::get_game_state(
                        cx.world,
                        uqm_rust::engine::StoryKey::BATTLE_PLANET,
                    )
                }),
                index as u32 + 128
            );
        }
        crate::battle_context::with(|cx| {
            uqm_rust::engine::Kernel::release_encounter_text(cx.kernel)
        });
        uqm_rust::graphics::state::flush_ex(true);
        println!("Sol name ABI: real game strings, parent names and parent battle frames for planets and moons");
    }
}

unsafe fn scenes(h: &mut Harness) {
    let _quiet = crate::battle_context::with(|cx| cx.kernel.diagnostics.suppress_interaction());
    for pluto in [true, false] {
        h.state.sys.info = std::mem::zeroed();
        h.state.sys.planets = std::mem::zeroed();
        h.state.sys.moons = std::mem::zeroed();
        h.set_star(0, 0);
        let system = &raw mut *h.state;
        let planet = &raw mut (*system).sys.planets[if pluto { 8 } else { 2 }];
        (*planet).radius = 100;
        (*planet).data_index = if pluto { 22 } else { 42 | 128 };
        let world = if pluto {
            planet
        } else {
            let moon = &raw mut (*system).sys.moons[1];
            (*moon).radius = 35;
            (*moon).data_index = 27;
            moon
        };
        (*world).rand_seed = 104729;
        crate::battle_context::with(|cx| {
            set_game_state(cx.world, uqm_rust::engine::StoryKey::FOUND_PLUTO_SPATHI, 0)
        });
        crate::battle_context::with(|cx| {
            set_game_state(cx.world, uqm_rust::engine::StoryKey::MOONBASE_DESTROYED, 0)
        });
        // SAFETY: Harness retains installed owners and live solar/planet
        // descriptors throughout this synchronous orbital callback.
        let typography = crate::battle_context::with(|cx| {
            let game = std::cell::RefCell::new(cx.reborrow());
            let mut observer = uqm_rust::engine::observe::Unobserved;
            let io = std::cell::RefCell::new(uqm_rust::engine::observe::Io {
                observer: &mut observer,
            });
            let mut pickup = uqm_rust::planet_side::special_effects::PickupContext::NoLander;
            let mut site = uqm_rust::planets::solarsys::site::Site::new(
                &game,
                &io,
                (*system).star(),
                (*system).sys.star_index(),
                (*system).in_orbit(),
                &mut pickup,
                None,
            );
            assert!(site.invoke(&mut (*system).sys, |_, view, host| {
                GENERATE_SOL_FUNCTIONS.generate_orbital(
                    view,
                    if pluto {
                        uqm_rust::engine::world::generation::Body::Planet(8)
                    } else {
                        uqm_rust::engine::world::generation::Body::Moon { planet: 2, moon: 1 }
                    },
                    host,
                )
            }));
            let Some(typography) = site.report_typography() else {
                panic!("Sol discovery report has no typography");
            };
            typography
        });
        let font = typography.font();
        let effect = typography.effect();
        let measured = font.measure(
            c"A",
            uqm_rust::graphics::geometry::Point { x: 0, y: 0 },
            uqm_rust::graphics::geometry::TextAlign::Left,
            1,
        );
        assert!(
            measured.visible,
            "lander A glyph missing, leading={}",
            measured.leading
        );
        assert!(effect.frame_rect(0).extent.width > 0);
        let info = &(*system).sys.info.planet_info;
        assert!(h.state.sys.report.current().is_some());
        assert!(h.state.sys.frames[1].is_some());
        assert_eq!(info.planet_density, if pluto { 33 } else { 60 });
        let planet_system = &mut *system;
        crate::battle_context::with(|cx| c_api::free_planet(cx, planet_system));
        uqm_rust::graphics::state::flush_ex(true);
        assert!((*system).sys.report.current().is_none());
        drop(typography);
        assert!(
            font.measure(
                c"A",
                uqm_rust::graphics::geometry::Point { x: 0, y: 0 },
                uqm_rust::graphics::geometry::TextAlign::Left,
                1
            )
            .visible
        );
        assert!(effect.frame_rect(0).extent.width > 0);
        drop((font, effect));
        uqm_rust::graphics::state::flush_ex(true);
        assert!((*system).sys.frames.iter().all(|frame| frame.is_none()));
        println!("Sol real scene {}: font + effect + mask + report loaded through actual orbital ABI, then explicit font leases survive FreePlanet and release after their final user", if pluto { "Pluto" } else { "moonbase" });
    }
}

unsafe fn indexed_rescale() {
    let original = CapturedSurface::load(c"graphics.earthmask");
    let mut pixels = vec![0; usize::from(MAP_WIDTH) * usize::from(MAP_HEIGHT)];
    uqm_rust::graphics::drawable::ContextFrame::retain(original.first_frame().at(1)).read_indexes(
        &mut pixels,
        i32::from(MAP_WIDTH),
        i32::from(MAP_HEIGHT),
    );
    let width = MAP_WIDTH / 2;
    let height = MAP_HEIGHT / 2;
    let normalized = original
        .into_definition(width as i16, height as i16)
        .unwrap();
    let mut actual = vec![0; usize::from(width) * usize::from(height)];
    uqm_rust::graphics::drawable::ContextFrame::retain(normalized.first_frame().at(1))
        .read_indexes(&mut actual, i32::from(width), i32::from(height));
    let dx = ((u32::from(MAP_WIDTH) - 1) << 16) / (u32::from(width) - 1);
    let dy = ((u32::from(MAP_HEIGHT) - 1) << 16) / (u32::from(height) - 1);
    for y in 0..u32::from(height) {
        for x in 0..u32::from(width) {
            let source_x = (0x6000 + x * dx) >> 16;
            let source_y = (0x6000 + y * dy) >> 16;
            assert_eq!(
                actual[(y * u32::from(width) + x) as usize],
                pixels[(source_y * u32::from(MAP_WIDTH) + source_x) as usize]
            );
        }
    }
    drop(normalized);
    uqm_rust::graphics::state::flush_ex(true);
    println!("real Earth rescale: every indexed elevation pixel matches nearest-neighbor mapping and both frame owners released");
}

use uqm_rust::planets::generate::SystemGenerator;

unsafe fn binary_palettes() {
    use uqm_rust::strings::sresins;
    for planet in &uqm_rust::planets::tables::PLANET_ARRAY {
        for resource in [planet.cmap_instance, planet.xlat_tab_instance] {
            let entries = sresins::load_binary_entries(resource)
                .unwrap_or_else(|| panic!("packaged binary palette {resource:?} is missing"));
            assert!(!entries.is_empty());
            for index in [0, 1, 2, 3, u16::MAX] {
                let owned = sresins::load_binary_entry(resource, index)
                    .unwrap_or_else(|| panic!("packaged binary palette {resource:?} is missing"));
                assert_eq!(
                    owned,
                    entries[usize::from(index) % entries.len()],
                    "binary palette {resource:?} variant {index}"
                );
            }
        }
    }
    println!("Binary palette ownership: all 59 planet resource pairs, temperature entries and wrapped indices select whole entries");
}
