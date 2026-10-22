//! Calls the production Rust Mycon ABI slots against retained C fixtures.

use super::{generateMyconFunctions, Harness};
use crate::planets_common::mycon::{self, MyconOps, RustMycon, ScanCase};
use uqm_rust::c_bindings::global_state::{PlanetFrame, NODE_INFO, PLANET_DESC};
use uqm_rust::planets::generate::special::Body;
use uqm_rust::state::game_state_keys::set_game_state;

impl MyconOps for Harness {
    fn layout(
        &mut self,
        star: u8,
        case: (u8, u8, u32),
        planets: &mut [PLANET_DESC],
    ) -> (bool, u8, u32) {
        self.star_index = star;
        self.set_star(case.0, case.1);
        self.state.PlanetDesc.copy_from_slice(planets);
        self.state.SunDesc[0].NumPlanets = 0;
        self.seed(case.2);
        let hook = unsafe { generateMyconFunctions.generate_planets }.unwrap();
        let result = unsafe { hook(&mut *self.state) };
        planets.copy_from_slice(&self.state.PlanetDesc);
        (
            result,
            self.state.SunDesc[0].NumPlanets,
            self.rng_position(),
        )
    }

    fn scan(&mut self, case: ScanCase, node: &mut NODE_INFO) -> (u16, u32) {
        self.star_index = case.star;
        self.set_star(0, 0);
        set_game_state("SUN_DEVICE", u32::from(case.taken));
        self.state.PlanetDesc = unsafe { std::mem::zeroed() };
        self.state.MoonDesc = unsafe { std::mem::zeroed() };
        self.state.SysInfo = mycon::scan_info(case);
        let sun: *mut PLANET_DESC = &mut self.state.SunDesc[0];
        let world: *const PLANET_DESC = match case.body {
            Body::Planet(p) => {
                self.state.PlanetDesc[usize::from(p)].pPrevDesc = sun;
                &self.state.PlanetDesc[usize::from(p)]
            }
            Body::Moon { planet, moon } => {
                self.state.PlanetDesc[usize::from(planet)].pPrevDesc = sun;
                let parent: *mut PLANET_DESC = &mut self.state.PlanetDesc[usize::from(planet)];
                self.state.MoonDesc[usize::from(moon)].pPrevDesc = parent;
                &self.state.MoonDesc[usize::from(moon)]
            }
        };
        let hook = unsafe {
            if case.life {
                generateMyconFunctions.generate_life
            } else {
                generateMyconFunctions.generate_energy
            }
        }
        .unwrap();
        self.seed(case.seed);
        let before = format!("{:?}", self.state);
        let count = unsafe { hook(&*self.state, world, case.which, node) };
        let after_rng = self.rng_position();
        self.seed(case.seed);
        let count_only = unsafe { hook(&*self.state, world, case.which, std::ptr::null_mut()) };
        assert_eq!(count_only, count, "NULL output count: {case:?}");
        assert_eq!(self.rng_position(), after_rng, "NULL output RNG: {case:?}");
        assert_eq!(
            format!("{:?}", self.state),
            before,
            "scan mutated its solar-system input: {case:?}"
        );
        assert_eq!(
            uqm_rust::state::game_state_keys::get_game_state("SUN_DEVICE"),
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
