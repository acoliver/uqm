//! Full-field Mycon layout and scan records, shared by pure and linked tests.

use std::fmt::Write as _;

use super::{fresh_node, scan_seeds};
use uqm_rust::battle::gendef::{
    EGG_CASE0_DEFINED, EGG_CASE1_DEFINED, EGG_CASE2_DEFINED, MYCON_DEFINED, SUN_DEVICE_DEFINED,
};
use uqm_rust::c_bindings::global_state::{PlanetFrame, NODE_INFO, PLANET_DESC, SYSTEM_INFO};
use uqm_rust::math::RandomContext;
use uqm_rust::planets::generate::myc;
use uqm_rust::planets::generate::special::{Body, Story};
use uqm_rust::planets::surface::ENERGY_SCAN;

const STARS: [u8; 5] = [
    MYCON_DEFINED,
    SUN_DEVICE_DEFINED,
    EGG_CASE0_DEFINED,
    EGG_CASE1_DEFINED,
    EGG_CASE2_DEFINED,
];
const BODIES: [Body; 5] = [
    Body::Planet(0),
    Body::Planet(1),
    Body::Moon { planet: 0, moon: 0 },
    Body::Moon { planet: 0, moon: 1 },
    Body::Moon { planet: 1, moon: 0 },
];

#[derive(Clone, Copy, Debug)]
pub struct ScanCase {
    pub star: u8,
    pub body: Body,
    pub taken: bool,
    pub strings: bool,
    pub retrieved: bool,
    pub seed: u32,
    pub which: u16,
    pub life: bool,
}

pub trait MyconOps {
    fn layout(
        &mut self,
        star: u8,
        case: (u8, u8, u32),
        planets: &mut [PLANET_DESC],
    ) -> (bool, u8, u32);
    fn scan(&mut self, case: ScanCase, node: &mut NODE_INFO) -> (u16, u32);
}

pub fn scan_info(case: ScanCase) -> SYSTEM_INFO {
    let mut info: SYSTEM_INFO = unsafe { std::mem::zeroed() };
    info.PlanetInfo.ScanSeed = scan_seeds(case.seed);
    info.PlanetInfo.ScanRetrieveMask[ENERGY_SCAN] = u32::from(case.retrieved);
    if case.strings {
        // Neither scan hook dereferences this handle.
        info.PlanetInfo.DiscoveryString = std::ptr::NonNull::dangling().as_ptr();
    }
    info
}

pub fn render(ops: &mut impl MyconOps) -> String {
    let mut out = String::new();
    for star in STARS {
        for size in 0..3 {
            for color in 0..6 {
                for seed in [7_919, 15_838, 23_757, 190_056] {
                    let mut planets: Vec<PLANET_DESC> = vec![unsafe { std::mem::zeroed() }; 16];
                    for (i, desc) in planets.iter_mut().enumerate() {
                        desc.rand_seed = 0xfeed_0000 + i as u32;
                        desc.NumPlanets = 0x7f;
                        desc.radius = -17;
                        desc.location.x = -23;
                        desc.location.y = 29;
                        desc.temp_color.r = 31;
                        desc.temp_color.g = 37;
                        desc.temp_color.b = 41;
                        desc.temp_color.a = 43;
                        desc.NextIndex = 47;
                        desc.image.origin.x = -53;
                        desc.image.origin.y = 59;
                    }
                    let case = (size, color, seed);
                    let result = ops.layout(star, case, &mut planets);
                    // Debug includes every field, including null frame/parent handles.
                    writeln!(out, "layout {star} {case:?} {result:?} {planets:?}").unwrap();
                }
            }
        }
    }
    for star in STARS {
        for body in BODIES {
            for taken in [false, true] {
                for strings in [false, true] {
                    for retrieved in [false, true] {
                        for seed in [104_729, 209_458, 4_189_160] {
                            for which in [0, 1, 32, u16::MAX] {
                                for life in [false, true] {
                                    let case = ScanCase {
                                        star,
                                        body,
                                        taken,
                                        strings,
                                        retrieved,
                                        seed,
                                        which,
                                        life,
                                    };
                                    let mut nodes = [fresh_node(); 3];
                                    let result = ops.scan(case, &mut nodes[1]);
                                    assert_eq!(
                                        format!("{:?}", nodes[0]),
                                        format!("{:?}", fresh_node())
                                    );
                                    assert_eq!(
                                        format!("{:?}", nodes[2]),
                                        format!("{:?}", fresh_node())
                                    );
                                    writeln!(out, "scan {case:?} {result:?} {nodes:?}").unwrap();
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    out
}

struct SunDevice(bool);

impl Story for SunDevice {
    fn flag(&self, name: &'static str) -> u32 {
        assert_eq!(name, "SUN_DEVICE");
        u32::from(self.0)
    }
    fn set_flag(&mut self, _name: &'static str, _value: u32) {
        panic!("scan hooks do not write story flags");
    }
    fn word(&self, _name: &'static str) -> u32 {
        panic!("scan hooks do not read group references");
    }
    fn set_word(&mut self, _name: &'static str, _value: u32) {
        panic!("scan hooks do not write group references");
    }
}

pub struct RustMycon<'a>(pub &'a [PlanetFrame]);

impl MyconOps for RustMycon<'_> {
    fn layout(
        &mut self,
        _star: u8,
        case: (u8, u8, u32),
        planets: &mut [PLANET_DESC],
    ) -> (bool, u8, u32) {
        let mut rng = RandomContext::with_seed(case.2);
        let drawn = myc::generate_planets(&mut rng, self.0, case.0, case.1, planets);
        (true, drawn, rng.seed())
    }

    fn scan(&mut self, case: ScanCase, node: &mut NODE_INFO) -> (u16, u32) {
        let mut rng = RandomContext::with_seed(case.seed);
        let count = if case.life {
            myc::generate_life()
        } else {
            myc::generate_energy(
                &mut rng,
                &scan_info(case),
                case.star,
                case.body,
                case.which,
                node,
                &SunDevice(case.taken),
            )
        };
        (count, rng.seed())
    }
}
