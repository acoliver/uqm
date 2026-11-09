//! Short-lived access to the engine records used by system generators.

pub mod report;

use super::planet::{PlanetDesc, PlanetFrame, PlanetInfo, SystemInfo};
use crate::graphics::native_abi::{FRAME, POINT};
use crate::math::RandomContext;
use std::marker::PhantomData;
use std::ptr::NonNull;

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
}

pub struct GenerationRecords {
    pub sun: NonNull<PlanetDesc>,
    pub planets: NonNull<[PlanetDesc; 16]>,
    pub moons: NonNull<[PlanetDesc; 4]>,
    pub info: NonNull<SystemInfo>,
    pub scene: NonNull<FRAME>,
}

pub struct SolarSys<'a> {
    records: GenerationRecords,
    rng: NonNull<RandomContext>,
    plan_data: &'a [PlanetFrame],
    star_type: u8,
    star_index: u8,
    exclusive: PhantomData<&'a mut ()>,
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
    /// # Safety
    /// All records and RNG must be live, disjoint and exclusively available for
    /// this view's lifetime. External effects may access them only between
    /// callbacks, after all references returned by the view have expired.
    pub unsafe fn from_records(
        records: GenerationRecords,
        rng: NonNull<RandomContext>,
        plan_data: &'a [PlanetFrame],
        star_type: u8,
        star_index: u8,
    ) -> Self {
        Self {
            records,
            rng,
            plan_data,
            star_type,
            star_index,
            exclusive: PhantomData,
        }
    }

    pub fn star_index(&self) -> u8 {
        self.star_index
    }

    pub fn layout(
        &mut self,
        run: impl FnOnce(&mut RandomContext, &[PlanetFrame], u8, u8, &mut [PlanetDesc; 16]) -> u8,
    ) {
        // SAFETY: construction retains disjoint records, and no borrow escapes the callback.
        unsafe {
            let sun = self.records.sun.as_mut();
            sun.num_planets = run(
                self.rng.as_mut(),
                self.plan_data,
                sun.data_index,
                self.star_type >> 5,
                self.records.planets.as_mut(),
            );
        }
    }

    pub fn layout_moons<R>(
        &mut self,
        planet: u8,
        run: impl FnOnce(&mut RandomContext, u8, u8, &PlanetDesc, &mut [PlanetDesc; 4]) -> R,
    ) -> R {
        // SAFETY: the arrays are disjoint and these borrows end with the callback.
        unsafe {
            run(
                self.rng.as_mut(),
                self.records.sun.as_ref().data_index,
                self.star_type >> 5,
                &self.records.planets.as_ref()[usize::from(planet)],
                self.records.moons.as_mut(),
            )
        }
    }

    pub fn body_data_index(&self, body: Body) -> u8 {
        self.world(body).data_index
    }

    pub fn orbital_location(&self, body: Body) -> POINT {
        self.world(match body {
            Body::Planet(_) => body,
            Body::Moon { planet, .. } => Body::Planet(planet),
        })
        .location
    }

    pub fn planet_data_index(&self, planet: usize) -> u8 {
        // SAFETY: construction retains the array, and this is a shared read.
        unsafe { self.records.planets.as_ref()[planet].data_index }
    }

    fn world(&self, body: Body) -> &PlanetDesc {
        // SAFETY: construction retains both arrays; indices are bounds checked.
        unsafe {
            match body {
                Body::Planet(planet) => &self.records.planets.as_ref()[usize::from(planet)],
                Body::Moon { moon, .. } => &self.records.moons.as_ref()[usize::from(moon)],
            }
        }
    }

    pub fn orbital<R>(&mut self, body: Body, run: impl FnOnce(OrbitalData<'_>) -> R) -> R {
        // SAFETY: all records and RNG are disjoint; no reference escapes the callback.
        unsafe {
            let planets = self.records.planets.as_ref();
            let (world, parent_radius) = match body {
                Body::Planet(planet) => (&planets[usize::from(planet)], None),
                Body::Moon { planet, moon } => (
                    &self.records.moons.as_ref()[usize::from(moon)],
                    Some(planets[usize::from(planet)].radius),
                ),
            };
            run(OrbitalData {
                rng: self.rng.as_mut(),
                plan_data: self.plan_data,
                star_size: self.records.sun.as_ref().data_index,
                star_type: self.star_type,
                parent_radius,
                info: self.records.info.as_mut(),
                world,
            })
        }
    }

    pub fn scan<R>(
        &mut self,
        body: Body,
        run: impl FnOnce(&mut RandomContext, &SystemInfo, &PlanetFrame) -> R,
    ) -> R {
        let frame = usize::from(self.body_data_index(body) & 0x7f);
        // SAFETY: the RNG and info are disjoint; the frame table is retained.
        unsafe {
            run(
                self.rng.as_mut(),
                self.records.info.as_ref(),
                &self.plan_data[frame],
            )
        }
    }

    pub fn system_info(&mut self) -> &mut SystemInfo {
        // SAFETY: the returned borrow excludes other access through this view.
        unsafe { self.records.info.as_mut() }
    }

    pub fn planet_info(&mut self) -> &mut PlanetInfo {
        // SAFETY: this borrow excludes other access through the view until it ends.
        unsafe { &mut self.records.info.as_mut().planet_info }
    }

    pub fn scene<R>(&mut self, run: impl FnOnce(&mut PlanetInfo, &mut FRAME) -> R) -> R {
        // SAFETY: info and the frame slot are disjoint and exclusively retained.
        unsafe {
            run(
                &mut self.records.info.as_mut().planet_info,
                self.records.scene.as_mut(),
            )
        }
    }
}

/// How a report string table is consumed after presentation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportDisposition {
    Once,
    Cycle,
}
