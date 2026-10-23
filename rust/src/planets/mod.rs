//! Planet generation and physics, ported from `sc2/src/uqm/planets`.
//!
//! The functions here are plain Rust over explicit inputs (an RNG stream, the
//! planet table, star description) so they can be tested without C globals.

#[cfg(any(not(test), feature = "linked_c_archive"))]
pub mod c_api;
pub mod calc;
pub mod consts;
#[cfg(any(not(test), feature = "linked_c_archive"))]
pub mod ffi;
pub mod generate;
pub mod orbit_tables;
pub mod orbits;
pub mod render;
pub mod starmap;
pub mod surface;
pub mod tables;
pub mod topography;
pub mod universe_units;

#[cfg(any(not(test), feature = "linked_c_archive"))]
pub mod starmap_cursor;

#[cfg(not(test))]
pub mod devices;

pub mod cargo;

pub mod report;

pub mod roster;

pub mod orbit;
pub mod scan;
pub mod sphere;
