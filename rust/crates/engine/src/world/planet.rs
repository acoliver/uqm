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

impl Default for PlanetInfo {
    fn default() -> Self {
        Self {
            axial_tilt: 0,
            tectonics: 0,
            weather: 0,
            planet_density: 0,
            planet_radius: 0,
            gravity: 0,
            temperature: 0,
            rotation_period: 0,
            atmo_density: 0,
            life_chance: 0,
            planet_to_sun_dist: 0,
            plan_data_ptr: std::ptr::null(),
            scan_seed: [0; 3],
            scan_retrieve_mask: [0; 3],
            discovery_string: std::ptr::null_mut(),
        }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Default)]
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
}

impl Default for PlanetDescriptor {
    fn default() -> Self {
        Self {
            rand_seed: 0,
            data_index: 0,
            num_planets: 0,
            radius: 0,
            location: POINT::default(),
            temp_color: Color::default(),
            next_index: 0,
            image: STAMP {
                origin: POINT::default(),
                frame: std::ptr::null_mut(),
            },
        }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Default)]
pub struct NodeRecord {
    pub loc_pt: POINT,
    pub density: COUNT,
    pub type_: COUNT,
}
