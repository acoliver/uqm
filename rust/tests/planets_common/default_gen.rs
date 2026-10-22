//! Input domain and record format for the default system generator
//! (`GenerateDefault_*`), shared by the Rust port and the linked C original.

use std::fmt::Write as _;

use super::{fresh_node, scan_seeds, write_node, WorldCase};
use uqm_rust::c_bindings::global_state::{NODE_INFO, PLANET_DESC, SYSTEM_INFO};

/// Node-producing hooks of the default generator.
#[derive(Clone, Copy, Debug)]
pub enum NodeKind {
    Minerals,
    Energy,
    Life,
    Artifact,
    Ruins,
}

pub const NODE_KINDS: [NodeKind; 5] = [
    NodeKind::Minerals,
    NodeKind::Energy,
    NodeKind::Life,
    NodeKind::Artifact,
    NodeKind::Ruins,
];

/// One world as the planet layout leaves it: `data_index radius rand_seed moons`.
pub type World = (u8, i16, u32, u8);

/// What a layout call leaves behind.
pub struct Layout {
    pub worlds: Vec<World>,
    /// The planet count `SunDesc[0].NumPlanets` ends with (planets only).
    pub drawn: u8,
    /// `RandomContext_GetSeed (SysGenRNG)` afterwards.
    pub rng: u32,
}

/// The hooks under test, implemented by the Rust port and by the C original.
pub trait DefaultOps {
    fn planets(&mut self, star_size: u8, star_color: u8, seed: u32) -> Layout;
    fn moons(&mut self, star_size: u8, star_color: u8, seed: u32, count: u8) -> Layout;
    /// Returns the node count and the RNG position afterwards.
    fn nodes(
        &mut self,
        kind: NodeKind,
        info: &SYSTEM_INFO,
        which: u16,
        node: &mut NODE_INFO,
    ) -> (u16, u32);
}

pub fn world_of(desc: &PLANET_DESC) -> World {
    (
        desc.data_index,
        desc.radius,
        desc.rand_seed,
        desc.NumPlanets,
    )
}

pub fn layout_cases() -> Vec<(u8, u8, u32)> {
    let mut cases = Vec::new();
    for star_color in 0..6u8 {
        for star_size in 0..3u8 {
            for seed in 1..=24u32 {
                cases.push((star_size, star_color, seed * 7919));
            }
        }
    }
    cases
}

pub fn moon_cases() -> Vec<(u8, u8, u32, u8)> {
    let mut cases = Vec::new();
    for (star_size, star_color, seed) in layout_cases() {
        for count in 1..=4u8 {
            if seed % 3 == 0 {
                cases.push((star_size, star_color, seed, count));
            }
        }
    }
    cases
}

fn layout_text(out: &mut String, layout: &Layout) {
    let _ = write!(out, "n{} rng{} [", layout.drawn, layout.rng);
    for (data_index, radius, seed, moons) in &layout.worlds {
        let _ = write!(out, "{data_index},{radius},{seed},{moons};");
    }
    out.push(']');
}

pub fn planets_line<O: DefaultOps>(ops: &mut O, case: &(u8, u8, u32)) -> String {
    let layout = ops.planets(case.0, case.1, case.2);
    let mut out = format!("P s{} c{} seed{} ", case.0, case.1, case.2);
    layout_text(&mut out, &layout);
    out
}

pub fn moons_line<O: DefaultOps>(ops: &mut O, case: &(u8, u8, u32, u8)) -> String {
    let layout = ops.moons(case.0, case.1, case.2, case.3);
    let mut out = format!("M s{} c{} seed{} k{} ", case.0, case.1, case.2, case.3);
    layout_text(&mut out, &layout);
    out
}

/// Every node hook over one analysed world: the count call, then each node.
pub fn nodes_line<O: DefaultOps>(ops: &mut O, case: &WorldCase, mut info: SYSTEM_INFO) -> String {
    info.PlanetInfo.ScanSeed = scan_seeds(case.rand_seed);
    let mut out = format!(
        "N c{} s{} t{} seed{} r{} p{:?} o{}",
        case.star_color,
        case.star_size,
        case.data_index,
        case.rand_seed,
        case.desc_radius,
        case.parent_radius,
        u8::from(case.is_orbit_entry)
    );
    for kind in NODE_KINDS {
        let mut node = fresh_node();
        let (count, rng) = ops.nodes(kind, &info, !0, &mut node);
        let _ = write!(out, " {kind:?}{count}@{rng}[");
        for which in 0..count.min(32) {
            let mut node = fresh_node();
            let (ret, rng) = ops.nodes(kind, &info, which, &mut node);
            write_node(&mut out, ret, &node);
            let _ = write!(out, "@{rng};");
        }
        out.push(']');
    }
    out
}
