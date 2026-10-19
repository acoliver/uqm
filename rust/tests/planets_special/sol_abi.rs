//! Headless calls through Sol's actual exported table and drawable ABI.

use super::Harness;
use uqm_rust::battle::gendef::{getGenerateFunctions, SOL_DEFINED};
use uqm_rust::comm::locdata::CQueue;
use uqm_rust::planets::c_api::GLOB_DATA as GlobData;
use uqm_rust::planets::c_api::{self, CapturedSurface};
use uqm_rust::planets::consts::{MAP_HEIGHT, MAP_WIDTH};
use uqm_rust::planets::generate::sol_exports::GENERATE_SOL_FUNCTIONS;
use uqm_rust::planets::generate::vtable::DEFAULT_HOOKS;
use uqm_rust::state::game_state_keys::{get_game_state, set_game_state};

pub fn check() {
    let table = unsafe { &super::sol::generateSolFunctions };
    assert!(std::ptr::eq(table, &GENERATE_SOL_FUNCTIONS));
    assert_eq!(
        getGenerateFunctions(SOL_DEFINED),
        std::ptr::from_ref(table).cast()
    );
    assert!(table.init_npcs.is_some());
    assert!(table.reinit_npcs.is_some());
    assert!(table.generate_planets.is_some());
    assert!(table.generate_moons.is_some());
    assert!(table.generate_name.is_some());
    assert!(table.generate_orbital.is_some());
    assert!(table.generate_energy.is_some());
    assert!(table.generate_life.is_some());
    assert!(table.pickup_energy.is_some());
    assert_eq!(
        table.uninit_npcs.unwrap() as usize,
        DEFAULT_HOOKS.uninit_npcs.unwrap() as usize
    );
    assert_eq!(
        table.generate_minerals.unwrap() as usize,
        DEFAULT_HOOKS.generate_minerals.unwrap() as usize
    );
    assert_eq!(
        table.pickup_minerals.unwrap() as usize,
        DEFAULT_HOOKS.pickup_minerals.unwrap() as usize
    );
    assert_eq!(
        table.pickup_life.unwrap() as usize,
        DEFAULT_HOOKS.pickup_life.unwrap() as usize
    );

    let mut h = Harness::new();
    h.set_star(0, 0);
    h.seed(104729);
    let system = &raw mut *h.state;
    unsafe {
        let world = &raw mut (*system).PlanetDesc[0];
        (*world).pPrevDesc = &raw mut (*system).SunDesc[0];
        assert!(!table.pickup_energy.unwrap()(system, world, u16::MAX));
        assert!(table.pickup_minerals.unwrap()(system, world, u16::MAX));
        assert!(table.pickup_life.unwrap()(system, world, u16::MAX));
        // No resource/report infrastructure is needed for the no-scene pickup.
        assert_eq!(h.rng_position(), 104729);
    }
    check_bomb_queue(&mut h);
    super::sol_npcs::check(&mut h);
    check_drawable_ownership(&mut h);
    println!("Sol ABI: exact exported table/dispatch, 13 populated slots, 4 inherited Rust defaults, callable pickups and bomb queue reset");
}

fn check_bomb_queue(h: &mut Harness) {
    // This uses the real live queue adapter, not a recording FleetGroups host.
    let old_bomb = get_game_state("CHMMR_BOMB_STATE");
    let old_probe = get_game_state("PROBE_MESSAGE_DELIVERED");
    set_game_state("CHMMR_BOMB_STATE", 3);
    set_game_state("PROBE_MESSAGE_DELIVERED", 1);
    unsafe {
        let ip = &raw mut GlobData.game_state.ip_group_q;
        let npc = &raw mut GlobData.game_state.npc_built_ship_q;
        let reference = &raw mut GlobData.game_state.battle_group_ref;
        let saved = (ip.read(), npc.read(), reference.read());
        ip.write(CQueue::default());
        npc.write(CQueue::default());
        reference.write(0x12345678);
        assert!(GENERATE_SOL_FUNCTIONS.reinit_npcs.unwrap()(
            &raw mut *h.state
        ));
        assert_eq!(reference.read(), 0);
        assert!(ip.read().head.is_null());
        assert!(npc.read().head.is_null());
        ip.write(saved.0);
        npc.write(saved.1);
        reference.write(saved.2);
    }
    set_game_state("CHMMR_BOMB_STATE", old_bomb);
    set_game_state("PROBE_MESSAGE_DELIVERED", old_probe);
}

fn allocations() -> i32 {
    unsafe { sdl2::sys::SDL_GetNumAllocations() }
}

