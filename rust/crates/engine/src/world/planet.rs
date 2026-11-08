//! Planetary generation records shared by content and solar-system modes.

use crate::graphics::native_abi::{
    Color, BYTE, COUNT, DWORD, POINT, RESOURCE, SIZE, STAMP, STRING, UWORD,
};

pub type PlanetDesc = PlanetDescriptor;
pub type NodeInfo = NodeRecord;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct ElementEntry {
    pub element_type: BYTE,
    pub density: BYTE,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct PlanetFrame {
    pub type_: BYTE,
    pub base_tectonics: BYTE,
    pub atmo_and_density: BYTE,
    pub useful_elements: [ElementEntry; 8usize],
    pub cmap_instance: RESOURCE,
    pub xlat_tab_instance: RESOURCE,
    pub num_faults: COUNT,
    pub fault_depth: SIZE,
    pub num_blemishes: COUNT,
    pub base_elevation: SIZE,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct PlanetInfo {
    pub axial_tilt: SIZE,
    pub tectonics: UWORD,
    pub weather: UWORD,
    pub planet_density: UWORD,
    pub planet_radius: UWORD,
    pub gravity: UWORD,
    pub temperature: SIZE,
    pub rotation_period: UWORD,
    pub atmo_density: UWORD,
    pub life_chance: SIZE,
    pub planet_to_sun_dist: UWORD,
    pub plan_data_ptr: *const PlanetFrame,
    pub scan_seed: [DWORD; 3usize],
    pub scan_retrieve_mask: [DWORD; 3usize],
    pub discovery_string: STRING,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct SystemInfo {
    pub star_size: BYTE,
    pub star_intensity: BYTE,
    pub star_energy: UWORD,
    pub planet_info: PlanetInfo,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct PlanetDescriptor {
    pub rand_seed: DWORD,
    pub data_index: BYTE,
    pub num_planets: BYTE,
    pub radius: SIZE,
    pub location: POINT,
    pub temp_color: Color,
    pub next_index: COUNT,
    pub image: STAMP,
    pub prev_desc: *mut PlanetDesc,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Default)]
pub struct NodeRecord {
    pub loc_pt: POINT,
    pub density: COUNT,
    pub type_: COUNT,
}
