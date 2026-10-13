//! `SaveSolarSysLocation` and `saveNonOrbitalLocation` (solarsys.c) in Rust:
//! the part of a save that records where in a solar system the flagship is.
//!
//! The record is three fields of `GAME_STATE` (`ShipFacing`, `ip_planet`,
//! `in_orbit`) plus `ip_location`, written for `SaveGameState` to pick up. It
//! is a two-stage procedure, as in C:
//!
//! * Not in orbit (outer or inner system): `ShipFacing` becomes the index of
//!   the ship's frame plus one (the reentry test needs it non-zero) and
//!   `in_orbit` becomes 0. In the outer system `ip_planet` is 0. In an inner
//!   system `ip_planet` is one plus the index of the planet whose system it is
//!   and `ip_location` is the sun's location.
//! * In orbit: `ip_planet`, `ShipFacing` and `ip_location` are already what
//!   the first stage left, and only `in_orbit` is written: 1 for the planet,
//!   2 plus the moon's index for a moon. Before that, a set `PLANETARY_CHANGE`
//!   flag makes the scan masks of the world in orbit go into `STARINFO` (the
//!   `PutPlanetInfo` of state.c, which here is `PlanetInfoManager`), and only
//!   once that succeeded is the flag cleared.
//!
//! Everything is worked out before the first write, so a call that returns an
//! error has changed neither the record nor the flag nor `STARINFO`.
//!
//! The solar-system tables are read through the same checks the summary uses
//! (`solar`); the star index is the position of `CurStarDescPtr` in the
//! catalog (`star::catalog_index`), the file offsets come from the world's
//! position in `PlanetDesc`/`MoonDesc` and the state files and the game-state
//! bits are the Rust singletons, locked together for the whole write.
//!
//! Where this differs from the C code on purpose:
//!
//! * `PutPlanetInfo` is a `void` function whose write failures C ignores and
//!   then clears the flag anyway. Here a failed write is an error and the flag
//!   stays set, so the change is written by a later save.
//! * C asserts `ip_planet != 0` after `PutPlanetInfo`; here the check comes
//!   first so that a rejected call has written nothing.
//! * C does pointer arithmetic on whatever `pOrbitalDesc` and
//!   `pBaseDesc->pPrevDesc` hold. A pointer that is not an element of its
//!   table, or a planet that `pBaseDesc` and `pOrbitalDesc` disagree about, is
//!   an error. Being in orbit outside an inner system is an error too.
//! * The caller names the star; C reads `CurStarDescPtr` itself.
//!
//! Nothing here moves the flagship: no field other than the four above is
//! written, and `ip_location` only takes the sun's location, which is where
//! the inner system puts the ship's saved position.

use std::ffi::c_void;
use std::fmt;

use super::solar::{position_in, SolarError, SolarSystem, World};
use crate::c_bindings::global_state::{MAX_PLANETS, SOLARSYS_STATE};
use crate::comm::locdata::{CGameState, CPoint};
use crate::state::ffi::{with_state_owners, GameStateOwnerError};
use crate::state::game_state::GameState;
use crate::state::planet_info::{PlanetInfoManager, ScanRetrieveMask};
use crate::state::state_file::StateFileError;

/// The first bytes of `FRAME_DESC` (drawable.h): `Type`, then `Index`. The
/// rest of the frame is not read.
#[repr(C)]
struct FramePrefix {
    _frame_type: u16,
    index: u16,
}

/// `GetFrameIndex (frame)`: 0 for NULL, else the frame's `Index`.
///
/// # Safety
/// `frame` must be NULL or point at a live `FRAME_DESC`.
unsafe fn frame_index(frame: *const c_void) -> u16 {
    if frame.is_null() {
        return 0;
    }
    (*frame.cast::<FramePrefix>()).index
}

/// Why the location could not be prepared.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LocationError {
    /// The solar-system tables are not the ones C wires up.
    Solar(SolarError),
    /// The frame index is 65535, so `ShipFacing` would wrap to 0.
    FrameIndexOverflow,
    /// `in_orbit` is set but `pBaseDesc` is not `MoonDesc`: C only enters
    /// orbit from an inner system.
    OrbitOutsideInnerSystem,
    /// Stage two ran before stage one: `ip_planet` is still 0.
    IpPlanetNotSet,
    /// `SunDesc[0].NumPlanets` is more than `MAX_PLANETS`.
    PlanetCountOutOfRange { found: u8 },
    /// The game-state singleton or the state files could not be reached.
    Owners(GameStateOwnerError),
    /// `STARINFO` rejected the scan record.
    PlanetInfo(StateFileError),
}

impl fmt::Display for LocationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for LocationError {}

impl From<SolarError> for LocationError {
    fn from(error: SolarError) -> Self {
        Self::Solar(error)
    }
}

/// What the orbit stage stores in `in_orbit`, and which world's scan record
/// goes with it.
struct OrbitTarget {
    in_orbit: u8,
    planet: usize,
    /// `get_current_moon_index`: 0 for the planet, else the moon's index plus 1.
    scan_moon: usize,
}

