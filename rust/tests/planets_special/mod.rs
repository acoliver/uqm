//! Linked check of the `generateXFunctions` tables of the small per-system
//! generators: the deterministic hooks (planet layout, moons, energy nodes)
//! are run through the tables, Rust-exported, over the same
//! inputs and must leave the descriptions, node outputs and `SysGenRNG`
//! position captured from the C originals in the checked-in fixture.

use uqm_rust::planets::generate::andro::Generator as generateAndrosynthFunctions;
use uqm_rust::planets::generate::burv::Generator as generateBurvixeseFunctions;
use uqm_rust::planets::generate::chmmr::Generator as generateChmmrFunctions;
use uqm_rust::planets::generate::colony::Generator as generateColonyFunctions;
use uqm_rust::planets::generate::dru::Generator as generateDruugeFunctions;
use uqm_rust::planets::generate::ilw::Generator as generateIlwrathFunctions;
use uqm_rust::planets::generate::myc::Generator as generateMyconFunctions;
use uqm_rust::planets::generate::orz::Generator as generateOrzFunctions;
use uqm_rust::planets::generate::pet::Generator as generateTalkingPetFunctions;
use uqm_rust::planets::generate::pku::Generator as generatePkunkFunctions;
use uqm_rust::planets::generate::rainbow::Generator as generateRainbowWorldFunctions;
use uqm_rust::planets::generate::shof::Generator as generateShofixtiFunctions;
use uqm_rust::planets::generate::slylandro::Generator as generateSlylandroFunctions;
use uqm_rust::planets::generate::sup::Generator as generateSupoxFunctions;
use uqm_rust::planets::generate::syreen::Generator as generateSyreenFunctions;
use uqm_rust::planets::generate::thrad::Generator as generateThraddashFunctions;
use uqm_rust::planets::generate::trap::Generator as generateTrapFunctions;
use uqm_rust::planets::generate::utw::Generator as generateUtwigFunctions;
use uqm_rust::planets::generate::vault::Generator as generateVaultFunctions;
use uqm_rust::planets::generate::wreck::Generator as generateWreckFunctions;
use uqm_rust::planets::generate::yehat::Generator as generateYehatFunctions;
use uqm_rust::planets::generate::zoq::Generator as generateZoqFotPikFunctions;
use uqm_rust::planets::solar_system::SolarSystem;
mod late_abi;
mod late_gen;
mod mycon;
mod shield_additive;
mod sol;
mod sol_abi;
mod sol_npcs;
mod sol_orbital;
mod sol_resources;

use uqm_rust::savegame::runtime::star::{CStarDesc, CATALOG_LEN};

use std::fmt::Write as _;

use crate::planets_common::special_gen::{self, desc_of, Desc, SpecialOps};
use crate::planets_common::{fresh_node, scan_seeds, write_node};
use uqm_rust::battle::gendef::{
    AQUA_HELIX_DEFINED, BOMB_DEFINED, EGG_CASE0_DEFINED, MYCON_DEFINED, ORZ_DEFINED,
    SUN_DEVICE_DEFINED, TAALO_PROTECTOR_DEFINED, THRADD_DEFINED, UTWIG_DEFINED,
};

use uqm_rust::math::RandomContext;
use uqm_rust::planets::generate::special::Body;
use uqm_rust::planets::generate::SystemGenerator;
use uqm_rust::state::game_state_keys::set_game_state;

use uqm_rust::state::ffi::rust_init_game_state;
extern "C" {

    static mut ScreenWidth: i32;

}

struct Harness {
    state: Box<SolarSystem>,
    star: Box<[CStarDesc; CATALOG_LEN]>,
    previous_catalog: [CStarDesc; CATALOG_LEN],
    previous_star: Option<uqm_rust::engine::StarId>,
    /// `CurStarDescPtr->Index` the next `set_star` installs.
    star_index: u8,
}

