//! Linked test of the C-ABI exports that replaced `calc.c`, `surface.c` and
//! `orbits.c`.
//!
//! C callers reach the planet port through `DoPlanetaryAnalysis`,
//! `GenerateMineralDeposits`, `FillOrbits` and friends, which read the C-owned
//! `SysGenRNG`, `pSolarSysState`, `CurStarDescPtr` and `PlanData`. This test
//! drives those exact symbols over the whole input domain and requires the
//! results to equal the golden fixtures captured from the C originals in
//! `tests/fixtures/planets/*`, so it proves the adapters wire that state the
//! way the C did.

mod planets_common;

use std::ffi::c_void;
use std::fmt::Write as _;
use std::path::PathBuf;

use planets_common::{
    collect, orbit_cases, orbit_line, render_fixture, star_type_byte, world_cases, world_line,
    Generators, OrbitCase, WorldCase, PRESET_LIFE,
};
use uqm_rust::c_bindings::global_state::{
    PlanetFrame, NODE_INFO, PLANET_DESC, POINT, SOLARSYS_STATE, SYSTEM_INFO,
};
use uqm_rust::math::RandomContext;
use uqm_rust::planets::tables;
use uqm_rust::savegame::runtime::star::{CStarDesc, CATALOG_LEN};

#[cfg_attr(
    target_os = "macos",
    link(name = "uqm_c", kind = "static", modifiers = "+whole-archive")
)]
unsafe extern "C" {
    static mut SysGenRNG: *mut RandomContext;
    static mut pSolarSysState: *mut SOLARSYS_STATE;
    static mut CurStarDescPtr: *mut c_void;
    static mut ScreenWidth: i32;
    static planet_array: [PlanetFrame; 59];
    static element_array: [u8; 133];
    static mut starmap_array: [CStarDesc; CATALOG_LEN];

    fn DoPlanetaryAnalysis(sys_info: *mut SYSTEM_INFO, desc: *mut PLANET_DESC);
    fn CalcGravity(info: *const c_void) -> u16;
    fn GenerateMineralDeposits(
        sys_info: *const SYSTEM_INFO,
        which: u16,
        info: *mut NODE_INFO,
    ) -> u16;
    fn GenerateLifeForms(sys_info: *const SYSTEM_INFO, which: u16, info: *mut NODE_INFO) -> u16;
    fn GeneratePresetLife(
        sys_info: *const SYSTEM_INFO,
        types: *const i8,
        which: u16,
        info: *mut NODE_INFO,
    ) -> u16;
    fn GenerateRandomNodes(
        sys_info: *const SYSTEM_INFO,
        scan: u16,
        num_nodes: u16,
        node_type: u16,
        which: u16,
        info: *mut NODE_INFO,
    ) -> u16;
    fn GenerateRandomLocation(loc: *mut POINT);
    fn FillOrbits(
        system: *mut SOLARSYS_STATE,
        num_planets: u8,
        base: *mut PLANET_DESC,
        types_defined: u32,
    );
}

struct ExportedGenerators {
    state: Box<SOLARSYS_STATE>,
    /// Fake `STAR_DESC`: `POINT` then `Type`.
    star: Box<[u8; 8]>,
}

impl ExportedGenerators {
    fn new() -> Self {
        let mut state: Box<SOLARSYS_STATE> = Box::new(unsafe { std::mem::zeroed() });
        let star = Box::new([0u8; 8]);
        unsafe {
            SysGenRNG = uqm_rust::math::RandomContext_New();
            // `MAP_WIDTH` derives from the screen width the graphics init sets.
            ScreenWidth = 320;
            uqm_rust::planets::ffi::PLAN_DATA = planet_array.as_ptr();
            pSolarSysState = &mut *state;
        }
        Self { state, star }
    }

    fn set_star(&mut self, size: u8, star_type: u8) {
        self.state.SunDesc[0].data_index = size;
        self.star[4] = star_type;
        unsafe { CurStarDescPtr = self.star.as_mut_ptr().cast() };
    }
}

