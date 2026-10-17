//! C-ABI entry points for the planet generation code that used to live in
//! `planets/{calc,orbits,surface}.c`.
//!
//! Each export is a thin adapter: it reads the C-owned state the C original
//! read (`SysGenRNG`, `pSolarSysState`, `CurStarDescPtr`, `PlanData`), hands it
//! to the plain Rust port as arguments and writes the results back through the
//! caller's pointers. All of it runs on the game thread, the only thread that
//! touches this state.

use crate::c_bindings::global_state::{
    PlanetFrame, NODE_INFO, PLANET_DESC, PLANET_INFO, POINT, SOLARSYS_STATE, SYSTEM_INFO,
};
use crate::math::RandomContext;
use crate::planets::calc::{self, AnalysisSite, StarDescription};
use crate::planets::orbit_tables::NUMBER_OF_PLANET_TYPES;
use crate::planets::orbits::{self, OrbitRequest};
use crate::planets::surface;
use crate::planets::tables::{ELEMENT_ARRAY, PLANET_ARRAY};
use crate::planets::topography;
use crate::savegame::runtime::star::CStarDesc;

/// `PlanData` (formerly defined in `surface.c`): the planet-frame table.
#[export_name = "PlanData"]
pub static mut PLAN_DATA: *const PlanetFrame = PLANET_ARRAY.0.as_ptr();

/// `Elements` (formerly defined in `surface.c`): the element category table.
#[export_name = "Elements"]
pub static mut ELEMENTS: *const u8 = ELEMENT_ARRAY.as_ptr();

extern "C" {
    #[link_name = "SysGenRNG"]
    static mut SYS_GEN_RNG: *mut RandomContext;
    #[link_name = "CurStarDescPtr"]
    static mut CUR_STAR_DESC_PTR: *const CStarDesc;
}

/// `*SysGenRNG`: the stream shared with the rest of the system generators.
///
/// # Safety
/// `SysGenRNG` must have been created, and no other reference to the stream
/// may be live.
pub(crate) unsafe fn sys_gen_rng<'a>() -> &'a mut RandomContext {
    &mut *std::ptr::addr_of!(SYS_GEN_RNG).read()
}

/// `PlanData[0..NUMBER_OF_PLANET_TYPES]`.
///
/// # Safety
/// `PlanData` must point at the planet-frame table.
pub(crate) unsafe fn plan_data<'a>() -> &'a [PlanetFrame] {
    std::slice::from_raw_parts(std::ptr::addr_of!(PLAN_DATA).read(), NUMBER_OF_PLANET_TYPES)
}

/// A caller that does not want the node still gets the draws, so the stream
/// stays in step: C wrote into a local in that case.
pub(crate) unsafe fn node_or(info: *mut NODE_INFO, spare: &mut NODE_INFO) -> &mut NODE_INFO {
    if info.is_null() {
        spare
    } else {
        &mut *info
    }
}

pub(crate) fn blank_node() -> NODE_INFO {
    // SAFETY: `NODE_INFO` is plain integers.
    unsafe { std::mem::zeroed() }
}

/// `*CurStarDescPtr`'s `Type` byte.
///
/// # Safety
/// `CurStarDescPtr` must point at a valid star description.
pub(crate) unsafe fn current_star_type() -> u8 {
    (*std::ptr::addr_of!(CUR_STAR_DESC_PTR).read()).star_type
}

/// The star and the orbit `DoPlanetaryAnalysis` reads for `desc`: the sun and
/// the star description come from `pSolarSysState` and `CurStarDescPtr`, the
/// parent radius from `desc.pPrevDesc`.
///
/// # Safety
/// As [`do_planetary_analysis_c`].
pub(crate) unsafe fn analysis_context(
    sys_info: *mut SYSTEM_INFO,
    desc: *const PLANET_DESC,
) -> (StarDescription, AnalysisSite) {
    let state = crate::c_bindings::solar_system::current();
    let sun = std::ptr::addr_of_mut!((*state).SunDesc[0]);
    let parent = (*desc).pPrevDesc;
    let parent_radius = if parent == sun {
        None
    } else {
        Some((*parent).radius)
    };
    let star = StarDescription {
        size: (*sun).data_index,
        star_type: current_star_type(),
    };
    let site = AnalysisSite {
        parent_radius,
        is_orbit_entry: sys_info == std::ptr::addr_of_mut!((*state).SysInfo),
    };
    (star, site)
}

