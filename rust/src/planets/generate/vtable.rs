//! The C `GenerateFunctions` table (`generate.h`): one hook per generation or
//! pickup step, selected per star system by `getGenerateFunctions`.

use crate::c_bindings::assert_c_layout;
use crate::c_bindings::global_state::{
    GenerateFunctions as CGenerateFunctions, NODE_INFO, PLANET_DESC, SOLARSYS_STATE,
};
#[cfg(not(test))]
use crate::planets::generate::exports;

/// `bool (*)(SOLARSYS_STATE *)`: `initNpcs`, `reinitNpcs`, `uninitNpcs` and
/// `generatePlanets`.
pub type SystemHook = Option<unsafe extern "C" fn(*mut SOLARSYS_STATE) -> bool>;
/// `bool (*)(SOLARSYS_STATE *, PLANET_DESC *)`: `generateMoons`,
/// `generateOrbital`.
pub type WorldHook = Option<unsafe extern "C" fn(*mut SOLARSYS_STATE, *mut PLANET_DESC) -> bool>;
/// `bool (*)(const SOLARSYS_STATE *, const PLANET_DESC *)`: `generateName`.
pub type NameHook = Option<unsafe extern "C" fn(*const SOLARSYS_STATE, *const PLANET_DESC) -> bool>;
/// `COUNT (*)(const SOLARSYS_STATE *, const PLANET_DESC *, COUNT, NODE_INFO *)`.
pub type NodeCountHook = Option<
    unsafe extern "C" fn(*const SOLARSYS_STATE, *const PLANET_DESC, u16, *mut NODE_INFO) -> u16,
>;
/// `bool (*)(SOLARSYS_STATE *, PLANET_DESC *, COUNT)`: the three pickups.
pub type PickupHook =
    Option<unsafe extern "C" fn(*mut SOLARSYS_STATE, *mut PLANET_DESC, u16) -> bool>;

/// C `GenerateFunctions`: thirteen hook pointers in this order.
#[repr(C)]
pub struct GenerateFunctions {
    pub init_npcs: SystemHook,
    pub reinit_npcs: SystemHook,
    pub uninit_npcs: SystemHook,
    pub generate_planets: SystemHook,
    pub generate_moons: WorldHook,
    pub generate_name: NameHook,
    pub generate_orbital: WorldHook,
    pub generate_minerals: NodeCountHook,
    pub generate_energy: NodeCountHook,
    pub generate_life: NodeCountHook,
    pub pickup_minerals: PickupHook,
    pub pickup_energy: PickupHook,
    pub pickup_life: PickupHook,
}

assert_c_layout!(
    GenerateFunctions,
    CGenerateFunctions {
        init_npcs: initNpcs,
        reinit_npcs: reinitNpcs,
        uninit_npcs: uninitNpcs,
        generate_planets: generatePlanets,
        generate_moons: generateMoons,
        generate_name: generateName,
        generate_orbital: generateOrbital,
        generate_minerals: generateMinerals,
        generate_energy: generateEnergy,
        generate_life: generateLife,
        pickup_minerals: pickupMinerals,
        pickup_energy: pickupEnergy,
        pickup_life: pickupLife,
    }
);

/// The default hooks; a special system overrides some of them.
#[cfg(not(test))]
pub const DEFAULT_HOOKS: GenerateFunctions = GenerateFunctions {
    init_npcs: Some(exports::init_npcs_c),
    reinit_npcs: Some(exports::reinit_npcs_c),
    uninit_npcs: Some(exports::uninit_npcs_c),
    generate_planets: Some(exports::generate_planets_c),
    generate_moons: Some(exports::generate_moons_c),
    generate_name: Some(exports::generate_name_c),
    generate_orbital: Some(exports::generate_orbital_c),
    generate_minerals: Some(exports::generate_minerals_c),
    generate_energy: Some(exports::generate_energy_c),
    generate_life: Some(exports::generate_life_c),
    pickup_minerals: Some(exports::pickup_minerals_c),
    pickup_energy: Some(exports::pickup_energy_c),
    pickup_life: Some(exports::pickup_life_c),
};

/// `generateDefaultFunctions`: what a system without a special generator uses.
#[cfg(not(test))]
#[export_name = "generateDefaultFunctions"]
pub static GENERATE_DEFAULT_FUNCTIONS: GenerateFunctions = DEFAULT_HOOKS;
