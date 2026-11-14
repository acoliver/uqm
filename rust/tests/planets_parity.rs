//! Linked production generation over supplied solar-system records.
//!
//! Analysis, scan and layout views must reproduce the full domains and frozen
//! fixtures captured from the C originals in `tests/fixtures/planets/*`.
//! Native resource and rendering fixtures share the serialized host below.

use uqm_rust::engine::world::planet::{NodeInfo, PlanetDesc, PlanetFrame, SystemInfo};
#[path = "planets_render/oval.rs"]
mod oval_parity;
mod planets_common;
#[path = "planets_special/mod.rs"]
mod planets_special;

#[path = "support/battle_context.rs"]
mod battle_context;
#[path = "planets_render/sphere_geometry.rs"]
mod sphere_geometry_parity;
#[path = "planets_render/topography.rs"]
mod topography_parity;

use std::fmt::Write as _;
use std::path::PathBuf;
use uqm_rust::engine::world::generation::Body;
use uqm_rust::planets::generate::{calc, orbits, surface};
use uqm_rust::planets::solar_system::SolarSystem;

use planets_common::default_gen::{
    self, layout_cases, moon_cases, moons_line, nodes_line, planets_line, DefaultOps, Layout,
    NodeKind,
};
use planets_common::topography::{self, TopoCase, Topography};
use planets_common::{
    collect, orbit_cases, orbit_line, render_fixture, star_type_byte, world_cases, world_line,
    Generators, OrbitCase, WorldCase, PRESET_LIFE,
};
use uqm_rust::graphics::native_abi::POINT;
use uqm_rust::math::RandomContext;
use uqm_rust::savegame::runtime::star::{CStarDesc, CATALOG_LEN};

unsafe extern "C" {

    static mut ScreenWidth: i32;

}

/// Cells past the map the C side gets, so an overrun reads as a mismatch
/// instead of corrupting memory.
const GUARD_CELLS: usize = 4096;

/// Production topography with a fixture-owned random stream.
struct OwnedTopography;

impl Topography for OwnedTopography {
    fn run(&mut self, case: &TopoCase, map: &mut [i8]) -> u32 {
        let mut padded = map.to_vec();
        padded.resize(map.len() + GUARD_CELLS, 0);
        let mut rng = RandomContext::with_seed(case.seed);
        uqm_rust::planets::topography::delta_topography(
            &mut rng,
            case.iterations,
            &mut padded,
            case.width,
            case.height,
            case.depth_delta,
        );
        assert!(
            padded[map.len()..].iter().all(|cell| *cell == 0),
            "DeltaTopography wrote past the map for {case:?}"
        );
        map.copy_from_slice(&padded[..map.len()]);
        rng.seed()
    }
}

/// Production topography reproduces the checked-in C fixture: every
/// case's whole depth map and RNG position.
fn exported_topography_reproduces_the_fixture(table: &[PlanetFrame]) {
    assert_eq!(
        topography::cases(table).len(),
        171,
        "topography domain changed"
    );
    check_fixture(
        "topography.txt",
        &topography::render(&mut OwnedTopography, table),
    );
}

struct SuppliedGenerators {
    state: Box<SolarSystem>,
    /// The fixture star occupies slot zero of this retained catalog.
    star: Box<[CStarDesc; CATALOG_LEN]>,
    previous_catalog: [CStarDesc; CATALOG_LEN],
    previous_star: Option<uqm_rust::engine::StarId>,
}

impl SuppliedGenerators {
    fn new() -> Self {
        unsafe {
            // `MAP_WIDTH` derives from the screen width the graphics init sets.
            ScreenWidth = 320;
        }
        let previous_catalog = uqm_rust::engine::World::current().read(|world| *world.catalog());
        let previous_star = uqm_rust::engine::World::current().read(|world| world.current_star);
        // SAFETY: the serialized harness copies the initialized integer-only catalog.
        let mut star = Box::new(unsafe { starmap_array });
        // SAFETY: a star record contains only integer fields; zero initializes its fixture values.
        star[0] = unsafe { std::mem::zeroed() };
        let state = uqm_rust::engine::World::current().write(|world| {
            world.set_catalog(std::borrow::Cow::Owned(*star));
            Box::new(SolarSystem::new(world, uqm_rust::engine::StarId(0)))
        });
        Self {
            state,
            star,
            previous_catalog,
            previous_star,
        }
    }

