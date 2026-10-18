//! The small per-system generators (`genyeh.c` ... `gensly.c`) over a fixed
//! input domain, rendered one text line per case. `planets_fixtures` renders
//! the Rust port; `planets_parity` renders the exported tables and the C
//! originals' captured output must equal both.

use std::fmt::Write as _;

use super::default_gen::{layout_cases, moon_cases};
use super::{fresh_node, scan_seeds, write_node};
use uqm_rust::c_bindings::global_state::{PlanetFrame, NODE_INFO, PLANET_DESC, SYSTEM_INFO};
use uqm_rust::math::RandomContext;
use uqm_rust::planets::consts::star_color;
use uqm_rust::planets::generate::special::Body;
use uqm_rust::planets::generate::{colony, rainbow, slylandro, syreen, trap, vault, wreck, yehat};

pub type Desc = (u8, i16, u32, u8, i16, i16);

pub fn desc_of(d: &PLANET_DESC) -> Desc {
    (
        d.data_index,
        d.radius,
        d.rand_seed,
        d.NumPlanets,
        d.location.x,
        d.location.y,
    )
}

/// The systems whose `generatePlanets` is overridden.
pub const LAYOUT_SYSTEMS: [&str; 6] = ["yehat", "colony", "rainbow", "syreen", "trap", "sly"];

/// The systems whose `generateEnergy` is overridden, with the bodies probed.
pub fn energy_systems() -> Vec<(&'static str, Vec<Body>)> {
    vec![
        (
            "yehat",
            vec![
                Body::Planet(0),
                Body::Planet(1),
                Body::Moon { planet: 0, moon: 0 },
            ],
        ),
        (
            "vault",
            vec![
                Body::Planet(0),
                Body::Moon { planet: 0, moon: 0 },
                Body::Moon { planet: 0, moon: 1 },
                Body::Moon { planet: 1, moon: 0 },
            ],
        ),
        (
            "wreck",
            vec![
                Body::Planet(6),
                Body::Planet(0),
                Body::Moon { planet: 6, moon: 0 },
            ],
        ),
    ]
}

/// Where the outputs of one implementation come from.
pub trait SpecialOps {
    /// `generatePlanets` of `system`: planet table, drawn count, RNG position.
    fn layout(&mut self, system: &str, case: (u8, u8, u32)) -> (Vec<Desc>, u8, u32);
    /// Syreen `generateMoons` for planet `planet_i`: moon table, RNG position.
    fn moons(&mut self, planet_i: usize, case: (u8, u8, u32, u8)) -> (Vec<Desc>, u32);
    /// `generateEnergy` of `system` on `body`: the node list as text.
    fn energy(&mut self, system: &str, body: Body, seed: u32) -> String;
}

pub fn energy_seeds() -> impl Iterator<Item = u32> {
    (1..=40u32).map(|s| s * 104_729)
}

/// One line per case, in a fixed order.
pub fn render(ops: &mut impl SpecialOps) -> String {
    let mut out = String::new();
    for system in LAYOUT_SYSTEMS {
        for case in layout_cases() {
            let _ = writeln!(
                out,
                "layout {system} {case:?} {:?}",
                ops.layout(system, case)
            );
        }
    }
    for planet_i in [0usize, 1] {
        for case in moon_cases() {
            let _ = writeln!(
                out,
                "moons syreen {planet_i} {case:?} {:?}",
                ops.moons(planet_i, case)
            );
        }
    }
    for (system, bodies) in energy_systems() {
        for body in bodies {
            for seed in energy_seeds() {
                let _ = writeln!(
                    out,
                    "energy {system} {body:?} {seed} {}",
                    ops.energy(system, body, seed)
                );
            }
        }
    }
    out
}

/// The Rust port called directly.
pub struct RustSpecial<'a> {
    pub table: &'a [PlanetFrame],
}

impl SpecialOps for RustSpecial<'_> {
    fn layout(&mut self, system: &str, case: (u8, u8, u32)) -> (Vec<Desc>, u8, u32) {
        let mut rng = RandomContext::with_seed(case.2);
        let mut planets: Vec<PLANET_DESC> = vec![unsafe { std::mem::zeroed() }; 16];
        let drawn = match system {
            "colony" => colony::generate_planets(
                &mut rng,
                case.0,
                star_color((case.1 << 5) | 0b101),
                &mut planets,
            ),
            other => {
                let hook = match other {
                    "yehat" => yehat::generate_planets,
                    "rainbow" => rainbow::generate_planets,
                    "syreen" => syreen::generate_planets,
                    "trap" => trap::generate_planets,
                    "sly" => slylandro::generate_planets,
                    _ => panic!("no such system {other}"),
                };
                hook(&mut rng, self.table, case.0, case.1, &mut planets)
            }
        };
        (planets.iter().map(desc_of).collect(), drawn, rng.seed())
    }

    fn moons(&mut self, planet_i: usize, case: (u8, u8, u32, u8)) -> (Vec<Desc>, u32) {
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
        (moons.iter().map(desc_of).collect(), rng.seed())
    }

    fn energy(&mut self, system: &str, body: Body, seed: u32) -> String {
        type EnergyFn = fn(&mut RandomContext, &SYSTEM_INFO, Body, u16, &mut NODE_INFO) -> u16;
        let hook: EnergyFn = match system {
            "yehat" => yehat::generate_energy,
            "vault" => vault::generate_energy,
            "wreck" => wreck::generate_energy,
            other => panic!("no such system {other}"),
        };
        let mut info: SYSTEM_INFO = unsafe { std::mem::zeroed() };
        info.PlanetInfo.ScanSeed = scan_seeds(seed);
        let mut out = String::new();
        let mut rng = RandomContext::with_seed(seed);
        let mut node = fresh_node();
        let count = hook(&mut rng, &info, body, !0, &mut node);
        let _ = write!(out, "{count}@{}[", rng.seed());
        for which in 0..count.min(32) {
            rng.reseed(seed);
            let mut node = fresh_node();
            let ret = hook(&mut rng, &info, body, which, &mut node);
            write_node(&mut out, ret, &node);
            let _ = write!(out, "@{};", rng.seed());
        }
        out.push(']');
        out
    }
}
