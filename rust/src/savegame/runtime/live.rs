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

use crate::engine::{World, WorldData};
use std::fmt;

use super::hyper_save::{self, HyperFlagship, HyperSaveError};
use super::location::{self, LocationError};
use super::solar::{GameStrings, SolarSystem};
use super::star::{self, CStarDesc, StarError, CATALOG_LEN};
use super::summary::{self, SummaryError};
use crate::c_bindings::global_state::RPG_PLAYER_NUM;
use crate::c_bindings::solar_system;
use crate::engine::battle::access::live_queues;
use crate::savegame::types::{StarDesc, Summary};

extern "C" {

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
    let glob = **WorldData::current();
    let state = solar_system::current();
    let current = (**World::current()).star_pointer(Some(live_catalog()));
    let solar = SolarSystem::new(
        state.as_ref(),
        current.as_ref().map(|star| star.index),
        glob.flagship.planet_name.map(|byte| byte),
        &LiveGameStrings,
    );
    // SAFETY: the existing codec views assert all field offsets and alignments against these records.
    summary::capture_summary(
        &*std::ptr::addr_of!(glob.flagship).cast::<crate::comm::locdata::CSisState>(),
        &*std::ptr::addr_of!(glob.game).cast::<crate::comm::locdata::CGameState>(),
        &solar,
        name,
    )
}

/// The live star catalog.
///
/// # Safety
/// `star_array` must point at the `CATALOG_LEN` entries of `starmap_array`.
unsafe fn live_catalog<'a>() -> &'a [CStarDesc; CATALOG_LEN] {
    let table = std::ptr::addr_of!(STAR_ARRAY).read();
    assert!(!table.is_null(), "the star catalog is not set up");
    &*table.cast::<[CStarDesc; CATALOG_LEN]>()
}

/// # Safety
/// Game-thread access with the live star catalog retained for this selection.
pub unsafe fn select_live_star(world: &mut World, star: Option<&CStarDesc>) {
    // SAFETY: the caller retains the initialized catalog for this game-thread selection.
    let catalog = unsafe { live_catalog() };
    world.select_star(star, Some(catalog));
}

/// The "Star" chunk for the live `CurStarDescPtr`.
///
/// # Safety
/// Game thread only; the star map must be initialised.
pub unsafe fn capture_live_star() -> Result<StarDesc, StarError> {
    // SAFETY: this game-thread operation retains the initialized star catalog.
    let catalog = unsafe { Some(live_catalog()) };
    let current = (**World::current()).star_pointer(catalog);
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

/// Why the live solar-system location could not be prepared.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LiveLocationError {
    /// `CurStarDescPtr` is not an entry of the star catalog.
    Star(StarError),
    Location(LocationError),
}

impl fmt::Display for LiveLocationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Star(error) => write!(f, "star: {error}"),
            Self::Location(error) => write!(f, "solar location: {error}"),
        }
    }
}

impl std::error::Error for LiveLocationError {}

/// `SaveSolarSysLocation ()` over the live `GlobData`, `pSolarSysState` and
/// `CurStarDescPtr`: the step of a save that writes where the flagship is in
/// its solar system, and persists the planet's scan record when
/// `PLANETARY_CHANGE` is set.
///
/// Panics when `pSolarSysState` is NULL, which is the `assert (playerInSolarSystem ())`
/// of the C function; a caller outside a solar system has a bug, not an error
/// to report.
///
/// The solar-system state is borrowed once, for this call only.
///
/// # Safety
/// Game thread only; `GlobData` must be initialised, `pSolarSysState` must be
/// NULL or point at the live solar-system state, `CurStarDescPtr` must point at
/// a catalog entry and `GlobData.game.ship_stamp.frame` must be NULL or a
/// live frame.
pub unsafe fn prepare_live_solar_location() -> Result<(), LiveLocationError> {
    let state = solar_system::current();
    assert!(!state.is_null(), "not in a solar system");
    // SAFETY: this game-thread operation retains the initialized star catalog.
    let catalog = unsafe { Some(live_catalog()) };
    let current = (**World::current()).star_pointer(catalog);
    let star_index =
        star::catalog_index(live_catalog(), current).map_err(LiveLocationError::Star)?;
    let mut game = WorldData::current().read(|world| world.game);
    location::save_solar_location(
        &mut *std::ptr::addr_of_mut!(game).cast::<crate::comm::locdata::CGameState>(),
        &*state,
        star_index,
    )
    .map_err(LiveLocationError::Location)?;
    WorldData::current().write(|world| {
        world.game.in_orbit = game.in_orbit;
        world.game.ship_facing = game.ship_facing;
        world.game.ip_planet = game.ip_planet;
        world.game.ip_location = game.ip_location;
    });
    Ok(())
}

/// `SaveSisHyperState ()` over the live `GlobData`, `race_q[RPG_PLAYER_NUM]`
/// and `disp_q`: the step of a save made in hyperspace that records which way
/// the flagship faces.
///
/// Both queues are borrowed once, for this call only.
///
/// # Safety
/// Game thread only; `GlobData` must be initialised and `race_q` and `disp_q`
/// must have been set up by `InitQueue` (`InitKernel` and `InitSpace`).
pub unsafe fn prepare_live_hyper_state() -> Result<HyperFlagship, HyperSaveError> {
    let player_ships = &*live_queues::player_ships(RPG_PLAYER_NUM as usize);
    // SAFETY: the save operation retains the published game-thread Kernel.
    let display_list = unsafe { &*live_queues::display_list() };
    let glob = &mut **WorldData::current();
    hyper_save::save_hyper_facing(
        &mut *std::ptr::addr_of_mut!(glob.game).cast::<crate::comm::locdata::CGameState>(),
        player_ships,
        display_list,
    )
}

/// The live flagship's own `ShipFacing` (0..16), read from `race_q[RPG_PLAYER_NUM]`
/// and `disp_q` the way `SaveSisHyperState` finds it. Nothing is written.
///
/// # Safety
/// Game thread only; `race_q` and `disp_q` must have been set up by
/// `InitQueue` (`InitKernel` and `InitSpace`).
pub unsafe fn live_flagship_facing() -> Result<Option<u16>, HyperSaveError> {
    let player_ships = &*live_queues::player_ships(RPG_PLAYER_NUM as usize);
    // SAFETY: the save operation retains the published game-thread Kernel.
    let display_list = unsafe { &*live_queues::display_list() };
    hyper_save::read_flagship_facing(player_ships, display_list)
}
