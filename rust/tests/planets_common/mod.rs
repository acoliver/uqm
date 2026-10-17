//! Shared input domain and record format for the planet-physics parity tests.
//!
//! The same cases run against the Rust port directly and through the exported
//! C symbols (the `linked_c_archive` build). Each case becomes one text line,
//! so a mismatch names the exact input and field.

use std::fmt::Write as _;

pub mod topography;

use uqm_rust::c_bindings::global_state::{PlanetFrame, NODE_INFO, PLANET_DESC, SYSTEM_INFO};

pub const NODE_SENTINEL_DENSITY: u16 = 0xBEEF;
pub const NODE_SENTINEL_TYPE: u16 = 0xCAFE;
pub const PRESET_LIFE: [i8; 4] = [3, 7, 11, -1];

/// One planetary analysis plus the node lists derived from it.
#[derive(Clone, Copy, Debug)]
pub struct WorldCase {
    pub star_color: u8,
    pub star_size: u8,
    pub data_index: u8,
    pub rand_seed: u32,
    pub desc_radius: i16,
    pub parent_radius: Option<i16>,
    pub is_orbit_entry: bool,
}

/// One `FillOrbits` call.
#[derive(Clone, Copy, Debug)]
pub struct OrbitCase {
    pub star_color: u8,
    pub star_size: u8,
    pub seed: u32,
    pub mode: OrbitMode,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OrbitMode {
    /// Planets, count drawn from the RNG.
    DrawnPlanets,
    /// Planets, five of them.
    FivePlanets,
    /// Three moons.
    ThreeMoons,
    /// Planets with the types preset by the caller.
    PresetPlanets,
}

impl OrbitMode {
    pub const ALL: [OrbitMode; 4] = [
        OrbitMode::DrawnPlanets,
        OrbitMode::FivePlanets,
        OrbitMode::ThreeMoons,
        OrbitMode::PresetPlanets,
    ];

    pub fn num_planets(self) -> u8 {
        match self {
            OrbitMode::DrawnPlanets => !0,
            OrbitMode::FivePlanets => 5,
            OrbitMode::ThreeMoons => 3,
            OrbitMode::PresetPlanets => 4,
        }
    }

    pub fn generating_moons(self) -> bool {
        self == OrbitMode::ThreeMoons
    }

    pub fn types_defined(self) -> bool {
        self == OrbitMode::PresetPlanets
    }

