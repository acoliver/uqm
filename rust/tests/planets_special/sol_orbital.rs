//! Invoke the production Rust Sol orbital slot, using the existing DEBUG
//! universe traversal exit in `LoadPlanet`. No surface/display interposition.

use super::{Harness, ScreenWidth};
use crate::planets_common::sol_orbital::{self, Case, Ops};
use uqm_rust::c_bindings::global_state::{PlanetFrame, SYSTEM_INFO};
use uqm_rust::planets::c_api::DISABLE_INTERACTIVITY;
use uqm_rust::planets::generate::special::Body;
use uqm_rust::state::game_state_keys::{get_game_state, set_game_state};

impl Ops for Harness {
    fn orbital(&mut self, case: Case) -> (bool, SYSTEM_INFO, u32) {
        self.set_star(case.layout.size, case.layout.color);
        self.state.SysInfo = sol_orbital::initial_info(case.layout.seed);
        self.state.PlanetDesc = unsafe { std::mem::zeroed() };
        self.state.MoonDesc = unsafe { std::mem::zeroed() };
        let (desc, parent) = sol_orbital::input(case);
        let system = &raw mut *self.state;
        // All interior links and call arguments derive from this raw root.
        // The only resulting table pointer is into the static linked table.
        let world = unsafe {
            let sun = &raw mut (*system).SunDesc[0];
            match case.body {
                Body::Planet(p) => {
                    let world = &raw mut (*system).PlanetDesc[usize::from(p)];
                    world.write(desc);
                    (*world).pPrevDesc = sun;
                    world
                }
                Body::Moon { planet, moon } => {
                    let p = &raw mut (*system).PlanetDesc[usize::from(planet)];
                    p.write(parent.unwrap());
                    (*p).pPrevDesc = sun;
                    let world = &raw mut (*system).MoonDesc[usize::from(moon)];
                    world.write(desc);
                    (*world).pPrevDesc = p;
                    world
                }
            }
        };
        let mut expected = *self.state;
        self.seed(0xabcdef01);
        let result =
            unsafe { super::sol::generateSolFunctions.generate_orbital.unwrap()(system, world) };
        let info = self.state.SysInfo;
        expected.SysInfo = info;
        assert_eq!(
            format!("{:?}", self.state),
            format!("{expected:?}"),
            "orbital wrote unrelated state: {case:?}"
        );
        assert_eq!(get_game_state("FOUND_PLUTO_SPATHI"), 1);
        assert_eq!(get_game_state("MOONBASE_DESTROYED"), 1);
        (result, info, self.rng_position())
    }
}

struct DebugTraversal(u32, u32, u32);

impl DebugTraversal {
    fn enter() -> Self {
        let saved = Self(
            unsafe { DISABLE_INTERACTIVITY },
            get_game_state("FOUND_PLUTO_SPATHI"),
            get_game_state("MOONBASE_DESTROYED"),
        );
        unsafe {
            DISABLE_INTERACTIVITY = 1;
        }
        set_game_state("FOUND_PLUTO_SPATHI", 1);
        set_game_state("MOONBASE_DESTROYED", 1);
        saved
    }
}

impl Drop for DebugTraversal {
    fn drop(&mut self) {
        unsafe {
            DISABLE_INTERACTIVITY = self.0;
        }
        set_game_state("FOUND_PLUTO_SPATHI", self.1);
        set_game_state("MOONBASE_DESTROYED", self.2);
    }
}

pub fn check(c: &mut Harness, table: &[PlanetFrame]) {
    let _traversal = DebugTraversal::enter();
    assert_eq!(unsafe { ScreenWidth }, 320);
    let actual = sol_orbital::render(c, table);
    assert_eq!(actual.lines().count(), 3_168);
    crate::check_fixture("sol_orbital.txt", &actual);
    assert_eq!(
        actual,
        sol_orbital::render(&mut sol_orbital::Rust(table), table)
    );
    println!("Sol orbital: 3168 complete SYSTEM_INFO/RNG records: Rust ABI == immutable C fixture == Rust core; LoadPlanet exits via existing DEBUG traversal seam");
}
