//! The Rust planet port against checked-in outputs of the C original.
//!
//! These fixtures were captured from the C code before `calc.c`, `surface.c`
//! and `orbits.c` were deleted, so this test needs no C and remains the
//! permanent parity proof. `planets_parity` replays them through the exported
//! C symbols.

mod planets_common;

use planets_common::{
    collect, orbit_cases, orbit_line, render_fixture, star_type_byte, world_cases, world_line,
    Generators, OrbitCase, Records, WorldCase, PRESET_LIFE,
};
use uqm_rust::c_bindings::global_state::{
    PlanetFrame, ELEMENT_ENTRY, NODE_INFO, PLANET_DESC, SYSTEM_INFO,
};

const PLAN_DATA: &str = include_str!("fixtures/planets/plan_data.txt");
const WORLDS: &str = include_str!("fixtures/planets/worlds.txt");
const ORBITS: &str = include_str!("fixtures/planets/orbits.txt");
const TOPOGRAPHY: &str = include_str!("fixtures/planets/topography.txt");

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
    let text =
        planets_common::topography::render(&mut planets_common::topography::RustTopography, &table);
    assert_eq!(text, TOPOGRAPHY);
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