/// C: `UWORD CalcGravity (const PLANET_INFO *PlanetInfoPtr)`.
///
/// # Safety
/// `info` must point at a valid `PLANET_INFO`.
#[export_name = "CalcGravity"]
pub unsafe extern "C" fn calc_gravity_c(info: *const PLANET_INFO) -> u16 {
    calc::calc_gravity(&*info)
}

/// C: `void DoPlanetaryAnalysis (SYSTEM_INFO *SysInfoPtr, PLANET_DESC *pPlanetDesc)`.
///
/// The star comes from `pSolarSysState->SunDesc[0]` and `CurStarDescPtr`; the
/// orbit radius from the parent description, as in C.
///
/// # Safety
/// Both pointers, `pSolarSysState`, `CurStarDescPtr`, `SysGenRNG` and
/// `PlanData` must be valid, and `desc.pPrevDesc` must point at either
/// `pSolarSysState->SunDesc[0]` or a valid description.
#[export_name = "DoPlanetaryAnalysis"]
pub unsafe extern "C" fn do_planetary_analysis_c(
    sys_info: *mut SYSTEM_INFO,
    desc: *mut PLANET_DESC,
) {
    let (star, site) = analysis_context(sys_info, desc);
    calc::do_planetary_analysis(
        sys_gen_rng(),
        plan_data(),
        star,
        site,
        &mut *sys_info,
        &*desc,
    );
}

/// C: `COUNT GenerateMineralDeposits (const SYSTEM_INFO*, COUNT whichDeposit, NODE_INFO*)`.
///
/// # Safety
/// `sys_info` must be a valid analysed `SYSTEM_INFO`; `info` may be NULL.
#[export_name = "GenerateMineralDeposits"]
pub unsafe extern "C" fn generate_mineral_deposits_c(
    sys_info: *const SYSTEM_INFO,
    which_deposit: u16,
    info: *mut NODE_INFO,
) -> u16 {
    let mut spare = blank_node();
    surface::generate_mineral_deposits(
        sys_gen_rng(),
        &*sys_info,
        which_deposit,
        node_or(info, &mut spare),
    )
}

/// C: `COUNT GenerateLifeForms (const SYSTEM_INFO*, COUNT whichLife, NODE_INFO*)`.
///
/// # Safety
/// `sys_info` must be a valid analysed `SYSTEM_INFO`; `info` may be NULL.
#[export_name = "GenerateLifeForms"]
pub unsafe extern "C" fn generate_life_forms_c(
    sys_info: *const SYSTEM_INFO,
    which_life: u16,
    info: *mut NODE_INFO,
) -> u16 {
    let mut spare = blank_node();
    surface::generate_life_forms(
        sys_gen_rng(),
        &*sys_info,
        which_life,
        node_or(info, &mut spare),
    )
}

/// C: `COUNT GeneratePresetLife (const SYSTEM_INFO*, const SBYTE *lifeTypes,
/// COUNT whichLife, NODE_INFO*)`; `life_types` ends at the first negative entry.
///
/// # Safety
/// `life_types` must be a `-1` terminated array; `info` may be NULL.
#[export_name = "GeneratePresetLife"]
pub unsafe extern "C" fn generate_preset_life_c(
    sys_info: *const SYSTEM_INFO,
    life_types: *const i8,
    which_life: u16,
    info: *mut NODE_INFO,
) -> u16 {
    let mut len = 0;
    while *life_types.add(len) >= 0 {
        len += 1;
    }
    let types = std::slice::from_raw_parts(life_types, len);
    let mut spare = blank_node();
    surface::generate_preset_life(
        sys_gen_rng(),
        &*sys_info,
        types,
        which_life,
        node_or(info, &mut spare),
    )
}

