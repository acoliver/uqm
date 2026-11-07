//! Complete descriptor buffers and guarded scans for the retained Sol C vtable.

use std::fmt::Write as _;
use uqm_rust::engine::world::planet::{NodeInfo, PlanetDesc, SystemInfo};
use uqm_rust::engine::StoryKey;

use super::late_gen::descriptors;
use super::{fresh_node, scan_seeds};
use uqm_rust::math::RandomContext;
use uqm_rust::planets::generate::sol;
use uqm_rust::planets::generate::special::{Body, Story};

#[derive(Clone, Copy, Debug)]
pub struct Layout {
    pub size: u8,
    pub color: u8,
    pub seed: u32,
}

#[derive(Clone, Copy, Debug)]
pub struct Scan {
    pub body: Body,
    pub pluto: bool,
    pub destroyed: bool,
    pub seed: u32,
    pub which: u16,
    pub life: bool,
}

pub trait Ops {
    fn planets(&mut self, case: Layout, descs: &mut [PlanetDesc; 16]) -> (bool, u8, u32);
    fn moons(
        &mut self,
        case: Layout,
        planet: u8,
        count: u8,
        descs: &mut [PlanetDesc; 4],
    ) -> (bool, u32);
    fn scan(&mut self, case: Scan, info: &SystemInfo, node: &mut NodeInfo) -> (u16, u32);
}

pub fn render_layout(ops: &mut impl Ops) -> String {
    let mut out = String::new();
    for size in 0..3 {
        for color in 0..6 {
            for seed in [0, 1, 104_729, u32::MAX] {
                let case = Layout { size, color, seed };
                let mut planets: [PlanetDesc; 16] = descriptors(16).try_into().unwrap();
                let result = ops.planets(case, &mut planets);
                writeln!(out, "planets {case:?} {result:?} {planets:?}").unwrap();
                for planet in 0..9 {
                    // Also test counts outside the usual Sol layout: overrides
                    // are unconditional, including writes to inactive slots.
                    for count in 0..=4 {
                        let mut moons: [PlanetDesc; 4] = descriptors(4).try_into().unwrap();
                        let result = ops.moons(case, planet, count, &mut moons);
                        writeln!(out, "moons {case:?} {planet} {count} {result:?} {moons:?}")
                            .unwrap();
                    }
                }
            }
        }
    }
    super::record_labels::captured_labels(out)
}

pub fn scan_info(seed: u32) -> SystemInfo {
    let mut info: SystemInfo = unsafe { std::mem::zeroed() };
    info.planet_info.scan_seed = scan_seeds(seed);
    info.planet_info.scan_retrieve_mask = [0x55555555, 0xa5a5a5a5, u32::MAX];
    info.planet_info.atmo_density = 53;
    info.planet_info.planet_density = 59;
    info.planet_info.planet_radius = 61;
    info.planet_info.temperature = -67;
    info.planet_info.axial_tilt = 71;
    info.planet_info.rotation_period = 73;
    info.planet_info.gravity = 79;
    info
}

pub fn render_scans(ops: &mut impl Ops) -> String {
    let mut out = String::new();
    for planet in 0..9 {
        let bodies = std::iter::once(Body::Planet(planet))
            .chain((0..4).map(|moon| Body::Moon { planet, moon }));
        for body in bodies {
            for pluto in [false, true] {
                for destroyed in [false, true] {
                    for seed in [1, 104_729, u32::MAX] {
                        // Every tractor node, the end, an out-of-range index,
                        // and GENERATE_ALL. Energy ignores whichNode entirely.
                        for which in (0..=11).chain([32, u16::MAX]) {
                            for life in [false, true] {
                                let case = Scan {
                                    body,
                                    pluto,
                                    destroyed,
                                    seed,
                                    which,
                                    life,
                                };
                                let info = scan_info(seed);
                                let before = format!("{info:?}");
                                let mut nodes = [fresh_node(); 3];
                                let result = ops.scan(case, &info, &mut nodes[1]);
                                assert_eq!(
                                    format!("{:?}", nodes[0]),
                                    format!("{:?}", fresh_node())
                                );
                                assert_eq!(
                                    format!("{:?}", nodes[2]),
                                    format!("{:?}", fresh_node())
                                );
                                assert_eq!(format!("{info:?}"), before);
                                writeln!(out, "scan {case:?} {result:?} {nodes:?}").unwrap();
                            }
                        }
                    }
                }
            }
        }
    }
    super::record_labels::captured_labels(out)
}

pub struct RustSol;

impl Ops for RustSol {
    fn planets(&mut self, case: Layout, descs: &mut [PlanetDesc; 16]) -> (bool, u8, u32) {
        let mut rng = RandomContext::with_seed(case.seed);
        let count = sol::generate_planets(&mut rng, descs);
        (true, count, rng.seed())
    }
    fn moons(
        &mut self,
        case: Layout,
        planet: u8,
        count: u8,
        descs: &mut [PlanetDesc; 4],
    ) -> (bool, u32) {
        let mut rng = RandomContext::with_seed(case.seed);
        sol::generate_moons(&mut rng, case.size, case.color, planet, count, descs);
        (true, rng.seed())
    }
    fn scan(&mut self, case: Scan, info: &SystemInfo, node: &mut NodeInfo) -> (u16, u32) {
        let mut rng = RandomContext::with_seed(case.seed);
        let story = ScanStory(case.pluto, case.destroyed);
        let count = if case.life {
            sol::generate_life(&mut rng, info, case.body, case.which, node)
        } else {
            sol::generate_energy(case.body, Some(node), &story)
        };
        if !case.life {
            assert_eq!(sol::generate_energy(case.body, None, &story), count);
        }
        (count, rng.seed())
    }
}

struct ScanStory(bool, bool);

impl Story for ScanStory {
    fn flag(&self, key: StoryKey) -> u32 {
        match key {
            StoryKey::FOUND_PLUTO_SPATHI => u32::from(self.0),
            StoryKey::MOONBASE_DESTROYED => u32::from(self.1),
            _ => unreachable!("Sol scans read only their two flags"),
        }
    }
    fn set_flag(&mut self, _key: StoryKey, _value: u32) {
        unreachable!("scans do not write flags");
    }
    fn word(&self, _key: StoryKey) -> u32 {
        unreachable!("scans do not read words");
    }
    fn set_word(&mut self, _key: StoryKey, _value: u32) {
        unreachable!("scans do not write words");
    }
}
