//! Owned mutable generation data with borrowed immutable planet definitions.

pub mod report;
mod snapshot;
pub use report::ReportSlot;
pub use snapshot::{SolarBodySnapshot, SolarSnapshot};

#[cfg(test)]
mod ownership_tests;

use super::planet::{PlanetDesc, PlanetFrame, PlanetInfo, SystemInfo};
use crate::graphics::{drawable_abi::LoadedDrawable, native_abi::POINT};
use crate::math::RandomContext;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Body {
    Planet(u8),
    Moon { planet: u8, moon: u8 },
}

impl Body {
    pub fn planet(self) -> usize {
        match self {
            Self::Planet(planet) | Self::Moon { planet, .. } => usize::from(planet),
        }
    }

    pub fn persistence_moon(self) -> u8 {
        match self {
            Self::Planet(_) => 0,
            Self::Moon { moon, .. } => moon
                .checked_add(1)
                .unwrap_or_else(|| panic!("moon index exceeds persistence encoding")),
        }
    }

    pub fn saved_orbit(self) -> u8 {
        match self {
            Self::Planet(_) => 1,
            Self::Moon { moon, .. } => moon
                .checked_add(2)
                .unwrap_or_else(|| panic!("moon index exceeds saved-orbit encoding")),
        }
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct SolarFlight {
    pub max_ship_speed: u8,
    pub turn_wait: u8,
}

pub struct SolarSys<'a> {
    pub sun: PlanetDesc,
    pub planets: [PlanetDesc; 16],
    pub moons: [PlanetDesc; 4],
    pub info: SystemInfo,
    pub rng: RandomContext,
    pub frames: [Option<LoadedDrawable>; 6],
    pub report: ReportSlot,
    plan_data: &'a [PlanetFrame],
    star_type: u8,
    star_index: u8,
}

pub struct OrbitalData<'a> {
    pub rng: &'a mut RandomContext,
    pub plan_data: &'a [PlanetFrame],
    pub star_size: u8,
    pub star_type: u8,
    pub parent_radius: Option<i16>,
    pub info: &'a mut SystemInfo,
    pub world: &'a PlanetDesc,
}

impl<'a> SolarSys<'a> {
    pub fn new(
        plan_data: &'a [PlanetFrame],
        star_type: u8,
        star_index: u8,
        rng: RandomContext,
    ) -> Self {
        Self {
            sun: PlanetDesc::default(),
            planets: [PlanetDesc::default(); 16],
            moons: [PlanetDesc::default(); 4],
            info: SystemInfo::default(),
            rng,
            frames: std::array::from_fn(|_| None),
            report: ReportSlot::default(),
            plan_data,
            star_type,
            star_index,
        }
    }

    pub fn star_index(&self) -> u8 {
        self.star_index
    }

    pub fn layout(
        &mut self,
        run: impl FnOnce(&mut RandomContext, &[PlanetFrame], u8, u8, &mut [PlanetDesc; 16]) -> u8,
    ) {
        self.sun.num_planets = run(
            &mut self.rng,
            self.plan_data,
            self.sun.data_index,
            self.star_type >> 5,
            &mut self.planets,
        );
    }

    pub fn layout_moons<R>(
        &mut self,
        planet: u8,
        run: impl FnOnce(&mut RandomContext, u8, u8, &PlanetDesc, &mut [PlanetDesc; 4]) -> R,
    ) -> R {
        run(
            &mut self.rng,
            self.sun.data_index,
            self.star_type >> 5,
            &self.planets[usize::from(planet)],
            &mut self.moons,
        )
    }

    pub fn body_data_index(&self, body: Body) -> u8 {
        self.descriptor(body).data_index
    }

    pub fn orbital_location(&self, body: Body) -> POINT {
        self.planets[body.planet()].location
    }

    pub fn planet_data_index(&self, planet: usize) -> u8 {
        self.planets[planet].data_index
    }

    pub fn descriptor(&self, body: Body) -> &PlanetDesc {
        match body {
            Body::Planet(planet) => &self.planets[usize::from(planet)],
            Body::Moon { moon, .. } => &self.moons[usize::from(moon)],
        }
    }

    pub fn descriptor_mut(&mut self, body: Body) -> &mut PlanetDesc {
        match body {
            Body::Planet(planet) => &mut self.planets[usize::from(planet)],
            Body::Moon { moon, .. } => &mut self.moons[usize::from(moon)],
        }
    }

    pub fn orbital<R>(&mut self, body: Body, run: impl FnOnce(OrbitalData<'_>) -> R) -> R {
        let (world, parent_radius) = match body {
            Body::Planet(planet) => (&self.planets[usize::from(planet)], None),
            Body::Moon { planet, moon } => (
                &self.moons[usize::from(moon)],
                Some(self.planets[usize::from(planet)].radius),
            ),
        };
        run(OrbitalData {
            rng: &mut self.rng,
            plan_data: self.plan_data,
            star_size: self.sun.data_index,
            star_type: self.star_type,
            parent_radius,
            info: &mut self.info,
            world,
        })
    }

    pub fn scan<R>(
        &mut self,
        body: Body,
        run: impl FnOnce(&mut RandomContext, &SystemInfo, &PlanetFrame) -> R,
    ) -> R {
        let frame = usize::from(self.body_data_index(body) & 0x7f);
        run(&mut self.rng, &self.info, &self.plan_data[frame])
    }

    pub fn system_info(&mut self) -> &mut SystemInfo {
        &mut self.info
    }

    pub fn planet_info(&mut self) -> &mut PlanetInfo {
        &mut self.info.planet_info
    }

    pub fn scene<R>(
        &mut self,
        run: impl FnOnce(&mut PlanetInfo, &mut Option<LoadedDrawable>) -> R,
    ) -> R {
        run(&mut self.info.planet_info, &mut self.frames[1])
    }
}

/// How a report string table is consumed after presentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportDisposition {
    Once,
    Cycle,
}
