//! Full `SYSTEM_INFO` and RNG at Sol's noninteractive C orbit entry.
//! Resource-bearing scene flags are already set: actual scene effects are
//! covered by recording hosts, not replaced in the linked test.

use super::sol::{self, Layout};
use std::fmt::Write as _;
use uqm_rust::engine::world::planet::{PlanetDesc, PlanetFrame, SystemInfo};
use uqm_rust::math::RandomContext;
use uqm_rust::planets::generate::calc::{AnalysisSite, StarDescription};
use uqm_rust::planets::generate::sol::orbital;
use uqm_rust::planets::generate::special::{Body, Orbit};

#[derive(Clone, Copy, Debug)]
pub struct Case {
    pub layout: Layout,
    pub body: Body,
}

pub trait Ops {
    fn orbital(&mut self, case: Case) -> (bool, SystemInfo, u32);
}

pub fn input(case: Case) -> (PlanetDesc, Option<PlanetDesc>) {
    let mut planets = unsafe { std::mem::zeroed() };
    uqm_rust::planets::generate::sol::generate_planets(
        &mut RandomContext::with_seed(91),
        &mut planets,
    );
    let (mut world, parent) = match case.body {
        Body::Planet(p) => (planets[usize::from(p)], None),
        Body::Moon { planet, moon } => {
            let mut moons = unsafe { std::mem::zeroed() };
            uqm_rust::planets::generate::sol::generate_moons(
                &mut RandomContext::with_seed(case.layout.seed),
                case.layout.size,
                case.layout.color,
                planet,
                4,
                &mut moons,
            );
            (moons[usize::from(moon)], Some(planets[usize::from(planet)]))
        }
    };
    world.rand_seed = case.layout.seed;
    (world, parent)
}

pub fn initial_info(seed: u32) -> SystemInfo {
    let mut info = sol::scan_info(seed);
    info.star_size = 83;
    info.star_intensity = 89;
    info.star_energy = 97;
    info.planet_info.tectonics = 101;
    info.planet_info.weather = 103;
    info.planet_info.life_chance = -107;
    info.planet_info.planet_to_sun_dist = 109;
    info
}

pub fn render(ops: &mut impl Ops, table: &[PlanetFrame]) -> String {
    let mut out = String::new();
    for size in 0..3 {
        for color in 0..6 {
            for seed in [0, 1, 104_729, u32::MAX] {
                let layout = Layout { size, color, seed };
                for planet in 0..9 {
                    for body in std::iter::once(Body::Planet(planet))
                        .chain((0..4).map(|moon| Body::Moon { planet, moon }))
                    {
                        // Starbase returns before any analysis and is tested
                        // with the queue/encounter host, not a display seam.
                        if body == (Body::Moon { planet: 2, moon: 0 }) {
                            continue;
                        }
                        let case = Case { layout, body };
                        let (result, mut info, rng) = ops.orbital(case);
                        let index = table
                            .iter()
                            .position(|frame| std::ptr::eq(frame, info.planet_info.plan_data_ptr))
                            .expect("analysis frame belongs to the live table");
                        // Only the address varies by process; all other fields,
                        // including all handles, masks and seeds, are retained.
                        info.planet_info.plan_data_ptr = std::ptr::null();
                        let (desc, parent) = input(case);
                        writeln!(out, "{case:?} input={desc:?} parent={parent:?} => {result} frame={index} rng={rng} {info:?}").unwrap();
                    }
                }
            }
        }
    }
    super::record_labels::captured_labels(out)
}

pub struct Rust<'a>(pub &'a [PlanetFrame]);

impl Ops for Rust<'_> {
    fn orbital(&mut self, case: Case) -> (bool, SystemInfo, u32) {
        let (world, parent) = input(case);
        let mut info = initial_info(case.layout.seed);
        let mut rng = RandomContext::with_seed(0xabcdef01);
        orbital::analyse_orbital(
            &mut Orbit {
                rng: &mut rng,
                plan_data: self.0,
                star: StarDescription {
                    size: case.layout.size,
                    star_type: (case.layout.color << 5) | 5,
                },
                site: AnalysisSite {
                    parent_radius: parent.map(|p| p.radius),
                    is_orbit_entry: true,
                },
                sys_info: &mut info,
                world: &world,
            },
            case.body,
        );
        orbital::physical_overrides(&mut info, case.body);
        (true, info, rng.seed())
    }
}
