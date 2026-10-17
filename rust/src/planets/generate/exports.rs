//! C-ABI `GenerateDefault_*` entry points (`generate.h`).
//!
//! Each export is a thin adapter over [`super::default`]: it reads the C-owned
//! state the C original read (`SysGenRNG`, `pSolarSysState`, `CurStarDescPtr`,
//! `PlanData`) and hands it to the plain Rust port. Special-system generators
//! in C call these directly when they fall back to the default behaviour, and
//! the `generateDefaultFunctions` table in [`super::vtable`] points at them.
//! All of it runs on the game thread.

use std::ffi::c_void;

use crate::c_bindings::global_state::{NODE_INFO, PLANET_DESC, SOLARSYS_STATE};
use crate::planet_side::report_adapter::CffiDiscoveryReport;
use crate::planets::c_api::{load_planet_default_surface, LiveGame};
use crate::planets::consts::star_color;
use crate::planets::ffi::{
    analysis_context, blank_node, current_star_type, node_or, plan_data, sys_gen_rng,
};
use crate::planets::generate::default;

/// `planetIndex (system, world)`: the planet itself, or the planet a moon
/// orbits, as its position in `system->PlanetDesc`.
///
/// # Safety
/// `world` must be a planet of `system` or a moon whose `pPrevDesc` is one.
unsafe fn planet_index(system: *const SOLARSYS_STATE, world: *const PLANET_DESC) -> u16 {
    let sun = std::ptr::addr_of!((*system).SunDesc).cast::<PLANET_DESC>();
    let planet = if (*world).pPrevDesc.cast_const() == sun {
        world
    } else {
        (*world).pPrevDesc.cast_const()
    };
    let first = std::ptr::addr_of!((*system).PlanetDesc).cast::<PLANET_DESC>();
    u16::try_from(planet.offset_from(first)).expect("world is not in the system's planet table")
}

/// The discovery-string slot of `system`.
///
/// # Safety
/// `system` must be valid.
unsafe fn discovery_report(system: *mut SOLARSYS_STATE) -> CffiDiscoveryReport {
    let slot = std::ptr::addr_of_mut!((*system).SysInfo.PlanetInfo.DiscoveryString);
    CffiDiscoveryReport::new(slot.cast::<*mut c_void>())
        .expect("the discovery string slot is part of the system state")
}

/// C: `bool GenerateDefault_initNpcs (SOLARSYS_STATE *)`.
///
/// # Safety
/// Game thread, with `GlobData` initialised.
#[export_name = "GenerateDefault_initNpcs"]
pub unsafe extern "C" fn init_npcs_c(_system: *mut SOLARSYS_STATE) -> bool {
    default::init_npcs(&mut LiveGame)
}

/// C: `bool GenerateDefault_reinitNpcs (SOLARSYS_STATE *)`.
///
/// # Safety
/// Game thread, with `GlobData` initialised.
#[export_name = "GenerateDefault_reinitNpcs"]
pub unsafe extern "C" fn reinit_npcs_c(_system: *mut SOLARSYS_STATE) -> bool {
    default::reinit_npcs(&mut LiveGame)
}

/// C: `bool GenerateDefault_uninitNpcs (SOLARSYS_STATE *)`.
///
/// # Safety
/// Game thread, with `GlobData` initialised.
#[export_name = "GenerateDefault_uninitNpcs"]
pub unsafe extern "C" fn uninit_npcs_c(_system: *mut SOLARSYS_STATE) -> bool {
    default::uninit_npcs(&mut LiveGame)
}

/// C: `bool GenerateDefault_generatePlanets (SOLARSYS_STATE *)`.
///
/// Stores the drawn planet count in `SunDesc[0].NumPlanets`.
///
/// # Safety
/// `system` must be the live solar system state; `CurStarDescPtr`, `SysGenRNG`
/// and `PlanData` must be valid.
#[export_name = "GenerateDefault_generatePlanets"]
pub unsafe extern "C" fn generate_planets_c(system: *mut SOLARSYS_STATE) -> bool {
    let drawn = default::generate_planets(
        sys_gen_rng(),
        plan_data(),
        (*system).SunDesc[0].data_index,
        star_color(current_star_type()),
        &mut (*system).PlanetDesc,
    );
    (*system).SunDesc[0].NumPlanets = drawn;
    true
}

