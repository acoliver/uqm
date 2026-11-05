//! Map explicitly borrowed campaign owners to and from decoded save sections.

mod activity;
pub mod events;
pub mod global;
pub mod groups;
pub mod ip_group_save;
pub mod location;
pub mod queues;
pub mod statefiles;
pub mod summary;

use std::fmt;

use crate::engine::battle::schema::GAME_STATE as CGameState;
use crate::engine::world::story::GameState;
use crate::engine::world::story::GameStateOwnerError;
use uqm_save::{EventEntry, GlobalState, GAME_STATE_BYTES};

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
pub fn capture_runtime(
    game: &CGameState,
    activity: u16,
    story: &Option<GameState>,
    clock: &crate::engine::clock::GameClock,
) -> Result<RuntimeSections, RuntimeError> {
    Ok(RuntimeSections {
        global: global::capture_global_state(game, activity),
        game_state: *story
            .as_ref()
            .ok_or(GameStateOwnerError::NotInitialized)?
            .as_bytes(),
        events: events::capture_events(clock)?,
    })
}

/// Make the live game hold the given sections.
///
/// Everything that can be rejected is checked before anything changes, and the
/// one remaining fallible step, replacing the game-state singleton, runs
/// before the first write to `game`. The singleton owns all story-state bits.
pub fn restore_runtime(
    game: &mut CGameState,
    story: &mut Option<GameState>,
    global: &GlobalState,
    game_state: &[u8; GAME_STATE_BYTES],
    events: &[EventEntry],
    clock: &mut crate::engine::clock::GameClock,
) -> Result<Restored, RuntimeError> {
    events::validate_restore(events)?;
    *story = Some(GameState::from_bytes(game_state));

    events::install_events(clock, events);
    Ok(Restored {
        next_activity: global::apply_global_state(game, global),
    })
}

/// Session-derived save fields prepared by the active mode.
pub struct SessionView {
    pub solar: summary::SessionSummary,
    pub star: uqm_save::StarDesc,
    pub ip_location: crate::graphics::native_abi::POINT,
}

/// Capture prepared state before restoring the live IP position and orbit marker.
pub fn snapshot(
    world: &mut crate::engine::World,
    view: SessionView,
    name: &[u8],
) -> anyhow::Result<uqm_save::SaveFile> {
    let summary = summary::capture_summary(
        world.activity(),
        &world.data.flagship,
        &world.data.game,
        world.built_ships_ref(),
        &world.story,
        view.solar,
        name,
    )?;
    let runtime = capture_runtime(&world.data.game, world.activity(), &world.story, &world.clock)?;
    world.data.game.ip_location = view.ip_location;
    world.data.game.in_orbit = 0;
    let queues = queues::capture_queues_for_activity(world)?;
    let groups = groups::capture_groups(world)?;
    let files = statefiles::capture_state_files(&world.data.game, &world.story, &world.files)?;
    Ok(uqm_save::SaveFile {
        summary,
        global: runtime.global,
        game_state: runtime.game_state,
        race_queue: queues.race_queue,
        ip_group_queue: groups.ip_group_queue,
        npc_ship_queue: queues.npc_ship_queue,
        ship_queue: queues.ship_queue,
        events: runtime.events,
        encounters: groups.encounters,
        scan: files.scan,
        canned_nodes: files.canned_nodes,
        group_list: files.group_list,
        battle_groups: files.battle_groups,
        star: view.star,
    })
}

enum StateFiles {
    Sections,
    Legacy {
        images: [Vec<u8>; 3],
        battle_group_ref: u32,
    },
}

/// Restore the save while retaining the activity of the running load menu.
pub fn apply(save: uqm_save::SaveFile, world: &mut crate::engine::World) -> anyhow::Result<u16> {
    apply_with_files(save, world, StateFiles::Sections)
}

/// Legacy saves carry complete working images instead of state-file sections.
pub fn apply_legacy(
    save: uqm_save::SaveFile,
    world: &mut crate::engine::World,
    images: [Vec<u8>; 3],
    battle_group_ref: u32,
) -> anyhow::Result<u16> {
    apply_with_files(
        save,
        world,
        StateFiles::Legacy {
            images,
            battle_group_ref,
        },
    )
}

fn apply_with_files(
    save: uqm_save::SaveFile,
    world: &mut crate::engine::World,
    files: StateFiles,
) -> anyhow::Result<u16> {
    let restored = restore_runtime(
        &mut world.data.game,
        &mut world.story,
        &save.global,
        &save.game_state,
        &save.events,
        &mut world.clock,
    )?;
    queues::restore_queues(
        world,
        &queues::QueueSections {
            race_queue: save.race_queue,
            npc_ship_queue: save.npc_ship_queue,
            ship_queue: save.ship_queue,
        },
    )?;
    groups::restore_loaded_groups(
        world,
        &groups::GroupSections {
            ip_group_queue: save.ip_group_queue,
            encounters: save.encounters,
        },
    )?;
    match files {
        StateFiles::Sections => statefiles::restore_state_files(
            &mut world.data.game,
            &mut world.story,
            &mut world.files,
            &statefiles::StateFileSections {
                scan: save.scan,
                canned_nodes: save.canned_nodes,
                group_list: save.group_list,
                battle_groups: save.battle_groups,
            },
        )?,
        StateFiles::Legacy {
            images,
            battle_group_ref,
        } => {
            world.files.canned_nodes.clear();
            for (index, image) in [0, 2, 1].into_iter().zip(&images) {
                world.files.replace_contents(index, image)?;
            }
            world.data.game.battle_group_ref = battle_group_ref;
        }
    }
    world.data.flagship = (&save.summary.sis).into();
    Ok(restored.next_activity)
}