impl Harness {
    fn new() -> Self {
        unsafe {
            // `set_game_state` is a no-op until the game state exists.
            uqm_rust::engine::World::current().write(rust_init_game_state);
            ScreenWidth = 320;
        }
        let previous_catalog = uqm_rust::engine::World::current().read(|world| *world.catalog());
        let previous_star = uqm_rust::engine::World::current().read(|world| world.current_star);
        // SAFETY: the serialized harness copies the initialized integer-only catalog.
        let mut star = Box::new(unsafe { super::starmap_array });
        // SAFETY: a star record contains only integer fields; zero initializes its fixture values.
        star[0] = unsafe { std::mem::zeroed() };
        let state = uqm_rust::engine::World::current().write(|world| {
            world.set_catalog(std::borrow::Cow::Owned(*star));
            Box::new(SolarSystem::new(world, uqm_rust::engine::StarId(0)))
        });
        Self {
            state,
            star,
            previous_catalog,
            previous_star,
            star_index: 0,
        }
    }

    fn set_star(&mut self, size: u8, color: u8) {
        self.state.sys.sun.data_index = size;
        self.star[0].star_type = (color << 5) | 0b101;
        self.star[0].index = self.star_index;
        uqm_rust::engine::World::current().write(|world| {
            world.set_catalog(std::borrow::Cow::Owned(*self.star));
            world.current_star = Some(uqm_rust::engine::StarId(0));
        });
        let mut next = uqm_rust::engine::world::generation::SolarSys::new(
            &uqm_rust::planets::tables::PLANET_ARRAY,
            self.star[0].star_type,
            self.star[0].index,
            std::mem::replace(&mut self.state.sys.rng, RandomContext::with_seed(12345)),
        );
        next.sun = self.state.sys.sun;
        next.planets = self.state.sys.planets;
        next.moons = self.state.sys.moons;
        next.info = self.state.sys.info;
        next.frames = std::mem::take(&mut self.state.sys.frames);
        next.report = std::mem::take(&mut self.state.sys.report);
        self.state.sys = next;
    }

    fn seed(&mut self, seed: u32) {
        self.state.sys.rng.reseed(seed);
    }

    fn rng_position(&self) -> u32 {
        self.state.sys.rng.seed()
    }

    /// The C `generatePlanets` slot: planet table, drawn count, RNG position.
    fn c_planets(
        &mut self,
        table: &dyn SystemGenerator,
        case: (u8, u8, u32),
    ) -> (Vec<Desc>, u8, u32) {
        self.set_star(case.0, case.1);
        self.state.sys.planets = unsafe { std::mem::zeroed() };
        self.state.sys.sun.num_planets = 0;
        self.seed(case.2);

        assert!(unsafe {
            crate::invoke_generation(&mut self.state, |system, host| {
                table.generate_planets(system, host)
            })
        });
        (
            self.state.sys.planets.iter().map(desc_of).collect(),
            self.state.sys.sun.num_planets,
            self.rng_position(),
        )
    }

    /// The C `generateMoons` slot for planet `planet_i`.
    fn c_moons(
        &mut self,
        table: &dyn SystemGenerator,
        case: (u8, u8, u32, u8),
        planet_i: usize,
    ) -> (Vec<Desc>, u32) {
        self.set_star(case.0, case.1);
        self.state.sys.moons = unsafe { std::mem::zeroed() };
        self.state.sys.planets = unsafe { std::mem::zeroed() };

        self.state.sys.planets[planet_i].num_planets = case.3;
        self.seed(case.2);

        assert!(unsafe {
            crate::invoke_generation(&mut self.state, |system, host| {
                table.generate_moons(system, planet_i as u8, host)
            })
        });
        (
            self.state.sys.moons.iter().map(desc_of).collect(),
            self.rng_position(),
        )
    }

