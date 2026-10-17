//! Input domain and record format for the `DeltaTopography` parity tests.
//!
//! The Rust port and the linked C original run the same cases; each case
//! becomes one line holding an FNV-1a hash of the whole depth map and the RNG
//! state left behind, so a mismatch names the exact input.

use std::fmt::Write as _;

use uqm_rust::c_bindings::global_state::PlanetFrame;
use uqm_rust::planets::consts::{MAP_HEIGHT, MAP_WIDTH};
use uqm_rust::planets::topography::delta_topography;

/// Rows of the narrow band `MakeGasGiant` perturbs, and its iteration count.
const BAND_HEIGHT: i16 = 5;
const BAND_ITERATIONS: u16 = 50;

#[derive(Clone, Copy, Debug)]
pub enum Fill {
    /// `memset (lpTopoData, 0, ...)`, as the planet generator does.
    Zero,
    /// Every cell starts at this value, so additions hit the clamp.
    Level(i8),
}

#[derive(Clone, Debug)]
pub struct TopoCase {
    pub label: String,
    pub seed: u32,
    pub iterations: u16,
    pub width: i16,
    pub height: i16,
    pub depth_delta: i16,
    pub fill: Fill,
}

impl TopoCase {
    pub fn cells(&self) -> usize {
        usize::try_from(self.width * self.height).expect("positive map")
    }

    pub fn initial_map(&self) -> Vec<i8> {
        let level = match self.fill {
            Fill::Zero => 0,
            Fill::Level(level) => level,
        };
        vec![level; self.cells()]
    }
}

/// A topography run: fills `map` and returns the seed the RNG ended on.
pub trait Topography {
    fn run(&mut self, case: &TopoCase, map: &mut [i8]) -> u32;
}

pub fn rust_run(case: &TopoCase, map: &mut [i8]) -> u32 {
    let mut rng = uqm_rust::math::RandomContext::with_seed(case.seed);
    delta_topography(
        &mut rng,
        case.iterations,
        map,
        case.width,
        case.height,
        case.depth_delta,
    );
    rng.reseed(1)
}

pub fn fnv_bytes(bytes: &[i8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for byte in bytes {
        hash ^= u64::from(*byte as u8);
        hash = hash.wrapping_mul(0x0000_0100_0000_01B3);
    }
    hash
}

const FRAME_SEEDS: [u32; 2] = [1, 0xdead_beef];
const COMBO_SEEDS: [u32; 3] = [2, 0x1234_5678, 0x7fff_ffff];
const BAND_SEEDS: [u32; 4] = [1, 0, 0x1234_5678, 0xdead_beef];
const BAND_DEPTHS: [i16; 4] = [1, 2, 5, 20];
const SATURATED_LEVELS: [i8; 4] = [127, -128, 120, -120];

fn map_dimensions() -> (i16, i16) {
    (MAP_WIDTH as i16, MAP_HEIGHT as i16)
}

/// Every case: each planet frame's own fault parameters over the whole map,
/// every distinct fault parameter pair on more seeds, the gas giant band, and
/// saturated starting maps that exercise the signed-byte clamp.
pub fn cases(table: &[PlanetFrame]) -> Vec<TopoCase> {
    let (width, height) = map_dimensions();
    let mut cases = Vec::new();
    let mut combos: Vec<(u16, i16)> = Vec::new();

    for (index, frame) in table.iter().enumerate() {
        let params = (frame.num_faults, frame.fault_depth);
        if !combos.contains(&params) {
            combos.push(params);
        }
        for seed in FRAME_SEEDS {
            cases.push(TopoCase {
                label: format!("frame{index}"),
                seed,
                iterations: params.0,
                width,
                height,
                depth_delta: params.1,
                fill: Fill::Zero,
            });
        }
    }
    for (faults, depth) in combos {
        for seed in COMBO_SEEDS {
            cases.push(TopoCase {
                label: format!("combo{faults}x{depth}"),
                seed,
                iterations: faults,
                width,
                height,
                depth_delta: depth,
                fill: Fill::Zero,
            });
        }
    }
    for depth in BAND_DEPTHS {
        for seed in BAND_SEEDS {
            cases.push(TopoCase {
                label: "band".to_owned(),
                seed,
                iterations: BAND_ITERATIONS,
                width,
                height: BAND_HEIGHT,
                depth_delta: depth,
                fill: Fill::Zero,
            });
        }
    }
    for level in SATURATED_LEVELS {
        for seed in BAND_SEEDS {
            cases.push(TopoCase {
                label: "saturated".to_owned(),
                seed,
                iterations: 120,
                width,
                height,
                depth_delta: 9,
                fill: Fill::Level(level),
            });
        }
    }
    cases
}

pub fn case_line(runner: &mut impl Topography, case: &TopoCase) -> String {
    let mut map = case.initial_map();
    let rng_seed = runner.run(case, &mut map);
    format!(
        "t {} seed{} n{} {}x{} d{} {:?} | {:016x} rng{}",
        case.label,
        case.seed,
        case.iterations,
        case.width,
        case.height,
        case.depth_delta,
        case.fill,
        fnv_bytes(&map),
        rng_seed
    )
}

/// Full maps for the narrow band, small enough to store whole.
pub fn buffer_lines(runner: &mut impl Topography) -> Vec<String> {
    let (width, _) = map_dimensions();
    [(1u32, 2i16), (0xdead_beef, 5)]
        .into_iter()
        .map(|(seed, depth_delta)| {
            let case = TopoCase {
                label: "buffer".to_owned(),
                seed,
                iterations: BAND_ITERATIONS,
                width,
                height: BAND_HEIGHT,
                depth_delta,
                fill: Fill::Zero,
            };
            let mut map = case.initial_map();
            runner.run(&case, &mut map);
            let mut line = format!("b seed{seed} d{depth_delta} ");
            for cell in &map {
                let _ = write!(line, "{:02x}", *cell as u8);
            }
            line
        })
        .collect()
}

pub fn render(runner: &mut impl Topography, table: &[PlanetFrame]) -> String {
    let mut text = String::new();
    for case in cases(table) {
        text.push_str(&case_line(runner, &case));
        text.push('\n');
    }
    for line in buffer_lines(runner) {
        text.push_str(&line);
        text.push('\n');
    }
    text
}

pub struct RustTopography;

impl Topography for RustTopography {
    fn run(&mut self, case: &TopoCase, map: &mut [i8]) -> u32 {
        rust_run(case, map)
    }
}
