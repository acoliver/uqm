//! Apply prepared solar location and scan data to the supplied campaign.

use super::summary::SolarError;
use crate::engine::world::planet_info::{PlanetInfoManager, ScanRetrieveMask};
use crate::engine::world::state_file::StateFileError;
use crate::engine::world::story::GameStateOwnerError;
use crate::engine::{StoryKey, World};
use crate::graphics::native_abi::POINT;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LocationError {
    Solar(SolarError),
    FrameIndexOverflow,
    OrbitOutsideInnerSystem,
    IpPlanetNotSet,
    PlanetCountOutOfRange { found: u8 },
    Owners(GameStateOwnerError),
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

pub struct OrbitLocation {
    pub in_orbit: u8,
    pub star_index: usize,
    pub planet: usize,
    /// Zero for the planet, otherwise the moon's index plus one.
    pub moon: usize,
    pub scan: ScanRetrieveMask,
    pub planet_count: u8,
    pub moon_counts: [u8; 16],
}

pub enum SolarLocation {
    NonOrbital {
        ship_facing: u16,
        inner: Option<(u8, POINT)>,
    },
    Orbit(OrbitLocation),
}

pub fn apply_solar_location(
    world: &mut World,
    location: SolarLocation,
) -> Result<(), LocationError> {
    match location {
        SolarLocation::NonOrbital { ship_facing, inner } => {
            let game = &mut world.data.game;
            game.ship_facing = ship_facing;
            game.in_orbit = 0;
            match inner {
                None => game.ip_planet = 0,
                Some((ip_planet, location)) => {
                    game.ip_planet = ip_planet;
                    game.ip_location = location;
                }
            }
        }
        SolarLocation::Orbit(orbit) => {
            if world.data.game.ip_planet == 0 {
                return Err(LocationError::IpPlanetNotSet);
            }
            let bits = world
                .story
                .as_mut()
                .ok_or(LocationError::Owners(GameStateOwnerError::NotInitialized))?;
            let key = StoryKey::PLANETARY_CHANGE;
            if bits.get_state(key.start(), key.end()) != 0 {
                let planets = usize::from(orbit.planet_count);
                if planets > orbit.moon_counts.len() {
                    return Err(LocationError::PlanetCountOutOfRange {
                        found: orbit.planet_count,
                    });
                }
                PlanetInfoManager::new(&mut world.files)
                    .put_planet_info(
                        orbit.star_index,
                        orbit.planet,
                        orbit.moon,
                        &orbit.scan,
                        &orbit.moon_counts[..planets],
                    )
                    .map_err(LocationError::PlanetInfo)?;
                bits.set_state(key.start(), key.end(), 0);
            }
            world.data.game.in_orbit = orbit.in_orbit;
        }
    }
    Ok(())
}
