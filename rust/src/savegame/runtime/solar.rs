//! The solar-system questions `PrepareSummary` (save.c) asks, answered in Rust
//! from the live `SOLARSYS_STATE`.
//!
//! This is the read side of solarsys.c ported: `playerInSolarSystem`,
//! `playerInPlanetOrbit`, `playerInInnerSystem`, `worldIsPlanet`,
//! `worldIsMoon`, `planetIndex`, `moonIndex`, `matchWorld`,
//! `GetNamedPlanetaryBody` and `GetPlanetOrMoonName`. None of them is called
//! in C any more from the summary: the answers are computed here from the
//! state the C game keeps.
//!
//! What stays C is the *initialisation* of that state: `pSolarSysState`, the
//! orbit tables `ExploreSolarSys` fills in, `CurStarDescPtr` and the game
//! string table. They are borrowed through [`SolarSystem`], which holds one
//! shared borrow of the state, the star index and a string source. The layout
//! of `SOLARSYS_STATE` is bindgen's reading of planets.h
//! (`c_bindings::global_state`), so no field offset is written by hand.
//!
//! The reads follow the C code, including its limits:
//!
//! * a world is an entry of `PlanetDesc` (its `pPrevDesc` is the sun) or of
//!   `MoonDesc` (its `pPrevDesc` is a planet); indexes are the element
//!   positions in those arrays;
//! * the "inner system" test compares `pBaseDesc` with the start of
//!   `PlanetDesc`;
//! * only Sol (planets, Earth's starbase and the moons of Earth, Jupiter,
//!   Saturn and Neptune) and Sa-Matra's system have named bodies; Spathi's
//!   planet is `NOTYET` in C and unnamed here;
//! * an unnamed moon is the flagship's planet name plus `-A`, `-B`, ... as long
//!   as the suffix fits the 16-byte name.
//!
//! Where this differs from the C code on purpose: C does pointer arithmetic on
//! whatever `pOrbitalDesc` holds; here a pointer that is not an element of
//! `PlanetDesc` or `MoonDesc`, or whose `pPrevDesc` does not fit the array it
//! sits in, is a [`SolarError`]. `pBaseDesc` being neither array is the
//! assertion C has in `playerInInnerSystem`, and panics.

use std::fmt;

use super::summary::{utf8_string_copy, SolarSystemOwner, SA_MATRA_STRING};
use crate::c_bindings::global_state::{self, PLANET_DESC, SOLARSYS_STATE};
use crate::savegame::types::SIS_NAME_SIZE;

/// `PLANET_NUMBER_BASE` (gamestr.h): the first planet name in `GameStrings`.
const PLANET_NUMBER_BASE: u16 = global_state::PLANET_NUMBER_BASE as u16;
/// `STARBASE_STRING_BASE` (gamestr.h).
const STARBASE_STRING_BASE: u16 = global_state::STARBASE_STRING_BASE as u16;
/// `SOL_DEFINED`, `SPATHI_DEFINED` and `SAMATRA_DEFINED` (gendef.h): the
/// `STAR_DESC.Index` of the systems with named bodies.
const SOL_DEFINED: u8 = global_state::SOL_DEFINED as u8;
const SAMATRA_DEFINED: u8 = global_state::SAMATRA_DEFINED as u8;

const _: () = {
    // `GAME_STRING (PLANET_NUMBER_BASE + 32)` is Sa-Matra.
    assert!(SA_MATRA_STRING == PLANET_NUMBER_BASE + 32);
    // `u16`/`u8` hold every one of these.
    assert!(global_state::PLANET_NUMBER_BASE as u64 <= u16::MAX as u64);
    assert!(global_state::STARBASE_STRING_BASE as u64 <= u16::MAX as u64);
    assert!(global_state::SOL_DEFINED as u64 <= u8::MAX as u64);
    assert!(global_state::SAMATRA_DEFINED as u64 <= u8::MAX as u64);
};

/// Why a solar-system question could not be answered.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SolarError {
    /// The flagship is in an inner system but `pOrbitalDesc` is NULL, which
    /// C asserts against.
    OrbitalWorldMissing,
    /// `pOrbitalDesc` is not an element of `PlanetDesc` or `MoonDesc`, or
    /// does not sit on an element boundary.
    WorldNotInSystem,
    /// An entry of `PlanetDesc` does not orbit the sun, or an entry of
    /// `MoonDesc` orbits the sun or something that is not in `PlanetDesc`.
    WorldDisagreesWithItsTable,
}

impl fmt::Display for SolarError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for SolarError {}

/// Where `GAME_STRING (index)` text comes from.
///
/// The game string table is still loaded by the C resource code; the live
/// implementation borrows it through the established `comm::hail::game_string`
/// accessor. Tests use a table of their own.
pub trait GameStrings {
    /// The NUL-free text of `GAME_STRING (index)`.
    fn game_string(&self, index: u16) -> Vec<u8>;
}

/// A world of the live system, by position in its table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum World {
    /// `PlanetDesc[planet]`.
    Planet { planet: usize },
    /// `MoonDesc[moon]`, orbiting `PlanetDesc[planet]`.
    Moon { planet: usize, moon: usize },
}

/// The position of `world` in `table`, if it is exactly one of its elements.
/// Only addresses are compared; nothing is dereferenced.
fn position_in(table: &[PLANET_DESC], world: *const PLANET_DESC) -> Option<usize> {
    let size = std::mem::size_of::<PLANET_DESC>();
    let offset = (world as usize).wrapping_sub(table.as_ptr() as usize);
    (offset < std::mem::size_of_val(table) && offset.is_multiple_of(size)).then_some(offset / size)
}

