//! C-ABI `GenerateDefault_*` entry points (`generate.h`).
//!
//! Each export is a thin adapter over [`super::default`]: it reads the C-owned
//! state the C original read (`SysGenRNG`, `pSolarSysState`, `CurStarDescPtr`,
//! `PlanData`) and hands it to the plain Rust port. Special-system generators
//! in C call these directly when they fall back to the default behaviour, and
//! the `generateDefaultFunctions` table in [`super::vtable`] points at them.
//! All of it runs on the game thread.

use crate::engine::world::planet::{NodeInfo, PlanetDesc};
use std::ffi::c_void;

use crate::c_bindings::global_state::SOLARSYS_STATE;
use crate::planet_side::report_adapter::CffiDiscoveryReport;
use crate::planets::c_api::{load_planet_default_surface, LiveNames};
use crate::planets::ffi::{
    analysis_context, blank_node, current_star_type, node_or, plan_data, sys_gen_rng,
};
use crate::planets::generate::consts::star_color;
use crate::planets::generate::default;

/// `planetIndex (system, world)`: the planet itself, or the planet a moon
/// orbits, as its position in `system->PlanetDesc`.
///
/// # Safety
/// `world` must be a planet of `system` or a moon whose `pPrevDesc` is one.
pub(super) unsafe fn planet_index(system: *const SOLARSYS_STATE, world: *const PlanetDesc) -> u16 {
    let sun = std::ptr::addr_of!((*system).SunDesc).cast::<PlanetDesc>();
    let planet = if (*world).prev_desc.cast_const() == sun {
        world
    } else {
        (*world).prev_desc.cast_const()
    };
    let first = std::ptr::addr_of!((*system).PlanetDesc).cast::<PlanetDesc>();
    u16::try_from(planet.offset_from(first)).expect("world is not in the system's planet table")
}

/// The discovery-string slot of `system`.
///
/// # Safety
/// `system` must be valid.
pub(super) unsafe fn discovery_report(system: *mut SOLARSYS_STATE) -> CffiDiscoveryReport {
    let slot = std::ptr::addr_of_mut!((*system).SysInfo.planet_info.discovery_string);
    CffiDiscoveryReport::new(slot.cast::<*mut c_void>(), system)
        .expect("the discovery string slot is part of the system state")
}

/// C: `bool GenerateDefault_initNpcs (SOLARSYS_STATE *)`.
/// The supplied owner owns all NPC/group state; the ABI record pointer is unused.
pub fn init_npcs_c(owner: &mut crate::engine::World, _system: *mut SOLARSYS_STATE) -> bool {
    default::init_npcs(&mut crate::planets::c_api::NpcWorld::new(owner))
}

/// C: `bool GenerateDefault_reinitNpcs (SOLARSYS_STATE *)`.
/// The supplied owner owns all NPC/group state; the ABI record pointer is unused.
pub fn reinit_npcs_c(owner: &mut crate::engine::World, _system: *mut SOLARSYS_STATE) -> bool {
    default::reinit_npcs(&mut crate::planets::c_api::NpcWorld::new(owner))
}

/// C: `bool GenerateDefault_uninitNpcs (SOLARSYS_STATE *)`.
/// The supplied owner owns all NPC/group state; the ABI record pointer is unused.
pub fn uninit_npcs_c(owner: &mut crate::engine::World, _system: *mut SOLARSYS_STATE) -> bool {
    default::uninit_npcs(&mut crate::planets::c_api::NpcWorld::new(owner))
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
    (*system).SunDesc[0].num_planets = drawn;
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
    planet: *mut PlanetDesc,
) -> bool {
    let moons = (*planet).num_planets;
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
    world: *const PlanetDesc,
) -> bool {
    default::generate_name(
        &mut LiveNames,
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
pub unsafe fn generate_orbital_c(
    cx: &mut crate::engine::battle::Cx<'_>,
    system: *mut SOLARSYS_STATE,
    world: *mut PlanetDesc,
) -> bool {
    let owner = std::cell::RefCell::new(cx);
    let owner = &owner;

    let sys_info = std::ptr::addr_of_mut!((*system).SysInfo);
    let (star, site) = owner
        .borrow_mut()
        .with_legacy(|| analysis_context(sys_info, world));
    default::generate_orbital(
        sys_gen_rng(),
        plan_data(),
        star,
        site,
        &mut *sys_info,
        &*world,
        || {
            owner
                .borrow_mut()
                .with_legacy(|| load_planet_default_surface())
        },
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
    _world: *const PlanetDesc,
    which_node: u16,
    info: *mut NodeInfo,
) -> u16 {
    let mut spare = blank_node();
    default::generate_minerals(
        sys_gen_rng(),
        &(*system).SysInfo,
        // SAFETY: the caller supplies an analysed system whose frame table remains live.
        unsafe { &*(*system).SysInfo.planet_info.plan_data_ptr },
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
    _world: *mut PlanetDesc,
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
    _world: *const PlanetDesc,
    _which_node: u16,
    _info: *mut NodeInfo,
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
    _world: *mut PlanetDesc,
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
    _world: *const PlanetDesc,
    which_node: u16,
    info: *mut NodeInfo,
) -> u16 {
    let mut spare = blank_node();
    default::generate_life(
        sys_gen_rng(),
        &(*system).SysInfo,
        // SAFETY: the caller supplies an analysed system whose frame table remains live.
        unsafe { &*(*system).SysInfo.planet_info.plan_data_ptr },
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
    _world: *mut PlanetDesc,
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
    info: *mut NodeInfo,
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
    info: *mut NodeInfo,
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
