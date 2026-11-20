//! Headless calls through Sol's actual exported table and drawable ABI.

use super::Harness;
use uqm_rust::battle::gendef::SOL_DEFINED;
use uqm_rust::engine::graphics::surface::CapturedSurface;
use uqm_rust::engine::test_support::native_probe;
use uqm_rust::planets::c_api;
use uqm_rust::planets::generate::consts::{MAP_HEIGHT, MAP_WIDTH};
use uqm_rust::planets::generate::sol::Generator as GENERATE_SOL_FUNCTIONS;
use uqm_rust::planets::generate::{generator, SystemGenerator};

pub fn check() {
    let table = generator(SOL_DEFINED);
    let mut h = Harness::new();
    h.set_star(0, 0);
    check_initialized_identity(&mut h);
    h.seed(104729);
    let system = &raw mut *h.state;
    unsafe {
        assert!(!crate::invoke_generation(&mut *system, |view, host| table
            .pickup_energy(
                view,
                uqm_rust::planets::generate::special::Body::Planet(0),
                u16::MAX,
                host
            )));
        assert!(crate::invoke_generation(&mut *system, |view, host| table
            .pickup_minerals(
                view,
                uqm_rust::planets::generate::special::Body::Planet(0),
                u16::MAX,
                host
            )));
        assert!(crate::invoke_generation(&mut *system, |view, host| table
            .pickup_life(
                view,
                uqm_rust::planets::generate::special::Body::Planet(0),
                u16::MAX,
                host
            )));
        // No resource/report infrastructure is needed for the no-scene pickup.
        assert_eq!(h.rng_position(), 104729);
    }
    check_bomb_queue(&mut h);
    super::sol_npcs::check(&mut h);
    check_drawable_ownership(&mut h);
    println!("Sol traits: initialized dispatch identity, inherited defaults, callable pickups and bomb queue reset");
}

fn check_initialized_identity(h: &mut Harness) {
    h.star_index = SOL_DEFINED;
    h.set_star(0, 0);
    h.star[0].index = uqm_rust::battle::gendef::SHOFIXTI_DEFINED;
    crate::battle_context::with(|cx| cx.world.set_catalog(std::borrow::Cow::Owned(*h.star)));
    // SAFETY: the serialized harness retains its catalog, RNG and solar records;
    // the callback releases all generated references before returning.
    let generated = {
        crate::battle_context::with(|cx| {
            let mut observer = uqm_rust::engine::observe::Unobserved;
            let mut io = uqm_rust::engine::observe::Io {
                observer: &mut observer,
            };
            uqm_rust::front_end::uqmdebug::Generation::invoke(
                &mut uqm_rust::planets::solarsys::site::dispatch::DebugGeneration,
                cx,
                &mut io,
                &mut h.state,
                |generator, system, host| {
                    assert_eq!(system.star_index(), SOL_DEFINED);
                    generator.generate_planets(system, host)
                },
            )
        })
    };
    assert!(generated);
    assert_eq!(
        h.state.sys.sun.num_planets, 9,
        "the initialized Sol generator must survive a later star-index change"
    );
    h.star_index = 0;
    h.set_star(0, 0);
}

fn check_bomb_queue(h: &mut Harness) {
    // This uses the real live queue adapter, not a recording FleetGroups host.
    let old_bomb = crate::battle_context::with(|cx| {
        cx.world
            .story_state(uqm_rust::engine::StoryKey::CHMMR_BOMB_STATE)
    });
    let old_probe = crate::battle_context::with(|cx| {
        cx.world
            .story_state(uqm_rust::engine::StoryKey::PROBE_MESSAGE_DELIVERED)
    });
    crate::battle_context::with(|cx| {
        cx.world
            .set_story_state(uqm_rust::engine::StoryKey::CHMMR_BOMB_STATE, 3)
    });
    crate::battle_context::with(|cx| {
        cx.world
            .set_story_state(uqm_rust::engine::StoryKey::PROBE_MESSAGE_DELIVERED, 1)
    });
    unsafe {
        let saved = crate::battle_context::with(|cx| {
            let saved = (
                std::mem::take(&mut cx.world.queues),
                cx.world.data.game.battle_group_ref,
            );
            cx.world.data.game.battle_group_ref = 0x12345678;
            saved
        });
        assert!(crate::invoke_generation(&mut h.state, |system, host| {
            GENERATE_SOL_FUNCTIONS.reinit_npcs(system, host)
        }));
        crate::battle_context::with(|cx| {
            assert_eq!(cx.world.data.game.battle_group_ref, 0);
            assert!(cx.world.ip_groups_ref().head().is_none());
            assert!(cx.world.npc_built_ships_ref().head().is_none());
            cx.world.queues = saved.0;
            cx.world.data.game.battle_group_ref = saved.1;
        });
    }
    crate::battle_context::with(|cx| {
        cx.world
            .set_story_state(uqm_rust::engine::StoryKey::CHMMR_BOMB_STATE, old_bomb)
    });
    crate::battle_context::with(|cx| {
        cx.world.set_story_state(
            uqm_rust::engine::StoryKey::PROBE_MESSAGE_DELIVERED,
            old_probe,
        )
    });
}

