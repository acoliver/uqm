//! Invoke the production Rust ABI tables against retained C fixtures.
use super::Harness;
use crate::planets_common::late_gen::{self, Kind, Layout, Ops, Scan};
use uqm_rust::engine::world::planet::{NodeInfo, PlanetDesc, PlanetFrame, SystemInfo};
use uqm_rust::planets::generate::sam::Generator as generateSaMatraFunctions;
use uqm_rust::planets::generate::spa::Generator as generateSpathiFunctions;
use uqm_rust::planets::generate::vux::Generator as generateVuxFunctions;
use uqm_rust::planets::generate::SystemGenerator;
use uqm_rust::state::game_state_keys::get_game_state;

fn table(kind: Kind) -> &'static dyn SystemGenerator {
    match kind {
        Kind::Vux => &generateVuxFunctions as &dyn SystemGenerator,
        Kind::Samatra => &generateSaMatraFunctions as &dyn SystemGenerator,
        Kind::Spathi => &generateSpathiFunctions as &dyn SystemGenerator,
    }
}
impl Ops for Harness {
    fn layout(&mut self, case: Layout, planets: &mut [PlanetDesc]) -> (bool, u8, u32) {
        self.star_index = case.star;
        self.set_star(case.size, case.color);
        self.state.sys.planets.copy_from_slice(planets);
        self.state.sys.sun.num_planets = 0;
        uqm_rust::engine::World::current().write(|world| {
            (world).set_story_state(
                uqm_rust::engine::StoryKey::SPATHI_SHIELDED_SELVES,
                u32::from(case.shielded),
            )
        });
        self.seed(case.seed);

        let mut expected = crate::generation_data(&self.state);
        let result = unsafe {
            crate::invoke_generation(&mut self.state, |system, host| {
                table(case.kind).generate_planets(system, host)
            })
        };
        expected.1 = self.state.sys.planets;
        expected.0.num_planets = self.state.sys.sun.num_planets;
        assert_eq!(
            format!("{:?}", crate::generation_data(&self.state)),
            format!("{expected:?}")
        );
        planets.copy_from_slice(&self.state.sys.planets);
        (result, self.state.sys.sun.num_planets, self.rng_position())
    }
    fn moons(
        &mut self,
        case: Layout,
        planet: u8,
        count: u8,
        moons: &mut [PlanetDesc],
    ) -> (bool, u32) {
        self.star_index = case.star;
        self.set_star(case.size, case.color);
        self.state.sys.moons.copy_from_slice(moons);
        self.state.sys.planets = unsafe { std::mem::zeroed() };
        self.state.sys.planets[usize::from(planet)].num_planets = count;
        self.seed(case.seed);

        let mut expected = crate::generation_data(&self.state);
        let result = unsafe {
            crate::invoke_generation(&mut self.state, |system, host| {
                table(case.kind).generate_moons(system, planet, host)
            })
        };
        expected.2 = self.state.sys.moons;
        assert_eq!(
            format!("{:?}", crate::generation_data(&self.state)),
            format!("{expected:?}")
        );
        moons.copy_from_slice(&self.state.sys.moons);
        (result, self.rng_position())
    }
    fn scan(&mut self, case: Scan, info: &SystemInfo, node: &mut NodeInfo) -> (u16, u32) {
        self.star_index = case.star;
        self.set_star(0, 0);
        for key in [
            uqm_rust::engine::StoryKey::SHOFIXTI_MAIDENS,
            uqm_rust::engine::StoryKey::UMGAH_BROADCASTERS,
        ] {
            uqm_rust::engine::World::current()
                .write(|world| (world).set_story_state(key, u32::from(case.taken)));
        }
        self.state.sys.planets = unsafe { std::mem::zeroed() };
        self.state.sys.moons = unsafe { std::mem::zeroed() };
        self.state.sys.info = *info;
        match case.body {
            uqm_rust::planets::generate::special::Body::Planet(p) => {
                self.state.sys.planets[usize::from(p)].data_index = case.frame
            }
            uqm_rust::planets::generate::special::Body::Moon { moon, .. } => {
                self.state.sys.moons[usize::from(moon)].data_index = case.frame
            }
        }
        let generate =
            |view: &mut uqm_rust::engine::world::generation::SolarSys<'_>,
             info: &mut NodeInfo,
             host: &mut dyn uqm_rust::planets::generate::SiteHost| {
                if case.life {
                    table(case.kind).generate_life(view, case.body, case.which, info, host)
                } else {
                    table(case.kind).generate_energy(view, case.body, case.which, info, host)
                }
            };
        self.seed(case.seed);
        let before = format!("{:?}", crate::generation_data(&self.state));
        let result = unsafe {
            crate::invoke_generation(&mut self.state, |view, host| generate(view, node, host))
        };
        let after_rng = self.rng_position();
        self.seed(case.seed);
        let count_only = unsafe {
            crate::invoke_generation(&mut self.state, |view, host| {
                generate(view, &mut crate::planets_common::fresh_node(), host)
            })
        };
        assert_eq!(count_only, result, "NULL output count: {case:?}");
        assert_eq!(self.rng_position(), after_rng, "NULL output RNG: {case:?}");
        assert_eq!(
            format!("{:?}", crate::generation_data(&self.state)),
            before,
            "scan input changed: {case:?}"
        );
        for key in [
            uqm_rust::engine::StoryKey::SHOFIXTI_MAIDENS,
            uqm_rust::engine::StoryKey::UMGAH_BROADCASTERS,
        ] {
            assert_eq!(
                uqm_rust::engine::World::current().read(|world| get_game_state(world, key)),
                u32::from(case.taken)
            );
        }
        (result, self.rng_position())
    }
}
pub fn check(frames: &[PlanetFrame]) {
    for (kind, name) in [
        (Kind::Vux, "vux.txt"),
        (Kind::Samatra, "samatra.txt"),
        (Kind::Spathi, "spathi.txt"),
    ] {
        let from_export = late_gen::render(kind, frames, &mut Harness::new());
        crate::check_fixture(name, &from_export);
        assert_eq!(
            from_export,
            late_gen::render(kind, frames, &mut late_gen::Rust(frames)),
            "{kind:?}"
        );
        println!(
            "{kind:?}: {} exact Rust export == retained C fixture == Rust cases",
            from_export.lines().count()
        );
    }
}