    fn set_star(&mut self, size: u8, star_type: u8) {
        self.state.sys.sun.data_index = size;
        self.star[0].star_type = star_type;
        uqm_rust::engine::World::current().write(|world| {
            world.set_catalog(std::borrow::Cow::Owned(*self.star));
            world.current_star = Some(uqm_rust::engine::StarId(0));
        });
        let mut next = uqm_rust::engine::world::generation::SolarSys::new(
            &uqm_rust::planets::tables::PLANET_ARRAY,
            self.star[0].star_type,
            self.star[0].index,
            std::mem::replace(&mut self.state.sys.rng, RandomContext::with_seed(12345)),
        );
        next.sun = self.state.sys.sun;
        next.planets = self.state.sys.planets;
        next.moons = self.state.sys.moons;
        next.info = self.state.sys.info;
        next.frames = std::mem::take(&mut self.state.sys.frames);
        next.report = std::mem::take(&mut self.state.sys.report);
        self.state.sys = next;
    }

    fn select_info(&mut self, info: &SystemInfo) {
        self.state.sys.info = *info;
        let Some(index) = uqm_rust::planets::tables::PLANET_ARRAY
            .iter()
            .zip(0u8..)
            .find_map(|(frame, index)| {
                std::ptr::eq(frame, info.planet_info.plan_data_ptr).then_some(index)
            })
        else {
            panic!("fixture analysis retains an immutable planet frame");
        };
        self.state.sys.planets[0].data_index = index;
    }

    fn with_scan<R>(
        &mut self,
        info: &SystemInfo,
        run: impl FnOnce(&mut RandomContext, &SystemInfo, &PlanetFrame) -> R,
    ) -> R {
        self.select_info(info);
        unsafe {
            crate::invoke_generation(&mut self.state, |system, _| {
                system.scan(Body::Planet(0), run)
            })
        }
    }
}

impl Generators for SuppliedGenerators {
    fn analyze(&mut self, case: &WorldCase) -> SystemInfo {
        self.set_star(case.star_size, star_type_byte(case));
        let mut desc: PlanetDesc = unsafe { std::mem::zeroed() };
        desc.rand_seed = case.rand_seed;
        desc.data_index = case.data_index;
        desc.radius = case.desc_radius;
        let body = match case.parent_radius {
            None => {
                self.state.sys.planets[0] = desc;
                Body::Planet(0)
            }
            Some(radius) => {
                let mut parent: PlanetDesc = unsafe { std::mem::zeroed() };
                parent.radius = radius;
                self.state.sys.planets[0] = parent;
                self.state.sys.moons[0] = desc;
                Body::Moon { planet: 0, moon: 0 }
            }
        };
        self.state.sys.info = SystemInfo::default();
        unsafe {
            crate::invoke_generation(&mut self.state, |system, _| {
                if case.is_orbit_entry {
                    calc::analyse_body(system, body);
                    system.orbital(body, |data| *data.info)
                } else {
                    calc::preview_body(system, body)
                }
            })
        }
    }

    fn minerals(&mut self, info: &SystemInfo, which: u16, node: &mut NodeInfo) -> u16 {
        self.with_scan(info, |rng, info, frame| {
            surface::generate_mineral_deposits(rng, info, frame, which, node)
        })
    }

    fn life(&mut self, info: &SystemInfo, which: u16, node: &mut NodeInfo) -> u16 {
        self.with_scan(info, |rng, info, frame| {
            surface::generate_life_forms(rng, info, frame, which, node)
        })
    }

    fn preset_life(&mut self, info: &SystemInfo, which: u16, node: &mut NodeInfo) -> u16 {
        self.with_scan(info, |rng, info, _| {
            surface::generate_preset_life(
                rng,
                info,
                &PRESET_LIFE[..PRESET_LIFE.len() - 1],
                which,
                node,
            )
        })
    }

    fn random_nodes(&mut self, info: &SystemInfo, which: u16, node: &mut NodeInfo) -> u16 {
        self.with_scan(info, |rng, info, _| {
            surface::generate_random_nodes(rng, info, 1, 4, 9, which, node)
        })
    }

