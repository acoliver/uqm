//! Remaining state.c entry points. File storage stays in `ffi`'s sole manager.

#![allow(non_snake_case)]

use super::ffi::*;
use std::ffi::{c_char, c_int, c_long, c_void};
use std::ptr;

/// Opaque to C. Its address identifies a file, not an independently owned buffer.
#[repr(C)]
pub struct GameStateFile {
    index: c_int,
}

static mut FILES: [GameStateFile; 3] = [
    GameStateFile { index: 0 },
    GameStateFile { index: 1 },
    GameStateFile { index: 2 },
];

/// # Safety
/// `mode` must be a NUL-terminated C string.
#[no_mangle]
pub unsafe extern "C" fn OpenStateFile(index: c_int, mode: *const c_char) -> *mut GameStateFile {
    if !(0..3).contains(&index) || rust_open_state_file(index, mode) == 0 {
        return ptr::null_mut();
    }
    ptr::addr_of_mut!(FILES)
        .cast::<GameStateFile>()
        .add(index as usize)
}

/// # Safety
/// `file` must be a handle returned by `OpenStateFile`.
#[no_mangle]
pub unsafe extern "C" fn CloseStateFile(file: *mut GameStateFile) {
    rust_close_state_file((*file).index);
}

/// # Safety
/// Called on the game thread, without concurrent state-file access.
#[no_mangle]
pub unsafe extern "C" fn DeleteStateFile(index: c_int) {
    rust_delete_state_file(index);
}

/// # Safety
/// `file` must be a handle returned by `OpenStateFile`.
#[no_mangle]
pub unsafe extern "C" fn LengthStateFile(file: *mut GameStateFile) -> u32 {
    rust_length_state_file((*file).index) as u32
}

/// # Safety
/// `buffer` must hold `size * count` writable bytes; `file` must be a live handle.
#[no_mangle]
pub unsafe extern "C" fn ReadStateFile(
    buffer: *mut c_void,
    size: u16,
    count: u16,
    file: *mut GameStateFile,
) -> c_int {
    rust_read_state_file((*file).index, buffer.cast(), size as usize, count as usize) as c_int
}

/// # Safety
/// `buffer` must hold `size * count` readable bytes; `file` must be a live handle.
#[no_mangle]
pub unsafe extern "C" fn WriteStateFile(
    buffer: *const c_void,
    size: u16,
    count: u16,
    file: *mut GameStateFile,
) -> c_int {
    rust_write_state_file((*file).index, buffer.cast(), size as usize, count as usize) as c_int
}

/// # Safety
/// `file` must be a handle returned by `OpenStateFile`.
#[no_mangle]
pub unsafe extern "C" fn SeekStateFile(
    file: *mut GameStateFile,
    offset: c_long,
    whence: c_int,
) -> c_int {
    rust_seek_state_file((*file).index, offset, whence)
}

#[cfg(not(test))]
pub(crate) mod planets {
    use super::*;
    use crate::c_bindings::{global_state::SOLARSYS_STATE, solar_system};
    use crate::savegame::runtime::star::CStarDesc;

    extern "C" {
        static mut star_array: *mut CStarDesc;
        static mut CurStarDescPtr: *mut CStarDesc;
    }

    pub(crate) unsafe fn location(system: *mut SOLARSYS_STATE) -> (c_int, c_int, c_int) {
        let star = CurStarDescPtr.offset_from(star_array) as u16;
        let planet = (*(*system).pBaseDesc)
            .pPrevDesc
            .offset_from((*system).PlanetDesc.as_ptr()) as u16;
        let orbital = (*system).pOrbitalDesc;
        let moon = if (*orbital).pPrevDesc == (*system).SunDesc.as_mut_ptr() {
            0
        } else {
            (orbital.offset_from((*system).MoonDesc.as_ptr()) + 1) as u16
        };
        (star as c_int, planet as c_int, moon as c_int)
    }

    unsafe fn moons(system: *mut SOLARSYS_STATE) -> ([u8; 16], c_int) {
        let mut moons = [0; 16];
        let count = (*system).SunDesc[0].NumPlanets as usize;
        for (i, moon) in moons[..count].iter_mut().enumerate() {
            *moon = (*system).PlanetDesc[i].NumPlanets;
        }
        (moons, count as c_int)
    }

    /// # Safety
    /// The star catalog must be initialized and terminated by its sentinel.
    #[no_mangle]
    pub unsafe extern "C" fn InitPlanetInfo() {
        let mut star = star_array;
        let mut count = 0;
        while (*star).star_pt.x <= 9999 && (*star).star_pt.y <= 9999 {
            count += 1;
            star = star.add(1);
        }
        rust_init_planet_info(count);
    }

    /// # Safety
    /// Called on the game thread, without concurrent state-file access.
    #[no_mangle]
    pub unsafe extern "C" fn UninitPlanetInfo() {
        rust_uninit_planet_info();
    }

    /// # Safety
    /// A live solar system and current orbital descriptor must be selected.
    #[no_mangle]
    pub unsafe extern "C" fn GetPlanetInfo() {
        let system = solar_system::current();
        (*system).SysInfo.PlanetInfo.ScanRetrieveMask = [0; 3];
        let (star, planet, moon) = location(system);
        let (moons, count) = moons(system);
        let mut mask = [0; 3];
        if rust_get_planet_info(star, planet, moon, moons.as_ptr(), count, mask.as_mut_ptr()) != 0 {
            (*system).SysInfo.PlanetInfo.ScanRetrieveMask = mask;
        }
    }

    /// # Safety
    /// A live solar system and current orbital descriptor must be selected.
    #[no_mangle]
    pub unsafe extern "C" fn PutPlanetInfo() {
        let system = solar_system::current();
        let (star, planet, moon) = location(system);
        let (moons, count) = moons(system);
        rust_put_planet_info(
            star,
            planet,
            moon,
            moons.as_ptr(),
            count,
            (*system).SysInfo.PlanetInfo.ScanRetrieveMask.as_ptr(),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[serial_test::serial]
    fn handles_share_the_existing_file_manager_and_preserve_io() {
        unsafe {
            rust_init_game_state();
            assert!(OpenStateFile(-1, c"wb".as_ptr()).is_null());
            assert!(OpenStateFile(3, c"wb".as_ptr()).is_null());
            let file = OpenStateFile(1, c"wb".as_ptr());
            assert!(!file.is_null());
            let data = [11u8, 22, 33, 44];
            assert_eq!(WriteStateFile(data.as_ptr().cast(), 2, 2, file), 2);
            assert_eq!(LengthStateFile(file), 4);
            assert_eq!(rust_length_state_file(1), 4);
            assert_eq!(SeekStateFile(file, -2, 2), 1);
            let mut output = [0u8; 2];
            assert_eq!(ReadStateFile(output.as_mut_ptr().cast(), 1, 2, file), 2);
            assert_eq!(output, [33, 44]);
            CloseStateFile(file);
            assert_eq!(OpenStateFile(1, c"rb".as_ptr()), file);
            CloseStateFile(file);
            DeleteStateFile(1);
        }
    }
}
