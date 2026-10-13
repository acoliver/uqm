//! Capture and restore of the running game's "GlSt", "GmSt", "Evts", "Scan",
//! "Grps", "BtGp", "RacQ", "NpcQ", "ShpQ", "IGpQ" and "Enct" sections, plus the
//! "Summ" summary and the "Star" location of a save.
//!
//! The first three are the part of a save that lives in `GlobData`, the Rust
//! game-state singleton and the game clock's event queue; the next three are
//! the state files (`statefiles`); then come the fleet and ship queues of
//! `GlobData` (`queues`) and its interplanetary group and encounter queues
//! (`groups`). `summary` and `star` read the flagship, the clock, the game
//! state and the star catalog the same way. The functions here work on those
//! live structures, not on copies. The `SaveGame`/`LoadGame` orchestration, the
//! initialisation of the solar-system state and the owner of the star catalog
//! are not ported yet (`solar` ports only the reads of the summary), so nothing
//! here is wired to a menu and no selector here makes a save atomic.

pub mod events;
pub mod global;
pub mod groups;
#[cfg(not(test))]
pub mod live;
pub mod queues;
pub mod solar;
pub mod star;
pub mod statefiles;
pub mod summary;

use std::fmt;

use super::types::{EventEntry, GlobalState, GAME_STATE_BYTES};
use crate::comm::locdata::CGameState;
use crate::state::ffi::{replace_game_state, snapshot_game_state, GameStateOwnerError};

pub use events::EventError;

/// The sections of a save that this module owns, as read from the live game.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeSections {
    pub global: GlobalState,
    pub game_state: [u8; GAME_STATE_BYTES],
    pub events: Vec<EventEntry>,
}

/// What a restore leaves for the caller to act on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Restored {
    /// The activity the save was made in (`NextActivity` in `LoadGame`).
    /// The live `current_activity` is unchanged.
    pub next_activity: u16,
}

/// Why the runtime sections could not be captured or restored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeError {
    Events(EventError),
    GameState(GameStateOwnerError),
    StateFiles(statefiles::StateSectionError),
    Queues(queues::QueueError),
    Groups(groups::GroupError),
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Events(error) => write!(f, "event queue: {error}"),
            Self::GameState(error) => write!(f, "game state: {error}"),
            Self::StateFiles(error) => write!(f, "state files: {error}"),
            Self::Queues(error) => write!(f, "ship queues: {error}"),
            Self::Groups(error) => write!(f, "group queues: {error}"),
        }
    }
}

impl std::error::Error for RuntimeError {}

impl From<EventError> for RuntimeError {
    fn from(error: EventError) -> Self {
        Self::Events(error)
    }
}

impl From<queues::QueueError> for RuntimeError {
    fn from(error: queues::QueueError) -> Self {
        Self::Queues(error)
    }
}

impl From<groups::GroupError> for RuntimeError {
    fn from(error: groups::GroupError) -> Self {
        Self::Groups(error)
    }
}

impl From<GameStateOwnerError> for RuntimeError {
    fn from(error: GameStateOwnerError) -> Self {
        Self::GameState(error)
    }
}

/// Read "GlSt" from `game`, "GmSt" from the Rust game-state singleton (the
/// owner of those bits; the array in `game` is only a shadow) and "Evts" from
/// the clock's event queue.
pub fn capture_runtime(game: &CGameState) -> Result<RuntimeSections, RuntimeError> {
    Ok(RuntimeSections {
        global: global::capture_global_state(game),
        game_state: snapshot_game_state()?,
        events: events::capture_events(&game.game_clock.event_q)?,
    })
}

/// Make the live game hold the given sections.
///
/// Everything that can be rejected is checked before anything changes, and the
/// one remaining fallible step, replacing the game-state singleton, runs
/// before the first write to `game`. The C shadow of the game-state bits is
/// written too, as `LoadGameState` does, so C code that still reads it sees
/// the loaded bits.
pub fn restore_runtime(
    game: &mut CGameState,
    global: &GlobalState,
    game_state: &[u8; GAME_STATE_BYTES],
    events: &[EventEntry],
) -> Result<Restored, RuntimeError> {
    events::validate_restore(&game.game_clock.event_q, events)?;
    replace_game_state(game_state)?;
    game.game_state_bytes = *game_state;
    events::install_events(&mut game.game_clock.event_q, events);
    Ok(Restored {
        next_activity: global::apply_global_state(game, global),
    })
}

#[cfg(test)]
mod group_tests;
#[cfg(test)]
mod queue_tests;
#[cfg(test)]
mod solar_tests;
#[cfg(test)]
mod statefile_tests;
#[cfg(test)]
mod summary_tests;
#[cfg(test)]
mod tests;
