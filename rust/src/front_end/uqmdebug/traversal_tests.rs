use super::*;
use crate::engine::{world::CStarDesc, StarId};
use crate::planets::generate::{testing::NoEffects, SiteHost, SystemGenerator};
use std::{borrow::Cow, io};

struct GenerationCalls(usize);
impl Generation for GenerationCalls {
    fn invoke<R>(
        &mut self,
        _: &mut Cx<'_>,
        _: &mut Io<'_>,
        solar: &mut SolarSystem,
        run: impl FnOnce(
            &dyn SystemGenerator,
            &mut crate::engine::world::generation::SolarSys<'_>,
            &mut dyn SiteHost,
        ) -> R,
    ) -> R {
        self.0 += 1;
        run(
            crate::planets::generate::generator(solar.sys.star_index()),
            &mut solar.sys,
            &mut NoEffects,
        )
    }
}

fn game() -> crate::engine::Game {
    let mut game = crate::engine::Game::new();
    game.world.story = Some(crate::engine::world::story::StoryState::new());
    let mut catalog = crate::planets::tables::STARMAP_ARRAY;
    catalog[0] = CStarDesc {
        star_pt: crate::graphics::native_abi::POINT { x: 1000, y: 2000 },
        star_type: 0,
        index: 0,
        prefix: 0,
        postfix: 0,
    };
    game.world.set_catalog(Cow::Owned(catalog));
    game
}

#[test]
fn traversal_uses_real_generation_and_restores_supplied_state_on_writer_error() {
    fn fail(
        kernel: &Kernel,
        star: &CStarDesc,
        solar: &SolarSystem,
        calls: &mut Vec<(u32, u8)>,
    ) -> io::Result<()> {
        assert!(!kernel.diagnostics.interactive());
        assert_eq!((star.star_pt.x, star.star_pt.y), (1000, 2000));
        assert!(solar.sys.sun.num_planets > 0);
        calls.push((solar.sys.rng.seed(), solar.sys.sun.num_planets));
        Err(io::ErrorKind::BrokenPipe.into())
    }
    let mut game = game();
    game.world.current_star = None;
    game.world.set_activity(IN_INTERPLANETARY as u16);
    let mut expected = SolarSystem::new(&game.world, StarId(0));
    expected
        .sys
        .rng
        .reseed(crate::planets::solar_coords::star_seed(
            game.world.catalog()[0].star_pt,
        ));
    expected.sys.sun.rand_seed = expected.sys.rng.next_random();
    crate::planets::generate::default::Generator
        .generate_planets(&mut expected.sys, &mut NoEffects);
    let mut battle = crate::engine::battle::Battle::new();
    let mut cx = Cx::new(&mut battle, &mut game.world, &mut game.kernel, || {
        panic!("universe traversal must use per-system RNG")
    });
    let mut observer = crate::engine::observe::Unobserved;
    let mut io = Io {
        observer: &mut observer,
    };
    let mut calls = Vec::new();
    let mut generation = GenerationCalls(0);
    let error = universe_recurse(
        &mut cx,
        &mut io,
        &mut generation,
        &mut UniverseRecurseArg {
            system_func_pre: Some(fail),
            system_func_post: None,
            planet_func_pre: None,
            planet_func_post: None,
            moon_func: None,
            arg: &mut calls,
        },
    )
    .unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::BrokenPipe);
    assert_eq!(
        calls,
        [(expected.sys.rng.seed(), expected.sys.sun.num_planets)]
    );
    assert_eq!(generation.0, 1);
    assert_eq!(cx.world.current_star, None);
    assert_eq!(cx.world.activity(), IN_INTERPLANETARY as u16);
    assert!(cx.kernel.diagnostics.interactive());
    assert!(cx.solar_frame.is_none());
}

