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

use crate::engine::{Kernel, World};
use crate::strings::string_table::{GetStringAddress, SetAbsStringTableIndex};
use std::fmt;

use super::location::{self, LocationError};
use super::solar::{GameStrings, SolarSystem, SolarSystemOwner};
use super::star::{self, StarError};
use crate::c_bindings::solar_system;
use crate::engine::save::{snapshot, SessionView};
use crate::savegame::types::{SaveFile, StarDesc};

/// `GAME_STRING (index)` over the loaded `GameStrings` table.
struct LiveGameStrings<'a>(&'a Kernel);

impl GameStrings for LiveGameStrings<'_> {
    fn game_string(&self, index: u16) -> Vec<u8> {
        // SAFETY: `GameStrings` is loaded for the whole game and every string
        // of it is NUL-terminated; game thread only.
        unsafe {
            let entry = SetAbsStringTableIndex(self.0.game_strings.cast(), index);
            let text = GetStringAddress(entry);
            assert!(!text.is_null(), "GAME_STRING ({index}) has no text");
            std::ffi::CStr::from_ptr(text).to_bytes().to_vec()
        }
    }
}

/// Supply the active solar session to the owned engine snapshot.
///
/// # Safety
/// Game thread only; the solar-system pointer and star catalog must remain
/// valid for this synchronous capture.
pub unsafe fn capture_prepared(
    world: &mut World,
    kernel: &Kernel,
    star: StarDesc,
    ip_location: crate::graphics::native_abi::POINT,
    name: &[u8],
) -> anyhow::Result<SaveFile> {
    let state = solar_system::current();
    let current = world.star_pointer();
    let strings = LiveGameStrings(kernel);
    let solar = SolarSystem::new(
        state.as_ref(),
        current.as_ref().map(|star| star.index),
        world.data.flagship.planet_name,
        &strings,
    );
    if world.story.is_none() {
        return Err(crate::engine::world::story::GameStateOwnerError::NotInitialized.into());
    }
    let solar = solar.prepare_summary(
        world.activity(),
        world.story_state(crate::engine::StoryKey::GLOBAL_FLAGS_AND_DATA) == u32::from(u8::MAX),
    )?;
    snapshot(
        world,
        SessionView {
            solar,
            star,
            ip_location,
        },
        name,
    )
}

/// The selected star record for the supplied campaign's save.
pub fn capture_live_star(world: &World) -> Result<StarDesc, StarError> {
    star::capture_star(world.catalog(), world.star_pointer())
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
pub unsafe fn prepare_live_solar_location(world: &mut World) -> Result<(), LiveLocationError> {
    let state = solar_system::current();
    assert!(!state.is_null(), "not in a solar system");
    let current = world.star_pointer();
    let star_index =
        star::catalog_index(world.catalog(), current).map_err(LiveLocationError::Star)?;
    let location = location::prepare_solar_location(&world.data.game, &*state, star_index)
        .map_err(LiveLocationError::Location)?;
    crate::engine::save::location::apply_solar_location(world, location)
        .map_err(LiveLocationError::Location)?;
    Ok(())
}
