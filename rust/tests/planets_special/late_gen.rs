//! Invoke the production Rust ABI tables against retained C fixtures.
use super::Harness;
use crate::planets_common::late_gen::{self, Kind, Layout, Ops, Scan};
use uqm_rust::engine::world::planet::{NodeInfo, PlanetDesc, PlanetFrame, SystemInfo};
use uqm_rust::planets::generate::sam::Generator as generateSaMatraFunctions;
use uqm_rust::planets::generate::spa::Generator as generateSpathiFunctions;
use uqm_rust::planets::generate::special::Body;
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
        self.state.PlanetDesc.copy_from_slice(planets);
        self.state.SunDesc[0].num_planets = 0;
        uqm_rust::engine::World::current().write(|world| {
            (world).set_story_state(
                uqm_rust::engine::StoryKey::SPATHI_SHIELDED_SELVES,
                u32::from(case.shielded),
            )
        });
        self.seed(case.seed);

        let mut expected = *self.state;
        let result = unsafe {
            crate::invoke_generation(&raw mut *self.state, |system, host| {
                table(case.kind).generate_planets(system, host)
            })
        };
        expected.PlanetDesc = self.state.PlanetDesc;
        expected.SunDesc[0].num_planets = self.state.SunDesc[0].num_planets;
        assert_eq!(format!("{:?}", self.state), format!("{expected:?}"));
        planets.copy_from_slice(&self.state.PlanetDesc);
        (
            result,
            self.state.SunDesc[0].num_planets,
            self.rng_position(),
        )
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
        self.state.MoonDesc.copy_from_slice(moons);
        self.state.PlanetDesc = unsafe { std::mem::zeroed() };
        let sun: *mut PlanetDesc = &mut self.state.SunDesc[0];
        self.state.PlanetDesc[usize::from(planet)].prev_desc = sun;
        self.state.PlanetDesc[usize::from(planet)].num_planets = count;
        let _parent: *mut PlanetDesc = &mut self.state.PlanetDesc[usize::from(planet)];
        self.seed(case.seed);

        let mut expected = *self.state;
        let result = unsafe {
            crate::invoke_generation(&raw mut *self.state, |system, host| {
                table(case.kind).generate_moons(system, planet, host)
            })
        };
        expected.MoonDesc = self.state.MoonDesc;
        assert_eq!(format!("{:?}", self.state), format!("{expected:?}"));
        moons.copy_from_slice(&self.state.MoonDesc);
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
        self.state.PlanetDesc = unsafe { std::mem::zeroed() };
        self.state.MoonDesc = unsafe { std::mem::zeroed() };
        self.state.SysInfo = *info;
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
        match case.body {
            uqm_rust::planets::generate::special::Body::Planet(p) => {
                self.state.PlanetDesc[usize::from(p)].data_index = case.frame
            }
            uqm_rust::planets::generate::special::Body::Moon { moon, .. } => {
                self.state.MoonDesc[usize::from(moon)].data_index = case.frame
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
        let before = format!("{:?}", self.state);
        let result = unsafe {
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
        assert_eq!(count_only, result, "NULL output count: {case:?}");
        assert_eq!(self.rng_position(), after_rng, "NULL output RNG: {case:?}");
        assert_eq!(
            format!("{:?}", self.state),
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
