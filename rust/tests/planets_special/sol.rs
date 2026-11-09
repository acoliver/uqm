//! Actual Rust-exported Sol slots, checked against retained original-C fixtures.

use super::Harness;
use crate::planets_common::sol::{self, Layout, Ops, Scan};
use uqm_rust::engine::world::planet::{NodeInfo, PlanetDesc, SystemInfo};
pub(super) use uqm_rust::planets::generate::sol::Generator as generateSolFunctions;
use uqm_rust::state::game_state_keys::get_game_state;

impl Ops for Harness {
    fn planets(&mut self, case: Layout, descs: &mut [PlanetDesc; 16]) -> (bool, u8, u32) {
        self.set_star(case.size, case.color);
        self.state.sys.planets = *descs;
        self.state.sys.sun.num_planets = 0x7f;
        self.seed(case.seed);
        let mut expected = crate::generation_data(&self.state);
        let result = unsafe {
            crate::invoke_generation(&mut self.state, |system, host| {
                generateSolFunctions.generate_planets(system, host)
            })
        };
        *descs = self.state.sys.planets;
        expected.1 = *descs;
        expected.0.num_planets = self.state.sys.sun.num_planets;
        assert_eq!(
            format!("{:?}", crate::generation_data(&self.state)),
            format!("{expected:?}"),
            "planet hook wrote unrelated state"
        );
        (result, self.state.sys.sun.num_planets, self.rng_position())
    }

    fn moons(
        &mut self,
        case: Layout,
        planet: u8,
        count: u8,
        descs: &mut [PlanetDesc; 4],
    ) -> (bool, u32) {
        self.set_star(case.size, case.color);
        self.state.sys.moons = *descs;
        self.state.sys.planets = unsafe { std::mem::zeroed() };
        self.state.sys.planets[usize::from(planet)].num_planets = count;
        self.seed(case.seed);
        let system = &raw mut *self.state;
        let (mut expected, result) = unsafe {
            let expected = crate::generation_data(&*system);
            (
                expected,
                crate::invoke_generation(&mut *system, |system, host| {
                    generateSolFunctions.generate_moons(system, planet, host)
                }),
            )
        };
        *descs = self.state.sys.moons;
        expected.2 = *descs;
        assert_eq!(
            format!("{:?}", crate::generation_data(&self.state)),
            format!("{expected:?}"),
            "moon hook wrote unrelated state"
        );
        (result, self.rng_position())
    }

    fn scan(&mut self, case: Scan, info: &SystemInfo, node: &mut NodeInfo) -> (u16, u32) {
        self.set_star(0, 0);
        uqm_rust::engine::World::current().write(|world| {
            (world).set_story_state(
                uqm_rust::engine::StoryKey::FOUND_PLUTO_SPATHI,
                u32::from(case.pluto),
            )
        });
        uqm_rust::engine::World::current().write(|world| {
            (world).set_story_state(
                uqm_rust::engine::StoryKey::MOONBASE_DESTROYED,
                u32::from(case.destroyed),
            )
        });
        self.state.sys.info = *info;
        self.state.sys.planets = unsafe { std::mem::zeroed() };
        self.state.sys.moons = unsafe { std::mem::zeroed() };
        let system = &raw mut *self.state;
        let generate =
            |view: &mut uqm_rust::engine::world::generation::SolarSys<'_>,
             info: &mut NodeInfo,
             host: &mut dyn uqm_rust::planets::generate::SiteHost| {
                if case.life {
                    generateSolFunctions.generate_life(view, case.body, case.which, info, host)
                } else {
                    generateSolFunctions.generate_energy(view, case.body, case.which, info, host)
                }
            };
        let before = format!("{:?}", crate::generation_data(&self.state));
        self.seed(case.seed);
        let count = unsafe {
            crate::invoke_generation(&mut *system, |view, host| generate(view, node, host))
        };
        let rng = self.rng_position();
        self.seed(case.seed);
        let count_only = unsafe {
            crate::invoke_generation(&mut *system, |view, host| {
                generate(view, &mut crate::planets_common::fresh_node(), host)
            })
        };
        assert_eq!(
            (count_only, self.rng_position()),
            (count, rng),
            "null output: {case:?}"
        );
        assert_eq!(
            format!("{:?}", crate::generation_data(&self.state)),
            before,
            "scan changed solar system: {case:?}"
        );
        assert_eq!(
            uqm_rust::engine::World::current().read(|world| get_game_state(
                world,
                uqm_rust::engine::StoryKey::FOUND_PLUTO_SPATHI
            )),
            u32::from(case.pluto)
        );
        assert_eq!(
            uqm_rust::engine::World::current().read(|world| get_game_state(
                world,
                uqm_rust::engine::StoryKey::MOONBASE_DESTROYED
            )),
            u32::from(case.destroyed)
        );
        // The count-only query above must leave the same post-call seed.
        assert_eq!(self.state.sys.rng.seed(), rng);
        (count, rng)
    }
}

pub fn check(table: &[uqm_rust::engine::world::planet::PlanetFrame]) {
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

use uqm_rust::planets::generate::SystemGenerator;
