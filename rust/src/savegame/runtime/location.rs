//! Save-location preparation from explicit session topology and persistence identity.

use super::solar::SolarError;
use crate::engine::battle::schema::GameRecord as CGameState;
pub use crate::engine::save::location::LocationError;
use crate::engine::save::location::{OrbitLocation, SolarLocation};
use crate::engine::world::generation::SolarSnapshot;
use crate::engine::world::planet_info::ScanRetrieveMask;
use crate::planets::solar_system::BodyLocation;

/// `GetFrameIndex (frame)`: 0 for NULL, else the frame's `Index`.
fn frame_index(frame: crate::graphics::drawable::FrameId) -> u16 {
    frame.index()
}

/// Prepare the save location from the active solar session.
pub fn prepare_solar_location(
    game: &CGameState,
    state: &SolarSnapshot,
) -> Result<SolarLocation, LocationError> {
    if !state.in_orbit {
        return prepare_non_orbital_location(game, state);
    }
    if state.inner_planet.is_none() {
        return Err(LocationError::OrbitOutsideInnerSystem);
    }
    let body = state.selected.ok_or(SolarError::OrbitalWorldMissing)?;
    let location = BodyLocation::new(state.star, body);
    Ok(SolarLocation::Orbit(OrbitLocation {
        in_orbit: body.saved_orbit(),
        star_index: usize::from(location.star),
        planet: usize::from(location.planet),
        moon: usize::from(location.moon),
        scan: ScanRetrieveMask::from_array(&state.retrieval_masks),
        planet_count: state.planet_count,
        moon_counts: state.planets.map(|planet| planet.satellites),
    }))
}

/// `saveNonOrbitalLocation ()`.
fn prepare_non_orbital_location(
    game: &CGameState,
    state: &SolarSnapshot,
) -> Result<SolarLocation, LocationError> {
    let ship_facing = frame_index(game.ship_stamp.frame)
        .checked_add(1)
        .ok_or(LocationError::FrameIndexOverflow)?;
    // (ip_planet, ip_location): the outer system keeps the location.
    let inner = state
        .inner_planet
        .map(|planet| (planet + 1, state.sun_location));

    Ok(SolarLocation::NonOrbital { ship_facing, inner })
}
