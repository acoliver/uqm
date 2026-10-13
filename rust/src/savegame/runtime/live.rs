//! The C-owned live state that `summary` and `star` borrow in a running game.
//!
//! Everything here is a read of a C global or a call into a C owner that has
//! no Rust owner yet: `GlobData` (globdata.c), `CurStarDescPtr` and
//! `star_array` (starmap.c), the solar-system owner (`GetPlanetOrMoonName`,
//! `playerInPlanetOrbit`) and the game string table. The capture logic stays in
//! `summary` and `star`; these functions only hand it the live owners. They are
//! not in the unit-test build, which has no C archive; the tests of those two
//! modules build the same owners by hand.
//!
//! All of them must run on the game thread.

use std::ffi::c_char;

use super::star::{self, CStarDesc, StarError, CATALOG_LEN};
use super::summary::{self, SolarSystemOwner, SummaryError, SA_MATRA_STRING};
use crate::comm::locdata::CGlobData;
use crate::savegame::types::{StarDesc, Summary, SIS_NAME_SIZE};

extern "C" {
    #[link_name = "GlobData"]
    static mut GLOB_DATA: CGlobData;
    #[link_name = "CurStarDescPtr"]
    static mut CUR_STAR_DESC_PTR: *mut CStarDesc;
    #[link_name = "star_array"]
    static mut STAR_ARRAY: *mut CStarDesc;
    fn GetPlanetOrMoonName(buf: *mut c_char, bufsize: u16);
    fn playerInPlanetOrbit() -> bool;
}

/// The solar-system and string owners of the running C game.
pub struct LiveSolarSystem;

impl SolarSystemOwner for LiveSolarSystem {
    fn planet_or_moon_name(&self) -> [u8; SIS_NAME_SIZE] {
        let mut buf = [0u8; SIS_NAME_SIZE];
        // SAFETY: the buffer is `SIS_NAME_SIZE` bytes and that size is passed;
        // game thread only.
        unsafe { GetPlanetOrMoonName(buf.as_mut_ptr().cast::<c_char>(), SIS_NAME_SIZE as u16) };
        buf
    }

    fn player_in_planet_orbit(&self) -> bool {
        // SAFETY: reads the solar-system state; game thread only.
        unsafe { playerInPlanetOrbit() }
    }

    fn last_battle_planet_name(&self) -> [u8; SIS_NAME_SIZE] {
        // SAFETY: `GameStrings` is loaded for the whole game; the string is
        // NUL-terminated.
        let text = unsafe {
            std::ffi::CStr::from_ptr(crate::comm::hail::game_string(i32::from(SA_MATRA_STRING)))
        };
        summary::utf8_string_copy(text.to_bytes())
    }
}

/// `PrepareSummary` over the live `GlobData`.
///
/// # Safety
/// Game thread only; `GlobData` and the solar-system owner must be initialised.
pub unsafe fn capture_live_summary(name: &[u8]) -> Result<Summary, SummaryError> {
    let glob = &*std::ptr::addr_of!(GLOB_DATA);
    summary::capture_summary(&glob.sis_state, &glob.game_state, &LiveSolarSystem, name)
}

/// The live star catalog.
///
/// # Safety
/// `star_array` must point at the `CATALOG_LEN` entries of `starmap_array`.
unsafe fn live_catalog<'a>() -> &'a [CStarDesc] {
    let table = std::ptr::addr_of!(STAR_ARRAY).read();
    assert!(!table.is_null(), "the star catalog is not set up");
    std::slice::from_raw_parts(table, CATALOG_LEN)
}

/// The "Star" chunk for the live `CurStarDescPtr`.
///
/// # Safety
/// Game thread only; the star map must be initialised.
pub unsafe fn capture_live_star() -> Result<StarDesc, StarError> {
    let current = std::ptr::addr_of!(CUR_STAR_DESC_PTR).read();
    star::capture_star(live_catalog(), current)
}

/// Check a loaded "Star" chunk against the live catalog.
///
/// # Safety
/// Game thread only; the star map must be initialised.
pub unsafe fn resolve_live_star(
    arilou_space_side: u8,
    saved: &StarDesc,
) -> Result<star::LoadedStar, StarError> {
    star::resolve_star(live_catalog(), arilou_space_side, saved)
}