    fn fill_orbits(&mut self, case: &OrbitCase, descs: &mut [PlanetDesc]) -> Option<u8> {
        self.set_star(case.star_size, (case.star_color << 5) | 0b101);
        self.state.sys.sun.num_planets = 0;
        let moons = case.mode.generating_moons();
        if moons {
            self.state.sys.moons[..descs.len()].copy_from_slice(descs);
        } else {
            self.state.sys.planets[..descs.len()].copy_from_slice(descs);
        }
        let slots = if case.mode.num_planets() == u8::MAX {
            9
        } else {
            usize::from(case.mode.num_planets())
        };
        let request = orbits::OrbitRequest {
            star_size: case.star_size,
            star_color: case.star_color,
            num_planets: case.mode.num_planets(),
            generating_moons: moons,
            types_defined: case.mode.types_defined(),
        };
        unsafe {
            self.state.sys.rng.reseed(case.seed);
            crate::invoke_generation(&mut self.state, |system, _| {
                if moons {
                    system.layout_moons(0, |rng, size, color, _, worlds| {
                        assert_eq!(size, request.star_size);
                        assert_eq!(color, request.star_color);
                        assert!(orbits::fill_orbits(rng, request, &mut worlds[..slots]).is_none());
                    });
                } else {
                    system.layout(|rng, _, size, color, worlds| {
                        assert_eq!(size, request.star_size);
                        assert_eq!(color, request.star_color);
                        orbits::fill_orbits(rng, request, &mut worlds[..slots]).unwrap_or(0)
                    });
                }
            });
        }
        if moons {
            descs.copy_from_slice(&self.state.sys.moons[..descs.len()]);
        } else {
            descs.copy_from_slice(&self.state.sys.planets[..descs.len()]);
        }
        let drawn = self.state.sys.sun.num_planets;
        (case.mode.num_planets() == u8::MAX).then_some(drawn)
    }
}

/// Production default generators over retained fixture records.
impl DefaultOps for SuppliedGenerators {
    fn planets(&mut self, star_size: u8, star_color: u8, seed: u32) -> Layout {
        self.set_star(star_size, (star_color << 5) | 0b101);
        self.state.sys.planets = unsafe { std::mem::zeroed() };
        self.state.sys.sun.num_planets = 0;
        let rng = unsafe {
            self.state.sys.rng.reseed(seed);
            assert!(crate::invoke_generation(&mut self.state, |system, host| {
                uqm_rust::planets::generate::generator(0).generate_planets(system, host)
            }));
            self.state.sys.rng.seed()
        };
        let drawn = self.state.sys.sun.num_planets;
        Layout {
            worlds: self.state.sys.planets[..usize::from(drawn)]
                .iter()
                .map(default_gen::world_of)
                .collect(),
            drawn,
            rng,
        }
    }

    fn moons(&mut self, star_size: u8, star_color: u8, seed: u32, count: u8) -> Layout {
        self.set_star(star_size, (star_color << 5) | 0b101);
        self.state.sys.moons = unsafe { std::mem::zeroed() };
        self.state.sys.planets[0].num_planets = count;
        let rng = unsafe {
            self.state.sys.rng.reseed(seed);
            assert!(crate::invoke_generation(&mut self.state, |system, host| {
                uqm_rust::planets::generate::generator(0).generate_moons(system, 0, host)
            }));
            self.state.sys.rng.seed()
        };
        Layout {
            worlds: self.state.sys.moons[..usize::from(count)]
                .iter()
                .map(default_gen::world_of)
                .collect(),
            drawn: 0,
            rng,
        }
    }

    fn nodes(
        &mut self,
        kind: NodeKind,
        info: &SystemInfo,
        which: u16,
        node: &mut NodeInfo,
    ) -> (u16, u32) {
        self.select_info(info);
        let record = &raw mut *self.state;
        unsafe {
            let count = crate::invoke_generation(&mut *record, |system, host| {
                use uqm_rust::planets::generate::{default, generator};
                let body = uqm_rust::planets::generate::special::Body::Planet(0);
                match kind {
                    NodeKind::Minerals => {
                        generator(0).generate_minerals(system, body, which, node, host)
                    }
                    NodeKind::Energy => {
                        generator(0).generate_energy(system, body, which, node, host)
                    }
                    NodeKind::Life => generator(0).generate_life(system, body, which, node, host),
                    NodeKind::Artifact => system.scan(body, |rng, info, _| {
                        default::generate_artifact(rng, info, which, node)
                    }),
                    NodeKind::Ruins => system.scan(body, |rng, info, _| {
                        default::generate_ruins(rng, info, which, node)
                    }),
                }
            });
            (count, self.state.sys.rng.seed())
        }
    }
}