    pub fn slots(self) -> usize {
        if self.generating_moons() {
            4
        } else {
            16
        }
    }
}

/// The world-generation entry points, implemented by the Rust port and by the
/// linked C archive.
pub trait Generators {
    fn analyze(&mut self, case: &WorldCase) -> SYSTEM_INFO;
    fn minerals(&mut self, info: &SYSTEM_INFO, which: u16, node: &mut NODE_INFO) -> u16;
    fn life(&mut self, info: &SYSTEM_INFO, which: u16, node: &mut NODE_INFO) -> u16;
    fn preset_life(&mut self, info: &SYSTEM_INFO, which: u16, node: &mut NODE_INFO) -> u16;
    fn random_nodes(&mut self, info: &SYSTEM_INFO, which: u16, node: &mut NODE_INFO) -> u16;
    fn fill_orbits(&mut self, case: &OrbitCase, descs: &mut [PLANET_DESC]) -> Option<u8>;
}

pub fn star_type_byte(case: &WorldCase) -> u8 {
    (case.star_color << 5) | (case.star_size << 3) | 0b101
}

pub fn fresh_node() -> NODE_INFO {
    let mut node: NODE_INFO = unsafe { std::mem::zeroed() };
    node.loc_pt.x = -7;
    node.loc_pt.y = -7;
    node.density = NODE_SENTINEL_DENSITY;
    node.type_ = NODE_SENTINEL_TYPE;
    node
}

pub fn scan_seeds(rand_seed: u32) -> [u32; 3] {
    [
        rand_seed ^ 0x5A5A_5A5A,
        rand_seed.rotate_left(7).wrapping_add(12345),
        rand_seed.wrapping_mul(3).wrapping_add(1),
    ]
}

fn write_node(out: &mut String, ret: u16, node: &NODE_INFO) {
    let _ = write!(
        out,
        "{ret}:{},{},{},{};",
        node.loc_pt.x, node.loc_pt.y, node.density, node.type_
    );
}

/// Run one world case and render every observable output as a line.
pub fn world_line<G: Generators>(
    generators: &mut G,
    case: &WorldCase,
    table_base: usize,
) -> String {
    let mut info = generators.analyze(case);
    info.PlanetInfo.ScanSeed = scan_seeds(case.rand_seed);

    let planet = &info.PlanetInfo;
    let frame_index =
        (planet.PlanDataPtr as usize).wrapping_sub(table_base) / std::mem::size_of::<PlanetFrame>();
    let mut out = format!(
        "w c{} s{} t{} seed{} r{} p{:?} o{} | {} {} {} | {} {} {} {} {} {} {} {} {} {} {} f{} |",
        case.star_color,
        case.star_size,
        case.data_index,
        case.rand_seed,
        case.desc_radius,
        case.parent_radius,
        u8::from(case.is_orbit_entry),
        info.StarSize,
        info.StarIntensity,
        info.StarEnergy,
        planet.AxialTilt,
        planet.Tectonics,
        planet.Weather,
        planet.PlanetDensity,
        planet.PlanetRadius,
        planet.SurfaceGravity,
        planet.SurfaceTemperature,
        planet.RotationPeriod,
        planet.AtmoDensity,
        planet.LifeChance,
        planet.PlanetToSunDist,
        frame_index,
    );

    let mut node = fresh_node();
    let count = generators.minerals(&info, !0, &mut node);
    let _ = write!(out, " m{count}[");
    for which in 0..count.min(32) {
        let mut node = fresh_node();
        let ret = generators.minerals(&info, which, &mut node);
        write_node(&mut out, ret, &node);
    }
    let _ = write!(out, "] l");
    let mut node = fresh_node();
    let count = generators.life(&info, !0, &mut node);
    let _ = write!(out, "{count}[");
    for which in 0..count.min(32) {
        let mut node = fresh_node();
        let ret = generators.life(&info, which, &mut node);
        write_node(&mut out, ret, &node);
    }
    let _ = write!(out, "] p[");
    for which in [0u16, 1, 2, 3, !0] {
        let mut node = fresh_node();
        let ret = generators.preset_life(&info, which, &mut node);
        write_node(&mut out, ret, &node);
    }
    let _ = write!(out, "] e[");
    for which in [0u16, 1, 3, 4, 5, !0] {
        let mut node = fresh_node();
        let ret = generators.random_nodes(&info, which, &mut node);
        write_node(&mut out, ret, &node);
    }
    out.push(']');
    out
}

pub fn initial_descs(mode: OrbitMode) -> Vec<PLANET_DESC> {
    (0..mode.slots())
        .map(|slot| {
            let mut desc: PLANET_DESC = unsafe { std::mem::zeroed() };
            desc.rand_seed = 0x1111_0000 + slot as u32;
            desc.data_index = if mode.types_defined() {
                [5, 50, 54, 58][slot % 4]
            } else {
                0xEE
            };
            desc.NumPlanets = 0xAA;
            desc.radius = -1;
            desc.location.x = -2;
            desc.location.y = -3;
            desc.temp_color.r = 1;
            desc.temp_color.g = 2;
            desc.temp_color.b = 3;
            desc.temp_color.a = 4;
            desc.NextIndex = 0x7777;
            desc.image.origin.x = 5;
            desc.image.origin.y = 6;
            desc
        })
        .collect()
}

fn fnv(hash: &mut u64, text: &str) {
    for byte in text.bytes() {
        *hash ^= u64::from(byte);
        *hash = hash.wrapping_mul(0x0000_0100_0000_01B3);
    }
}

fn describe_desc(desc: &PLANET_DESC) -> String {
    format!(
        "{},{},{},{},{},{},{},{},{},{},{},{},{}",
        desc.rand_seed,
        desc.data_index,
        desc.NumPlanets,
        desc.radius,
        desc.location.x,
        desc.location.y,
        desc.temp_color.r,
        desc.temp_color.g,
        desc.temp_color.b,
        desc.temp_color.a,
        desc.NextIndex,
        desc.image.origin.x,
        desc.image.origin.y
    )
}

pub fn orbit_line<G: Generators>(generators: &mut G, case: &OrbitCase) -> String {
    let mut descs = initial_descs(case.mode);
    let generated = generators.fill_orbits(case, &mut descs);
    let used = match generated {
        Some(count) => usize::from(count),
        None => usize::from(case.mode.num_planets()),
    };

    let mut out = format!(
        "o c{} s{} seed{} {:?} n{:?} |",
        case.star_color, case.star_size, case.seed, case.mode, generated
    );
    let mut tail_hash = 0xcbf2_9ce4_8422_2325u64;
    for (slot, desc) in descs.iter().enumerate() {
        if slot < used {
            let _ = write!(out, " {};", describe_desc(desc));
        } else {
            fnv(&mut tail_hash, &describe_desc(desc));
        }
    }
    let _ = write!(out, " tail{tail_hash:x}");
    out
}

// ---------------------------------------------------------------------------
// Input domain
// ---------------------------------------------------------------------------

pub const NUMBER_OF_PLANET_TYPES: u8 = 59;
const SEEDS: [u32; 6] = [1, 2, 0x1234_5678, 0x7fff_ffff, 0xdead_beef, 0];
const SITES: [(i16, Option<i16>, bool); 5] = [
    (300, None, false),
    (3000, None, true),
    (100, Some(512), true),
    (100, Some(7000), false),
    (100, Some(256), true),
];

fn data_indices() -> Vec<u8> {
    let mut indices: Vec<u8> = (0..NUMBER_OF_PLANET_TYPES).collect();
    indices.extend([0x80, 0x80 | 5, 0x80 | 30, 0x80 | 50]);
    indices
}

/// Every analysis case: all star colours and sizes, all planet types, seeds and
/// orbit sites. `golden` selects the subset stored in the fixture.
pub fn world_cases(golden: bool) -> Vec<WorldCase> {
    let mut cases = Vec::new();
    for star_color in 0..6u8 {
        for star_size in 0..3u8 {
            let is_golden_star = matches!((star_color, star_size), (5, 0) | (0, 2) | (3, 1));
            if golden && !is_golden_star {
                continue;
            }
            for data_index in data_indices() {
                for (seed_slot, rand_seed) in SEEDS.iter().enumerate() {
                    if golden && seed_slot > 1 {
                        continue;
                    }
                    for (site_slot, (desc_radius, parent_radius, is_orbit_entry)) in
                        SITES.iter().enumerate()
                    {
                        if golden && site_slot > 1 {
                            continue;
                        }
                        cases.push(WorldCase {
                            star_color,
                            star_size,
                            data_index,
                            rand_seed: *rand_seed,
                            desc_radius: *desc_radius,
                            parent_radius: *parent_radius,
                            is_orbit_entry: *is_orbit_entry,
                        });
                    }
                }
            }
        }
    }
    cases
}

pub fn orbit_cases(golden: bool) -> Vec<OrbitCase> {
    let seeds: u32 = if golden { 8 } else { 60 };
    let mut cases = Vec::new();
    for star_color in 0..6u8 {
        for star_size in 0..3u8 {
            for seed in 0..seeds {
                for mode in OrbitMode::ALL {
                    // C relocates a planet until its orbit is 2+ steps from every
                    // earlier one. A supergiant's gas-giant range (33..124) holds
                    // six such orbits, so up to nine drawn planets can leave C
                    // looping forever on some streams (colour 2, seed 419708).
                    if mode == OrbitMode::DrawnPlanets && star_size == 2 {
                        continue;
                    }
                    cases.push(OrbitCase {
                        star_color,
                        star_size,
                        seed: seed * 7919 + 1,
                        mode,
                    });
                }
            }
        }
    }
    cases
}

pub struct Records {
    pub golden_lines: Vec<String>,
    pub digest: u64,
    pub count: usize,
}

/// Run `lines_for` over the full domain, keeping the golden subset and an
/// FNV-1a digest of every line.
pub fn collect<T>(full: Vec<T>, golden: Vec<T>, mut line_for: impl FnMut(&T) -> String) -> Records {
    let mut digest = 0xcbf2_9ce4_8422_2325u64;
    for case in &full {
        fnv(&mut digest, &line_for(case));
    }
    let golden_lines = golden.iter().map(&mut line_for).collect();
    Records {
        golden_lines,
        digest,
        count: full.len(),
    }
}

pub fn render_fixture(records: &Records) -> String {
    let mut text = format!("# digest {:016x} count {}\n", records.digest, records.count);
    for line in &records.golden_lines {
        text.push_str(line);
        text.push('\n');
    }
    text
}
