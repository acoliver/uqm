//! The Rust planet port against checked-in outputs of the C original.
//!
//! These fixtures were captured from the C code before `calc.c`, `surface.c`
//! and `orbits.c` were deleted, so this test needs no C and remains the
//! permanent parity proof. `planets_parity` replays them through the exported
//! C symbols.

mod planets_common;

use planets_common::default_gen::{
    self, layout_cases, moon_cases, moons_line, nodes_line, planets_line, DefaultOps, Layout,
    NodeKind,
};
use planets_common::topography::{TopoCase, Topography};
use planets_common::{
    collect, orbit_cases, orbit_line, render_fixture, star_type_byte, world_cases, world_line,
    Generators, OrbitCase, Records, WorldCase, PRESET_LIFE,
};
use uqm_rust::c_bindings::global_state::{
    PlanetFrame, ELEMENT_ENTRY, NODE_INFO, PLANET_DESC, SYSTEM_INFO,
};
use uqm_rust::math::RandomContext;
use uqm_rust::planets::topography::delta_topography;

const PLAN_DATA: &str = include_str!("fixtures/planets/plan_data.txt");
const WORLDS: &str = include_str!("fixtures/planets/worlds.txt");
const ORBITS: &str = include_str!("fixtures/planets/orbits.txt");
const TOPOGRAPHY: &str = include_str!("fixtures/planets/topography.txt");
const DEFAULT_LAYOUT: &str = include_str!("fixtures/planets/default_layout.txt");
const DEFAULT_NODES: &str = include_str!("fixtures/planets/default_nodes.txt");

/// Parses the golden planet-frame table captured from the C original.
fn parse_plan_data(text: &str) -> Vec<PlanetFrame> {
    text.lines()
        .map(|line| {
            let fields: Vec<&str> = line.split(' ').collect();
            let mut elements = [ELEMENT_ENTRY {
                ElementType: 0,
                Density: 0,
            }; 8];
            for (slot, pair) in fields[3..11].iter().enumerate() {
                let (kind, density) = pair.split_once(':').expect("element pair");
                elements[slot] = ELEMENT_ENTRY {
                    ElementType: kind.parse().expect("element type"),
                    Density: density.parse().expect("element density"),
                };
            }
            PlanetFrame {
                Type: fields[0].parse().expect("type"),
                BaseTectonics: fields[1].parse().expect("tectonics"),
                AtmoAndDensity: fields[2].parse().expect("atmo"),
                UsefulElements: elements,
                CMapInstance: std::ptr::null(),
                XlatTabInstance: std::ptr::null(),
                num_faults: fields[11].parse().expect("faults"),
                fault_depth: fields[12].parse().expect("depth"),
                num_blemishes: fields[13].parse().expect("blemishes"),
                base_elevation: fields[14].parse().expect("elevation"),
            }
        })
        .collect()
}

#[test]
fn planetary_analysis_and_nodes_match_the_c_fixture() {
    let table = parse_plan_data(PLAN_DATA);
    assert_eq!(table.len(), 59);
    let records = world_records(&table);
    assert_eq!(render_fixture(&records), WORLDS);
}

#[test]
fn fault_line_topography_matches_the_c_fixture() {
    let table = parse_plan_data(PLAN_DATA);
    let text = planets_common::topography::render(&mut RustTopography, &table);
    assert_eq!(text, TOPOGRAPHY);
}

/// `delta_topography` called directly over a seeded private stream.
struct RustTopography;

impl Topography for RustTopography {
    fn run(&mut self, case: &TopoCase, map: &mut [i8]) -> u32 {
        let mut rng = RandomContext::with_seed(case.seed);
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
}

#[test]
fn default_generator_layout_matches_the_c_fixture() {
    let table = parse_plan_data(PLAN_DATA);
    let mut ops = RustDefault::new(&table);
    let planets = collect(layout_cases(), layout_cases(), |case| {
        planets_line(&mut ops, case)
    });
    let moons = collect(moon_cases(), moon_cases(), |case| {
        moons_line(&mut ops, case)
    });
    let text = format!("{}{}", render_fixture(&planets), render_fixture(&moons));
    check_default("default_layout.txt", &text, DEFAULT_LAYOUT);
}

#[test]
fn default_generator_nodes_match_the_c_fixture() {
    let table = parse_plan_data(PLAN_DATA);
    let mut generators = RustGenerators::new(&table);
    let mut ops = RustDefault::new(&table);
    let records = collect(world_cases(false), world_cases(true), |case| {
        nodes_line(&mut ops, case, generators.analyze(case))
    });
    check_default(
        "default_nodes.txt",
        &render_fixture(&records),
        DEFAULT_NODES,
    );
}

/// With `UQM_BLESS_DEFAULT_FIXTURES` set, write the fixture instead of
/// checking it; `planets_parity` then holds the Rust output to the C original.
fn check_default(name: &str, text: &str, stored: &str) {
    if std::env::var_os("UQM_BLESS_DEFAULT_FIXTURES").is_some() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/planets")
            .join(name);
        std::fs::write(path, text).expect("write fixture");
        return;
    }
    assert_eq!(text, stored);
}

/// `uqm_rust::planets::generate::default` called directly.
struct RustDefault<'a> {
    table: &'a [PlanetFrame],
    rng: RandomContext,
}

impl<'a> RustDefault<'a> {
    fn new(table: &'a [PlanetFrame]) -> Self {
        Self {
            table,
            rng: RandomContext::with_seed(1),
        }
    }

