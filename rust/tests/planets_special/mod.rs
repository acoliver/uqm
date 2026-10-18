//! Linked check of the `generateXFunctions` tables of the small per-system
//! generators: the deterministic hooks (planet layout, moons, energy nodes)
//! are run through the tables, Rust-exported, over the same
//! inputs and must leave the descriptions, node outputs and `SysGenRNG`
//! position captured from the C originals in the checked-in fixture.

use std::ffi::c_void;
use std::fmt::Write as _;

use crate::planets_common::special_gen::{self, desc_of, Desc, SpecialOps};
use crate::planets_common::{fresh_node, scan_seeds, write_node};
use uqm_rust::battle::gendef::{
    AQUA_HELIX_DEFINED, ORZ_DEFINED, TAALO_PROTECTOR_DEFINED, THRADD_DEFINED,
};
use uqm_rust::c_bindings::global_state::{PLANET_DESC, SOLARSYS_STATE};
use uqm_rust::math::RandomContext;
use uqm_rust::planets::generate::special::Body;
use uqm_rust::planets::generate::vtable::GenerateFunctions;
use uqm_rust::state::game_state_keys::set_game_state;

#[cfg_attr(
    target_os = "macos",
    link(name = "uqm_c", kind = "static", modifiers = "+whole-archive")
)]
unsafe extern "C" {
    static mut SysGenRNG: *mut RandomContext;
    static mut pSolarSysState: *mut SOLARSYS_STATE;
    static mut CurStarDescPtr: *mut c_void;
    static mut ScreenWidth: i32;
    fn rust_init_game_state();
    static generateYehatFunctions: GenerateFunctions;
    static generateIlwrathFunctions: GenerateFunctions;
    static generateChmmrFunctions: GenerateFunctions;
    static generateBurvixeseFunctions: GenerateFunctions;
    static generateThraddashFunctions: GenerateFunctions;
    static generateOrzFunctions: GenerateFunctions;
    static generateZoqFotPikFunctions: GenerateFunctions;
    static generateShofixtiFunctions: GenerateFunctions;
    static generateAndrosynthFunctions: GenerateFunctions;
    static generateTalkingPetFunctions: GenerateFunctions;
    static generatePkunkFunctions: GenerateFunctions;
    static generateSupoxFunctions: GenerateFunctions;
    static generateDruugeFunctions: GenerateFunctions;
    static generateVaultFunctions: GenerateFunctions;
    static generateColonyFunctions: GenerateFunctions;
    static generateWreckFunctions: GenerateFunctions;
    static generateRainbowWorldFunctions: GenerateFunctions;
    static generateSyreenFunctions: GenerateFunctions;
    static generateTrapFunctions: GenerateFunctions;
    static generateSlylandroFunctions: GenerateFunctions;
}

struct Harness {
    state: Box<SOLARSYS_STATE>,
    star: Box<[u8; 8]>,
    /// `CurStarDescPtr->Index` the next `set_star` installs.
    star_index: u8,
}

impl Harness {
    fn new() -> Self {
        let mut state: Box<SOLARSYS_STATE> = Box::new(unsafe { std::mem::zeroed() });
        unsafe {
            // `set_game_state` is a no-op until the game state exists.
            rust_init_game_state();
            SysGenRNG = uqm_rust::math::RandomContext_New();
            ScreenWidth = 320;
            pSolarSysState = &mut *state;
        }
        Self {
            state,
            star: Box::new([0; 8]),
            star_index: 0,
        }
    }

    fn set_star(&mut self, size: u8, color: u8) {
        self.state.SunDesc[0].data_index = size;
        self.star[4] = (color << 5) | 0b101;
        self.star[5] = self.star_index;
        unsafe { CurStarDescPtr = self.star.as_mut_ptr().cast() };
    }

    fn seed(&self, seed: u32) {
        unsafe { uqm_rust::math::RandomContext_SeedRandom(SysGenRNG, seed) };
    }

    fn rng_position(&self) -> u32 {
        unsafe { uqm_rust::math::RandomContext_GetSeed(SysGenRNG) }
    }

    /// The C `generatePlanets` slot: planet table, drawn count, RNG position.
    fn c_planets(
        &mut self,
        table: &GenerateFunctions,
        case: (u8, u8, u32),
    ) -> (Vec<Desc>, u8, u32) {
        self.set_star(case.0, case.1);
        self.state.PlanetDesc = unsafe { std::mem::zeroed() };
        self.state.SunDesc[0].NumPlanets = 0;
        self.seed(case.2);
        let hook = table.generate_planets.expect("generatePlanets is set");
        assert!(unsafe { hook(&mut *self.state) });
        (
            self.state.PlanetDesc.iter().map(desc_of).collect(),
            self.state.SunDesc[0].NumPlanets,
            self.rng_position(),
        )
    }

