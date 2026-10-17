//! Linked parity of the Rust ports of the small per-system generators
//! (`genyeh.c` ... `gensly.c`) against the C `generateXFunctions` tables:
//! the deterministic hooks (planet layout, moons, energy nodes) are run on
//! both sides over the same inputs and must leave the same descriptions, node
//! outputs and `SysGenRNG` position.

use std::ffi::c_void;
use std::fmt::Write as _;

use crate::planets_common::default_gen::{layout_cases, moon_cases};
use crate::planets_common::{fresh_node, scan_seeds, write_node};
use uqm_rust::c_bindings::global_state::{
    PlanetFrame, NODE_INFO, PLANET_DESC, SOLARSYS_STATE, SYSTEM_INFO,
};
use uqm_rust::math::RandomContext;
use uqm_rust::planets::consts::star_color;
use uqm_rust::planets::generate::special::Body;
use uqm_rust::planets::generate::vtable::GenerateFunctions;
use uqm_rust::planets::generate::{colony, rainbow, slylandro, syreen, trap, vault, wreck, yehat};

#[cfg_attr(
    target_os = "macos",
    link(name = "uqm_c", kind = "static", modifiers = "+whole-archive")
)]
unsafe extern "C" {
    static mut SysGenRNG: *mut RandomContext;
    static mut pSolarSysState: *mut SOLARSYS_STATE;
    static mut CurStarDescPtr: *mut c_void;
    static mut ScreenWidth: i32;
    static planet_array: [PlanetFrame; 59];
    static generateYehatFunctions: GenerateFunctions;
    static generateVaultFunctions: GenerateFunctions;
    static generateColonyFunctions: GenerateFunctions;
    static generateWreckFunctions: GenerateFunctions;
    static generateRainbowWorldFunctions: GenerateFunctions;
    static generateSyreenFunctions: GenerateFunctions;
    static generateTrapFunctions: GenerateFunctions;
    static generateSlylandroFunctions: GenerateFunctions;
}

type Desc = (u8, i16, u32, u8, i16, i16);

fn desc_of(d: &PLANET_DESC) -> Desc {
    (
        d.data_index,
        d.radius,
        d.rand_seed,
        d.NumPlanets,
        d.location.x,
        d.location.y,
    )
}

struct Harness {
    state: Box<SOLARSYS_STATE>,
    star: Box<[u8; 8]>,
}

impl Harness {
    fn new() -> Self {
        let mut state: Box<SOLARSYS_STATE> = Box::new(unsafe { std::mem::zeroed() });
        unsafe {
            SysGenRNG = uqm_rust::math::RandomContext_New();
            ScreenWidth = 320;
            pSolarSysState = &mut *state;
        }
        Self {
            state,
            star: Box::new([0; 8]),
        }
    }