fn allocations() -> (usize, usize) {
    uqm_rust::graphics::state::flush_ex(true);
    native_probe::allocations()
}

fn check_drawable_ownership(h: &mut Harness) {
    let _graphics = native_probe::DrawQueue::init();
    let baseline = allocations();
    let map_width = MAP_WIDTH as i16;
    let map_height = MAP_HEIGHT as i16;
    for (width, height) in [(map_width, map_height), (map_width / 2, map_height / 2)] {
        let definition = CapturedSurface::pixmap(width, height, 2);
        let frame = definition.first_frame();
        assert!(!frame.is_null());
        let created = allocations();
        assert!(created.0 > baseline.0 && created.1 > baseline.1);
        let (bounds, pixels) = native_probe::frame_bounds(frame);
        assert_eq!((bounds.width, bounds.height), (width, height));
        assert!(pixels);
        let retained = if (width, height) == (map_width, map_height) {
            definition
        } else {
            let scaled = definition.rescaled(i32::from(map_width), i32::from(map_height));
            assert!(!scaled.first_frame().is_null());
            assert_ne!(scaled.first_frame(), frame);
            let both = allocations();
            assert!(both.0 > created.0 && both.1 > created.1);
            drop(scaled);
            assert_eq!(allocations(), created);
            let normalized = definition.into_definition(map_width, map_height).unwrap();
            assert_ne!(normalized.first_frame(), frame);
            let elevation = normalized.first_frame().at(1);
            assert_eq!(elevation.index(), 1);
            assert_eq!(elevation.drawable(), normalized.first_frame().drawable());
            let (bounds, pixels) = native_probe::frame_bounds(elevation);
            assert_eq!((bounds.width, bounds.height), (map_width, map_height));
            assert!(pixels);
            assert_eq!(allocations(), created);
            normalized
        };
        let live = retained.first_frame();
        let (bounds, _) = native_probe::frame_bounds(live);
        assert_eq!((bounds.width, bounds.height), (map_width, map_height));
        let owned = retained.into_drawable().unwrap();
        assert_eq!(owned.lease().frame(0), Some(live));
        drop(owned);
        assert_eq!(
            allocations(),
            baseline,
            "source/rescale owners leaked graphics allocations"
        );
        println!("Sol drawable ownership: {width}x{height} two-frame allocation -> retained {map_width}x{map_height} -> owned release; allocations {created:?} -> {baseline:?}");
    }
    // DEBUG's early-return LoadPlanet path must release an unconsumed non-null
    // definition as well, rather than proving only a missing-resource branch.
    let quiet = crate::battle_context::with(|cx| cx.kernel.diagnostics.suppress_interaction());
    {
        crate::battle_context::with(|cx| {
            let mut observer = uqm_rust::engine::observe::Unobserved;
            let game = std::cell::RefCell::new(cx.reborrow());
            let io = std::cell::RefCell::new(uqm_rust::engine::observe::Io {
                observer: &mut observer,
            });
            let definition = CapturedSurface::pixmap(map_width / 2, map_height / 2, 2);
            let mut pickup = uqm_rust::planet_side::special_effects::PickupContext::NoLander;
            let mut site = uqm_rust::planets::solarsys::site::Site::new(
                &game,
                &io,
                h.state.star(),
                h.state.sys.star_index(),
                h.state.in_orbit(),
                &mut pickup,
                None,
            );
            site.invoke(&mut h.state.sys, |_, system, _| {
                assert!(c_api::load_planet(
                    &game,
                    system,
                    uqm_rust::engine::world::generation::Body::Planet(0),
                    false,
                    definition.into_drawable(),
                    (Default::default(), None),
                    |_, _| panic!("noninteractive orbit load must not populate scan nodes"),
                )
                .is_none());
            });
        });
    }
    drop(quiet);
    assert_eq!(allocations(), baseline);
    super::sol_resources::check(h);
}