/// C: `bool GenerateDefault_generateMoons (SOLARSYS_STATE *, PLANET_DESC *planet)`.
///
/// # Safety
/// `system` must be the live solar system state and `planet` valid;
/// `CurStarDescPtr` and `SysGenRNG` must be valid.
#[export_name = "GenerateDefault_generateMoons"]
pub unsafe extern "C" fn generate_moons_c(
    system: *mut SOLARSYS_STATE,
    planet: *mut PLANET_DESC,
) -> bool {
    let moons = (*planet).NumPlanets;
    let moon_table = &mut *std::ptr::addr_of_mut!((*system).MoonDesc);
    default::generate_moons(
        sys_gen_rng(),
        (*system).SunDesc[0].data_index,
        star_color(current_star_type()),
        moons,
        &mut moon_table[..usize::from(moons)],
    );
    true
}

/// C: `bool GenerateDefault_generateName (const SOLARSYS_STATE *, const PLANET_DESC *)`.
///
/// # Safety
/// `world` must be a planet of `system` or a moon of one of its planets.
#[export_name = "GenerateDefault_generateName"]
pub unsafe extern "C" fn generate_name_c(
    system: *const SOLARSYS_STATE,
    world: *const PLANET_DESC,
) -> bool {
    default::generate_name(
        &mut LiveGame,
        planet_index(system, world),
        (*world).data_index,
    )
}

/// C: `bool GenerateDefault_generateOrbital (SOLARSYS_STATE *, PLANET_DESC *)`.
///
/// # Safety
/// `system` must be the live solar system state; `world`, `CurStarDescPtr`,
/// `SysGenRNG` and `PlanData` must be valid, and `world.pPrevDesc` must point
/// at the sun or a valid description.
#[export_name = "GenerateDefault_generateOrbital"]
pub unsafe extern "C" fn generate_orbital_c(
    system: *mut SOLARSYS_STATE,
    world: *mut PLANET_DESC,
) -> bool {
    let sys_info = std::ptr::addr_of_mut!((*system).SysInfo);
    let (star, site) = analysis_context(sys_info, world);
    default::generate_orbital(
        sys_gen_rng(),
        plan_data(),
        star,
        site,
        &mut *sys_info,
        &*world,
        || load_planet_default_surface(),
    )
}

/// C: `COUNT GenerateDefault_generateMinerals (const SOLARSYS_STATE *,
/// const PLANET_DESC *, COUNT whichNode, NODE_INFO *)`; `info` may be NULL.
///
/// # Safety
/// `system` must be valid with an analysed `SysInfo`.
#[export_name = "GenerateDefault_generateMinerals"]
pub unsafe extern "C" fn generate_minerals_c(
    system: *const SOLARSYS_STATE,
    _world: *const PLANET_DESC,
    which_node: u16,
    info: *mut NODE_INFO,
) -> u16 {
    let mut spare = blank_node();
    default::generate_minerals(
        sys_gen_rng(),
        &(*system).SysInfo,
        which_node,
        node_or(info, &mut spare),
    )
}

/// C: `bool GenerateDefault_pickupMinerals (SOLARSYS_STATE *, PLANET_DESC *, COUNT)`.
///
/// # Safety
/// None of the arguments is read.
#[export_name = "GenerateDefault_pickupMinerals"]
pub unsafe extern "C" fn pickup_minerals_c(
    _system: *mut SOLARSYS_STATE,
    _world: *mut PLANET_DESC,
    _which_node: u16,
) -> bool {
    default::pickup_minerals()
}

/// C: `COUNT GenerateDefault_generateEnergy (const SOLARSYS_STATE *,
/// const PLANET_DESC *, COUNT whichNode, NODE_INFO *)`.
///
/// # Safety
/// None of the arguments is read.
#[export_name = "GenerateDefault_generateEnergy"]
pub unsafe extern "C" fn generate_energy_c(
    _system: *const SOLARSYS_STATE,
    _world: *const PLANET_DESC,
    _which_node: u16,
    _info: *mut NODE_INFO,
) -> u16 {
    default::generate_energy()
}

