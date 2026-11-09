//! Calls the production Rust Mycon ABI slots against retained C fixtures.

use super::{generateMyconFunctions, Harness};
use crate::planets_common::mycon::{self, MyconOps, RustMycon, ScanCase};
use uqm_rust::engine::world::planet::{NodeInfo, PlanetDesc, PlanetFrame};

use uqm_rust::state::game_state_keys::set_game_state;

impl MyconOps for Harness {
    fn layout(
        &mut self,
        star: u8,
        case: (u8, u8, u32),
        planets: &mut [PlanetDesc],
    ) -> (bool, u8, u32) {
        self.star_index = star;
        self.set_star(case.0, case.1);
        self.state.sys.planets.copy_from_slice(planets);
        self.state.sys.sun.num_planets = 0;
        self.seed(case.2);

        let result = unsafe {
            crate::invoke_generation(&mut self.state, |system, host| {
                generateMyconFunctions.generate_planets(system, host)
            })
        };
        planets.copy_from_slice(&self.state.sys.planets);
        (result, self.state.sys.sun.num_planets, self.rng_position())
    }

    fn scan(&mut self, case: ScanCase, node: &mut NodeInfo) -> (u16, u32) {
        self.star_index = case.star;
        self.set_star(0, 0);
        uqm_rust::engine::World::current().write(|world| {
            set_game_state(
                world,
                uqm_rust::engine::StoryKey::SUN_DEVICE,
                u32::from(case.taken),
            )
        });
        self.state.sys.planets = unsafe { std::mem::zeroed() };
        self.state.sys.moons = unsafe { std::mem::zeroed() };
        self.state.sys.info = mycon::scan_info(case);
        self.state.sys.info.planet_info.discovery_string = std::ptr::null_mut();
        self.state.sys.report =
            uqm_rust::engine::world::generation::ReportSlot::new(case.strings.then(|| {
                uqm_rust::engine::strings::owned_table::LoadedTextTable::new(vec![
                    c"egg case report".to_owned(),
                ])
                .entry(0)
                .unwrap()
            }));

        let generate =
            |view: &mut uqm_rust::engine::world::generation::SolarSys<'_>,
             info: &mut NodeInfo,
             host: &mut dyn uqm_rust::planets::generate::SiteHost| {
                if case.life {
                    generateMyconFunctions.generate_life(view, case.body, case.which, info, host)
                } else {
                    generateMyconFunctions.generate_energy(view, case.body, case.which, info, host)
                }
            };
        self.seed(case.seed);
        let before = format!("{:?}", crate::generation_data(&self.state));
        let count = unsafe {
            crate::invoke_generation(&mut self.state, |view, host| generate(view, node, host))
        };
        let after_rng = self.rng_position();
        self.seed(case.seed);
        let count_only = unsafe {
            crate::invoke_generation(&mut self.state, |view, host| {
                generate(view, &mut crate::planets_common::fresh_node(), host)
            })
        };
        assert_eq!(count_only, count, "NULL output count: {case:?}");
        assert_eq!(self.rng_position(), after_rng, "NULL output RNG: {case:?}");
        assert_eq!(
            format!("{:?}", crate::generation_data(&self.state)),
            before,
            "scan mutated its solar-system input: {case:?}"
        );
        assert_eq!(
            uqm_rust::engine::World::current().read(|world| {
                uqm_rust::state::game_state_keys::get_game_state(
                    world,
                    uqm_rust::engine::StoryKey::SUN_DEVICE,
                )
            }),
            u32::from(case.taken)
        );
        (count, self.rng_position())
    }
}

pub fn check(table: &[PlanetFrame]) {
    let from_export = mycon::render(&mut Harness::new());
    crate::check_fixture("mycon.txt", &from_export);
    assert_eq!(from_export, mycon::render(&mut RustMycon(table)));
    println!(
        "Mycon: {} exact Rust export == retained C fixture == Rust cases",
        from_export.lines().count()
    );
}

use uqm_rust::planets::generate::SystemGenerator;
