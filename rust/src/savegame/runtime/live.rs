//! The C-owned live state that `summary` and `star` borrow in a running game.
//!
//! Everything here is a read of a C global: `GlobData` (globdata.c),
//! `pSolarSysState` (solarsys.c), `CurStarDescPtr` and `star_array`
//! (starmap.c) and the game string table. Those are still initialised by C
//! (`ExploreSolarSys`, the star map, the resource loader) and have no Rust
//! owner yet, so Rust borrows them. The capture logic stays in `summary`,
//! `solar` and `star`: the solar-system questions of the summary (the planet
//! or moon name, the orbit test, the final-battle name) are computed in Rust
//! over the borrowed `SOLARSYS_STATE` (declared once, in
//! `c_bindings::solar_system`), and no C function of solarsys.c is
//! called for them. They are not in the unit-test build, which has no C
//! archive; the tests of those modules build the same state by hand.
//!
//! All of them must run on the game thread.

use super::solar::{GameStrings, SolarSystem};
use super::star::{self, CStarDesc, StarError, CATALOG_LEN};
use super::summary::{self, SummaryError};
use crate::c_bindings::solar_system;
use crate::comm::locdata::CGlobData;
use crate::savegame::types::{StarDesc, Summary};

extern "C" {
    #[link_name = "GlobData"]
    static mut GLOB_DATA: CGlobData;
    #[link_name = "CurStarDescPtr"]
    static mut CUR_STAR_DESC_PTR: *mut CStarDesc;
    #[link_name = "star_array"]
    static mut STAR_ARRAY: *mut CStarDesc;
}

/// `GAME_STRING (index)` over the loaded `GameStrings` table.
struct LiveGameStrings;

impl GameStrings for LiveGameStrings {
    fn game_string(&self, index: u16) -> Vec<u8> {
        // SAFETY: `GameStrings` is loaded for the whole game and every string
        // of it is NUL-terminated; game thread only.
        unsafe {
            let text = crate::comm::hail::game_string(i32::from(index));
            assert!(!text.is_null(), "GAME_STRING ({index}) has no text");
            std::ffi::CStr::from_ptr(text).to_bytes().to_vec()
        }
    }
}

/// `PrepareSummary` over the live `GlobData`, `pSolarSysState` and
/// `CurStarDescPtr`.
///
/// The solar-system state is borrowed once, for this call only.
///
/// # Safety
/// Game thread only; `GlobData` must be initialised, `pSolarSysState` must be
/// NULL or point at the live solar-system state, and `CurStarDescPtr` must be
/// NULL or point at a catalog entry.
pub unsafe fn capture_live_summary(name: &[u8]) -> Result<Summary, SummaryError> {
    let glob = &*std::ptr::addr_of!(GLOB_DATA);
    let state = solar_system::current();
    let current = std::ptr::addr_of!(CUR_STAR_DESC_PTR).read();
    let solar = SolarSystem::new(
        state.as_ref(),
        current.as_ref().map(|star| star.index),
        glob.sis_state.planet_name,
        &LiveGameStrings,
    );
    summary::capture_summary(&glob.sis_state, &glob.game_state, &solar, name)
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