/// C: `bool GenerateDefault_pickupEnergy (SOLARSYS_STATE *, PLANET_DESC *, COUNT)`.
///
/// C asserts that this is never called; the abort is the same contract.
///
/// # Safety
/// None of the arguments is read.
#[export_name = "GenerateDefault_pickupEnergy"]
pub unsafe extern "C" fn pickup_energy_c(
    _system: *mut SOLARSYS_STATE,
    _world: *mut PLANET_DESC,
    _which_node: u16,
) -> bool {
    default::pickup_energy()
}

/// C: `COUNT GenerateDefault_generateLife (const SOLARSYS_STATE *,
/// const PLANET_DESC *, COUNT whichNode, NODE_INFO *)`; `info` may be NULL.
///
/// # Safety
/// `system` must be valid with an analysed `SysInfo`.
#[export_name = "GenerateDefault_generateLife"]
pub unsafe extern "C" fn generate_life_c(
    system: *const SOLARSYS_STATE,
    _world: *const PLANET_DESC,
    which_node: u16,
    info: *mut NODE_INFO,
) -> u16 {
    let mut spare = blank_node();
    default::generate_life(
        sys_gen_rng(),
        &(*system).SysInfo,
        which_node,
        node_or(info, &mut spare),
    )
}

/// C: `bool GenerateDefault_pickupLife (SOLARSYS_STATE *, PLANET_DESC *, COUNT)`.
///
/// # Safety
/// None of the arguments is read.
#[export_name = "GenerateDefault_pickupLife"]
pub unsafe extern "C" fn pickup_life_c(
    _system: *mut SOLARSYS_STATE,
    _world: *mut PLANET_DESC,
    _which_node: u16,
) -> bool {
    default::pickup_life()
}

/// C: `COUNT GenerateDefault_generateArtifact (const SOLARSYS_STATE *,
/// COUNT whichNode, NODE_INFO *)`; `info` may be NULL.
///
/// # Safety
/// `system` must be valid with an analysed `SysInfo`.
#[export_name = "GenerateDefault_generateArtifact"]
pub unsafe extern "C" fn generate_artifact_c(
    system: *const SOLARSYS_STATE,
    which_node: u16,
    info: *mut NODE_INFO,
) -> u16 {
    let mut spare = blank_node();
    default::generate_artifact(
        sys_gen_rng(),
        &(*system).SysInfo,
        which_node,
        node_or(info, &mut spare),
    )
}

/// C: `COUNT GenerateDefault_generateRuins (const SOLARSYS_STATE *,
/// COUNT whichNode, NODE_INFO *)`; `info` may be NULL.
///
/// # Safety
/// `system` must be valid with an analysed `SysInfo`.
#[export_name = "GenerateDefault_generateRuins"]
pub unsafe extern "C" fn generate_ruins_c(
    system: *const SOLARSYS_STATE,
    which_node: u16,
    info: *mut NODE_INFO,
) -> u16 {
    let mut spare = blank_node();
    default::generate_ruins(
        sys_gen_rng(),
        &(*system).SysInfo,
        which_node,
        node_or(info, &mut spare),
    )
}

/// C: `bool GenerateDefault_landerReport (SOLARSYS_STATE *)`.
///
/// # Safety
/// `system` must be the live solar system state, on the game thread.
#[export_name = "GenerateDefault_landerReport"]
pub unsafe extern "C" fn lander_report_c(system: *mut SOLARSYS_STATE) -> bool {
    default::lander_report(&mut discovery_report(system))
}

/// C: `bool GenerateDefault_landerReportCycle (SOLARSYS_STATE *)`.
///
/// # Safety
/// `system` must be the live solar system state, on the game thread.
#[export_name = "GenerateDefault_landerReportCycle"]
pub unsafe extern "C" fn lander_report_cycle_c(system: *mut SOLARSYS_STATE) -> bool {
    default::lander_report_cycle(&mut discovery_report(system))
}
