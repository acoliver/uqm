//! Full descriptor buffers and guarded node records for the retained C generators.

use super::{fresh_node, scan_seeds};
use std::fmt::Write as _;
use uqm_rust::battle::gendef::{MAIDENS_DEFINED, VUX_BEAST_DEFINED, VUX_DEFINED};
use uqm_rust::c_bindings::global_state::{PlanetFrame, NODE_INFO, PLANET_DESC, SYSTEM_INFO};
use uqm_rust::math::RandomContext;
use uqm_rust::planets::generate::special::{Body, Story};
use uqm_rust::planets::generate::{sam, spa, vux};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Vux,
    Samatra,
    Spathi,
}
#[derive(Clone, Copy, Debug)]
pub struct Layout {
    pub kind: Kind,
    pub star: u8,
    pub size: u8,
    pub color: u8,
    pub seed: u32,
    pub shielded: bool,
}
#[derive(Clone, Copy, Debug)]
pub struct Scan {
    pub kind: Kind,
    pub star: u8,
    pub body: Body,
    pub taken: bool,
    pub seed: u32,
    pub which: u16,
    pub life: bool,
    pub frame: u8,
    pub chance: i16,
}
pub trait Ops {
    fn layout(&mut self, case: Layout, planets: &mut [PLANET_DESC]) -> (bool, u8, u32);
    fn moons(
        &mut self,
        case: Layout,
        planet: u8,
        count: u8,
        moons: &mut [PLANET_DESC],
    ) -> (bool, u32);
    fn scan(&mut self, case: Scan, info: &SYSTEM_INFO, node: &mut NODE_INFO) -> (u16, u32);
}
pub fn descriptors(count: usize) -> Vec<PLANET_DESC> {
    (0..count)
        .map(|i| {
            let mut d: PLANET_DESC = unsafe { std::mem::zeroed() };
            d.rand_seed = 0xfeed0000 + i as u32;
            d.data_index = 0;
            d.NumPlanets = 0x7f;
            d.radius = -17;
            d.location.x = -23;
            d.location.y = 29;
            d.temp_color.r = 31;
            d.temp_color.g = 37;
            d.temp_color.b = 41;
            d.temp_color.a = 43;
            d.NextIndex = 47 + i as u16;
            d.image.origin.x = -53;
            d.image.origin.y = 59;
            d
        })
        .collect()
}
pub fn scan_info(seed: u32, frame: &PlanetFrame, chance: i16) -> SYSTEM_INFO {
    let mut info: SYSTEM_INFO = unsafe { std::mem::zeroed() };
    info.PlanetInfo.ScanSeed = scan_seeds(seed);
    info.PlanetInfo.ScanRetrieveMask = [0xa5a5a5a5, 0x55555555, 0xffffffff];
    info.PlanetInfo.AtmoDensity = 50;
    info.PlanetInfo.SurfaceTemperature = 28;
    info.PlanetInfo.PlanDataPtr = frame;
    info.PlanetInfo.LifeChance = chance;
    info.PlanetInfo.PlanetRadius = 120;
    info
}
pub fn render(kind: Kind, frames: &[PlanetFrame], ops: &mut impl Ops) -> String {
    let mut out = String::new();
    let stars: &[u8] = if kind == Kind::Vux {
        &[VUX_DEFINED, MAIDENS_DEFINED, VUX_BEAST_DEFINED, 0]
    } else {
        &[0]
    };
    for &star in stars {
        for size in 0..3 {
            for color in 0..6 {
                for seed in [1, 7919, 15838, 23757, 190056, u32::MAX] {
                    for shielded in [false, true] {
                        if kind != Kind::Spathi && shielded {
                            continue;
                        }
                        let case = Layout {
                            kind,
                            star,
                            size,
                            color,
                            seed,
                            shielded,
                        };
                        let mut planets = descriptors(16);
                        let result = ops.layout(case, &mut planets);
                        writeln!(out, "layout {case:?} {result:?} {planets:?}").unwrap();
                    }
                }
            }
        }
    }
    if kind != Kind::Vux {
        for size in 0..3 {
            for color in 0..6 {
                for seed in [1, 7919, 190056, u32::MAX] {
                    for planet in [0, 1, 4, 15] {
                        for count in 0..=4 {
                            let case = Layout {
                                kind,
                                star: 0,
                                size,
                                color,
                                seed,
                                shielded: false,
                            };
                            let mut moons = descriptors(4);
                            let result = ops.moons(case, planet, count, &mut moons);
                            writeln!(out, "moons {case:?} {planet} {count} {result:?} {moons:?}")
                                .unwrap();
                        }
                    }
                }
            }
        }
    }
    if kind != Kind::Samatra {
        for &star in stars {
            for body in [
                Body::Planet(0),
                Body::Planet(1),
                Body::Planet(15),
                Body::Moon { planet: 0, moon: 0 },
                Body::Moon { planet: 0, moon: 3 },
                Body::Moon {
                    planet: 15,
                    moon: 0,
                },
            ] {
                for (frame, chance) in [(0, 0), (42, 1024), (50, 1024)] {
                    for taken in [false, true] {
                        for seed in [1, 104729, 4_189_160, u32::MAX] {
                            for which in [0, 1, 10, 11, 12, 15, 16, 30, 31, 32, 33, u16::MAX] {
                                for life in [false, true] {
                                    let case = Scan {
                                        kind,
                                        star,
                                        body,
                                        taken,
                                        seed,
                                        which,
                                        life,
                                        frame,
                                        chance,
                                    };
                                    let info = scan_info(
                                        seed,
                                        &frames[usize::from(case.frame)],
                                        case.chance,
                                    );
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
pub struct Flags(pub bool);
impl Story for Flags {
    fn flag(&self, name: &'static str) -> u32 {
        assert!(matches!(
            name,
            "SHOFIXTI_MAIDENS" | "UMGAH_BROADCASTERS" | "SPATHI_SHIELDED_SELVES"
        ));
        u32::from(self.0)
    }
    fn set_flag(&mut self, _: &'static str, _: u32) {
        panic!("read only");
    }
    fn word(&self, _: &'static str) -> u32 {
        panic!("no group references");
    }
    fn set_word(&mut self, _: &'static str, _: u32) {
        panic!("read only");
    }
}
pub struct Rust<'a>(pub &'a [PlanetFrame]);
impl Ops for Rust<'_> {
    fn layout(&mut self, c: Layout, planets: &mut [PLANET_DESC]) -> (bool, u8, u32) {
        let mut rng = RandomContext::with_seed(c.seed);
        let count = match c.kind {
            Kind::Vux => vux::generate_planets(&mut rng, self.0, c.size, c.color, c.star, planets),
            Kind::Samatra => sam::generate_planets(&mut rng, self.0, c.size, c.color, planets),
            Kind::Spathi => {
                spa::generate_planets(&mut rng, c.size, c.color, planets, &Flags(c.shielded))
            }
        };
        (true, count, rng.seed())
    }
    fn moons(
        &mut self,
        c: Layout,
        planet: u8,
        count: u8,
        moons: &mut [PLANET_DESC],
    ) -> (bool, u32) {
        let mut rng = RandomContext::with_seed(c.seed);
        match c.kind {
            Kind::Samatra => sam::generate_moons(&mut rng, c.size, c.color, planet, count, moons),
            Kind::Spathi => spa::generate_moons(&mut rng, c.size, c.color, planet, count, moons),
            Kind::Vux => unreachable!(),
        }
        (true, rng.seed())
    }
    fn scan(&mut self, c: Scan, info: &SYSTEM_INFO, node: &mut NODE_INFO) -> (u16, u32) {
        let mut rng = RandomContext::with_seed(c.seed);
        let count = match (c.kind, c.life) {
            (Kind::Vux, true) => vux::generate_life(&mut rng, info, c.star, c.body, c.which, node),
            (Kind::Vux, false) => vux::generate_energy(
                &mut rng,
                info,
                c.star,
                c.body,
                c.which,
                node,
                &Flags(c.taken),
            ),
            (Kind::Spathi, true) => spa::generate_life(&mut rng, info, c.body, c.which, node),
            (Kind::Spathi, false) => {
                spa::generate_energy(&mut rng, info, c.body, c.which, node, &Flags(c.taken))
            }
            (Kind::Samatra, _) => unreachable!(),
        };
        (count, rng.seed())
    }
}