    fn set_star(&mut self, size: u8, color: u8) {
        self.state.SunDesc[0].data_index = size;
        self.star[4] = (color << 5) | 0b101;
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

type PlanetsFn = fn(&mut RandomContext, &[PlanetFrame], u8, u8, &mut [PLANET_DESC]) -> u8;

fn rust_planets(
    hook: PlanetsFn,
    table: &[PlanetFrame],
    case: (u8, u8, u32),
) -> (Vec<Desc>, u8, u32) {
    let mut rng = RandomContext::with_seed(case.2);
    let mut planets: Vec<PLANET_DESC> = vec![unsafe { std::mem::zeroed() }; 16];
    let drawn = hook(&mut rng, table, case.0, case.1, &mut planets);
    (planets.iter().map(desc_of).collect(), drawn, rng.seed())
}

fn rust_colony(case: (u8, u8, u32)) -> (Vec<Desc>, u8, u32) {
    let mut rng = RandomContext::with_seed(case.2);
    let mut planets: Vec<PLANET_DESC> = vec![unsafe { std::mem::zeroed() }; 16];
    let drawn = colony::generate_planets(
        &mut rng,
        case.0,
        star_color((case.1 << 5) | 0b101),
        &mut planets,
    );
    (planets.iter().map(desc_of).collect(), drawn, rng.seed())
}

/// Run every comparison; called from `planets_parity`'s single test because
/// the C globals they drive cannot be shared between parallel tests.
pub fn special_generators_match_the_c_originals() {
    let table = unsafe { &planet_array }.as_slice();
    let mut harness = Harness::new();

    let layouts: [(&str, &GenerateFunctions, Option<PlanetsFn>); 6] = [
        (
            "yehat",
            unsafe { &generateYehatFunctions },
            Some(yehat::generate_planets),
        ),
        ("colony", unsafe { &generateColonyFunctions }, None),
        (
            "rainbow",
            unsafe { &generateRainbowWorldFunctions },
            Some(rainbow::generate_planets),
        ),
        (
            "syreen",
            unsafe { &generateSyreenFunctions },
            Some(syreen::generate_planets),
        ),
        (
            "trap",
            unsafe { &generateTrapFunctions },
            Some(trap::generate_planets),
        ),
        (
            "sly",
            unsafe { &generateSlylandroFunctions },
            Some(slylandro::generate_planets),
        ),
    ];
    let mut cases = 0usize;
    for (name, c_table, rust) in layouts {
        for case in layout_cases() {
            let want = harness.c_planets(c_table, case);
            let got = match rust {
                Some(hook) => rust_planets(hook, table, case),
                None => rust_colony(case),
            };
            assert_eq!(got, want, "{name} layout {case:?}");
            cases += 1;
        }
    }

    for planet_i in [0usize, 1] {
        for case in moon_cases() {
            let (want, want_rng) =
                harness.c_moons(unsafe { &generateSyreenFunctions }, case, planet_i);
            let mut rng = RandomContext::with_seed(case.2);
            let mut moons: Vec<PLANET_DESC> = vec![unsafe { std::mem::zeroed() }; 4];
            syreen::generate_moons(
                &mut rng,
                case.0,
                star_color((case.1 << 5) | 0b101),
                case.3,
                &mut moons,
                Body::Planet(planet_i as u8),
            );
            assert_eq!(
                (moons.iter().map(desc_of).collect::<Vec<_>>(), rng.seed()),
                (want, want_rng),
                "syreen moons planet {planet_i} {case:?}"
            );
            cases += 1;
        }
    }

    type EnergyFn = fn(&mut RandomContext, &SYSTEM_INFO, Body, u16, &mut NODE_INFO) -> u16;
    let energy: [(&str, &GenerateFunctions, EnergyFn, Vec<Body>); 3] = [
        (
            "yehat",
            unsafe { &generateYehatFunctions },
            yehat::generate_energy,
            vec![
                Body::Planet(0),
                Body::Planet(1),
                Body::Moon { planet: 0, moon: 0 },
            ],
        ),
        (
            "vault",
            unsafe { &generateVaultFunctions },
            vault::generate_energy,
            vec![
                Body::Planet(0),
                Body::Moon { planet: 0, moon: 0 },
                Body::Moon { planet: 0, moon: 1 },
                Body::Moon { planet: 1, moon: 0 },
            ],
        ),
        (
            "wreck",
            unsafe { &generateWreckFunctions },
            wreck::generate_energy,
            vec![
                Body::Planet(6),
                Body::Planet(0),
                Body::Moon { planet: 6, moon: 0 },
            ],
        ),
    ];
    for (name, c_table, hook, bodies) in energy {
        for body in bodies {
            for seed in (1..=40u32).map(|s| s * 104_729) {
                let want = harness.c_energy(c_table, body, seed);
                let mut info: SYSTEM_INFO = unsafe { std::mem::zeroed() };
                info.PlanetInfo.ScanSeed = scan_seeds(seed);
                let mut got = String::new();
                let mut rng = RandomContext::with_seed(seed);
                let mut node = fresh_node();
                let count = hook(&mut rng, &info, body, !0, &mut node);
                let _ = write!(got, "{count}@{}[", rng.seed());
                for which in 0..count.min(32) {
                    rng.reseed(seed);
                    let mut node = fresh_node();
                    let ret = hook(&mut rng, &info, body, which, &mut node);
                    write_node(&mut got, ret, &node);
                    let _ = write!(got, "@{};", rng.seed());
                }
                got.push(']');
                assert_eq!(got, want, "{name} energy {body:?} seed {seed}");
                cases += 1;
            }
        }
    }
    println!("special generators: {cases} parity cases");
}
