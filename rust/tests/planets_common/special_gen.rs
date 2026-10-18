//! The small per-system generators (`genyeh.c` ... `gensly.c`) over a fixed
//! input domain, rendered one text line per case. `planets_fixtures` renders
//! the Rust port; `planets_parity` renders the exported tables and the C
//! originals' captured output must equal both.

use std::fmt::Write as _;

use super::default_gen::{layout_cases, moon_cases};
use uqm_rust::c_bindings::global_state::PLANET_DESC;
use uqm_rust::planets::generate::special::Body;

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
///
/// `chmmr0` and `chmmr1` are the Chmmr system with `CHMMR_UNLEASHED` clear and
/// set.
pub const LAYOUT_SYSTEMS: [&str; 9] = [
    "yehat", "colony", "rainbow", "syreen", "trap", "sly", "ilw", "chmmr0", "chmmr1",
];

/// The systems whose `generateEnergy` is overridden, with the bodies probed.
pub fn energy_systems() -> Vec<(&'static str, Vec<Body>)> {
    vec![
        (
            "ilw",
            vec![
                Body::Planet(0),
                Body::Planet(1),
                Body::Moon { planet: 0, moon: 0 },
            ],
        ),
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
    /// `generateMoons` of `system` (Syreen or Chmmr) for planet `planet_i`:
    /// moon table, RNG position.
    fn moons(&mut self, system: &str, planet_i: usize, case: (u8, u8, u32, u8))
        -> (Vec<Desc>, u32);
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
    for system in ["syreen", "chmmr"] {
        for planet_i in [0usize, 1] {
            for case in moon_cases() {
                let _ = writeln!(
                    out,
                    "moons {system} {planet_i} {case:?} {:?}",
                    ops.moons(system, planet_i, case)
                );
            }
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