/// Production default generators reproduce the checked-in C fixtures:
/// every layout (worlds, planet count, RNG position) and every node of every
/// analysed world.
fn exported_default_generator_reproduces_the_fixtures(exports: &mut SuppliedGenerators) {
    let layouts = [
        collect(layout_cases(), layout_cases(), |c| planets_line(exports, c)),
        collect(moon_cases(), moon_cases(), |c| moons_line(exports, c)),
    ];
    let text = layouts.iter().map(render_fixture).collect::<String>();
    check_fixture("default_layout.txt", &text);

    let nodes = collect(world_cases(false), world_cases(true), |case| {
        let info = exports.analyze(case);
        nodes_line(exports, case, info)
    });
    println!(
        "default generator exports: {} planet layouts, {} moon layouts, {} node worlds",
        layouts[0].count, layouts[1].count, nodes.count
    );
    check_fixture("default_nodes.txt", &render_fixture(&nodes));
}

fn render_plan_data(table: &[PlanetFrame]) -> String {
    let mut text = String::new();
    for frame in table {
        let _ = write!(
            text,
            "{} {} {}",
            frame.type_, frame.base_tectonics, frame.atmo_and_density
        );
        for element in &frame.useful_elements {
            let _ = write!(text, " {}:{}", element.element_type, element.density);
        }
        let _ = writeln!(
            text,
            " {} {} {} {}",
            frame.num_faults, frame.fault_depth, frame.num_blemishes, frame.base_elevation
        );
    }
    text
}

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/planets")
        .join(name)
}

fn check_fixture(name: &str, text: &str) {
    let stored = std::fs::read_to_string(fixture_path(name)).expect("read fixture");
    assert_eq!(stored, text, "{name} differs from the checked-in C fixture");
}

/// Native drawing and campaign fixtures use one serialized graphics owner.
#[test]
fn supplied_views_reproduce_the_c_fixtures() {
    battle_context::run(|_| {
        let table = unsafe { &planet_array }.as_slice();
        let mut exports = SuppliedGenerators::new();
        let base = table.as_ptr() as usize;
        gravity_and_random_locations_agree();

        // The planet-frame table the fixtures carry must be the C table.
        check_fixture("plan_data.txt", &render_plan_data(table));
        exported_topography_reproduces_the_fixture(table);

        let worlds = collect(world_cases(false), world_cases(true), |case| {
            world_line(&mut exports, case, base)
        });
        let orbits = collect(orbit_cases(false), orbit_cases(true), |case| {
            orbit_line(&mut exports, case)
        });
        check_fixture("worlds.txt", &render_fixture(&worlds));
        check_fixture("orbits.txt", &render_fixture(&orbits));
        exported_default_generator_reproduces_the_fixtures(&mut exports);
        planets_special::special_generators_match_the_c_originals();
        planets_special::mycon_matches_the_c_original(table);
        planets_special::late_generators_match(table);
        planets_special::sol_matches_the_c_original(table);
        oval_parity::retained_ovals_match_original_c_full_offscreen_buffers(&exports.state.sys.rng);
        topography_parity::full_c_fixture_core_and_host_parity(&exports.state.sys.rng);
        sphere_geometry_parity::original_c_and_rust_match_fixture_without_rng_effects(
            &exports.state.sys.rng,
        );
        assert!(unsafe {
            crate::invoke_generation(&mut exports.state, |system, host| {
                let generator = uqm_rust::planets::generate::generator(0);
                let body = uqm_rust::planets::generate::special::Body::Planet(0);
                generator.pickup_minerals(system, body, 0, host)
                    && generator.pickup_life(system, body, 0, host)
            })
        });
    });
}

