//! Actual Rust-exported Sol slots, checked against retained original-C fixtures.

use super::{Harness, SysGenRNG};
use crate::planets_common::sol::{self, Layout, Ops, Scan};
use uqm_rust::c_bindings::global_state::{NODE_INFO, PLANET_DESC, SYSTEM_INFO};
use uqm_rust::planets::generate::special::Body;
use uqm_rust::planets::generate::vtable::GenerateFunctions;
use uqm_rust::state::game_state_keys::{get_game_state, set_game_state};

unsafe extern "C" {
    pub(super) static generateSolFunctions: GenerateFunctions;
}

impl Ops for Harness {
    fn planets(&mut self, case: Layout, descs: &mut [PLANET_DESC; 16]) -> (bool, u8, u32) {
        self.set_star(case.size, case.color);
        self.state.PlanetDesc = *descs;
        self.state.SunDesc[0].NumPlanets = 0x7f;
        self.seed(case.seed);
        let mut expected = *self.state;
        let result =
            unsafe { generateSolFunctions.generate_planets.unwrap()(&raw mut *self.state) };
        *descs = self.state.PlanetDesc;
        expected.PlanetDesc = *descs;
        expected.SunDesc[0].NumPlanets = self.state.SunDesc[0].NumPlanets;
        assert_eq!(
            format!("{:?}", self.state),
            format!("{expected:?}"),
            "planet hook wrote unrelated state"
        );
        (
            result,
            self.state.SunDesc[0].NumPlanets,
            self.rng_position(),
        )
    }

    fn moons(
        &mut self,
        case: Layout,
        planet: u8,
        count: u8,
        descs: &mut [PLANET_DESC; 4],
    ) -> (bool, u32) {
        self.set_star(case.size, case.color);
        self.state.MoonDesc = *descs;
        self.state.PlanetDesc = unsafe { std::mem::zeroed() };
        self.state.PlanetDesc[usize::from(planet)].NumPlanets = count;
        self.seed(case.seed);
        // Derive both C arguments and the parent link from one raw root. No
        // covering Rust mutable reference is created while C uses the links.
        let system = &raw mut *self.state;
        let (mut expected, result) = unsafe {
            let parent = &raw mut (*system).PlanetDesc[usize::from(planet)];
            (*parent).pPrevDesc = &raw mut (*system).SunDesc[0];
            let expected = *system;
            (
                expected,
                generateSolFunctions.generate_moons.unwrap()(system, parent),
            )
        };
        *descs = self.state.MoonDesc;
        expected.MoonDesc = *descs;
        assert_eq!(
            format!("{:?}", self.state),
            format!("{expected:?}"),
            "moon hook wrote unrelated state"
        );
        (result, self.rng_position())
    }

    fn scan(&mut self, case: Scan, info: &SYSTEM_INFO, node: &mut NODE_INFO) -> (u16, u32) {
        self.set_star(0, 0);
        set_game_state("FOUND_PLUTO_SPATHI", u32::from(case.pluto));
        set_game_state("MOONBASE_DESTROYED", u32::from(case.destroyed));
        self.state.SysInfo = *info;
        self.state.PlanetDesc = unsafe { std::mem::zeroed() };
        self.state.MoonDesc = unsafe { std::mem::zeroed() };
        let system = &raw mut *self.state;
        let world = unsafe {
            let sun = &raw mut (*system).SunDesc[0];
            match case.body {
                Body::Planet(p) => {
                    (*system).PlanetDesc[usize::from(p)].pPrevDesc = sun;
                    &raw const (*system).PlanetDesc[usize::from(p)]
                }
                Body::Moon { planet, moon } => {
                    (*system).PlanetDesc[usize::from(planet)].pPrevDesc = sun;
                    (*system).MoonDesc[usize::from(moon)].pPrevDesc =
                        &raw mut (*system).PlanetDesc[usize::from(planet)];
                    &raw const (*system).MoonDesc[usize::from(moon)]
                }
            }
        };
        let hook = unsafe {
            if case.life {
                generateSolFunctions.generate_life
            } else {
                generateSolFunctions.generate_energy
            }
        }
        .unwrap();
        let before = format!("{:?}", self.state);
        self.seed(case.seed);
        let count = unsafe { hook(system, world, case.which, node) };
        let rng = self.rng_position();
        self.seed(case.seed);
        let count_only = unsafe { hook(system, world, case.which, std::ptr::null_mut()) };
        assert_eq!(
            (count_only, self.rng_position()),
            (count, rng),
            "null output: {case:?}"
        );
        assert_eq!(
            format!("{:?}", self.state),
            before,
            "scan changed solar system: {case:?}"
        );
        assert_eq!(get_game_state("FOUND_PLUTO_SPATHI"), u32::from(case.pluto));
        assert_eq!(
            get_game_state("MOONBASE_DESTROYED"),
            u32::from(case.destroyed)
        );
        // The count-only query above must leave the same post-call seed.
        assert_eq!(unsafe { (*SysGenRNG).seed() }, rng);
        (count, rng)
    }
}

pub fn check(table: &[uqm_rust::c_bindings::global_state::PlanetFrame]) {
    let mut c = Harness::new();
    let layout = sol::render_layout(&mut c);
    let scans = sol::render_scans(&mut c);
    crate::check_fixture("sol_layout.txt", &layout);
    crate::check_fixture("sol_scans.txt", &scans);
    assert_eq!(layout, sol::render_layout(&mut sol::RustSol));
    assert_eq!(scans, sol::render_scans(&mut sol::RustSol));
    super::sol_orbital::check(&mut c, table);
    println!(
        "Sol: {} full-buffer layouts and {} guarded scans: Rust ABI == immutable C fixture == Rust core",
        layout.lines().count(),
        scans.lines().count()
    );
}
