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

const STATUS_WIDTH: i32 = 64;
const SPACE_RIGHT_MARGIN: i32 = 14;
const SPACE_BOTTOM_MARGIN: i32 = 13;

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

#[no_mangle]
pub extern "C" fn locationToDisplay(pt: c::POINT, scale_radius: c::SIZE) -> c::POINT {
    let (width, height) = sis_dimensions();
    super::solar_coords::to_display(pt, scale_radius, width, height)
}

#[no_mangle]
pub extern "C" fn displayToLocation(pt: c::POINT, scale_radius: c::SIZE) -> c::POINT {
    let (width, height) = sis_dimensions();
    super::solar_coords::to_location(pt, scale_radius, width, height)
}

unsafe fn dimensions() -> (i32, i32) {
    extern "C" {
        static mut ScreenWidth: i32;
        static mut ScreenHeight: i32;
    }
    (
        ptr::addr_of!(ScreenWidth).read(),
        ptr::addr_of!(ScreenHeight).read(),
    )
}

fn sis_dimensions() -> (i32, i32) {
    let (width, height) = unsafe { dimensions() };
    (
        width - STATUS_WIDTH - SPACE_RIGHT_MARGIN,
        height - SPACE_BOTTOM_MARGIN,
    )
}

fn display_factor() -> i32 {
    (sis_dimensions().0 >> 1) - 8
}

pub mod site;
