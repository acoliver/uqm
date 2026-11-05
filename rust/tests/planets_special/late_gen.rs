//! Invoke the production Rust ABI tables against retained C fixtures.
use super::Harness;
use crate::planets_common::late_gen::{self, Kind, Layout, Ops, Scan};
use uqm_rust::c_bindings::global_state::{PlanetFrame, NODE_INFO, PLANET_DESC, SYSTEM_INFO};
use uqm_rust::planets::generate::late_exports::GENERATE_SAMATRA_FUNCTIONS as generateSaMatraFunctions;
use uqm_rust::planets::generate::late_exports::GENERATE_SPATHI_FUNCTIONS as generateSpathiFunctions;
use uqm_rust::planets::generate::late_exports::GENERATE_VUX_FUNCTIONS as generateVuxFunctions;
use uqm_rust::planets::generate::special::Body;
use uqm_rust::planets::generate::vtable::GenerateFunctions;
use uqm_rust::state::game_state_keys::get_game_state;

fn table(kind: Kind) -> &'static GenerateFunctions {
    match kind {
        Kind::Vux => &generateVuxFunctions,
        Kind::Samatra => &generateSaMatraFunctions,
        Kind::Spathi => &generateSpathiFunctions,
    }
}
impl Ops for Harness {
    fn layout(&mut self, case: Layout, planets: &mut [PLANET_DESC]) -> (bool, u8, u32) {
        self.star_index = case.star;
        self.set_star(case.size, case.color);
        self.state.PlanetDesc.copy_from_slice(planets);
        self.state.SunDesc[0].NumPlanets = 0;
        uqm_rust::engine::World::current().write(|world| {
            (world).set_story_state(
                uqm_rust::engine::StoryKey::SPATHI_SHIELDED_SELVES,
                u32::from(case.shielded),
            )
        });
        self.seed(case.seed);
        let hook = table(case.kind).generate_planets.unwrap();
        let mut expected = *self.state;
        let result = unsafe { hook(&mut *self.state) };
        expected.PlanetDesc = self.state.PlanetDesc;
        expected.SunDesc[0].NumPlanets = self.state.SunDesc[0].NumPlanets;
        assert_eq!(format!("{:?}", self.state), format!("{expected:?}"));
        planets.copy_from_slice(&self.state.PlanetDesc);
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
        moons: &mut [PLANET_DESC],
    ) -> (bool, u32) {
        self.star_index = case.star;
        self.set_star(case.size, case.color);
        self.state.MoonDesc.copy_from_slice(moons);
        self.state.PlanetDesc = unsafe { std::mem::zeroed() };
        let sun: *mut PLANET_DESC = &mut self.state.SunDesc[0];
        self.state.PlanetDesc[usize::from(planet)].pPrevDesc = sun;
        self.state.PlanetDesc[usize::from(planet)].NumPlanets = count;
        let parent: *mut PLANET_DESC = &mut self.state.PlanetDesc[usize::from(planet)];
        self.seed(case.seed);
        let hook = table(case.kind).generate_moons.unwrap();
        let mut expected = *self.state;
        let result = unsafe { hook(&mut *self.state, parent) };
        expected.MoonDesc = self.state.MoonDesc;
        assert_eq!(format!("{:?}", self.state), format!("{expected:?}"));
        moons.copy_from_slice(&self.state.MoonDesc);
        (result, self.rng_position())
    }
    fn scan(&mut self, case: Scan, info: &SYSTEM_INFO, node: &mut NODE_INFO) -> (u16, u32) {
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
        let hook = if case.life {
            table(case.kind).generate_life
        } else {
            table(case.kind).generate_energy
        }
        .unwrap();
        self.seed(case.seed);
        let before = format!("{:?}", self.state);
        let result = unsafe { hook(&*self.state, world, case.which, node) };
        let after_rng = self.rng_position();
        self.seed(case.seed);
        let count_only = unsafe { hook(&*self.state, world, case.which, std::ptr::null_mut()) };
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