fn check_drawable_ownership(h: &mut Harness) {
    let format =
        sdl2::surface::Surface::new(1, 1, sdl2::pixels::PixelFormatEnum::RGBA8888).unwrap();
    let old_format = unsafe { std::ptr::addr_of!(c_api::FORMAT_CONV_SURF).read() };
    let old_screen = unsafe { std::ptr::addr_of!(c_api::SDL_SCREEN).read() };
    unsafe {
        std::ptr::addr_of_mut!(c_api::FORMAT_CONV_SURF).write(format.raw());
        std::ptr::addr_of_mut!(c_api::SDL_SCREEN).write(format.raw());
        c_api::Init_DrawCommandQueue();
    }
    // Warm SDL's format cache before measuring resource-owned allocations.
    unsafe {
        drop(CapturedSurface::pixmap(2, 2, 1));
        c_api::TFB_FlushGraphicsEx(1);
    }
    let baseline = allocations();
    assert!(baseline >= 0);
    let map_width = MAP_WIDTH as i16;
    let map_height = MAP_HEIGHT as i16;
    for (width, height) in [(map_width, map_height), (map_width / 2, map_height / 2)] {
        let definition = unsafe { CapturedSurface::pixmap(width, height, 2) };
        let frame = definition.as_frame();
        assert!(!frame.is_null());
        let created = allocations();
        assert!(created > baseline);
        unsafe {
            assert_eq!(
                ((*frame).Bounds.width, (*frame).Bounds.height),
                (width, height)
            );
            assert!(!(*frame).image.is_null());
        }
        let retained = if (width, height) == (map_width, map_height) {
            definition
        } else {
            let scaled =
                unsafe { definition.rescaled(i32::from(map_width), i32::from(map_height)) };
            assert!(!scaled.as_frame().is_null());
            assert_ne!(scaled.as_frame(), frame);
            let both = allocations();
            assert!(both > created);
            drop(scaled);
            unsafe { c_api::TFB_FlushGraphicsEx(1) };
            assert_eq!(allocations(), created);
            let normalized = unsafe { definition.into_definition(map_width, map_height) }.unwrap();
            assert_ne!(normalized.as_frame(), frame);
            unsafe {
                c_api::TFB_FlushGraphicsEx(1);
                let elevation = normalized.as_frame().add(1);
                assert_eq!((*elevation).Index, 1);
                assert_eq!((*elevation).parent, (*normalized.as_frame()).parent);
                assert_eq!(
                    ((*elevation).Bounds.width, (*elevation).Bounds.height),
                    (map_width, map_height)
                );
                assert!(!(*elevation).image.is_null());
            }
            assert_eq!(allocations(), created);
            normalized
        };
        let live = retained.as_frame();
        unsafe {
            assert_eq!(
                ((*live).Bounds.width, (*live).Bounds.height),
                (map_width, map_height)
            );
        }
        h.state.TopoFrame = retained.into_frame();
        // The move cleared the owner slot. Only actual FreePlanet now releases
        // the retained source or rescaled surface; queued image frees drain below.
        unsafe {
            assert_eq!(h.state.TopoFrame, live);
            c_api::FreePlanet();
            assert!(h.state.TopoFrame.is_null());
            c_api::TFB_FlushGraphicsEx(1);
        }
        assert_eq!(
            allocations(),
            baseline,
            "source/rescale/FreePlanet leaked SDL allocations"
        );
        println!("Sol drawable ownership: {width}x{height} two-frame allocation -> retained {map_width}x{map_height} -> actual FreePlanet; SDL allocations {created} -> {baseline}");
    }
    // DEBUG's early-return LoadPlanet path must release an unconsumed non-null
    // definition as well, rather than proving only a missing-resource branch.
    let saved_debug = unsafe { c_api::DISABLE_INTERACTIVITY };
    unsafe {
        c_api::DISABLE_INTERACTIVITY = 1;
        c_api::load_planet_surface(Some(CapturedSurface::pixmap(
            map_width / 2,
            map_height / 2,
            2,
        )));
        c_api::TFB_FlushGraphicsEx(1);
        c_api::DISABLE_INTERACTIVITY = saved_debug;
    }
    assert_eq!(allocations(), baseline);
    super::sol_resources::check(h);
    unsafe {
        c_api::Uninit_DrawCommandQueue();
        std::ptr::addr_of_mut!(c_api::FORMAT_CONV_SURF).write(old_format);
        std::ptr::addr_of_mut!(c_api::SDL_SCREEN).write(old_screen);
    }
}
