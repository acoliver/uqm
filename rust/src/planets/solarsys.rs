//! Solar-system state, native resources, drawing and flight from `solarsys.c`.
//!
//! Resources retain their native graphics pointers while legacy callers remain.
//! Exploration owns each visit and supplies it explicitly to its callers.

#[path = "solarsys/drawing.rs"]
pub(crate) mod drawing;
#[path = "solarsys/flight.rs"]
mod flight;
#[path = "solarsys/lifecycle.rs"]
pub(crate) mod lifecycle;
#[path = "solarsys/resources.rs"]
pub(crate) mod resources;

use crate::c_bindings::global_state as c;
use std::ffi::c_void;
use std::ptr;

pub mod site;
