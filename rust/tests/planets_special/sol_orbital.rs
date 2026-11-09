//! Invoke the production Rust Sol orbital slot, using the existing DEBUG
//! universe traversal exit in `LoadPlanet`. No surface/display interposition.

use super::{Harness, ScreenWidth};
use crate::planets_common::sol_orbital::{self, Case, Ops};
use uqm_rust::engine::world::planet::{PlanetFrame, SystemInfo};

use uqm_rust::planets::c_api::DISABLE_INTERACTIVITY;
use uqm_rust::planets::generate::special::Body;
use uqm_rust::state::game_state_keys::get_game_state;

impl Ops for Harness {
    fn orbital(&mut self, case: Case) -> (bool, SystemInfo, u32) {
        self.set_star(case.layout.size, case.layout.color);
        self.state.sys.info = sol_orbital::initial_info(case.layout.seed);
        self.state.sys.planets = unsafe { std::mem::zeroed() };
        self.state.sys.moons = unsafe { std::mem::zeroed() };
        let (desc, parent) = sol_orbital::input(case);
        match case.body {
            Body::Planet(planet) => self.state.sys.planets[usize::from(planet)] = desc,
            Body::Moon { planet, moon } => {
                self.state.sys.planets[usize::from(planet)] = parent.unwrap();
                self.state.sys.moons[usize::from(moon)] = desc;
            }
        }
        let system = &raw mut *self.state;
        let mut expected = crate::generation_data(&self.state);
        self.seed(0xabcdef01);
        let result = unsafe {
            crate::invoke_generation(&mut *system, |view, host| {
                super::sol::generateSolFunctions.generate_orbital(view, case.body, host)
            })
        };
        let info = self.state.sys.info;
        expected.3 = info;
        assert_eq!(
            format!("{:?}", crate::generation_data(&self.state)),
            format!("{expected:?}"),
            "orbital wrote unrelated state: {case:?}"
        );
        assert_eq!(
            uqm_rust::engine::World::current().read(|world| get_game_state(
                world,
                uqm_rust::engine::StoryKey::FOUND_PLUTO_SPATHI
            )),
            1
        );
        assert_eq!(
            uqm_rust::engine::World::current().read(|world| get_game_state(
                world,
                uqm_rust::engine::StoryKey::MOONBASE_DESTROYED
            )),
            1
        );
        (result, info, self.rng_position())
    }
}

struct DebugTraversal(u32, u32, u32);

impl DebugTraversal {
    fn enter() -> Self {
        let saved = Self(
            unsafe { DISABLE_INTERACTIVITY },
            uqm_rust::engine::World::current()
                .read(|world| (world).story_state(uqm_rust::engine::StoryKey::FOUND_PLUTO_SPATHI)),
            uqm_rust::engine::World::current()
                .read(|world| (world).story_state(uqm_rust::engine::StoryKey::MOONBASE_DESTROYED)),
        );
        unsafe {
            DISABLE_INTERACTIVITY = 1;
        }
        uqm_rust::engine::World::current().write(|world| {
            (world).set_story_state(uqm_rust::engine::StoryKey::FOUND_PLUTO_SPATHI, 1)
        });
        uqm_rust::engine::World::current().write(|world| {
            (world).set_story_state(uqm_rust::engine::StoryKey::MOONBASE_DESTROYED, 1)
        });
        saved
    }
}

impl Drop for DebugTraversal {
    fn drop(&mut self) {
        unsafe {
            DISABLE_INTERACTIVITY = self.0;
        }
        uqm_rust::engine::World::current().write(|world| {
            (world).set_story_state(uqm_rust::engine::StoryKey::FOUND_PLUTO_SPATHI, self.1)
        });
        uqm_rust::engine::World::current().write(|world| {
            (world).set_story_state(uqm_rust::engine::StoryKey::MOONBASE_DESTROYED, self.2)
        });
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

use uqm_rust::planets::generate::SystemGenerator;