/// C: `COUNT GenerateRandomNodes (const SYSTEM_INFO*, COUNT scan, COUNT numNodes,
/// COUNT type, COUNT whichNode, NODE_INFO*)`.
///
/// # Safety
/// `sys_info` must be a valid analysed `SYSTEM_INFO`; `info` may be NULL.
#[export_name = "GenerateRandomNodes"]
pub unsafe extern "C" fn generate_random_nodes_c(
    sys_info: *const SYSTEM_INFO,
    scan: u16,
    num_nodes: u16,
    node_type: u16,
    which_node: u16,
    info: *mut NODE_INFO,
) -> u16 {
    let mut spare = blank_node();
    surface::generate_random_nodes(
        sys_gen_rng(),
        &*sys_info,
        usize::from(scan),
        num_nodes,
        node_type,
        which_node,
        node_or(info, &mut spare),
    )
}

/// C: `void GenerateRandomLocation (POINT *loc)`.
///
/// # Safety
/// `loc` must be valid for writes.
#[export_name = "GenerateRandomLocation"]
pub unsafe extern "C" fn generate_random_location_c(loc: *mut POINT) {
    surface::generate_random_location(sys_gen_rng(), &mut *loc);
}

/// C `RECT`: `{ POINT corner; EXTENT extent; }`, all `SWORD`s.
#[repr(C)]
pub struct CRect {
    pub corner: [i16; 2],
    pub extent: [i16; 2],
}

/// C: `void DeltaTopography (COUNT num_iterations, SBYTE *DepthArray, RECT *pRect,
/// SIZE depth_delta)`.
///
/// Only the extent of `pRect` matters; `DepthArray` holds `width * height`
/// cells.
///
/// # Safety
/// `depth` must be valid for reads and writes of `extent.width * extent.height`
/// cells, `rect` must be valid, and `SysGenRNG` must be valid.
#[export_name = "DeltaTopography"]
pub unsafe extern "C" fn delta_topography_c(
    num_iterations: u16,
    depth: *mut i8,
    rect: *const CRect,
    depth_delta: i16,
) {
    let [width, height] = (*rect).extent;
    let cells = usize::try_from(i32::from(width) * i32::from(height))
        .expect("topography map extent is negative");
    topography::delta_topography(
        sys_gen_rng(),
        num_iterations,
        std::slice::from_raw_parts_mut(depth, cells),
        width,
        height,
        depth_delta,
    );
}

/// `MAX_GENERATED_PLANETS`: the most worlds `FillOrbits` draws for itself.
const MAX_GENERATED_PLANETS: usize = 9;

/// C: `void FillOrbits (SOLARSYS_STATE*, BYTE NumPlanets, PLANET_DESC *pBaseDesc,
/// BOOLEAN TypesDefined)`.
///
/// Moons are laid out when `base` is `system->MoonDesc`. A `NumPlanets` of
/// `!0` draws the count from the stream and stores it in
/// `system->SunDesc[0].NumPlanets`.
///
/// # Safety
/// `system` must be the live solar system state, `base` must hold the number of
/// worlds requested (`MAX_GENERATED_PLANETS` when `!0`), and `CurStarDescPtr`
/// and `SysGenRNG` must be valid.
#[export_name = "FillOrbits"]
pub unsafe extern "C" fn fill_orbits_c(
    system: *mut SOLARSYS_STATE,
    num_planets: u8,
    base: *mut PLANET_DESC,
    types_defined: u32,
) {
    let slots = if num_planets == !0 {
        MAX_GENERATED_PLANETS
    } else {
        usize::from(num_planets)
    };
    let request = OrbitRequest {
        star_size: (*system).SunDesc[0].data_index,
        star_color: crate::planets::consts::star_color(current_star_type()),
        num_planets,
        generating_moons: base == (*system).MoonDesc.as_mut_ptr(),
        types_defined: types_defined != 0,
    };
    let descs = std::slice::from_raw_parts_mut(base, slots);
    if let Some(drawn) = orbits::fill_orbits(sys_gen_rng(), request, descs) {
        (*system).SunDesc[0].NumPlanets = drawn;
    }
}