fn gravity_and_random_locations_agree() {
    for density in [0u16, 1, 20, 33, 100, 200, 65535] {
        for radius in [0u16, 1, 25, 75, 300, 1500, 65535] {
            let mut info = SystemInfo::default();
            info.planet_info.planet_density = density;
            info.planet_info.planet_radius = radius;
            let expected = (u64::from(density) * u64::from(radius) / 100) as u16;
            assert_eq!(
                calc::calc_gravity(&info.planet_info),
                expected,
                "density {density} radius {radius}"
            );
        }
    }

    for seed in 1..2000u32 {
        let mut rng = RandomContext::with_seed(seed);
        let mut reference = RandomContext::with_seed(seed);
        let draw = reference.next_random();
        let expected = (
            i16::from((draw as u8) % 226) + 8,
            i16::from(((draw >> 8) as u8) % 59) + 8,
        );
        let mut point = POINT { x: 0, y: 0 };
        surface::generate_random_location(&mut rng, &mut point);
        assert_eq!((point.x, point.y), expected, "seed {seed}");
        assert!((8..234).contains(&point.x));
        assert!((8..67).contains(&point.y));
        assert_eq!(
            rng.seed(),
            reference.seed(),
            "one location draw, seed {seed}"
        );
    }
}

fn render_star_catalog(table: &[CStarDesc]) -> String {
    let mut text = String::new();
    for star in table {
        let _ = writeln!(
            text,
            "{} {} {} {} {} {}",
            star.star_pt.x, star.star_pt.y, star.star_type, star.index, star.prefix, star.postfix
        );
    }
    text
}

fn render_element_array(table: &[u8]) -> String {
    table
        .iter()
        .map(|category| format!("{category}\n"))
        .collect()
}

fn render_plan_resources(table: &[PlanetFrame]) -> String {
    let name = |resource: &std::ffi::CStr| {
        resource
            .to_str()
            .expect("resource names are ASCII")
            .to_owned()
    };
    table
        .iter()
        .map(|frame| {
            format!(
                "{} {}\n",
                name(frame.cmap_instance),
                name(frame.xlat_tab_instance)
            )
        })
        .collect()
}

/// The tables the game links against (`starmap_array`, `element_array` and
/// `planet_array`), read through their exported symbols, must equal the
/// fixtures captured from the C originals field for field (strings included).
#[test]
fn plandata_tables_equal_the_c_originals() {
    use uqm_rust::planets::tables::{ELEMENT_ARRAY, PLANET_ARRAY, STARMAP_ARRAY};
    let stars = &STARMAP_ARRAY;
    let frames = &PLANET_ARRAY;
    let elements = &ELEMENT_ARRAY;

    for (name, text) in [
        ("star_catalog.txt", render_star_catalog(stars)),
        ("element_array.txt", render_element_array(elements)),
        ("plan_resources.txt", render_plan_resources(frames)),
        ("plan_data.txt", render_plan_data(frames)),
    ] {
        check_fixture(name, &text);
    }
}

impl Drop for SuppliedGenerators {
    fn drop(&mut self) {
        uqm_rust::engine::World::current().write(|world| {
            world.set_catalog(std::borrow::Cow::Owned(self.previous_catalog));
            world.current_star = self.previous_star;
        });
    }
}

unsafe fn invoke_generation<R>(
    solar: &mut SolarSystem,
    run: impl FnOnce(
        &mut uqm_rust::engine::world::generation::SolarSys<'_>,
        &mut dyn uqm_rust::planets::generate::SiteHost,
    ) -> R,
) -> R {
    // SAFETY: the serialized fixture owns the published context and initialized record during the callback.
    unsafe {
        uqm_rust::app::bridge::enter(|cx| {
            let mut observer = uqm_rust::engine::observe::Unobserved;
            let mut io = uqm_rust::engine::observe::Io {
                observer: &mut observer,
            };
            uqm_rust::planets::solarsys::site::dispatch::invoke(
                cx,
                &mut io,
                solar,
                |_, system, host| run(system, host),
            )
            .0
        })
    }
}

type GenerationData = (
    PlanetDesc,
    [PlanetDesc; 16],
    [PlanetDesc; 4],
    SystemInfo,
    [Option<uqm_rust::graphics::native_abi::FRAME>; 6],
    Option<usize>,
);

fn generation_data(solar: &SolarSystem) -> GenerationData {
    (
        solar.sys.sun,
        solar.sys.planets,
        solar.sys.moons,
        solar.sys.info,
        std::array::from_fn(|i| {
            solar.sys.frames[i]
                .as_ref()
                .map(|frame| match frame.lease().frame(0) {
                    Some(frame) => frame,
                    None => panic!("generated scene has no first frame"),
                })
        }),
        solar.sys.report.current().map(|text| text.index()),
    )
}