    /// The C `generateMoons` slot for planet `planet_i`.
    fn c_moons(
        &mut self,
        table: &GenerateFunctions,
        case: (u8, u8, u32, u8),
        planet_i: usize,
    ) -> (Vec<Desc>, u32) {
        self.set_star(case.0, case.1);
        self.state.MoonDesc = unsafe { std::mem::zeroed() };
        self.state.PlanetDesc = unsafe { std::mem::zeroed() };
        let sun: *mut PLANET_DESC = &mut self.state.SunDesc[0];
        self.state.PlanetDesc[planet_i].pPrevDesc = sun;
        self.state.PlanetDesc[planet_i].NumPlanets = case.3;
        self.seed(case.2);
        let planet: *mut PLANET_DESC = &mut self.state.PlanetDesc[planet_i];
        let hook = table.generate_moons.expect("generateMoons is set");
        assert!(unsafe { hook(&mut *self.state, planet) });
        (
            self.state.MoonDesc.iter().map(desc_of).collect(),
            self.rng_position(),
        )
    }

    /// The C `generateEnergy` slot on `body`; the node list it produces.
    fn c_energy(&mut self, table: &GenerateFunctions, body: Body, seed: u32) -> String {
        self.state.PlanetDesc = unsafe { std::mem::zeroed() };
        self.state.MoonDesc = unsafe { std::mem::zeroed() };
        self.state.SysInfo = unsafe { std::mem::zeroed() };
        self.state.SysInfo.PlanetInfo.ScanSeed = scan_seeds(seed);
        let sun: *mut PLANET_DESC = &mut self.state.SunDesc[0];
        let world: *mut PLANET_DESC = match body {
            Body::Planet(p) => {
                self.state.PlanetDesc[usize::from(p)].pPrevDesc = sun;
                &mut self.state.PlanetDesc[usize::from(p)]
            }
            Body::Moon { planet, moon } => {
                let parent: *mut PLANET_DESC = &mut self.state.PlanetDesc[usize::from(planet)];
                self.state.PlanetDesc[usize::from(planet)].pPrevDesc = sun;
                self.state.MoonDesc[usize::from(moon)].pPrevDesc = parent;
                &mut self.state.MoonDesc[usize::from(moon)]
            }
        };
        let hook = table.generate_energy.expect("generateEnergy is set");
        let system: *const SOLARSYS_STATE = &*self.state;
        let mut out = String::new();
        self.seed(seed);
        let mut node = fresh_node();
        let count = unsafe { hook(system, world, !0, &mut node) };
        let _ = write!(out, "{count}@{}[", self.rng_position());
        for which in 0..count.min(32) {
            self.seed(seed);
            let mut node = fresh_node();
            let ret = unsafe { hook(system, world, which, &mut node) };
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
    } else {
        0
    }
}

/// The C tables, looked up by system name.
fn c_table(system: &str) -> &'static GenerateFunctions {
    unsafe {
        match system {
            "yehat" => &generateYehatFunctions,
            "ilw" => &generateIlwrathFunctions,
            "burv" | "burv0" | "burv1" => &generateBurvixeseFunctions,
            "aqua" | "aqua0" | "aqua1" | "thrad" => &generateThraddashFunctions,
            "orz" | "taalo" | "taalo0" | "taalo1" => &generateOrzFunctions,
            "zoq" => &generateZoqFotPikFunctions,
            "shof" => &generateShofixtiFunctions,
            "andro" => &generateAndrosynthFunctions,
            "pet" => &generateTalkingPetFunctions,
            "pku" => &generatePkunkFunctions,
            "sup" => &generateSupoxFunctions,
            "dru" => &generateDruugeFunctions,
            "chmmr0" | "chmmr1" | "chmmr" => &generateChmmrFunctions,
            "vault" => &generateVaultFunctions,
            "colony" => &generateColonyFunctions,
            "wreck" => &generateWreckFunctions,
            "rainbow" => &generateRainbowWorldFunctions,
            "syreen" => &generateSyreenFunctions,
            "trap" => &generateTrapFunctions,
            "sly" => &generateSlylandroFunctions,
            other => panic!("no such system {other}"),
        }
    }
}

impl SpecialOps for Harness {
    fn layout(&mut self, system: &str, case: (u8, u8, u32)) -> (Vec<Desc>, u8, u32) {
        if let Some(unleashed) = system.strip_prefix("chmmr") {
            set_game_state("CHMMR_UNLEASHED", unleashed.parse().expect("0 or 1"));
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
            set_game_state("BURVIXESE_BROADCASTERS", taken.parse().expect("0 or 1"));
        }
        if let Some(taken) = system.strip_prefix("aqua") {
            set_game_state("AQUA_HELIX", taken.parse().expect("0 or 1"));
        }
        if let Some(taken) = system.strip_prefix("taalo") {
            set_game_state("TAALO_PROTECTOR", taken.parse().expect("0 or 1"));
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
