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

pub static mut SISIPFRAME: *mut c::FRAME_DESC = ptr::null_mut();
pub static mut SUN_FRAME: *mut c::FRAME_DESC = ptr::null_mut();
pub static mut ORBITAL_FRAME: *mut c::FRAME_DESC = ptr::null_mut();
#[no_mangle]
pub static mut SpaceJunkFrame: *mut c::FRAME_DESC = ptr::null_mut();
pub static mut ORBITAL_CMAP: *mut c_void = ptr::null_mut();
pub static mut SUN_CMAP: *mut c_void = ptr::null_mut();
#[no_mangle]
pub static mut SpaceMusic: *mut c_void = ptr::null_mut();
static mut STARS_FRAME: *mut c::FRAME_DESC = ptr::null_mut();
static mut SOLAR_SYS_FRAME: *mut c::FRAME_DESC = ptr::null_mut();

pub mod site;
