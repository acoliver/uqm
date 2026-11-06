//! Map the live campaign's story offsets and owned state files to save sections.

use super::super::battle::schema::GAME_STATE as CGameState;
use super::super::world::state_file::StateFileManager;
use super::super::world::story::GameStateOwnerError;
use super::super::world::story::{get_state_32_raw, GameState, NUM_GAME_STATE_BITS};
use super::super::StoryKey;
use super::super::World;
use super::RuntimeError;
use uqm_save::statefiles;
pub use uqm_save::statefiles::{
    initial_defined_image, initial_random_image, StateFileSections, StateSectionError,
    GROUP_HEADER_BYTES, NUM_SAVED_BATTLE_GROUPS,
};
use uqm_save::statefiles::{RestorePlan, DEFGRPINFO_FILE, RANDGRPINFO_FILE, STARINFO_FILE};
use uqm_save::GAME_STATE_BYTES;

const OFFSET_STRIDE_BITS: usize = 32;

/// Defined (encounter) groups: from `SHOFIXTI_GRPOFFS0` to the end of the
/// game-state array, one 32-bit offset each.
fn defined_group_slots() -> (usize, usize) {
    let first = StoryKey::SHOFIXTI_GRPOFFS0.start();
    (first, (NUM_GAME_STATE_BITS - first) / OFFSET_STRIDE_BITS)
}

fn defined_offsets(game_state: &[u8; GAME_STATE_BYTES]) -> Vec<u32> {
    let (first, slots) = defined_group_slots();
    (0..slots)
        .map(|slot| get_state_32_raw(game_state, first + slot * OFFSET_STRIDE_BITS))
        .collect()
}

pub fn capture_from_images(
    star_info: &[u8],
    random_groups: &[u8],
    defined_groups: &[u8],
    game_state: &[u8; GAME_STATE_BYTES],
    battle_group_ref: u32,
) -> Result<StateFileSections, StateSectionError> {
    statefiles::capture_from_images(
        star_info,
        random_groups,
        defined_groups,
        &defined_offsets(game_state),
        battle_group_ref,
    )
}

impl From<StateSectionError> for RuntimeError {
    fn from(error: StateSectionError) -> Self {
        Self::StateFiles(error)
    }
}

/// Read "Scan", "Grps" and "BtGp" from the live state files, with the
/// defined-group offsets taken from the live game-state singleton and the
/// current group from `game`.
pub fn capture_state_files(
    game: &CGameState,
    story: &Option<GameState>,
    files: &StateFileManager,
) -> Result<StateFileSections, RuntimeError> {
    let state = story.as_ref().ok_or(GameStateOwnerError::NotInitialized)?;
    let contents = |index: usize| {
        files
            .get_file(index)
            .expect("the three state files always exist")
            .contents()
    };
    let mut sections = capture_from_images(
        contents(STARINFO_FILE),
        contents(RANDGRPINFO_FILE),
        contents(DEFGRPINFO_FILE),
        state.as_bytes(),
        game.battle_group_ref,
    )?;
    sections.canned_nodes = files.canned_nodes.clone();
    Ok(sections)
}

/// Install `plan`. Both owners are held by the caller; nothing here fails.
fn commit(plan: &RestorePlan, state: &mut GameState, files: &mut StateFileManager) {
    let mut install = |index: usize, image: &[u8]| {
        files
            .replace_contents(index, image)
            .expect("the three state files always exist");
    };
    if let Some(star_info) = &plan.star_info {
        install(STARINFO_FILE, star_info);
    }
    if let Some((random, defined)) = &plan.groups {
        install(RANDGRPINFO_FILE, random);
        install(DEFGRPINFO_FILE, defined);
    }
    let (first, _) = defined_group_slots();
    for (slot, &offset) in plan.defined_offsets.iter().enumerate() {
        state.set_state_32(first + slot * OFFSET_STRIDE_BITS, offset);
    }
}

/// Make the live state files, game-state singleton and `game` hold `sections`.
///
/// Call after the game state of the save is installed (`restore_runtime`): the
/// defined-group offsets of the new files are written to the singleton.
/// `BattleGroupRef` follows in the live game record.
pub fn restore_state_files(
    game: &mut CGameState,
    story: &mut Option<GameState>,
    files: &mut StateFileManager,
    sections: &StateFileSections,
) -> Result<(), RuntimeError> {
    let state = story.as_mut().ok_or(GameStateOwnerError::NotInitialized)?;
    let plan = statefiles::plan_restore(sections, &defined_offsets(state.as_bytes()))?;
    commit(&plan, state, files);
    files.canned_nodes.clone_from(&sections.canned_nodes);
    if let Some(reference) = plan.battle_group_ref {
        game.battle_group_ref = reference;
    }
    Ok(())
}

/// `InitGroupInfo` on the live files: empty the random groups and, when
/// `first_time`, restart the defined groups.
pub fn init_group_info(world: &mut World, first_time: bool) -> Result<(), RuntimeError> {
    world
        .files
        .replace_contents(RANDGRPINFO_FILE, &initial_random_image())
        .expect("the three state files always exist");
    if first_time {
        world
            .files
            .replace_contents(DEFGRPINFO_FILE, &initial_defined_image())
            .expect("the three state files always exist");
    }
    Ok(())
}
