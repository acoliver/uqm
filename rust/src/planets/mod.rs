//! Planet generation and physics, ported from `sc2/src/uqm/planets`.
//!
//! The functions here are plain Rust over explicit inputs (an RNG stream, the
//! planet table, star description) so they can be tested without C globals.

pub mod calc;
pub mod consts;
#[cfg(any(not(test), feature = "linked_c_archive"))]
pub mod ffi;
pub mod orbit_tables;
pub mod orbits;
pub mod surface;
