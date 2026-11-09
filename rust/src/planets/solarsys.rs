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

static mut SCALE_RECT: c::RECT = unsafe { std::mem::zeroed() };

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
pub static mut EncounterGroup: c::BYTE = 0;

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

/// # Safety
/// Game-thread caller; descriptor pointers must refer to the live system or a
/// complete, readable system description. Output pointers must be writable.
#[no_mangle]
pub unsafe extern "C" fn GetRandomSeedForStar(star: *const c::STAR_DESC) -> c::DWORD {
    unsafe {
        let point = (*star).star_pt;
        ((point.y as u16 as u32) << 16) | (point.x as u16 as u32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn random_seed_packs_signed_coordinates_as_words() {
        let star = c::STAR_DESC {
            star_pt: c::POINT { x: -2, y: 0x1234 },
            ..unsafe { std::mem::zeroed() }
        };
        assert_eq!(unsafe { GetRandomSeedForStar(&star) }, 0x1234_fffe);
    }
}

pub mod site;
