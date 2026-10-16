//! The Rust planet port against checked-in outputs of the C original.
//!
//! These fixtures were captured from the C code before `calc.c`, `surface.c`
//! and `orbits.c` were deleted, so this test needs no C and remains the
//! permanent parity proof. `planets_parity` replays them through the exported
//! C symbols.

mod planets_common;

use planets_common::{orbit_records, render_fixture, world_records};
use uqm_rust::c_bindings::global_state::{PlanetFrame, ELEMENT_ENTRY};

const PLAN_DATA: &str = include_str!("fixtures/planets/plan_data.txt");
const WORLDS: &str = include_str!("fixtures/planets/worlds.txt");
const ORBITS: &str = include_str!("fixtures/planets/orbits.txt");

/// The Rust side has no planet-frame table until `plandata.c` is ported.
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
fn orbit_layout_matches_the_c_fixture() {
    let records = orbit_records();
    assert_eq!(render_fixture(&records), ORBITS);
}