/// The planet `pBaseDesc->pPrevDesc` names (`get_current_planet_index`), which
/// must be the planet `pOrbitalDesc` belongs to.
fn orbit_target(state: &SOLARSYS_STATE) -> Result<OrbitTarget, LocationError> {
    if !SolarSystem::in_inner_system(state) {
        return Err(LocationError::OrbitOutsideInnerSystem);
    }
    let world = SolarSystem::orbital_world(state)?;
    // `pBaseDesc` is `MoonDesc` here, so its `pPrevDesc` is the system's planet.
    let host = position_in(&state.PlanetDesc, state.MoonDesc[0].pPrevDesc.cast_const())
        .ok_or(SolarError::WorldDisagreesWithItsTable)?;
    let (planet, in_orbit, scan_moon) = match world {
        World::Planet { planet } => (planet, 1, 0),
        // moon 0 is stored as 2: 1 is the planet itself
        World::Moon { planet, moon } => (planet, 2 + moon, 1 + moon),
    };
    if planet != host {
        return Err(SolarError::WorldDisagreesWithItsTable.into());
    }
    Ok(OrbitTarget {
        // The tables hold 16 planets and 4 moons: these fit a byte.
        in_orbit: in_orbit as u8,
        planet,
        scan_moon,
    })
}

/// `fill_planet_num_moons`: the moon count of each planet of the system.
fn moons_per_planet(state: &SOLARSYS_STATE) -> Result<Vec<u8>, LocationError> {
    let planets = state.SunDesc[0].NumPlanets;
    if u32::from(planets) > MAX_PLANETS {
        return Err(LocationError::PlanetCountOutOfRange { found: planets });
    }
    Ok(state.PlanetDesc[..usize::from(planets)]
        .iter()
        .map(|planet| planet.NumPlanets)
        .collect())
}

/// `PutPlanetInfo` and `SET_GAME_STATE (PLANETARY_CHANGE, 0)`, under one lock
/// of the game-state singleton and the state files.
fn persist_planet_change(
    star_index: usize,
    target: &OrbitTarget,
    state: &SOLARSYS_STATE,
) -> Result<(), LocationError> {
    let mask = ScanRetrieveMask::from_array(&state.SysInfo.PlanetInfo.ScanRetrieveMask);
    let (start, end) =
        GameState::lookup_bits("PLANETARY_CHANGE").expect("PLANETARY_CHANGE is a game-state key");
    with_state_owners(|bits, files| {
        if bits.get_state(start, end) == 0 {
            return Ok(());
        }
        // Every input is built before the first write.
        let moons = moons_per_planet(state)?;
        PlanetInfoManager::new(files)
            .put_planet_info(star_index, target.planet, target.scan_moon, &mask, &moons)
            .map_err(LocationError::PlanetInfo)?;
        bits.set_state(start, end, 0);
        Ok(())
    })
    .map_err(LocationError::Owners)?
}

/// `SaveSolarSysLocation ()` for a flagship that is in a solar system.
///
/// `star_index` is the position of `CurStarDescPtr` in the star catalog.
///
/// # Safety
/// `game.ship_stamp.frame` must be NULL or point at a live `FRAME_DESC`.
pub unsafe fn save_solar_location(
    game: &mut CGameState,
    state: &SOLARSYS_STATE,
    star_index: usize,
) -> Result<(), LocationError> {
    if state.InOrbit == 0 {
        return save_non_orbital_location(game, state);
    }
    let target = orbit_target(state)?;
    if game.ip_planet == 0 {
        return Err(LocationError::IpPlanetNotSet);
    }
    persist_planet_change(star_index, &target, state)?;
    game.in_orbit = target.in_orbit;
    Ok(())
}

/// `saveNonOrbitalLocation ()`.
///
/// # Safety
/// As [`save_solar_location`].
unsafe fn save_non_orbital_location(
    game: &mut CGameState,
    state: &SOLARSYS_STATE,
) -> Result<(), LocationError> {
    let ship_facing = frame_index(game.ship_stamp.frame)
        .checked_add(1)
        .ok_or(LocationError::FrameIndexOverflow)?;
    // (ip_planet, ip_location): the outer system keeps the location.
    let inner = if SolarSystem::in_inner_system(state) {
        let planet = planet_index(SolarSystem::orbital_world(state)?);
        // 1-based, because the reentry code tests for `ip_planet != 0`.
        let ip_planet = (1 + planet) as u8;
        let sun = state.SunDesc[0].location;
        Some((ip_planet, CPoint { x: sun.x, y: sun.y }))
    } else {
        None
    };

    game.ship_facing = ship_facing;
    game.in_orbit = 0;
    match inner {
        None => game.ip_planet = 0,
        Some((ip_planet, location)) => {
            game.ip_planet = ip_planet;
            game.ip_location = location;
        }
    }
    Ok(())
}

/// `planetIndex (pSolarSysState, world)`: the planet itself, or the planet a
/// moon orbits.
fn planet_index(world: World) -> usize {
    match world {
        World::Planet { planet } | World::Moon { planet, .. } => planet,
    }
}
