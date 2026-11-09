//! Planet generation and physics, ported from `sc2/src/uqm/planets`.
//!
//! The functions here are plain Rust over explicit inputs (an RNG stream, the
//! planet table, star description) so they can be tested without C globals.

pub mod solar_system;

#[cfg(not(test))]
pub mod c_api;
pub(crate) mod device_effects;
pub mod generate;
pub mod render;
pub mod starmap;
pub mod tables;
pub mod topography;
pub mod universe_units;

#[cfg(not(test))]
#[cfg(not(test))]
pub mod devices;

pub mod cargo;

pub mod report;

pub mod roster;

pub mod orbit;
pub mod plangen;
pub mod scan;
pub mod sphere;

pub(crate) mod solar_coords;
#[cfg(not(test))]
pub mod solarsys;

pub mod pstarmap;