/// The borrowed solar-system owner of a running game, as `PrepareSummary` and
/// `GetPlanetOrMoonName` see it.
pub struct SolarSystem<'a> {
    /// `pSolarSysState`; `None` when it is NULL (`!playerInSolarSystem ()`).
    state: Option<&'a SOLARSYS_STATE>,
    /// `CurStarDescPtr->Index`; `None` when `CurStarDescPtr` is NULL.
    star_index: Option<u8>,
    /// `GLOBAL_SIS (PlanetName)`.
    sis_planet_name: [u8; SIS_NAME_SIZE],
    strings: &'a dyn GameStrings,
}

impl<'a> SolarSystem<'a> {
    pub fn new(
        state: Option<&'a SOLARSYS_STATE>,
        star_index: Option<u8>,
        sis_planet_name: [u8; SIS_NAME_SIZE],
        strings: &'a dyn GameStrings,
    ) -> Self {
        Self {
            state,
            star_index,
            sis_planet_name,
            strings,
        }
    }

    /// `playerInInnerSystem ()` for a flagship that is in a solar system.
    fn in_inner_system(state: &SOLARSYS_STATE) -> bool {
        let planets = state.PlanetDesc.as_ptr();
        let moons = state.MoonDesc.as_ptr();
        assert!(
            std::ptr::eq(state.pBaseDesc, planets) || std::ptr::eq(state.pBaseDesc, moons),
            "pBaseDesc is neither PlanetDesc nor MoonDesc"
        );
        !std::ptr::eq(state.pBaseDesc, planets)
    }

    /// The world `pOrbitalDesc` names, checked against both tables.
    fn orbital_world(state: &SOLARSYS_STATE) -> Result<World, SolarError> {
        let orbital = state.pOrbitalDesc.cast_const();
        if orbital.is_null() {
            return Err(SolarError::OrbitalWorldMissing);
        }
        let sun = state.SunDesc.as_ptr();
        if let Some(planet) = position_in(&state.PlanetDesc, orbital) {
            // `worldIsPlanet`: the world orbits the sun.
            if !std::ptr::eq(state.PlanetDesc[planet].pPrevDesc, sun) {
                return Err(SolarError::WorldDisagreesWithItsTable);
            }
            return Ok(World::Planet { planet });
        }
        if let Some(moon) = position_in(&state.MoonDesc, orbital) {
            // `planetIndex` of a moon is the planet it orbits.
            let host = state.MoonDesc[moon].pPrevDesc.cast_const();
            let planet = position_in(&state.PlanetDesc, host)
                .ok_or(SolarError::WorldDisagreesWithItsTable)?;
            return Ok(World::Moon { planet, moon });
        }
        Err(SolarError::WorldNotInSystem)
    }

    /// `playerInPlanetOrbit ()`.
    fn in_orbit(&self) -> bool {
        self.state.is_some_and(|state| state.InOrbit != 0)
    }

    /// `GetNamedPlanetaryBody ()`: the text of the body's own name, or `None`
    /// when it has none.
    fn named_planetary_body(&self) -> Result<Option<Vec<u8>>, SolarError> {
        let (Some(star), Some(state)) = (self.star_index, self.state) else {
            return Ok(None);
        };
        if !Self::in_inner_system(state) {
            return Ok(None);
        }
        let world = Self::orbital_world(state)?;
        let string = |index: u16| Some(self.strings.game_string(index));
        let named = match (star, world) {
            (SOL_DEFINED, World::Planet { planet }) => string(PLANET_NUMBER_BASE + planet as u16),
            (SOL_DEFINED, World::Moon { planet, moon }) => match (planet, moon) {
                (2, 0) => string(STARBASE_STRING_BASE),
                (2, 1) => string(PLANET_NUMBER_BASE + 9),
                (4, 0..=3) => string(PLANET_NUMBER_BASE + 10 + moon as u16),
                (5, 0) => string(PLANET_NUMBER_BASE + 14),
                (7, 0) => string(PLANET_NUMBER_BASE + 15),
                _ => None,
            },
            // `matchWorld (.., 4, 0)`: the first moon of the fifth planet.
            (SAMATRA_DEFINED, World::Moon { planet: 4, moon: 0 }) => {
                string(PLANET_NUMBER_BASE + 32)
            }
            // Spathi's first planet is `#ifdef NOTYET` in C and has no name.
            _ => None,
        };
        Ok(named)
    }
}

impl SolarSystemOwner for SolarSystem<'_> {
    /// `GetPlanetOrMoonName (buf, SIS_NAME_SIZE)`.
    fn planet_or_moon_name(&self) -> Result<[u8; SIS_NAME_SIZE], SolarError> {
        if let Some(named) = self.named_planetary_body()? {
            return Ok(utf8_string_copy(&named));
        }
        // Either not named or we already have a name.
        let mut name = utf8_string_copy(&self.sis_planet_name);
        let Some(state) = self.state else {
            return Ok(name);
        };
        if !Self::in_inner_system(state) {
            return Ok(name);
        }
        let World::Moon { moon, .. } = Self::orbital_world(state)? else {
            return Ok(name);
        };
        // Orbiting an unnamed moon: "-A", "-B", ... when it fits.
        let used = name
            .iter()
            .position(|&byte| byte == 0)
            .unwrap_or(name.len());
        if SIS_NAME_SIZE - used >= 3 {
            name[used] = b'-';
            name[used + 1] = b'A' + moon as u8;
            name[used + 2] = 0;
        }
        Ok(name)
    }

    fn player_in_planet_orbit(&self) -> bool {
        self.in_orbit()
    }

    /// `GAME_STRING (PLANET_NUMBER_BASE + 32)` copied as `utf8StringCopy`.
    fn last_battle_planet_name(&self) -> [u8; SIS_NAME_SIZE] {
        utf8_string_copy(&self.strings.game_string(SA_MATRA_STRING))
    }
}