    /// The C `generateEnergy` slot on `body`; the node list it produces.
    fn c_energy(&mut self, table: &dyn SystemGenerator, body: Body, seed: u32) -> String {
        self.state.sys.planets = unsafe { std::mem::zeroed() };
        self.state.sys.moons = unsafe { std::mem::zeroed() };
        self.state.sys.info = unsafe { std::mem::zeroed() };
        self.state.sys.info.planet_info.scan_seed = scan_seeds(seed);

        let record = &raw mut *self.state;
        let mut out = String::new();
        self.seed(seed);
        let mut node = fresh_node();
        let count = unsafe {
            crate::invoke_generation(&mut *record, |system, host| {
                table.generate_energy(system, body, !0, &mut node, host)
            })
        };
        let _ = write!(out, "{count}@{}[", self.rng_position());
        for which in 0..count.min(32) {
            self.seed(seed);
            let mut node = fresh_node();
            let ret = unsafe {
                crate::invoke_generation(&mut *record, |system, host| {
                    table.generate_energy(system, body, which, &mut node, host)
                })
            };
            write_node(&mut out, ret, &node);
            let _ = write!(out, "@{};", self.rng_position());
        }
        out.push(']');
        out
    }
}

/// `CurStarDescPtr->Index` of the Thraddash and Orz generators' stars; 0 for
/// the systems that do not look at it.
fn star_index_of(system: &str) -> u8 {
    if system.starts_with("aqua") {
        AQUA_HELIX_DEFINED
    } else if system == "thrad" {
        THRADD_DEFINED
    } else if system == "orz" {
        ORZ_DEFINED
    } else if system.starts_with("taalo") {
        TAALO_PROTECTOR_DEFINED
    } else if system == "utw" {
        UTWIG_DEFINED
    } else if system.starts_with("bomb") {
        BOMB_DEFINED
    } else if system == "myc" {
        MYCON_DEFINED
    } else if system.starts_with("sun") {
        SUN_DEVICE_DEFINED
    } else if system == "egg" {
        EGG_CASE0_DEFINED
    } else {
        0
    }
}

/// The C tables, looked up by system name.
fn c_table(system: &str) -> &'static dyn SystemGenerator {
    match system {
        "yehat" => &generateYehatFunctions as &dyn SystemGenerator,
        "ilw" => &generateIlwrathFunctions as &dyn SystemGenerator,
        "burv" | "burv0" | "burv1" => &generateBurvixeseFunctions as &dyn SystemGenerator,
        "aqua" | "aqua0" | "aqua1" | "thrad" => &generateThraddashFunctions as &dyn SystemGenerator,
        "orz" | "taalo" | "taalo0" | "taalo1" => &generateOrzFunctions as &dyn SystemGenerator,
        "zoq" => &generateZoqFotPikFunctions as &dyn SystemGenerator,
        "shof" => &generateShofixtiFunctions as &dyn SystemGenerator,
        "andro" => &generateAndrosynthFunctions as &dyn SystemGenerator,
        "pet" => &generateTalkingPetFunctions as &dyn SystemGenerator,
        "utw" | "bomb" | "bomb0" | "bomb1" => &generateUtwigFunctions as &dyn SystemGenerator,
        "myc" | "sun0" | "sun1" | "egg" => &generateMyconFunctions as &dyn SystemGenerator,
        "pku" => &generatePkunkFunctions as &dyn SystemGenerator,
        "sup" => &generateSupoxFunctions as &dyn SystemGenerator,
        "dru" => &generateDruugeFunctions as &dyn SystemGenerator,
        "chmmr0" | "chmmr1" | "chmmr" => &generateChmmrFunctions as &dyn SystemGenerator,
        "vault" => &generateVaultFunctions as &dyn SystemGenerator,
        "colony" => &generateColonyFunctions as &dyn SystemGenerator,
        "wreck" => &generateWreckFunctions as &dyn SystemGenerator,
        "rainbow" => &generateRainbowWorldFunctions as &dyn SystemGenerator,
        "syreen" => &generateSyreenFunctions as &dyn SystemGenerator,
        "trap" => &generateTrapFunctions as &dyn SystemGenerator,
        "sly" => &generateSlylandroFunctions as &dyn SystemGenerator,
        other => panic!("no such system {other}"),
    }
}

