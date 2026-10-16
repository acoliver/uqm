//! Differential test: the C `calc.c`/`surface.c`/`orbits.c` against the Rust
//! port on the same inputs, with exact equality on every output field.
//!
//! Also the generator for `tests/fixtures/planets/*`: run with
//! `UQM_REGEN_PLANET_FIXTURES=1` to rewrite them from the C results.

mod planets_common;

use std::ffi::c_void;
use std::fmt::Write as _;
use std::path::PathBuf;

use planets_common::{
    collect, orbit_cases, orbit_line, orbit_records, render_fixture, star_type_byte, world_cases,
    world_line, world_records, Generators, OrbitCase, WorldCase, PRESET_LIFE,
};
use uqm_rust::c_bindings::global_state::{
    PlanetFrame, NODE_INFO, PLANET_DESC, POINT, SOLARSYS_STATE, SYSTEM_INFO,
};
use uqm_rust::math::RandomContext;

#[cfg_attr(
    target_os = "macos",
    link(name = "uqm_c", kind = "static", modifiers = "+whole-archive")
)]
unsafe extern "C" {
    static mut SysGenRNG: *mut RandomContext;
    static mut pSolarSysState: *mut SOLARSYS_STATE;
    static mut CurStarDescPtr: *mut c_void;
    static mut PlanData: *const PlanetFrame;
    static mut ScreenWidth: i32;
    static planet_array: [PlanetFrame; 59];

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

struct CGenerators {
    state: Box<SOLARSYS_STATE>,
    /// Fake `STAR_DESC`: `POINT` then `Type`.
    star: Box<[u8; 8]>,
}

impl CGenerators {
    fn new() -> Self {
        let mut state: Box<SOLARSYS_STATE> = Box::new(unsafe { std::mem::zeroed() });
        let star = Box::new([0u8; 8]);
        unsafe {
            SysGenRNG = uqm_rust::math::RandomContext_New();
            // `MAP_WIDTH` derives from the screen width the graphics init sets.
            ScreenWidth = 320;
            PlanData = planet_array.as_ptr();
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

impl Generators for CGenerators {
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

/// Reseed a C-owned `RandomContext` through the exported C symbol.
fn reseed_c(context: *mut uqm_rust::math::RandomContext, seed: u32) {
    uqm_rust::math::RandomContext_SeedRandom(context, seed);
}

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/planets")
        .join(name)
}

fn check_or_write(name: &str, text: &str) {
    let path = fixture_path(name);
    if std::env::var_os("UQM_REGEN_PLANET_FIXTURES").is_some() {
        std::fs::write(&path, text).expect("write fixture");
    } else {
        let stored = std::fs::read_to_string(&path).expect("read fixture");
        assert_eq!(stored, text, "{name} is stale; regenerate from C");
    }
}

fn first_difference(c_lines: &[String], rust_lines: &[String]) -> Option<String> {
    c_lines
        .iter()
        .zip(rust_lines)
        .find(|(c, rust)| c != rust)
        .map(|(c, rust)| format!("C:    {c}\nRust: {rust}"))
}

/// One test: the C side shares process-global state (`pSolarSysState`,
/// `SysGenRNG`), so the comparisons must not run in parallel.
#[test]
fn rust_planet_port_matches_c_exactly() {
    let table = unsafe { &planet_array }.as_slice();
    let mut c = CGenerators::new();
    let base = table.as_ptr() as usize;
    gravity_and_random_locations_agree();

    // The planet-frame table the fixtures carry must be the C table.
    check_or_write("plan_data.txt", &render_plan_data(table));

    // Every world case, line by line, so a mismatch names the input.
    let mut rust = planets_common::RustGenerators::new(table);
    let mut checked = 0usize;
    for case in world_cases(false) {
        let c_line = world_line(&mut c, &case, base);
        let rust_line = world_line(&mut rust, &case, base);
        assert_eq!(c_line, rust_line, "world case {case:?}");
        checked += 1;
    }
    assert!(checked > 30_000, "domain shrank: {checked}");

    let mut orbit_checked = 0usize;
    for case in orbit_cases(false) {
        let c_line = orbit_line(&mut c, &case);
        let rust_line = orbit_line(&mut rust, &case);
        assert_eq!(c_line, rust_line, "orbit case {case:?}");
        orbit_checked += 1;
    }
    assert_eq!(orbit_checked, 3_960, "orbit domain changed");

    // Fixtures come from the C side.
    let c_worlds = collect(world_cases(false), world_cases(true), |case| {
        world_line(&mut c, case, base)
    });
    let c_orbits = collect(orbit_cases(false), orbit_cases(true), |case| {
        orbit_line(&mut c, case)
    });
    check_or_write("worlds.txt", &render_fixture(&c_worlds));
    check_or_write("orbits.txt", &render_fixture(&c_orbits));

    let rust_worlds = world_records(table);
    assert_eq!(
        first_difference(&c_worlds.golden_lines, &rust_worlds.golden_lines),
        None
    );
    assert_eq!(c_worlds.digest, rust_worlds.digest);
    assert_eq!(c_orbits.digest, orbit_records().digest);
}

fn gravity_and_random_locations_agree() {
    let mut rng = RandomContext::with_seed(99);
    for density in [0u16, 1, 20, 33, 100, 200, 65535] {
        for radius in [0u16, 1, 25, 75, 300, 1500, 65535] {
            let mut info: SYSTEM_INFO = unsafe { std::mem::zeroed() };
            info.PlanetInfo.PlanetDensity = density;
            info.PlanetInfo.PlanetRadius = radius;
            let from_c = unsafe { CalcGravity((&raw const info.PlanetInfo).cast()) };
            assert_eq!(
                from_c,
                uqm_rust::planets::calc::calc_gravity(&info.PlanetInfo),
                "density {density} radius {radius}"
            );
        }
    }

    for seed in 1..2000u32 {
        let mut c_point = POINT { x: 0, y: 0 };
        let mut rust_point = POINT { x: 0, y: 0 };
        unsafe {
            reseed_c(SysGenRNG, seed);
            GenerateRandomLocation(&mut c_point);
        }
        rng.reseed(seed);
        uqm_rust::planets::surface::generate_random_location(&mut rng, &mut rust_point);
        assert_eq!(
            (c_point.x, c_point.y),
            (rust_point.x, rust_point.y),
            "seed {seed}"
        );
    }
}
