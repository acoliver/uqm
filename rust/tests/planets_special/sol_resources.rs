//! Real packaged resources and cleanup, without a window or gameplay loop.

use super::Harness;
use uqm_rust::io::uio_bridge;
use uqm_rust::mainloop::dir_prep::contentDir;
use uqm_rust::planets::c_api::{self, CapturedSurface};
use uqm_rust::planets::generate::consts::{MAP_HEIGHT, MAP_WIDTH};
use uqm_rust::planets::generate::sol::Generator as GENERATE_SOL_FUNCTIONS;
use uqm_rust::resource::ffi_bridge;
use uqm_rust::state::game_state_keys::set_game_state;

pub fn check(h: &mut Harness) {
    let content = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../sc2/content")
        .canonicalize()
        .unwrap();
    let path = std::ffi::CString::new(content.to_str().unwrap()).unwrap();
    unsafe {
        c_api::log_init(100);
        super::shield_additive::check();
        uio_bridge::uio_init();
        let repository = uio_bridge::uio_openRepository(0);
        let mount = uio_bridge::uio_mountDir(
            repository,
            c"/".as_ptr(),
            1,
            std::ptr::null_mut(),
            std::ptr::null(),
            path.as_ptr(),
            std::ptr::null_mut(),
            6,
            std::ptr::null_mut(),
        );
        assert!(!mount.is_null());
        let dir = uio_bridge::uio_openDir(repository, c"/".as_ptr(), 0);
        assert!(!dir.is_null(), "checked-in content directory must open");
        let saved_dir = std::ptr::addr_of!(contentDir).read();
        std::ptr::addr_of_mut!(contentDir).write(dir.cast());
        assert!(!ffi_bridge::InitResourceSystem().is_null());
        ffi_bridge::LoadResourceIndex(dir.cast(), c"uqm.rmp".as_ptr(), std::ptr::null());
        names(h);
        binary_palettes();
        scenes(h);
        let baseline = sdl2::sys::SDL_GetNumAllocations();
        indexed_rescale();
        let earth = CapturedSurface::load(c"graphics.earthmask");
        assert!(
            !earth.as_frame().is_null(),
            "real Earth definition must load"
        );
        let raw = earth.as_frame();
        let before_tint = frame_rgb(raw);
        let expected_tint = super::shield_additive::earth_expected(raw);
        let earth = earth
            .into_definition(MAP_WIDTH as i16, MAP_HEIGHT as i16)
            .unwrap();
        assert_eq!(
            (
                (*earth.as_frame()).Bounds.width,
                (*earth.as_frame()).Bounds.height
            ),
            (MAP_WIDTH as i16, MAP_HEIGHT as i16)
        );
        println!(
            "real Earth resource: source={raw:p}, normalized={:p}",
            earth.as_frame()
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
        let animation =
            uqm_rust::engine::Kernel::current().read(|kernel| kernel.planet_animation.clone());
        let mut sphere = uqm_rust::planets::sphere::SphereState::default();
        c_api::generate_planet_surface(
            &mut sphere,
            &animation,
            false,
            c_api::SurfaceInputs {
                rng: &mut (*system).sys.rng,
                planet: &*world,
                info: &(*system).sys.info.planet_info,
                location: uqm_rust::graphics::drawable::Point::new(
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
        assert_eq!(
            super::shield_additive::frame_bytes(topo_frame),
            expected_tint
        );
        for (index, (before, after)) in before_tint.iter().zip(&after_tint).enumerate() {
            assert_eq!(
                *after,
                [255, before[1], before[2]],
                "Earth shield tint pixel {index}"
            );
        }
        println!("real Earth shield tint: all {} RGB pixels equal [255, source.g, source.b]; complete buffer equals isolated Rust additive core", after_tint.len());
        uqm_rust::app::bridge::enter(|cx| c_api::free_planet(cx, &mut *system));
        assert_eq!(sphere.elevation()[0], first_elevation);
        drop(topography);
        drop(sphere);
        c_api::TFB_FlushGraphicsEx(1);
        println!("real Earth definition consumed by actual GeneratePlanetSurface, shield and sphere allocated, then FreePlanet plus owned release cleared surface/sphere/topography allocations");
        assert_eq!(
            sdl2::sys::SDL_GetNumAllocations(),
            baseline,
            "real Earth generation/rescale/cleanup must release all SDL surface allocations"
        );
        ffi_bridge::UninitResourceSystem();
        std::ptr::addr_of_mut!(contentDir).write(saved_dir);
        assert_eq!(uio_bridge::uio_closeDir(dir), 0);
        uio_bridge::uio_closeRepository(repository);
        uio_bridge::uio_unInit();
    }
}

unsafe fn frame_rgb(frame: *mut uqm_rust::c_bindings::global_state::FRAME_DESC) -> Vec<[u8; 3]> {
    let surface = (*frame).image.cast::<*mut sdl2::sys::SDL_Surface>().read();
    let info = uqm_rust::graphics::sdl_capture::query_surface_info(surface);
    let format = &*(*surface).format;
    println!("Earth canvas: {info:?}, palette={:p}", format.palette);
    assert_eq!(format.BytesPerPixel, 4);
    assert_eq!(sdl2::sys::SDL_LockSurface(surface), 0);
    let mut result = Vec::new();
    for y in 0..(*surface).h {
        for x in 0..(*surface).w {
            let pixel = (*surface)
                .pixels
                .cast::<u8>()
                .add((y * (*surface).pitch + x * 4) as usize)
                .cast::<u32>()
                .read_unaligned();
            let (mut r, mut g, mut b) = (0, 0, 0);
            sdl2::sys::SDL_GetRGB(pixel, (*surface).format, &mut r, &mut g, &mut b);
            result.push([r, g, b]);
        }
    }
    sdl2::sys::SDL_UnlockSurface(surface);
    result
}

unsafe fn names(h: &mut Harness) {
    // SAFETY: the serialized harness retains its captured strings and initialized system through each name callback.
    unsafe {
        let strings_owner =
            uqm_rust::engine::strings::owned_table::LoadedTextTable::load(c"text.starcon").unwrap();
        let strings = strings_owner.as_raw();
        let saved = uqm_rust::engine::Kernel::current()
            .write(|kernel| std::mem::replace(&mut kernel.game_strings, strings.cast()));
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
            let name =
                uqm_rust::engine::WorldData::current().read(|world| world.flagship.planet_name);
            let name = name.as_ptr().cast::<std::ffi::c_char>();
            assert_eq!(std::ffi::CStr::from_ptr(name).to_str().unwrap(), expected);
            assert_eq!(
                uqm_rust::engine::World::current().read(|world| {
                    uqm_rust::state::game_state_keys::get_game_state(
                        world,
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
            let name =
                uqm_rust::engine::WorldData::current().read(|world| world.flagship.planet_name);
            assert_eq!(
                std::ffi::CStr::from_ptr(name.as_ptr().cast())
                    .to_str()
                    .unwrap(),
                expected
            );
            assert_eq!(
                uqm_rust::engine::World::current().read(|world| {
                    uqm_rust::state::game_state_keys::get_game_state(
                        world,
                        uqm_rust::engine::StoryKey::BATTLE_PLANET,
                    )
                }),
                index as u32 + 128
            );
        }
        uqm_rust::engine::Kernel::current().write(|kernel| kernel.game_strings = saved);
        drop(strings_owner);
        c_api::TFB_FlushGraphicsEx(1);
        println!("Sol name ABI: real game strings, parent names and parent battle frames for planets and moons");
    }
}

unsafe fn scenes(h: &mut Harness) {
    let saved_debug = c_api::DISABLE_INTERACTIVITY;
    c_api::DISABLE_INTERACTIVITY = 1;
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
        uqm_rust::engine::World::current().write(|world| {
            set_game_state(world, uqm_rust::engine::StoryKey::FOUND_PLUTO_SPATHI, 0)
        });
        uqm_rust::engine::World::current().write(|world| {
            set_game_state(world, uqm_rust::engine::StoryKey::MOONBASE_DESTROYED, 0)
        });
        // SAFETY: Harness retains installed owners and live solar/planet
        // descriptors throughout this synchronous orbital callback.
        let typography = uqm_rust::app::bridge::enter(|cx| {
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
            uqm_rust::graphics::native_abi::POINT { x: 0, y: 0 },
            0,
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
        uqm_rust::app::bridge::enter(|cx| c_api::free_planet(cx, &mut *system));
        c_api::TFB_FlushGraphicsEx(1);
        assert!((*system).sys.report.current().is_none());
        drop(typography);
        assert!(
            font.measure(
                c"A",
                uqm_rust::graphics::native_abi::POINT { x: 0, y: 0 },
                0,
                1
            )
            .visible
        );
        assert!(effect.frame_rect(0).extent.width > 0);
        drop((font, effect));
        c_api::TFB_FlushGraphicsEx(1);
        assert!((*system).sys.frames.iter().all(|frame| frame.is_none()));
        println!("Sol real scene {}: font + effect + mask + report loaded through actual orbital ABI, then explicit font leases survive FreePlanet and release after their final user", if pluto { "Pluto" } else { "moonbase" });
    }
    c_api::DISABLE_INTERACTIVITY = saved_debug;
}

unsafe fn indexed_rescale() {
    let original = CapturedSurface::load(c"graphics.earthmask");
    let mut pixels = vec![0; usize::from(MAP_WIDTH) * usize::from(MAP_HEIGHT)];
    assert_ne!(
        c_api::ReadFramePixelIndexes(
            original.as_frame().add(1).cast(),
            pixels.as_mut_ptr(),
            i32::from(MAP_WIDTH),
            i32::from(MAP_HEIGHT)
        ),
        0
    );
    let width = MAP_WIDTH / 2;
    let height = MAP_HEIGHT / 2;
    let normalized = original
        .into_definition(width as i16, height as i16)
        .unwrap();
    let mut actual = vec![0; usize::from(width) * usize::from(height)];
    assert_ne!(
        c_api::ReadFramePixelIndexes(
            normalized.as_frame().add(1).cast(),
            actual.as_mut_ptr(),
            i32::from(width),
            i32::from(height)
        ),
        0
    );
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
    c_api::TFB_FlushGraphicsEx(1);
    println!("real Earth rescale: every indexed elevation pixel matches nearest-neighbor mapping and both frame owners released");
}

use uqm_rust::planets::generate::SystemGenerator;

unsafe fn binary_palettes() {
    use uqm_rust::strings::{sresins, string_table as strings};
    for planet in &uqm_rust::planets::tables::PLANET_ARRAY.0 {
        for resource in [planet.cmap_instance, planet.xlat_tab_instance] {
            let resource = std::ffi::CStr::from_ptr(resource);
            for index in [0, 1, 2, 3, u16::MAX] {
                let table = sresins::LoadStringTableInstance(resource.as_ptr());
                assert!(!table.is_null());
                let entry =
                    strings::SetAbsStringTableIndex(strings::CaptureStringTable(table), index);
                let expected = std::slice::from_raw_parts(
                    strings::GetStringAddress(entry).cast::<u8>(),
                    usize::from(strings::GetStringLengthBin(entry)),
                )
                .to_vec();
                strings::FreeStringTable(table);
                let owned = sresins::load_binary_entry(resource, index)
                    .unwrap_or_else(|| panic!("packaged binary palette {resource:?} is missing"));
                assert_eq!(
                    owned, expected,
                    "binary palette {resource:?} variant {index}"
                );
            }
        }
    }
    println!("Binary palette ownership: all 59 planet resource pairs, temperature entries and wrapped indices match legacy selected bytes after native release");
}
