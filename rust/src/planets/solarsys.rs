//! Solar-system state, native resources, drawing and flight from `solarsys.c`.
//!
//! Resources retain their native graphics pointers while legacy callers remain.
//! The lifecycle owns the current system; other modules obtain it
//! through `solar_system::current()`.

#[path = "solarsys/drawing.rs"]
mod drawing;
#[path = "solarsys/flight.rs"]
mod flight;
#[path = "solarsys/lifecycle.rs"]
mod lifecycle;
#[path = "solarsys/resources.rs"]
mod resources;

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
pub static mut SysGenRNG: *mut crate::math::RandomContext = ptr::null_mut();

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
        static ScreenWidth: i32;
        static ScreenHeight: i32;
    }
    (
        ptr::addr_of!(ScreenWidth).read(),
        ptr::addr_of!(ScreenHeight).read(),
    )
}

/// # Safety
/// Game-thread caller; descriptor pointers must refer to the live system or a
/// complete, readable system description. Output pointers must be writable.
#[no_mangle]
pub unsafe extern "C" fn planetOuterLocation(planet_i: c::COUNT) -> c::POINT {
    let system = crate::c_bindings::solar_system::current();
    let radius = (*system).SunDesc[0].radius;
    displayToLocation(
        (*system)
            .PlanetDesc
            .as_ptr()
            .add(usize::from(planet_i))
            .read()
            .image
            .origin,
        radius,
    )
}

/// # Safety
/// Game-thread caller; descriptor pointers must refer to the live system or a
/// complete, readable system description. Output pointers must be writable.
#[no_mangle]
pub unsafe extern "C" fn worldIsPlanet(
    solar_sys: *const c::SOLARSYS_STATE,
    world: *const c::PLANET_DESC,
) -> bool {
    ptr::eq((*world).pPrevDesc, (*solar_sys).SunDesc.as_ptr())
}

/// # Safety
/// Game-thread caller; descriptor pointers must refer to the live system or a
/// complete, readable system description. Output pointers must be writable.
#[no_mangle]
pub unsafe extern "C" fn worldIsMoon(
    solar_sys: *const c::SOLARSYS_STATE,
    world: *const c::PLANET_DESC,
) -> bool {
    !worldIsPlanet(solar_sys, world)
}

/// # Safety
/// Game-thread caller; descriptor pointers must refer to the live system or a
/// complete, readable system description. Output pointers must be writable.
#[no_mangle]
pub unsafe extern "C" fn planetIndex(
    solar_sys: *const c::SOLARSYS_STATE,
    world: *const c::PLANET_DESC,
) -> c::COUNT {
    let planet = if worldIsPlanet(solar_sys, world) {
        world
    } else {
        (*world).pPrevDesc
    };
    planet.offset_from((*solar_sys).PlanetDesc.as_ptr()) as c::COUNT
}

/// # Safety
/// Game-thread caller; descriptor pointers must refer to the live system or a
/// complete, readable system description. Output pointers must be writable.
#[no_mangle]
pub unsafe extern "C" fn moonIndex(
    solar_sys: *const c::SOLARSYS_STATE,
    moon: *const c::PLANET_DESC,
) -> c::COUNT {
    assert!(!worldIsPlanet(solar_sys, moon));
    moon.offset_from((*solar_sys).MoonDesc.as_ptr()) as c::COUNT
}

/// # Safety
/// Game-thread caller; descriptor pointers must refer to the live system or a
/// complete, readable system description. Output pointers must be writable.
#[no_mangle]
pub unsafe extern "C" fn matchWorld(
    solar_sys: *const c::SOLARSYS_STATE,
    world: *const c::PLANET_DESC,
    planet_i: c::BYTE,
    moon_i: c::BYTE,
) -> bool {
    const MATCH_PLANET: u8 = 0xff;
    if planetIndex(solar_sys, world) != c::COUNT::from(planet_i) {
        return false;
    }
    if moon_i == MATCH_PLANET {
        worldIsPlanet(solar_sys, world)
    } else {
        worldIsMoon(solar_sys, world) && moonIndex(solar_sys, world) == c::COUNT::from(moon_i)
    }
}

/// # Safety
/// Game-thread caller; descriptor pointers must refer to the live system or a
/// complete, readable system description. Output pointers must be writable.
#[no_mangle]
pub unsafe extern "C" fn playerInSolarSystem() -> bool {
    !crate::c_bindings::solar_system::current().is_null()
}

/// # Safety
/// Game-thread caller; descriptor pointers must refer to the live system or a
/// complete, readable system description. Output pointers must be writable.
#[no_mangle]
pub unsafe extern "C" fn playerInPlanetOrbit() -> bool {
    playerInSolarSystem() && (*crate::c_bindings::solar_system::current()).InOrbit != 0
}

/// # Safety
/// Game-thread caller; descriptor pointers must refer to the live system or a
/// complete, readable system description. Output pointers must be writable.
#[no_mangle]
pub unsafe extern "C" fn playerInInnerSystem() -> bool {
    let system = crate::c_bindings::solar_system::current();
    assert!(!system.is_null());
    assert!(
        (*system).pBaseDesc == (*system).PlanetDesc.as_mut_ptr()
            || (*system).pBaseDesc == (*system).MoonDesc.as_mut_ptr()
    );
    (*system).pBaseDesc != (*system).PlanetDesc.as_mut_ptr()
}

/// # Safety
/// Game-thread caller; descriptor pointers must refer to the live system or a
/// complete, readable system description. Output pointers must be writable.
#[no_mangle]
pub unsafe extern "C" fn XFormIPLoc(
    input: *mut c::POINT,
    output: *mut c::POINT,
    to_display: c::BOOLEAN,
) {
    let system = crate::c_bindings::solar_system::current();
    let radius = (*system).SunDesc[0].radius;
    let converted = if to_display != 0 {
        locationToDisplay(input.read(), radius)
    } else {
        displayToLocation(input.read(), radius)
    };
    output.write(converted);
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