#[test]
fn resource_totals_and_indices_use_actual_default_scan_nodes_in_order() {
    let mut game = game();
    game.world.data.game.element_worth = [1, 2, 4, 8, 16, 32, 64, 128];
    let mut solar = SolarSystem::new(&game.world, StarId(0));
    let generator = crate::planets::generate::default::Generator;
    generator.generate_planets(&mut solar.sys, &mut NoEffects);
    solar.sys.planets[0].data_index = 0;
    solar.enter_inner(0);
    let body = Body::Planet(0);
    crate::planets::generate::calc::analyse_body(&mut solar.sys, body);
    crate::planets::generate::special::with_orbit(&mut solar.sys, body, |orbit| orbit.analyse());
    solar.sys.info.planet_info.life_chance = i16::MAX;
    let mut expected_bio = [0u16; 26];
    let mut expected_minerals = [0u16; 8];
    let mut bio_value = 0u16;
    let mut mineral_value = 0u16;
    let mut node = crate::engine::world::planet::NodeInfo::default();
    let bio_count = generator.generate_life(&mut solar.sys, body, u16::MAX, &mut node, &NoEffects);
    for index in 0..bio_count {
        generator.generate_life(&mut solar.sys, body, index, &mut node, &NoEffects);
        expected_bio[usize::from(node.type_)] += 1;
        bio_value = bio_value.wrapping_add(
            (BIO_CREDIT_VALUE as i32 * uqm_debug_creature_bio_value(node.type_ as u8)) as u16,
        );
    }
    let mineral_count =
        generator.generate_minerals(&mut solar.sys, body, u16::MAX, &mut node, &NoEffects);
    assert!(
        bio_count > 0,
        "the fixture must exercise creature valuation"
    );
    assert!(
        mineral_count > 0,
        "the fixture must exercise mineral valuation"
    );
    for index in 0..mineral_count {
        generator.generate_minerals(&mut solar.sys, body, index, &mut node, &NoEffects);
        let category =
            usize::from(crate::planets::tables::ELEMENT_ARRAY[usize::from(node.type_)] & 7);
        expected_minerals[category] = expected_minerals[category].wrapping_add(node.density >> 8);
        mineral_value = mineral_value.wrapping_add(
            ((node.density >> 8) as u32 * u32::from(game.world.data.game.element_worth[category]))
                as u16,
        );
    }
    let mut battle = crate::engine::battle::Battle::new();
    let mut cx = Cx::new(&mut battle, &mut game.world, &mut game.kernel, || {
        panic!("campaign RNG")
    });
    let mut observer = crate::engine::observe::Unobserved;
    let mut io = Io {
        observer: &mut observer,
    };
    let mut generation = GenerationCalls(0);
    assert_eq!(
        calculate_bio_value(&mut cx, &mut io, &mut solar, body, &mut generation),
        bio_value
    );
    assert_eq!(
        generate_bio_index(&mut cx, &mut io, &mut solar, body, &mut generation),
        expected_bio
    );
    assert_eq!(
        calculate_mineral_value(&mut cx, &mut io, &mut solar, body, &mut generation),
        mineral_value
    );
    assert_eq!(
        generate_mineral_index(&mut cx, &mut io, &mut solar, body, &mut generation),
        expected_minerals
    );
    assert_eq!(
        generation.0,
        2 * (usize::from(bio_count) + usize::from(mineral_count) + 2)
    );
}

#[test]
fn empty_traversal_does_not_admit_generation_or_change_supplied_state() {
    let mut game = game();
    game.world.current_star = Some(StarId(7));
    game.world.set_activity(IN_INTERPLANETARY as u16);
    let mut battle = crate::engine::battle::Battle::new();
    let mut cx = Cx::new(&mut battle, &mut game.world, &mut game.kernel, || {
        panic!("an empty traversal cannot consume RNG")
    });
    let mut observer = crate::engine::observe::Unobserved;
    let mut io = Io {
        observer: &mut observer,
    };
    let mut generation = GenerationCalls(0);
    universe_recurse(
        &mut cx,
        &mut io,
        &mut generation,
        &mut UniverseRecurseArg {
            system_func_pre: None,
            system_func_post: None,
            planet_func_pre: None,
            planet_func_post: None,
            moon_func: None,
            arg: &mut (),
        },
    )
    .unwrap();
    assert_eq!(generation.0, 0);
    assert_eq!(cx.world.current_star, Some(StarId(7)));
    assert_eq!(cx.world.activity(), IN_INTERPLANETARY as u16);
    assert!(cx.kernel.diagnostics.interactive());
}

