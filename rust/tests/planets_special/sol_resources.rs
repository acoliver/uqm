//! Real packaged resources and cleanup, without a window or gameplay loop.

use super::Harness;
use uqm_engine::graphics::surface::CapturedSurface;
use uqm_engine::state::game_state_keys::set_game_state;
use uqm_engine::test_support::native_probe;
use uqm_modes::planets::c_api;
use uqm_modes::planets::generate::consts::{MAP_HEIGHT, MAP_WIDTH};
use uqm_modes::planets::generate::sol::Generator as GENERATE_SOL_FUNCTIONS;

pub fn check(h: &mut Harness) {
    let content = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../sc2/content")
        .canonicalize()
        .unwrap_or_else(|error| panic!("canonical content directory: {error}"));
    c_api::log_init(100);
    let mount = uqm_engine::test_support::native_session::ContentMount::new(&content);
    let Some(dir) = uqm_engine::io::content_dir() else {
        panic!("checked-in content directory must open");
    };
    uqm_engine::resource::init();
    uqm_engine::resource::load_index(
        &dir,
        c"uqm.rmp",
        None,
        &mut Default::default(),
        &mut Default::default(),
    );
    names(h);
    binary_palettes();
    scenes(h);
    uqm_engine::graphics::state::flush_ex(true);
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
        .unwrap_or_else(|error| panic!("normalize Earth's definition: {error}"));
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
    h.state.sys.planets[2].data_index = 42 | 128;
    h.state.sys.planets[2].rand_seed = 104729;
    h.state.sys.planets[2].location.x = 100;
    h.state.sys.planets[2].location.y = 0;
    h.state.sys.sun.num_planets = 9;
    h.state.enter_inner(2);
    h.state
        .enter_orbit(uqm_engine::world::generation::Body::Planet(2));
    uqm_modes::planets::generate::sol::orbital::physical_overrides(
        &mut h.state.sys.info,
        uqm_modes::planets::generate::special::Body::Planet(2),
    );
    let animation = crate::battle_context::with(|cx| cx.kernel.planet_animation.clone());
    let mut sphere = uqm_modes::planets::sphere::SphereState::default();
    uqm_modes::planets::plangen::live::generate_planet_surface(
        &mut sphere,
        &animation,
        false,
        uqm_modes::planets::plangen::live::SurfaceInputs {
            rng: &mut h.state.sys.rng,
            planet: &h.state.sys.planets[2],
            info: &h.state.sys.info.planet_info,
            location: uqm_engine::graphics::geometry::Point::new(
                h.state.sys.planets[2].location.x,
                h.state.sys.planets[2].location.y,
            ),
            width: usize::from(MAP_WIDTH),
        },
        earth.into_drawable(),
    );
    assert_eq!(
        h.state.sys.rng.seed(),
        104729,
        "packaged definitions still reseed the supplied system RNG"
    );

    let topography = sphere.topography();
    let topo_frame = topography
        .frame(0)
        .unwrap_or_else(|| panic!("topography has frame 0"));
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
    let planet_system = &mut h.state;
    crate::battle_context::with(|cx| c_api::free_planet(cx, planet_system));
    assert_eq!(sphere.elevation()[0], first_elevation);
    drop(topography);
    drop(sphere);
    uqm_engine::graphics::state::flush_ex(true);
    println!("real Earth definition consumed by actual GeneratePlanetSurface, shield and sphere allocated, then FreePlanet plus owned release cleared surface/sphere/topography allocations");
    assert_eq!(
        native_probe::allocations(),
        baseline,
        "real Earth generation/rescale/cleanup must release all graphics allocations"
    );
    uqm_engine::resource::uninit();
    drop(mount);
}

fn frame_rgb(frame: uqm_engine::graphics::drawable::FrameId) -> Vec<[u8; 3]> {
    let (bounds, _) = native_probe::frame_bounds(frame);
    native_probe::read_pixels(frame, i32::from(bounds.width), i32::from(bounds.height))
        .unwrap_or_else(|| panic!("Earth frame"))
        .into_iter()
        .map(|p| [p.r, p.g, p.b])
        .collect()
}

fn names(h: &mut Harness) {
    let strings_owner = uqm_engine::strings::owned_table::LoadedTextTable::load(c"text.starcon")
        .unwrap_or_else(|| panic!("text.starcon loads"));
    crate::battle_context::with(|cx| {
        assert!(cx.kernel.micro_font.is_none());
        cx.kernel.install_game_text(strings_owner);
    });
    for (index, expected) in [(0, "Mercury"), (2, "Earth"), (8, "Pluto")] {
        let parent = &mut h.state.sys.planets[index];
        parent.data_index = index as u8 + 128;
        assert!(crate::invoke_generation(&mut h.state, |view, host| {
            GENERATE_SOL_FUNCTIONS.generate_name(
                view,
                uqm_engine::world::generation::Body::Planet(index as u8),
                host,
            )
        }));
        let name = crate::battle_context::with(|cx| cx.world.data.flagship.planet_name);
        let name = std::ffi::CStr::from_bytes_until_nul(&name)
            .unwrap_or_else(|error| panic!("planet name: {error}"));
        assert_eq!(
            name.to_str()
                .unwrap_or_else(|error| panic!("planet name is UTF-8: {error}")),
            expected
        );
        assert_eq!(
            crate::battle_context::with(|cx| {
                uqm_engine::state::game_state_keys::get_game_state(
                    cx.world,
                    uqm_engine::StoryKey::BATTLE_PLANET,
                )
            }),
            index as u32 + 128
        );
        h.state.sys.moons[1].data_index = 27;
        assert!(crate::invoke_generation(&mut h.state, |view, host| {
            GENERATE_SOL_FUNCTIONS.generate_name(
                view,
                uqm_engine::world::generation::Body::Moon {
                    planet: index as u8,
                    moon: 1,
                },
                host,
            )
        }));
        let name = crate::battle_context::with(|cx| cx.world.data.flagship.planet_name);
        assert_eq!(
            std::ffi::CStr::from_bytes_until_nul(&name)
                .unwrap_or_else(|error| panic!("flagship planet name is terminated: {error}"))
                .to_str()
                .unwrap_or_else(|error| panic!("flagship planet name is UTF-8: {error}")),
            expected
        );
        assert_eq!(
            crate::battle_context::with(|cx| {
                uqm_engine::state::game_state_keys::get_game_state(
                    cx.world,
                    uqm_engine::StoryKey::BATTLE_PLANET,
                )
            }),
            index as u32 + 128
        );
    }
    crate::battle_context::with(|cx| uqm_engine::Kernel::release_encounter_text(cx.kernel));
    uqm_engine::graphics::state::flush_ex(true);
    println!("Sol name ABI: real game strings, parent names and parent battle frames for planets and moons");
}

