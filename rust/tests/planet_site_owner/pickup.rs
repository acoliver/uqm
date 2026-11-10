#[path = "../support/campaign_display.rs"]
mod campaign_display;

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
    struct Queue;
    impl Drop for Queue {
        fn drop(&mut self) {
            // SAFETY: this serialized fixture owns the initialized queue.
            unsafe { uqm_rust::graphics::dcqueue_abi::Uninit_DrawCommandQueue() };
        }
    }
    // SAFETY: the serialized fixture owns the queue until its guard drops.
    unsafe { uqm_rust::graphics::dcqueue_abi::Init_DrawCommandQueue() };
    let _queue = Queue;

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
        assert_eq!(provider.node_count(ScanType::Energy), Ok(1));
        assert_eq!(
            provider
                .generate(ScanType::Energy, ScanNodeId::new(0).unwrap())
                .unwrap()
                .kind,
            uqm_rust::planet_side::generation::GeneratedNodeKind::Energy
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

#[test]
#[serial_test::serial]
fn production_site_keeps_its_report_when_typography_is_missing() {
    use uqm_rust::engine::{
        strings::owned_table::LoadedTextTable,
        world::generation::{ReportDisposition, ReportSlot},
    };
    use uqm_rust::planets::generate::host::ReportSite;
    let mut game = campaign();
    let display = campaign_display::CampaignDisplay::install(&mut game).unwrap();
    with_site(&mut game, SOL_DEFINED, |host, system| {
        let table = LoadedTextTable::new(vec![c"Unconsumed report".to_owned()]);
        system.report = ReportSlot::new(table.entry(0));
        drop(table);
        let failed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            ReportSite::report(host, system, ReportDisposition::Once)
        }));
        let error = failed.unwrap_err();
        assert_eq!(
            error.downcast_ref::<&str>().copied(),
            Some("discovery report typography is not loaded")
        );
        assert_eq!(
            system.report.current().unwrap().current_label().0,
            c"Unconsumed report"
        );
    });
    display.finish(game);
}

#[test]
#[serial_test::serial]
fn production_site_presents_and_consumes_an_empty_report_with_supplied_owners() {
    use uqm_rust::engine::{strings::owned_table::LoadedTextTable, world::generation::ReportSlot};
    use uqm_rust::graphics::{
        context_abi::{ContextFrame, ContextLease},
        drawable_abi::LoadedDrawable,
    };
    use uqm_rust::planets::{
        generate::host::ReportSite,
        report::typography::{ReportContext, ReportResources},
    };
    let mut game = campaign();
    let resources = ReportResources::default();
    let display =
        campaign_display::CampaignDisplay::install_with_resources(&mut game, || resources.load())
            .unwrap();
    game.kernel.space_context = game.kernel.off_screen_context;
    let context = ContextLease::require(game.kernel.space_context);
    context.set_frame(ContextFrame::retain(std::ptr::NonNull::new(
        game.kernel.screen,
    )));
    let junk = LoadedDrawable::create(8, 1, 1, 22).unwrap();
    // SAFETY: this serialized fixture temporarily supplies the legacy visit's registered drawable.
    let previous = unsafe {
        let previous = uqm_rust::planets::solarsys::SpaceJunkFrame;
        uqm_rust::planets::solarsys::SpaceJunkFrame = junk.lease().frame(0).unwrap().cast();
        previous
    };
    game.kernel.install_input_runtime(std::rc::Rc::new(
        uqm_rust::mainloop::input_abi::NativeInputRuntime::new(
            None,
            Default::default(),
            game.kernel.configuration.clone(),
        ),
    ));
    {
        let mut battle = Battle::new();
        let owner = RefCell::new(Cx::new(
            &mut battle,
            &mut game.world,
            &mut game.kernel,
            || 0,
        ));
        let mut observer = Unobserved;
        let io = RefCell::new(Io {
            observer: &mut observer,
        });
        let mut pickup = PickupContext::NoLander;
        let mut host = Site::new(
            &owner,
            &io,
            uqm_rust::engine::StarId(0),
            SOL_DEFINED,
            true,
            &mut pickup,
            Some(ReportContext {
                context: context.clone(),
                resources: resources.clone(),
            }),
        );
        let mut system = SolarSys::new(
            &[],
            0,
            SOL_DEFINED,
            uqm_rust::math::RandomContext::with_seed(123),
        );
        let table = LoadedTextTable::new(vec![c"".to_owned()]);
        system.report = ReportSlot::new(table.entry(0));
        drop(table);
        assert!(ReportSite::report(
            &mut host,
            &mut system,
            uqm_rust::engine::world::generation::ReportDisposition::Once
        ));
        assert!(system.report.current().is_none());
        assert_eq!(uqm_rust::graphics::batch_depth::depth(&context), 0);
    }
    context.flush();
    // SAFETY: the serialized fixture restores the visit's publication before releasing its drawable.
    unsafe { uqm_rust::planets::solarsys::SpaceJunkFrame = previous };
    drop((context, resources, junk));
    display.finish(game);
}
