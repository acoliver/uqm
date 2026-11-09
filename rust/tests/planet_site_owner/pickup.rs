use super::*;
use uqm_rust::engine::world::generation::Body;
use uqm_rust::planet_side::{
    generation::{ScanNodeId, ScanType, SurfaceGenerator},
    generation_adapter::CffiSurfaceGenerator,
    special_effects::{PickupContext, SpecialPickupEffects},
};
use uqm_rust::planets::generate::{
    home_contact::LanderTakeoff, sol, vault::VaultSite, wreck::WreckSite,
};

#[test]
#[serial_test::serial]
fn real_site_routes_every_takeoff_trait_only_to_its_supplied_pickup() {
    let mut game = campaign();
    for method in 0..4 {
        for has_lander in [false, true] {
            let mut pickup = if has_lander {
                PickupContext::lander(12)
            } else {
                PickupContext::NoLander
            };
            with_pickup_site(
                &mut game,
                SOL_DEFINED,
                &mut pickup,
                |host, _| match method {
                    0 => LanderTakeoff::lander_takeoff(host),
                    1 => VaultSite::lander_takeoff(host),
                    2 => WreckSite::lander_takeoff(host),
                    3 => sol::Pickup::lander_takeoff(host),
                    _ => unreachable!(),
                },
            );
            assert_eq!(
                pickup.effects(),
                SpecialPickupEffects {
                    crew_killed: 0,
                    takeoff_requested: has_lander
                }
            );
            assert_eq!(
                PickupContext::lander(12).effects(),
                SpecialPickupEffects::default()
            );
        }
    }
}

#[test]
#[serial_test::serial]
fn real_sol_site_preserves_pluto_fatal_survivor_and_luna_acceptance() {
    for crew in [0, 3, 8, 9, 12] {
        let mut game = campaign();
        let mut pickup = PickupContext::lander(crew);
        with_pickup_site(&mut game, SOL_DEFINED, &mut pickup, |host, system| {
            assert!(!sol::pickup_energy(system, Body::Planet(8), 0, host));
        });
        assert_eq!(
            pickup.effects(),
            SpecialPickupEffects {
                crew_killed: crew.min(8),
                takeoff_requested: crew > 8
            }
        );
        assert_eq!(
            game.world.flag(StoryKey::FOUND_PLUTO_SPATHI),
            u32::from(crew > 8)
        );
        assert_eq!(game.world.flag(StoryKey::MOONBASE_DESTROYED), 0);
        let mut luna = PickupContext::lander(crew);
        with_pickup_site(&mut game, SOL_DEFINED, &mut luna, |host, system| {
            assert!(sol::pickup_energy(
                system,
                Body::Moon { planet: 2, moon: 1 },
                0,
                host
            ));
        });
        assert_eq!(
            luna.effects(),
            SpecialPickupEffects {
                crew_killed: 0,
                takeoff_requested: true
            }
        );
        assert_eq!(game.world.flag(StoryKey::MOONBASE_DESTROYED), 1);
        assert_eq!(game.world.flag(StoryKey::MOONBASE_ON_SHIP), 1);
    }
    let mut game = campaign();
    let mut pickup = PickupContext::NoLander;
    with_pickup_site(&mut game, SOL_DEFINED, &mut pickup, |host, system| {
        assert!(!sol::pickup_energy(system, Body::Planet(8), 0, host));
        assert_eq!(host.flag(StoryKey::FOUND_PLUTO_SPATHI), 0);
        assert!(sol::pickup_energy(
            system,
            Body::Moon { planet: 2, moon: 1 },
            0,
            host
        ));
    });
    assert_eq!(pickup.effects(), SpecialPickupEffects::default());
}

#[test]
#[serial_test::serial]
fn production_provider_threads_pickup_effects_and_only_persists_accepted_nodes() {
    let mut game = campaign();
    game.world.set_catalog(std::borrow::Cow::Borrowed(
        &uqm_rust::planets::tables::STARMAP_ARRAY,
    ));
    game.world.current_star = Some(uqm_rust::engine::StarId(0));
    let mut battle = Battle::new();
    let owner = RefCell::new(Cx::new(
        &mut battle,
        &mut game.world,
        &mut game.kernel,
        uqm_rust::math::random,
    ));
    let mut observer = Unobserved;
    let io = RefCell::new(Io {
        observer: &mut observer,
    });
    let mut solar = uqm_rust::planets::solar_system::SolarSystem::new(
        owner.borrow().world,
        uqm_rust::engine::StarId(82),
    );
    solar.sys.sun.num_planets = 9;
    solar.sys.planets[2].num_planets = 2;
    let scan =
        uqm_rust::engine::graphics::context_abi::LoadedContext::create(c"pickup-provider-test");
    for (pluto, crew) in [(true, 3), (true, 12), (false, 12)] {
        let target = if pluto {
            Body::Planet(8)
        } else {
            Body::Moon { planet: 2, moon: 1 }
        };
        solar.enter_inner(target.planet() as u8);
        solar.enter_orbit(target);
        let mut provider = CffiSurfaceGenerator::for_planet_side(
            &owner,
            &io,
            &mut solar,
            uqm_rust::planets::report::typography::ReportContext {
                context: scan.lease(),
                resources: Default::default(),
            },
        );
        let mut pickup = PickupContext::lander(crew);
        assert_eq!(
            provider.pickup(ScanType::Energy, ScanNodeId::new(0).unwrap(), &mut pickup),
            Ok(!pluto)
        );
        assert_eq!(
            pickup.effects(),
            SpecialPickupEffects {
                crew_killed: if pluto { crew.min(8) } else { 0 },
                takeoff_requested: !pluto || crew > 8
            }
        );
        assert_eq!(
            solar.sys.info.planet_info.scan_retrieve_mask[1],
            u32::from(!pluto)
        );
        assert_eq!(
            owner.borrow().world.flag(StoryKey::PLANETARY_CHANGE),
            u32::from(!pluto)
        );
    }
}
