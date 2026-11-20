//! Planetary generation records shared by content and solar-system modes.

use crate::graphics::geometry::{Color, Point};
use crate::graphics::intersection::Stamp;

pub type PlanetDesc = PlanetDescriptor;
pub type NodeInfo = NodeRecord;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct ElementEntry {
    pub element_type: u8,
    pub density: u8,
}

#[derive(Debug, Copy, Clone)]
pub struct PlanetFrame {
    pub type_: u8,
    pub base_tectonics: u8,
    pub atmo_and_density: u8,
    pub useful_elements: [ElementEntry; 8usize],
    pub cmap_instance: &'static std::ffi::CStr,
    pub xlat_tab_instance: &'static std::ffi::CStr,
    pub num_faults: u16,
    pub fault_depth: i16,
    pub num_blemishes: u16,
    pub base_elevation: i16,
}

impl PlanetFrame {
    pub const EMPTY: Self = Self {
        type_: 0,
        base_tectonics: 0,
        atmo_and_density: 0,
        useful_elements: [ElementEntry {
            element_type: 0,
            density: 0,
        }; 8],
        cmap_instance: c"",
        xlat_tab_instance: c"",
        num_faults: 0,
        fault_depth: 0,
        num_blemishes: 0,
        base_elevation: 0,
    };
}

impl Default for PlanetFrame {
    fn default() -> Self {
        Self::EMPTY
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct PlanetInfo {
    pub axial_tilt: i16,
    pub tectonics: u16,
    pub weather: u16,
    pub planet_density: u16,
    pub planet_radius: u16,
    pub gravity: u16,
    pub temperature: i16,
    pub rotation_period: u16,
    pub atmo_density: u16,
    pub life_chance: i16,
    pub planet_to_sun_dist: u16,
    pub plan_data_ptr: *const PlanetFrame,
    pub scan_seed: [u32; 3usize],
    pub scan_retrieve_mask: [u32; 3usize],
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
        }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Default)]
pub struct SystemInfo {
    pub star_size: u8,
    pub star_intensity: u8,
    pub star_energy: u16,
    pub planet_info: PlanetInfo,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Default)]
pub struct PlanetDescriptor {
    pub rand_seed: u32,
    pub data_index: u8,
    pub num_planets: u8,
    pub radius: i16,
    pub location: Point,
    pub temp_color: Color,
    pub next_index: u16,
    pub image: Stamp,
    pub prev_desc: *mut PlanetDesc,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Default)]
pub struct NodeRecord {
    pub loc_pt: Point,
    pub density: u16,
    pub type_: u16,
}
