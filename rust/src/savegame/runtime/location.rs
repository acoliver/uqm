//! Prepare `SaveSolarSysLocation` and `saveNonOrbitalLocation` inputs from the
//! live solar tables. `engine::save::location` applies them to the supplied World.
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
//! bits are borrowed from the supplied campaign for the whole write.
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

use super::solar::{position_in, SolarError, SolarSystem, World};
use crate::c_bindings::global_state::{FRAME_DESC, SOLARSYS_STATE};
use crate::engine::battle::schema::GAME_STATE as CGameState;
pub use crate::engine::save::location::LocationError;
use crate::engine::save::location::{OrbitLocation, SolarLocation};
use crate::engine::world::planet_info::ScanRetrieveMask;
use crate::graphics::native_abi::POINT as CPoint;

/// `GetFrameIndex (frame)`: 0 for NULL, else the frame's `Index`.
///
/// # Safety
/// `frame` must be NULL or point at a live `FRAME_DESC`.
unsafe fn frame_index(frame: *const FRAME_DESC) -> u16 {
    if frame.is_null() {
        return 0;
    }
    (*frame).Index
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
    let host = position_in(&state.PlanetDesc, state.MoonDesc[0].prev_desc.cast_const())
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

/// Prepare the save location from the active solar session.
///
/// # Safety
/// The supplied game record's frame must be null or remain live for this read.
pub unsafe fn prepare_solar_location(
    game: &CGameState,
    state: &SOLARSYS_STATE,
    star_index: usize,
) -> Result<SolarLocation, LocationError> {
    if state.InOrbit == 0 {
        return prepare_non_orbital_location(game, state);
    }
    let target = orbit_target(state)?;
    Ok(SolarLocation::Orbit(OrbitLocation {
        in_orbit: target.in_orbit,
        star_index,
        planet: target.planet,
        moon: target.scan_moon,
        scan: ScanRetrieveMask::from_array(&state.SysInfo.planet_info.scan_retrieve_mask),
        planet_count: state.SunDesc[0].num_planets,
        moon_counts: state.PlanetDesc.each_ref().map(|planet| planet.num_planets),
    }))
}

/// `saveNonOrbitalLocation ()`.
///
/// # Safety
/// As [`prepare_solar_location`].
unsafe fn prepare_non_orbital_location(
    game: &CGameState,
    state: &SOLARSYS_STATE,
) -> Result<SolarLocation, LocationError> {
    let ship_facing = frame_index(game.ship_stamp.frame.cast::<FRAME_DESC>())
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

    Ok(SolarLocation::NonOrbital { ship_facing, inner })
}

/// `planetIndex (pSolarSysState, world)`: the planet itself, or the planet a
/// moon orbits.
fn planet_index(world: World) -> usize {
    match world {
        World::Planet { planet } | World::Moon { planet, .. } => planet,
    }
}
