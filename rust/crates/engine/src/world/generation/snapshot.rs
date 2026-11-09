//! Value-only input and element-hook observations of a solar frame.

use super::{Body, PlanetDesc};
use crate::{graphics::native_abi::POINT, world::StarId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SolarBodySnapshot {
    pub data_index: u8,
    pub satellites: u8,
    pub display_origin: POINT,
}

impl From<&PlanetDesc> for SolarBodySnapshot {
    fn from(body: &PlanetDesc) -> Self {
        Self {
            data_index: body.data_index,
            satellites: body.num_planets,
            display_origin: body.image.origin,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SolarSnapshot {
    pub star: StarId,
    pub generator_index: u8,
    pub retrieval_masks: [u32; 3],
    pub screen: (i32, i32),
    pub in_ip_flight: bool,
    pub in_orbit: bool,
    pub wait_intersect: u16,
    pub inner_planet: Option<u8>,
    pub selected: Option<Body>,
    pub zoom_radius: i16,
    pub sun_location: POINT,
    pub planet_count: u8,
    pub planets: [SolarBodySnapshot; 16],
    pub moons: [SolarBodySnapshot; 4],
}