impl Generators for ExportedGenerators {
    fn analyze(&mut self, case: &WorldCase) -> SYSTEM_INFO {
        self.set_star(case.star_size, star_type_byte(case));

        let mut parent: PLANET_DESC = unsafe { std::mem::zeroed() };
        let mut desc: PLANET_DESC = unsafe { std::mem::zeroed() };
        desc.rand_seed = case.rand_seed;
        desc.data_index = case.data_index;
        desc.radius = case.desc_radius;
        desc.pPrevDesc = match case.parent_radius {
            None => &mut self.state.SunDesc[0],
            Some(radius) => {
                parent.radius = radius;
                &mut parent
            }
        };

        let mut detached: SYSTEM_INFO = unsafe { std::mem::zeroed() };
        self.state.SysInfo = unsafe { std::mem::zeroed() };
        let target: *mut SYSTEM_INFO = if case.is_orbit_entry {
            &mut self.state.SysInfo
        } else {
            &mut detached
        };
        unsafe {
            DoPlanetaryAnalysis(target, &mut desc);
            *target
        }
    }

    fn minerals(&mut self, info: &SYSTEM_INFO, which: u16, node: &mut NODE_INFO) -> u16 {
        unsafe { GenerateMineralDeposits(info, which, node) }
    }

    fn life(&mut self, info: &SYSTEM_INFO, which: u16, node: &mut NODE_INFO) -> u16 {
        unsafe { GenerateLifeForms(info, which, node) }
    }

    fn preset_life(&mut self, info: &SYSTEM_INFO, which: u16, node: &mut NODE_INFO) -> u16 {
        unsafe { GeneratePresetLife(info, PRESET_LIFE.as_ptr(), which, node) }
    }

    fn random_nodes(&mut self, info: &SYSTEM_INFO, which: u16, node: &mut NODE_INFO) -> u16 {
        unsafe { GenerateRandomNodes(info, 1, 4, 9, which, node) }
    }

    fn fill_orbits(&mut self, case: &OrbitCase, descs: &mut [PLANET_DESC]) -> Option<u8> {
        self.set_star(case.star_size, (case.star_color << 5) | 0b101);
        self.state.SunDesc[0].NumPlanets = 0;
        // C decides "moons" by comparing the base pointer with `MoonDesc`, so
        // the descriptors must live in the solar system state.
        let moons = case.mode.generating_moons();
        let base: *mut PLANET_DESC = if moons {
            self.state.MoonDesc.as_mut_ptr()
        } else {
            self.state.PlanetDesc.as_mut_ptr()
        };
        unsafe {
            for (slot, desc) in descs.iter().enumerate() {
                *base.add(slot) = *desc;
            }
            reseed_c(SysGenRNG, case.seed);
            FillOrbits(
                &mut *self.state,
                case.mode.num_planets(),
                base,
                u32::from(case.mode.types_defined()),
            );
            for (slot, desc) in descs.iter_mut().enumerate() {
                *desc = *base.add(slot);
            }
        }
        let drawn = self.state.SunDesc[0].NumPlanets;
        (case.mode.num_planets() == !0).then_some(drawn)
    }
}

fn render_plan_data(table: &[PlanetFrame]) -> String {
    let mut text = String::new();
    for frame in table {
        let _ = write!(
            text,
            "{} {} {}",
            frame.Type, frame.BaseTectonics, frame.AtmoAndDensity
        );
        for element in &frame.UsefulElements {
            let _ = write!(text, " {}:{}", element.ElementType, element.Density);
        }
        let _ = writeln!(
            text,
            " {} {} {} {}",
            frame.num_faults, frame.fault_depth, frame.num_blemishes, frame.base_elevation
        );
    }
    text
}

