//! Save-summary reads from the supplied owning solar session.

use super::summary::{utf8_string_copy, SessionSummary, SA_MATRA_STRING};
use crate::engine::save::summary as summary_values;
use crate::engine::world::generation::{Body, SolarSnapshot};
use crate::savegame::types::SIS_NAME_SIZE;

/// What the solar-system state and the game strings say about the game.
///
/// Each method is one question `PrepareSummary` asks. An implementation reads
/// the live state; it does not invent an answer when the state has none.
pub trait SolarSystemOwner {
    /// `GetPlanetOrMoonName` into a `PlanetName`-sized buffer.
    fn planet_or_moon_name(&self) -> Result<[u8; SIS_NAME_SIZE], SolarError>;
    /// `playerInPlanetOrbit ()`.
    fn player_in_planet_orbit(&self) -> bool;
    /// `GAME_STRING (PLANET_NUMBER_BASE + 32)` copied into a `PlanetName`
    /// buffer.
    fn last_battle_planet_name(&self) -> [u8; SIS_NAME_SIZE];
    fn prepare_summary(&self, activity: u16, starbase: bool) -> Result<SessionSummary, SolarError> {
        match activity & 0xff {
            4 => Ok(SessionSummary::Planet {
                name: self.planet_or_moon_name()?,
                in_orbit: !starbase && self.player_in_planet_orbit(),
            }),
            1 => Ok(SessionSummary::FinalBattle {
                name: self.last_battle_planet_name(),
            }),
            _ => Ok(SessionSummary::None),
        }
    }
}

/// `PLANET_NUMBER_BASE` (gamestr.h): the first planet name in `GameStrings`.
const PLANET_NUMBER_BASE: u16 = summary_values::PLANET_NUMBER_BASE as u16;
/// `STARBASE_STRING_BASE` (gamestr.h).
const STARBASE_STRING_BASE: u16 = summary_values::STARBASE_STRING_BASE as u16;
/// `SOL_DEFINED`, `SPATHI_DEFINED` and `SAMATRA_DEFINED` (gendef.h): the
/// `STAR_DESC.Index` of the systems with named bodies.
const SOL_DEFINED: u8 = summary_values::SOL_DEFINED as u8;
const SAMATRA_DEFINED: u8 = summary_values::SAMATRA_DEFINED as u8;

const _: () = {
    // `GAME_STRING (PLANET_NUMBER_BASE + 32)` is Sa-Matra.
    assert!(SA_MATRA_STRING == PLANET_NUMBER_BASE + 32);
    // `u16`/`u8` hold every one of these.
    assert!(summary_values::PLANET_NUMBER_BASE as u64 <= u16::MAX as u64);
    assert!(summary_values::STARBASE_STRING_BASE as u64 <= u16::MAX as u64);
    assert!(summary_values::SOL_DEFINED as u64 <= u8::MAX as u64);
    assert!(summary_values::SAMATRA_DEFINED as u64 <= u8::MAX as u64);
};

pub use super::summary::SolarError;

/// Where `GAME_STRING (index)` text comes from.
///
/// The game string table is still loaded by the C resource code; the live
/// implementation borrows it through the established `comm::hail::game_string`
/// accessor. Tests use a table of their own.
pub trait GameStrings {
    /// The NUL-free text of `GAME_STRING (index)`.
    fn game_string(&self, index: u16) -> Vec<u8>;
}

/// The borrowed solar-system owner of a running game, as `PrepareSummary` and
/// `GetPlanetOrMoonName` see it.
pub struct SolarSummary<'a> {
    /// No visit is supplied while another mode owns the campaign.
    state: Option<SolarSnapshot>,
    /// `CurStarDescPtr->Index`; `None` when `CurStarDescPtr` is NULL.
    star_index: Option<u8>,
    /// `GLOBAL_SIS (PlanetName)`.
    sis_planet_name: [u8; SIS_NAME_SIZE],
    strings: &'a dyn GameStrings,
}

impl<'a> SolarSummary<'a> {
    pub fn new(
        state: Option<SolarSnapshot>,
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

    fn in_inner_system(state: &SolarSnapshot) -> bool {
        state.inner_planet.is_some()
    }

    fn in_orbit(&self) -> bool {
        self.state.is_some_and(|state| state.in_orbit)
    }

    /// `GetNamedPlanetaryBody ()`: the text of the body's own name, or `None`
    /// when it has none.
    fn named_planetary_body(&self) -> Result<Option<Vec<u8>>, SolarError> {
        let (Some(star), Some(state)) = (self.star_index, self.state) else {
            return Ok(None);
        };
        if !Self::in_inner_system(&state) {
            return Ok(None);
        }
        let world = state.selected.ok_or(SolarError::OrbitalWorldMissing)?;
        let string = |index: u16| Some(self.strings.game_string(index));
        let named = match (star, world) {
            (SOL_DEFINED, Body::Planet(planet)) => string(PLANET_NUMBER_BASE + planet as u16),
            (SOL_DEFINED, Body::Moon { planet, moon }) => match (planet, moon) {
                (2, 0) => string(STARBASE_STRING_BASE),
                (2, 1) => string(PLANET_NUMBER_BASE + 9),
                (4, 0..=3) => string(PLANET_NUMBER_BASE + 10 + moon as u16),
                (5, 0) => string(PLANET_NUMBER_BASE + 14),
                (7, 0) => string(PLANET_NUMBER_BASE + 15),
                _ => None,
            },
            // `matchWorld (.., 4, 0)`: the first moon of the fifth planet.
            (SAMATRA_DEFINED, Body::Moon { planet: 4, moon: 0 }) => string(PLANET_NUMBER_BASE + 32),
            // Spathi's first planet is `#ifdef NOTYET` in C and has no name.
            _ => None,
        };
        Ok(named)
    }
}

impl SolarSystemOwner for SolarSummary<'_> {
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
        if !Self::in_inner_system(&state) {
            return Ok(name);
        }
        let Some(Body::Moon { moon, .. }) = state.selected else {
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