fn scenes(h: &mut Harness) {
    let _quiet = crate::battle_context::with(|cx| cx.kernel.diagnostics.suppress_interaction());
    for pluto in [true, false] {
        h.state.sys.info = Default::default();
        h.state.sys.planets = Default::default();
        h.state.sys.moons = Default::default();
        h.set_star(0, 0);
        let planet = &mut h.state.sys.planets[if pluto { 8 } else { 2 }];
        planet.radius = 100;
        planet.data_index = if pluto { 22 } else { 42 | 128 };
        let world = if pluto {
            planet
        } else {
            let moon = &mut h.state.sys.moons[1];
            moon.radius = 35;
            moon.data_index = 27;
            moon
        };
        world.rand_seed = 104729;
        crate::battle_context::with(|cx| {
            set_game_state(cx.world, uqm_engine::StoryKey::FOUND_PLUTO_SPATHI, 0)
        });
        crate::battle_context::with(|cx| {
            set_game_state(cx.world, uqm_engine::StoryKey::MOONBASE_DESTROYED, 0)
        });
        let typography = crate::battle_context::with(|cx| {
            let game = std::cell::RefCell::new(cx.reborrow());
            let mut observer = uqm_engine::observe::Unobserved;
            let io = std::cell::RefCell::new(uqm_engine::observe::Io {
                observer: &mut observer,
            });
            let mut pickup = uqm_modes::planet_side::special_effects::PickupContext::NoLander;
            let mut site = uqm_modes::planets::solarsys::site::Site::new(
                &game,
                &io,
                h.state.star(),
                h.state.sys.star_index(),
                h.state.in_orbit(),
                &mut pickup,
                None,
            );
            assert!(site.invoke(&mut h.state.sys, |_, view, host| {
                GENERATE_SOL_FUNCTIONS.generate_orbital(
                    view,
                    if pluto {
                        uqm_engine::world::generation::Body::Planet(8)
                    } else {
                        uqm_engine::world::generation::Body::Moon { planet: 2, moon: 1 }
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
            uqm_engine::graphics::geometry::Point { x: 0, y: 0 },
            uqm_engine::graphics::geometry::TextAlign::Left,
            1,
        );
        assert!(
            measured.visible,
            "lander A glyph missing, leading={}",
            measured.leading
        );
        assert!(effect.frame_rect(0).extent.width > 0);
        let info = &h.state.sys.info.planet_info;
        assert!(h.state.sys.report.current().is_some());
        assert!(h.state.sys.frames[1].is_some());
        assert_eq!(info.planet_density, if pluto { 33 } else { 60 });
        let planet_system = &mut h.state;
        crate::battle_context::with(|cx| c_api::free_planet(cx, planet_system));
        uqm_engine::graphics::state::flush_ex(true);
        assert!(h.state.sys.report.current().is_none());
        drop(typography);
        assert!(
            font.measure(
                c"A",
                uqm_engine::graphics::geometry::Point { x: 0, y: 0 },
                uqm_engine::graphics::geometry::TextAlign::Left,
                1
            )
            .visible
        );
        assert!(effect.frame_rect(0).extent.width > 0);
        drop((font, effect));
        uqm_engine::graphics::state::flush_ex(true);
        assert!(h.state.sys.frames.iter().all(|frame| frame.is_none()));
        println!("Sol real scene {}: font + effect + mask + report loaded through actual orbital ABI, then explicit font leases survive FreePlanet and release after their final user", if pluto { "Pluto" } else { "moonbase" });
    }
}

fn indexed_rescale() {
    let original = CapturedSurface::load(c"graphics.earthmask");
    let mut pixels = vec![0; usize::from(MAP_WIDTH) * usize::from(MAP_HEIGHT)];
    uqm_engine::graphics::drawable::ContextFrame::retain(original.first_frame().at(1))
        .read_indexes(&mut pixels, i32::from(MAP_WIDTH), i32::from(MAP_HEIGHT));
    let width = MAP_WIDTH / 2;
    let height = MAP_HEIGHT / 2;
    let normalized = original
        .into_definition(width as i16, height as i16)
        .unwrap_or_else(|error| panic!("normalize the rescaled definition: {error}"));
    let mut actual = vec![0; usize::from(width) * usize::from(height)];
    uqm_engine::graphics::drawable::ContextFrame::retain(normalized.first_frame().at(1))
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
    uqm_engine::graphics::state::flush_ex(true);
    println!("real Earth rescale: every indexed elevation pixel matches nearest-neighbor mapping and both frame owners released");
}

use uqm_modes::planets::generate::SystemGenerator;

fn binary_palettes() {
    use uqm_engine::strings::sresins;
    for planet in &uqm_modes::planets::tables::PLANET_ARRAY {
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