/// Reseed the `RandomContext` the exports use, through the exported C symbol.
fn reseed_c(context: *mut uqm_rust::math::RandomContext, seed: u32) {
    uqm_rust::math::RandomContext_SeedRandom(context, seed);
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

/// One test: the exports share process-global state (`pSolarSysState`,
/// `SysGenRNG`), so the comparisons must not run in parallel.
#[test]
fn exported_symbols_reproduce_the_c_fixtures() {
    let table = unsafe { &planet_array }.as_slice();
    let mut exports = ExportedGenerators::new();
    let base = table.as_ptr() as usize;
    gravity_and_random_locations_agree();

    // The planet-frame table the fixtures carry must be the C table.
    check_fixture("plan_data.txt", &render_plan_data(table));

    let worlds = collect(world_cases(false), world_cases(true), |case| {
        world_line(&mut exports, case, base)
    });
    let orbits = collect(orbit_cases(false), orbit_cases(true), |case| {
        orbit_line(&mut exports, case)
    });
    check_fixture("worlds.txt", &render_fixture(&worlds));
    check_fixture("orbits.txt", &render_fixture(&orbits));
}

fn gravity_and_random_locations_agree() {
    let mut rng = RandomContext::with_seed(99);
    for density in [0u16, 1, 20, 33, 100, 200, 65535] {
        for radius in [0u16, 1, 25, 75, 300, 1500, 65535] {
            let mut info: SYSTEM_INFO = unsafe { std::mem::zeroed() };
            info.PlanetInfo.PlanetDensity = density;
            info.PlanetInfo.PlanetRadius = radius;
            let exported = unsafe { CalcGravity((&raw const info.PlanetInfo).cast()) };
            assert_eq!(
                exported,
                uqm_rust::planets::calc::calc_gravity(&info.PlanetInfo),
                "density {density} radius {radius}"
            );
        }
    }

    for seed in 1..2000u32 {
        let mut exported_point = POINT { x: 0, y: 0 };
        let mut direct_point = POINT { x: 0, y: 0 };
        unsafe {
            reseed_c(SysGenRNG, seed);
            GenerateRandomLocation(&mut exported_point);
        }
        rng.reseed(seed);
        uqm_rust::planets::surface::generate_random_location(&mut rng, &mut direct_point);
        assert_eq!(
            (exported_point.x, exported_point.y),
            (direct_point.x, direct_point.y),
            "seed {seed}"
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
    let name = |resource: *const std::ffi::c_char| {
        unsafe { std::ffi::CStr::from_ptr(resource) }
            .to_str()
            .expect("resource names are ASCII")
            .to_owned()
    };
    table
        .iter()
        .map(|frame| {
            format!(
                "{} {}\n",
                name(frame.CMapInstance),
                name(frame.XlatTabInstance)
            )
        })
        .collect()
}

/// The `plandata.c` tables, read through the exported C symbols, must equal the
/// fixtures captured from the C originals field for field (strings included),
/// and the Rust tables must equal the same fixtures.
#[test]
fn plandata_tables_equal_the_c_originals() {
    let c_stars = unsafe { &*(&raw const starmap_array) };
    let c_frames = unsafe { &planet_array }.as_slice();
    let c_elements = unsafe { &element_array }.as_slice();

    let rust_stars = unsafe { &*(&raw const tables::STARMAP_ARRAY) };
    let rust_frames = tables::PLANET_ARRAY.0.as_slice();
    let rust_elements = tables::ELEMENT_ARRAY.as_slice();

    assert_eq!(c_stars.len(), rust_stars.len());
    assert_eq!(c_frames.len(), rust_frames.len());
    assert_eq!(c_elements.len(), rust_elements.len());

    for (name, text) in [
        ("star_catalog.txt", render_star_catalog(c_stars)),
        ("star_catalog.txt", render_star_catalog(rust_stars)),
        ("element_array.txt", render_element_array(c_elements)),
        ("element_array.txt", render_element_array(rust_elements)),
        ("plan_resources.txt", render_plan_resources(c_frames)),
        ("plan_resources.txt", render_plan_resources(rust_frames)),
        ("plan_data.txt", render_plan_data(c_frames)),
        ("plan_data.txt", render_plan_data(rust_frames)),
    ] {
        check_fixture(name, &text);
    }

    // Beyond the rendered fields, the bytes of the pointer-free tables match.
    let bytes = |ptr: *const u8, len: usize| unsafe { std::slice::from_raw_parts(ptr, len) };
    assert_eq!(
        bytes(c_stars.as_ptr().cast(), std::mem::size_of_val(c_stars)),
        bytes(
            rust_stars.as_ptr().cast(),
            std::mem::size_of_val(rust_stars)
        ),
    );
    assert_eq!(c_elements, rust_elements);
}