    fn blank_descs(count: usize) -> Vec<PLANET_DESC> {
        (0..count).map(|_| unsafe { std::mem::zeroed() }).collect()
    }
}

impl DefaultOps for RustDefault<'_> {
    fn planets(&mut self, star_size: u8, star_color: u8, seed: u32) -> Layout {
        use uqm_rust::planets::generate::default::generate_planets;

        let mut descs = Self::blank_descs(16);
        self.rng.reseed(seed);
        let drawn = generate_planets(&mut self.rng, self.table, star_size, star_color, &mut descs);
        Layout {
            worlds: descs[..usize::from(drawn)]
                .iter()
                .map(default_gen::world_of)
                .collect(),
            drawn,
            rng: self.rng.seed(),
        }
    }

    fn moons(&mut self, star_size: u8, star_color: u8, seed: u32, count: u8) -> Layout {
        use uqm_rust::planets::generate::default::generate_moons;

        let mut descs = Self::blank_descs(usize::from(count));
        self.rng.reseed(seed);
        generate_moons(&mut self.rng, star_size, star_color, count, &mut descs);
        Layout {
            worlds: descs.iter().map(default_gen::world_of).collect(),
            drawn: 0,
            rng: self.rng.seed(),
        }
    }

    fn nodes(
        &mut self,
        kind: NodeKind,
        info: &SYSTEM_INFO,
        which: u16,
        node: &mut NODE_INFO,
    ) -> (u16, u32) {
        use uqm_rust::planets::generate::default as gen;

        let rng = &mut self.rng;
        let count = match kind {
            NodeKind::Minerals => gen::generate_minerals(rng, info, which, node),
            NodeKind::Energy => gen::generate_energy(),
            NodeKind::Life => gen::generate_life(rng, info, which, node),
            NodeKind::Artifact => gen::generate_artifact(rng, info, which, node),
            NodeKind::Ruins => gen::generate_ruins(rng, info, which, node),
        };
        (count, rng.seed())
    }
}

#[test]
fn orbit_layout_matches_the_c_fixture() {
    let records = orbit_records();
    assert_eq!(render_fixture(&records), ORBITS);
}

// The Rust port called directly, with no C state.

struct RustGenerators<'a> {
    table: &'a [PlanetFrame],
    rng: uqm_rust::math::RandomContext,
}

impl<'a> RustGenerators<'a> {
    fn new(table: &'a [PlanetFrame]) -> Self {
        Self {
            table,
            rng: uqm_rust::math::RandomContext::with_seed(1),
        }
    }
}

impl Generators for RustGenerators<'_> {
    fn analyze(&mut self, case: &WorldCase) -> SYSTEM_INFO {
        use uqm_rust::planets::calc::{do_planetary_analysis, AnalysisSite, StarDescription};

        let mut info: SYSTEM_INFO = unsafe { std::mem::zeroed() };
        let mut desc: PLANET_DESC = unsafe { std::mem::zeroed() };
        desc.rand_seed = case.rand_seed;
        desc.data_index = case.data_index;
        desc.radius = case.desc_radius;
        do_planetary_analysis(
            &mut self.rng,
            self.table,
            StarDescription {
                size: case.star_size,
                star_type: star_type_byte(case),
            },
            AnalysisSite {
                parent_radius: case.parent_radius,
                is_orbit_entry: case.is_orbit_entry,
            },
            &mut info,
            &desc,
        );
        info
    }

    fn minerals(&mut self, info: &SYSTEM_INFO, which: u16, node: &mut NODE_INFO) -> u16 {
        uqm_rust::planets::surface::generate_mineral_deposits(&mut self.rng, info, which, node)
    }

    fn life(&mut self, info: &SYSTEM_INFO, which: u16, node: &mut NODE_INFO) -> u16 {
        uqm_rust::planets::surface::generate_life_forms(&mut self.rng, info, which, node)
    }

    fn preset_life(&mut self, info: &SYSTEM_INFO, which: u16, node: &mut NODE_INFO) -> u16 {
        let types = &PRESET_LIFE[..PRESET_LIFE.len() - 1];
        uqm_rust::planets::surface::generate_preset_life(&mut self.rng, info, types, which, node)
    }

    fn random_nodes(&mut self, info: &SYSTEM_INFO, which: u16, node: &mut NODE_INFO) -> u16 {
        uqm_rust::planets::surface::generate_random_nodes(&mut self.rng, info, 1, 4, 9, which, node)
    }

    fn fill_orbits(&mut self, case: &OrbitCase, descs: &mut [PLANET_DESC]) -> Option<u8> {
        use uqm_rust::planets::orbits::{fill_orbits, OrbitRequest};

        self.rng.reseed(case.seed);
        fill_orbits(
            &mut self.rng,
            OrbitRequest {
                star_size: case.star_size,
                star_color: case.star_color,
                num_planets: case.mode.num_planets(),
                generating_moons: case.mode.generating_moons(),
                types_defined: case.mode.types_defined(),
            },
            descs,
        )
    }
}

fn world_records(table: &[PlanetFrame]) -> Records {
    let mut generators = RustGenerators::new(table);
    let base = table.as_ptr() as usize;
    collect(world_cases(false), world_cases(true), |case| {
        world_line(&mut generators, case, base)
    })
}

fn orbit_records() -> Records {
    let table: Vec<PlanetFrame> = Vec::new();
    let mut generators = RustGenerators::new(&table);
    collect(orbit_cases(false), orbit_cases(true), |case| {
        orbit_line(&mut generators, case)
    })
}