#[test]
fn traversal_visits_each_planet_and_moon_in_order_before_system_post() {
    #[derive(Default)]
    struct OrbitalCalls {
        calls: usize,
        loads: Vec<Body>,
    }
    struct NoReport;
    impl crate::planet_side::report_adapter::DiscoveryReportPort for NoReport {
        fn present(
            &mut self,
            _: crate::engine::world::generation::ReportDisposition,
        ) -> Result<bool, crate::planet_side::runtime::AdapterError> {
            panic!("default traversal must not present a discovery report")
        }
    }
    impl<F: FnOnce()> SiteHost
        for crate::planets::generate::testing::Effects<
            '_,
            NoEffects,
            NoEffects,
            NoReport,
            NoEffects,
            F,
        >
    {
    }
    impl Generation for OrbitalCalls {
        fn invoke<R>(
            &mut self,
            _: &mut Cx<'_>,
            _: &mut Io<'_>,
            solar: &mut SolarSystem,
            run: impl FnOnce(
                &dyn SystemGenerator,
                &mut crate::engine::world::generation::SolarSys<'_>,
                &mut dyn SiteHost,
            ) -> R,
        ) -> R {
            self.calls += 1;
            let body = solar.selected_body();
            let mut no_effects = NoEffects;
            let mut host = crate::planets::generate::testing::Effects {
                story: NoEffects,
                site: &mut no_effects,
                report: NoReport,
                lander: NoEffects,
                load: Some(|| {
                    self.loads
                        .push(body.expect("orbital load requires selected body"))
                }),
            };
            run(
                crate::planets::generate::generator(solar.sys.star_index()),
                &mut solar.sys,
                &mut host,
            )
        }
    }
    #[derive(Default)]
    struct Visits {
        began: bool,
        planets: u8,
        active: Option<u8>,
        next_moon: u8,
        moons: usize,
        orbitals: Vec<Body>,
    }
    fn begin(
        _: &Kernel,
        _: &CStarDesc,
        solar: &SolarSystem,
        visits: &mut Visits,
    ) -> io::Result<()> {
        assert!(solar.sys.sun.num_planets > 0);
        assert!(!visits.began);
        visits.began = true;
        Ok(())
    }
    fn planet_pre(
        cx: &mut Cx<'_>,
        _: &mut Io<'_>,
        solar: &mut SolarSystem,
        body: Body,
        _: &mut OrbitalCalls,
        visits: &mut Visits,
    ) -> io::Result<()> {
        let Body::Planet(planet) = body else {
            panic!("planet callback received a moon")
        };
        assert_eq!(cx.world.current_star, Some(StarId(0)));
        assert!(!cx.kernel.diagnostics.interactive());
        assert_eq!(solar.selected_body(), Some(body));
        visits.orbitals.push(body);
        assert!(visits.began);
        assert_eq!(planet, visits.planets);
        assert!(visits.active.replace(planet).is_none());
        visits.next_moon = 0;
        Ok(())
    }
    fn moon(
        _: &mut Cx<'_>,
        _: &mut Io<'_>,
        solar: &mut SolarSystem,
        body: Body,
        _: &mut OrbitalCalls,
        visits: &mut Visits,
    ) -> io::Result<()> {
        let Body::Moon { planet, moon } = body else {
            panic!("moon callback received a planet")
        };
        assert_eq!(solar.selected_body(), Some(body));
        visits.orbitals.push(body);
        assert_eq!(visits.active, Some(planet));
        assert_eq!(moon, visits.next_moon);
        visits.next_moon += 1;
        visits.moons += 1;
        Ok(())
    }
    fn planet_post(
        _: &mut Cx<'_>,
        _: &mut Io<'_>,
        solar: &mut SolarSystem,
        body: Body,
        _: &mut OrbitalCalls,
        visits: &mut Visits,
    ) -> io::Result<()> {
        let Body::Planet(planet) = body else {
            panic!("planet post received a moon")
        };
        assert_eq!(solar.selected_body(), Some(body));
        visits.orbitals.push(body);
        assert_eq!(visits.active.take(), Some(planet));
        assert_eq!(
            visits.next_moon,
            solar.sys.planets[usize::from(planet)].num_planets
        );
        visits.planets += 1;
        Ok(())
    }
    fn end(_: &Kernel, _: &CStarDesc, solar: &SolarSystem, visits: &mut Visits) -> io::Result<()> {
        assert!(visits.active.is_none());
        assert_eq!(visits.planets, solar.sys.sun.num_planets);
        Err(io::ErrorKind::BrokenPipe.into())
    }
    let mut game = game();
    game.world.current_star = Some(StarId(7));
    game.world.set_activity(IN_INTERPLANETARY as u16);
    let mut battle = crate::engine::battle::Battle::new();
    let mut cx = Cx::new(&mut battle, &mut game.world, &mut game.kernel, || {
        panic!("traversal must not consume campaign RNG")
    });
    let mut observer = crate::engine::observe::Unobserved;
    let mut io = Io {
        observer: &mut observer,
    };
    let mut visits = Visits::default();
    let mut generation = OrbitalCalls::default();
    let error = universe_recurse(
        &mut cx,
        &mut io,
        &mut generation,
        &mut UniverseRecurseArg {
            system_func_pre: Some(begin),
            system_func_post: Some(end),
            planet_func_pre: Some(planet_pre),
            planet_func_post: Some(planet_post),
            moon_func: Some(moon),
            arg: &mut visits,
        },
    )
    .unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::BrokenPipe);
    assert!(
        visits.moons > 0,
        "the real system must exercise moon traversal"
    );
    assert_eq!(
        generation.calls,
        1 + usize::from(visits.planets) * 3 + visits.moons
    );
    assert_eq!(generation.loads, visits.orbitals);
    assert_eq!(cx.world.current_star, Some(StarId(7)));
    assert_eq!(cx.world.activity(), IN_INTERPLANETARY as u16);
    assert!(cx.kernel.diagnostics.interactive());
}
