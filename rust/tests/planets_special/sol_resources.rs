//! Real packaged resources and cleanup, without a window or gameplay loop.

use super::Harness;
use uqm_rust::io::uio_bridge;
use uqm_rust::mainloop::dir_prep::contentDir;
use uqm_rust::planets::c_api::{self, CapturedSurface};
use uqm_rust::planets::consts::{MAP_HEIGHT, MAP_WIDTH};
use uqm_rust::planets::generate::sol_exports::GENERATE_SOL_FUNCTIONS;
use uqm_rust::resource::ffi_bridge;
use uqm_rust::state::game_state_keys::set_game_state;

pub fn check(h: &mut Harness) {
    let content = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../sc2/content");
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
        let world = &raw mut (*system).PlanetDesc[2];
        (*world).pPrevDesc = &raw mut (*system).SunDesc[0];
        (*world).data_index = 42 | 128;
        (*world).rand_seed = 104729;
        (*world).location.x = 100;
        (*world).location.y = 0;
        (*system).pOrbitalDesc = world;
        (*system).InOrbit = 1;
        uqm_rust::planets::generate::sol::orbital::physical_overrides(
            &mut (*system).SysInfo,
            uqm_rust::planets::generate::special::Body::Planet(2),
        );
        c_api::GeneratePlanetSurface(world, earth.into_frame());
        assert!(!(*system).TopoFrame.is_null());
        assert!(!(*system).Orbit.lpTopoData.is_null());
        assert!(!(*system).Orbit.TopoColors.is_null());
        assert!(!(*system).Orbit.SphereFrame.is_null());
        let after_tint = frame_rgb((*system).TopoFrame);
        assert_eq!(after_tint.len(), before_tint.len());
        assert_eq!(
            super::shield_additive::frame_bytes((*system).TopoFrame),
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
        c_api::FreePlanet();
        c_api::TFB_FlushGraphicsEx(1);
        assert!((*system).TopoFrame.is_null());
        assert!((*system).Orbit.lpTopoData.is_null());
        assert!((*system).Orbit.TopoColors.is_null());
        assert!((*system).Orbit.SphereFrame.is_null());
        println!("real Earth definition consumed by actual GeneratePlanetSurface, shield and sphere allocated, then actual FreePlanet cleared surface/sphere/topography owners");
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
    let strings =
        c_api::CaptureStringTable(c_api::LoadStringTableInstance(c"text.starcon".as_ptr()));
    assert!(!strings.is_null());
    let saved = std::ptr::addr_of!(c_api::GAME_STRINGS).read();
    std::ptr::addr_of_mut!(c_api::GAME_STRINGS).write(strings);
    let system = &raw mut *h.state;
    for (index, expected) in [(0, "Mercury"), (2, "Earth"), (8, "Pluto")] {
        let parent = &raw mut (*system).PlanetDesc[index];
        (*parent).pPrevDesc = &raw mut (*system).SunDesc[0];
        (*parent).data_index = index as u8 + 128;
        assert!(GENERATE_SOL_FUNCTIONS.generate_name.unwrap()(
            system, parent
        ));
        let name =
            std::ptr::addr_of!(c_api::GLOB_DATA.sis_state.planet_name).cast::<std::ffi::c_char>();
        assert_eq!(std::ffi::CStr::from_ptr(name).to_str().unwrap(), expected);
        assert_eq!(
            uqm_rust::state::game_state_keys::get_game_state("BATTLE_PLANET"),
            index as u32 + 128
        );
        let moon = &raw mut (*system).MoonDesc[1];
        (*moon).pPrevDesc = parent;
        (*moon).data_index = 27;
        assert!(GENERATE_SOL_FUNCTIONS.generate_name.unwrap()(system, moon));
        assert_eq!(std::ffi::CStr::from_ptr(name).to_str().unwrap(), expected);
        assert_eq!(
            uqm_rust::state::game_state_keys::get_game_state("BATTLE_PLANET"),
            index as u32 + 128
        );
    }
    std::ptr::addr_of_mut!(c_api::GAME_STRINGS).write(saved);
    // Reuse the production discovery-table cleanup boundary for this captured
    // table after restoring the global that borrowed it.
    (*system).SysInfo.PlanetInfo.DiscoveryString = strings.cast();
    c_api::FreePlanet();
    c_api::TFB_FlushGraphicsEx(1);
    println!("Sol name ABI: real game strings, parent names and parent battle frames for planets and moons");
}

unsafe fn scenes(h: &mut Harness) {
    let saved_debug = c_api::DISABLE_INTERACTIVITY;
    c_api::DISABLE_INTERACTIVITY = 1;
    for pluto in [true, false] {
        h.state.SysInfo = std::mem::zeroed();
        h.state.PlanetDesc = std::mem::zeroed();
        h.state.MoonDesc = std::mem::zeroed();
        h.set_star(0, 0);
        let system = &raw mut *h.state;
        let planet = &raw mut (*system).PlanetDesc[if pluto { 8 } else { 2 }];
        (*planet).pPrevDesc = &raw mut (*system).SunDesc[0];
        (*planet).radius = 100;
        (*planet).data_index = if pluto { 22 } else { 42 | 128 };
        let world = if pluto {
            planet
        } else {
            let moon = &raw mut (*system).MoonDesc[1];
            (*moon).pPrevDesc = planet;
            (*moon).radius = 35;
            (*moon).data_index = 27;
            moon
        };
        (*world).rand_seed = 104729;
        set_game_state("FOUND_PLUTO_SPATHI", 0);
        set_game_state("MOONBASE_DESTROYED", 0);
        assert!(GENERATE_SOL_FUNCTIONS.generate_orbital.unwrap()(
            system, world
        ));
        let info = &(*system).SysInfo.PlanetInfo;
        assert!(!info.LanderFont.is_null());
        assert!(!info.LanderFontEff.is_null());
        assert!(!info.DiscoveryString.is_null());
        assert!(!(*system).PlanetSideFrame[1].is_null());
        assert_eq!(info.PlanetDensity, if pluto { 33 } else { 60 });
        c_api::FreePlanet();
        c_api::TFB_FlushGraphicsEx(1);
        let info = &(*system).SysInfo.PlanetInfo;
        assert!(
            info.LanderFont.is_null()
                && info.LanderFontEff.is_null()
                && info.DiscoveryString.is_null()
        );
        assert!((*system)
            .PlanetSideFrame
            .iter()
            .all(|frame| frame.is_null()));
        println!("Sol real scene {}: font + effect + mask + report loaded through actual orbital ABI, then FreePlanet cleared every owned slot", if pluto { "Pluto" } else { "moonbase" });
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
