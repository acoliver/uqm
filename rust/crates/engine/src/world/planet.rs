//! Planetary generation records shared by content and solar-system modes.
#![allow(non_camel_case_types, non_snake_case)]

use crate::graphics::native_abi::{
    Color, BYTE, COUNT, DWORD, FONT, FRAME, POINT, RESOURCE, SIZE, STAMP, STRING, UWORD,
};

pub type PLANET_DESC = planet_desc;
pub type NODE_INFO = node_info;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct ELEMENT_ENTRY {
    pub ElementType: BYTE,
    pub Density: BYTE,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct PlanetFrame {
    pub Type: BYTE,
    pub BaseTectonics: BYTE,
    pub AtmoAndDensity: BYTE,
    pub UsefulElements: [ELEMENT_ENTRY; 8usize],
    pub CMapInstance: RESOURCE,
    pub XlatTabInstance: RESOURCE,
    pub num_faults: COUNT,
    pub fault_depth: SIZE,
    pub num_blemishes: COUNT,
    pub base_elevation: SIZE,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct PLANET_INFO {
    pub AxialTilt: SIZE,
    pub Tectonics: UWORD,
    pub Weather: UWORD,
    pub PlanetDensity: UWORD,
    pub PlanetRadius: UWORD,
    pub SurfaceGravity: UWORD,
    pub SurfaceTemperature: SIZE,
    pub RotationPeriod: UWORD,
    pub AtmoDensity: UWORD,
    pub LifeChance: SIZE,
    pub PlanetToSunDist: UWORD,
    pub PlanDataPtr: *const PlanetFrame,
    pub ScanSeed: [DWORD; 3usize],
    pub ScanRetrieveMask: [DWORD; 3usize],
    pub DiscoveryString: STRING,
    pub LanderFont: FONT,
    pub LanderFontEff: FRAME,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct SYSTEM_INFO {
    pub StarSize: BYTE,
    pub StarIntensity: BYTE,
    pub StarEnergy: UWORD,
    pub PlanetInfo: PLANET_INFO,
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct planet_desc {
    pub rand_seed: DWORD,
    pub data_index: BYTE,
    pub NumPlanets: BYTE,
    pub radius: SIZE,
    pub location: POINT,
    pub temp_color: Color,
    pub NextIndex: COUNT,
    pub image: STAMP,
    pub pPrevDesc: *mut PLANET_DESC,
}

#[repr(C)]
#[derive(Debug, Copy, Clone, Default)]
pub struct node_info {
    pub loc_pt: POINT,
    pub density: COUNT,
    pub type_: COUNT,
}
