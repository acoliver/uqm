//! Solar-system state, native resources, drawing and flight from `solarsys.c`.
//!
//! Resources retain their native graphics pointers while legacy callers remain.
//! Exploration owns each visit and supplies it explicitly to its callers.

#[path = "solarsys/drawing.rs"]
pub(crate) mod drawing;
#[path = "solarsys/flight.rs"]
mod flight;
#[path = "solarsys/lifecycle.rs"]
pub mod lifecycle;
mod planet_info;
#[path = "solarsys/resources.rs"]
pub(crate) mod resources;

pub mod site;
