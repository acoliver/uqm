//! Planet generation and physics, ported from `sc2/src/uqm/planets`.
//!
//! The functions here are plain Rust over explicit inputs (an RNG stream, the
//! planet table, star description) so they can be tested without C globals.

pub mod solar_system;

pub mod c_api;
pub mod device_effects;
pub use uqm_content::planets::generate;
pub mod render;
pub mod starmap;
pub use uqm_content::planets::tables;
pub mod topography;
pub mod universe_units;

pub mod devices;

pub mod cargo;

pub mod report;

pub mod roster;

pub mod orbit;
pub mod plangen;
pub mod scan;
pub mod sphere;

pub mod solar_coords;
pub mod solarsys;

pub mod pstarmap;
