//! Live save capture over the supplied campaign, kernel and optional solar visit.

use crate::engine::{Kernel, World};
use crate::strings::string_table::{GetStringAddress, SetAbsStringTableIndex};
use std::fmt;

use super::location::{self, LocationError};
use super::solar::{GameStrings, SolarSummary, SolarSystemOwner};
use super::star::{self, StarError};
use crate::engine::save::{snapshot, SessionView};
use crate::engine::world::generation::SolarSnapshot;
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
    state: Option<SolarSnapshot>,
) -> anyhow::Result<SaveFile> {
    let strings = LiveGameStrings(kernel);
    let solar = SolarSummary::new(
        state,
        state.map(|solar| solar.generator_index),
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

/// # Safety
/// The campaign's ship frame remains live while its saved facing is read.
pub unsafe fn prepare_live_solar_location(
    world: &mut World,
    solar: &SolarSnapshot,
) -> Result<(), LiveLocationError> {
    let location = location::prepare_solar_location(&world.data.game, solar)
        .map_err(LiveLocationError::Location)?;
    crate::engine::save::location::apply_solar_location(world, location)
        .map_err(LiveLocationError::Location)
}
