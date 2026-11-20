//! Invoke the production Rust Sol orbital slot, using the existing DEBUG
//! universe traversal exit in `LoadPlanet`. No surface/display interposition.

use super::{Harness, ScreenWidth};
use crate::planets_common::sol_orbital::{self, Case, Ops};
use uqm_engine::world::planet::{PlanetFrame, SystemInfo};

use uqm_engine::kernel::diagnostics::DiagnosticGuard;
use uqm_engine::state::game_state_keys::get_game_state;
use uqm_modes::planets::generate::special::Body;

impl Ops for Harness {
    fn orbital(&mut self, case: Case) -> (bool, SystemInfo, u32) {
        self.set_star(case.layout.size, case.layout.color);
        self.state.sys.info = sol_orbital::initial_info(case.layout.seed);
        self.state.sys.planets = Default::default();
        self.state.sys.moons = Default::default();
        let (desc, parent) = sol_orbital::input(case);
        match case.body {
            Body::Planet(planet) => self.state.sys.planets[usize::from(planet)] = desc,
            Body::Moon { planet, moon } => {
                self.state.sys.planets[usize::from(planet)] = parent.unwrap();
                self.state.sys.moons[usize::from(moon)] = desc;
            }
        }
        let mut expected = crate::generation_data(&self.state);
        self.seed(0xabcdef01);
        let result = crate::invoke_generation(&mut self.state, |view, host| {
            super::sol::generateSolFunctions.generate_orbital(view, case.body, host)
        });
        let info = self.state.sys.info;
        expected.3 = info;
        assert_eq!(
            format!("{:?}", crate::generation_data(&self.state)),
            format!("{expected:?}"),
            "orbital wrote unrelated state: {case:?}"
        );
        assert_eq!(
            crate::battle_context::with(|cx| get_game_state(
                cx.world,
                uqm_engine::StoryKey::FOUND_PLUTO_SPATHI
            )),
            1
        );
        assert_eq!(
            crate::battle_context::with(|cx| get_game_state(
                cx.world,
                uqm_engine::StoryKey::MOONBASE_DESTROYED
            )),
            1
        );
        (result, info, self.rng_position())
    }
}

struct DebugTraversal {
    _quiet: DiagnosticGuard,
    pluto_spathi: u32,
    moonbase_destroyed: u32,
}

impl DebugTraversal {
    fn enter() -> Self {
        let saved = Self {
            _quiet: crate::battle_context::with(|cx| cx.kernel.diagnostics.suppress_interaction()),
            pluto_spathi: crate::battle_context::with(|cx| {
                cx.world
                    .story_state(uqm_engine::StoryKey::FOUND_PLUTO_SPATHI)
            }),
            moonbase_destroyed: crate::battle_context::with(|cx| {
                cx.world
                    .story_state(uqm_engine::StoryKey::MOONBASE_DESTROYED)
            }),
        };
        crate::battle_context::with(|cx| {
            cx.world
                .set_story_state(uqm_engine::StoryKey::FOUND_PLUTO_SPATHI, 1)
        });
        crate::battle_context::with(|cx| {
            cx.world
                .set_story_state(uqm_engine::StoryKey::MOONBASE_DESTROYED, 1)
        });
        saved
    }
}

impl Drop for DebugTraversal {
    fn drop(&mut self) {
        crate::battle_context::with(|cx| {
            cx.world
                .set_story_state(uqm_engine::StoryKey::FOUND_PLUTO_SPATHI, self.pluto_spathi)
        });
        crate::battle_context::with(|cx| {
            cx.world.set_story_state(
                uqm_engine::StoryKey::MOONBASE_DESTROYED,
                self.moonbase_destroyed,
            )
        });
    }
}

pub fn check(c: &mut Harness, table: &[PlanetFrame]) {
    let _traversal = DebugTraversal::enter();
    assert_eq!(ScreenWidth, 320);
    let actual = sol_orbital::render(c, table);
    assert_eq!(actual.lines().count(), 3_168);
    crate::check_fixture("sol_orbital.txt", &actual);
    assert_eq!(
        actual,
        sol_orbital::render(&mut sol_orbital::Rust(table), table)
    );
    println!("Sol orbital: 3168 complete SYSTEM_INFO/RNG records: Rust ABI == immutable C fixture == Rust core; LoadPlanet exits via existing DEBUG traversal seam");
}

use uqm_modes::planets::generate::SystemGenerator;
