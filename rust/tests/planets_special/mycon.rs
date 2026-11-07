//! Calls the production Rust Mycon ABI slots against retained C fixtures.

use super::{generateMyconFunctions, Harness};
use crate::planets_common::mycon::{self, MyconOps, RustMycon, ScanCase};
use uqm_rust::engine::world::planet::{NodeInfo, PlanetDesc, PlanetFrame};

use uqm_rust::planets::generate::special::Body;
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
        self.state.PlanetDesc.copy_from_slice(planets);
        self.state.SunDesc[0].num_planets = 0;
        self.seed(case.2);

        let result = unsafe {
            crate::invoke_generation(&raw mut *self.state, |system, host| {
                generateMyconFunctions.generate_planets(system, host)
            })
        };
        planets.copy_from_slice(&self.state.PlanetDesc);
        (
            result,
            self.state.SunDesc[0].num_planets,
            self.rng_position(),
        )
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
        self.state.PlanetDesc = unsafe { std::mem::zeroed() };
        self.state.MoonDesc = unsafe { std::mem::zeroed() };
        self.state.SysInfo = mycon::scan_info(case);
        let sun: *mut PlanetDesc = &mut self.state.SunDesc[0];
        let _world: *const PlanetDesc = match case.body {
            Body::Planet(p) => {
                self.state.PlanetDesc[usize::from(p)].prev_desc = sun;
                &self.state.PlanetDesc[usize::from(p)]
            }
            Body::Moon { planet, moon } => {
                self.state.PlanetDesc[usize::from(planet)].prev_desc = sun;
                let parent: *mut PlanetDesc = &mut self.state.PlanetDesc[usize::from(planet)];
                self.state.MoonDesc[usize::from(moon)].prev_desc = parent;
                &self.state.MoonDesc[usize::from(moon)]
            }
        };
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
        let before = format!("{:?}", self.state);
        let count = unsafe {
            crate::invoke_generation(&raw mut *self.state, |view, host| {
                generate(view, node, host)
            })
        };
        let after_rng = self.rng_position();
        self.seed(case.seed);
        let count_only = unsafe {
            crate::invoke_generation(&raw mut *self.state, |view, host| {
                generate(view, &mut crate::planets_common::fresh_node(), host)
            })
        };
        assert_eq!(count_only, count, "NULL output count: {case:?}");
        assert_eq!(self.rng_position(), after_rng, "NULL output RNG: {case:?}");
        assert_eq!(
            format!("{:?}", self.state),
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