impl SpecialOps for Harness {
    fn layout(&mut self, system: &str, case: (u8, u8, u32)) -> (Vec<Desc>, u8, u32) {
        if let Some(unleashed) = system.strip_prefix("chmmr") {
            uqm_rust::engine::World::current().write(|world| {
                set_game_state(
                    world,
                    uqm_rust::engine::StoryKey::CHMMR_UNLEASHED,
                    unleashed.parse().expect("0 or 1"),
                )
            });
        }
        self.star_index = star_index_of(system);
        self.c_planets(c_table(system), case)
    }

    fn moons(
        &mut self,
        system: &str,
        planet_i: usize,
        case: (u8, u8, u32, u8),
    ) -> (Vec<Desc>, u32) {
        self.c_moons(c_table(system), case, planet_i)
    }

    fn energy(&mut self, system: &str, body: Body, seed: u32) -> String {
        if let Some(taken) = system.strip_prefix("burv") {
            uqm_rust::engine::World::current().write(|world| {
                set_game_state(
                    world,
                    uqm_rust::engine::StoryKey::BURVIXESE_BROADCASTERS,
                    taken.parse().expect("0 or 1"),
                )
            });
        }
        if let Some(taken) = system.strip_prefix("aqua") {
            uqm_rust::engine::World::current().write(|world| {
                set_game_state(
                    world,
                    uqm_rust::engine::StoryKey::AQUA_HELIX,
                    taken.parse().expect("0 or 1"),
                )
            });
        }
        if let Some(taken) = system.strip_prefix("bomb") {
            uqm_rust::engine::World::current().write(|world| {
                set_game_state(
                    world,
                    uqm_rust::engine::StoryKey::UTWIG_BOMB,
                    taken.parse().expect("0 or 1"),
                )
            });
        }
        if let Some(taken) = system.strip_prefix("sun") {
            uqm_rust::engine::World::current().write(|world| {
                set_game_state(
                    world,
                    uqm_rust::engine::StoryKey::SUN_DEVICE,
                    taken.parse().expect("0 or 1"),
                )
            });
        }
        if let Some(taken) = system.strip_prefix("taalo") {
            uqm_rust::engine::World::current().write(|world| {
                set_game_state(
                    world,
                    uqm_rust::engine::StoryKey::TAALO_PROTECTOR,
                    taken.parse().expect("0 or 1"),
                )
            });
        }
        self.star_index = star_index_of(system);
        self.set_star(0, 0);
        self.c_energy(c_table(system), body, seed)
    }
}

/// Run every comparison; called from `planets_parity`'s single test because
/// the C globals they drive cannot be shared between parallel tests.
pub fn special_generators_match_the_c_originals() {
    let rendered = special_gen::render(&mut Harness::new());
    crate::check_fixture("special_generators.txt", &rendered);
    println!(
        "special generators: {} fixture cases",
        rendered.lines().count()
    );
}

pub fn mycon_matches_the_c_original(table: &[uqm_rust::engine::world::planet::PlanetFrame]) {
    mycon::check(table);
}

pub fn late_generators_match(frames: &[uqm_rust::engine::world::planet::PlanetFrame]) {
    late_gen::check(frames);
    late_abi::check();
}

pub fn sol_matches_the_c_original(table: &[uqm_rust::engine::world::planet::PlanetFrame]) {
    sol::check(table);
    sol_abi::check();
}

impl Drop for Harness {
    fn drop(&mut self) {
        uqm_rust::engine::World::current().write(|world| {
            world.set_catalog(std::borrow::Cow::Owned(self.previous_catalog));
            world.current_star = self.previous_star;
        });
    }
}
