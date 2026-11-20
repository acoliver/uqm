//! Story-state and state-file access on the campaign world.

use std::os::raw::{c_int, c_uchar};

use super::game_state::{StoryState, NUM_GAME_STATE_BYTES};
use crate::world::planet_info::PlanetInfoManager;
use crate::world::state_file::StateFileManager;

pub fn rust_init_game_state(world: &mut crate::World) {
    world.story = Some(StoryState::new());
    world.files.canned_nodes.clear();
}

pub fn rust_get_game_state_bits(world: &crate::World, start_bit: c_int, end_bit: c_int) -> c_uchar {
    if let Some((start_bit, end_bit)) = normalize_bit_range(start_bit, end_bit) {
        read_story(world, |state| state.get_state(start_bit, end_bit))
    } else {
        0
    }
}

pub fn rust_set_game_state_bits(
    world: &mut crate::World,
    start_bit: c_int,
    end_bit: c_int,
    value: c_uchar,
) {
    if let Some((start_bit, end_bit)) = normalize_bit_range(start_bit, end_bit) {
        write_story(world, |state| {
            state.set_state(start_bit, end_bit, value);
        });
    }
}

pub fn rust_get_game_state_32(world: &crate::World, start_bit: c_int) -> u32 {
    let Some(start_bit) = normalize_start_bit_32(start_bit) else {
        return 0;
    };

    read_story(world, |state| state.get_state_32(start_bit))
}

pub fn rust_set_game_state_32(world: &mut crate::World, start_bit: c_int, value: u32) {
    let Some(start_bit) = normalize_start_bit_32(start_bit) else {
        return;
    };

    write_story(world, |state| {
        state.set_state_32(start_bit, value);
    });
}

pub fn rust_copy_game_state(
    world: &mut crate::World,
    dest_bit: c_int,
    src_start_bit: c_int,
    src_end_bit: c_int,
) {
    let Some((dest_bit, src_start_bit, src_end_bit)) =
        normalize_copy_bits(dest_bit, src_start_bit, src_end_bit)
    else {
        return;
    };

    write_story(world, |state| {
        let snapshot = StoryState::from_bytes(state.as_bytes());
        state.copy_state(dest_bit, &snapshot, src_start_bit, src_end_bit);
    });
}

pub fn rust_reset_game_state(world: &mut crate::World) {
    write_story(world, |state| {
        state.reset();
    });
}

pub use crate::world::story::GameStateOwnerError;

/// Copy of the singleton's bytes: the game state a save writes as "GmSt".
pub fn snapshot_game_state(
    world: &crate::World,
) -> Result<[u8; NUM_GAME_STATE_BYTES], GameStateOwnerError> {
    world
        .story
        .as_ref()
        .map(|state| *state.as_bytes())
        .ok_or(GameStateOwnerError::NotInitialized)
}

/// Borrow the story and file fields together for a synchronous game-thread operation.
pub fn with_state_owners<R>(
    world: &mut crate::World,
    f: impl FnOnce(&mut StoryState, &mut StateFileManager) -> R,
) -> Result<R, GameStateOwnerError> {
    let game = world
        .story
        .as_mut()
        .ok_or(GameStateOwnerError::NotInitialized)?;
    Ok(f(game, &mut world.files))
}

/// Borrow the owned state files for a synchronous game-thread operation.
pub fn with_state_files<R>(
    world: &mut crate::World,
    f: impl FnOnce(&mut StateFileManager) -> R,
) -> Result<R, GameStateOwnerError> {
    Ok(write_files(world, f))
}

/// Replace the whole singleton with `bytes`, creating it when absent: a load
/// defines the game state whether or not a New Game ran first.
pub fn replace_game_state(
    world: &mut crate::World,
    bytes: &[u8; NUM_GAME_STATE_BYTES],
) -> Result<(), GameStateOwnerError> {
    world.story = Some(StoryState::from_bytes(bytes));
    Ok(())
}

pub fn rust_init_planet_info(world: &mut crate::World, num_stars: c_int) -> c_int {
    if num_stars < 0 {
        return 0;
    }

    match files_result(world, |files| {
        let mut manager = PlanetInfoManager::new(files);
        manager.init_planet_info(num_stars as usize)
    }) {
        Some(()) => 1,
        None => 0,
    }
}

pub fn rust_uninit_planet_info(world: &mut crate::World) {
    write_files(world, |files| {
        let mut manager = PlanetInfoManager::new(files);
        let _ = manager.uninit_planet_info();
    });
}

fn normalize_bit_range(start_bit: c_int, end_bit: c_int) -> Option<(usize, usize)> {
    if start_bit < 0 || end_bit < 0 || end_bit < start_bit {
        return None;
    }

    let start_bit = start_bit as usize;
    let end_bit = end_bit as usize;
    if end_bit >= super::game_state::NUM_GAME_STATE_BITS || end_bit - start_bit >= 8 {
        return None;
    }

    Some((start_bit, end_bit))
}

fn normalize_start_bit_32(start_bit: c_int) -> Option<usize> {
    if start_bit < 0 {
        return None;
    }

    let start_bit = start_bit as usize;
    if start_bit + 31 >= super::game_state::NUM_GAME_STATE_BITS {
        return None;
    }

    Some(start_bit)
}

fn normalize_copy_bits(
    dest_bit: c_int,
    src_start_bit: c_int,
    src_end_bit: c_int,
) -> Option<(usize, usize, usize)> {
    if dest_bit < 0 || src_start_bit < 0 || src_end_bit < 0 || src_end_bit < src_start_bit {
        return None;
    }

    let dest_bit = dest_bit as usize;
    let src_start_bit = src_start_bit as usize;
    let src_end_bit = src_end_bit as usize;
    let width = src_end_bit - src_start_bit;

    if src_end_bit >= super::game_state::NUM_GAME_STATE_BITS {
        return None;
    }
    if dest_bit + width >= super::game_state::NUM_GAME_STATE_BITS {
        return None;
    }

    Some((dest_bit, src_start_bit, src_end_bit))
}

fn read_story<R: Default>(world: &crate::World, f: impl FnOnce(&StoryState) -> R) -> R {
    world.story.as_ref().map_or_else(R::default, f)
}
fn write_story<R: Default>(world: &mut crate::World, f: impl FnOnce(&mut StoryState) -> R) -> R {
    world.story.as_mut().map_or_else(R::default, f)
}
fn write_files<R>(world: &mut crate::World, f: impl FnOnce(&mut StateFileManager) -> R) -> R {
    f(&mut world.files)
}
fn files_result<R, E>(
    world: &mut crate::World,
    f: impl FnOnce(&mut StateFileManager) -> Result<R, E>,
) -> Option<R> {
    write_files(world, f).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::StoryKey;
    use serial_test::serial;

    #[test]
    #[serial]
    fn test_rust_init_game_state() {
        let mut game = crate::Game::new();
        {
            {
                rust_init_game_state(&mut game.world);

                assert!(read_story(&game.world, |_| true));
            }
        }
    }

    #[test]
    #[serial]
    fn starting_a_second_game_does_not_inherit_the_first_games_state() {
        let mut game = crate::Game::new();
        {
            {
                // InitGlobData memsets the C shadow and calls this to match, so a New
                // Game after finishing one must not remember what the player already
                // did. Leaving the bits set makes the Ur-Quan probe never appear and
                // the moon base never generate, because the game believes both are
                // already done.
                {
                    rust_init_game_state(&mut game.world);
                    game.world.set_story_state(StoryKey::MOONBASE_DESTROYED, 1);
                    game.world.set_story_state(StoryKey::MOONBASE_ON_SHIP, 1);
                    assert_eq!(game.world.story_state(StoryKey::MOONBASE_DESTROYED), 1);

                    rust_init_game_state(&mut game.world);

                    assert_eq!(
                        game.world.story_state(StoryKey::MOONBASE_DESTROYED),
                        0,
                        "a new game must start with the moon base intact"
                    );
                    assert_eq!(game.world.story_state(StoryKey::MOONBASE_ON_SHIP), 0);
                }
            }
        }
    }

    #[test]
    #[serial]
    fn a_new_game_clears_every_state_bit() {
        let mut game = crate::Game::new();
        {
            {
                use super::super::game_state::NUM_GAME_STATE_BITS;

                // Every bit, including the partial trailing byte. The last six bits
                // belong to SAMATRA_GRPOFFS3, so stopping at the last whole byte would
                // let a reset regression confined to them pass unnoticed.
                let ranges: Vec<(c_int, c_int, c_uchar)> = (0..NUM_GAME_STATE_BITS)
                    .step_by(8)
                    .map(|start| {
                        let end = (start + 7).min(NUM_GAME_STATE_BITS - 1);
                        let width = end - start + 1;
                        let full = if width == 8 {
                            0xFF
                        } else {
                            // set_state_bits_raw rejects a value wider than the field.
                            ((1u16 << width) - 1) as u8
                        };
                        (start as c_int, end as c_int, full)
                    })
                    .collect();

                {
                    rust_init_game_state(&mut game.world);
                    for (start, end, value) in &ranges {
                        rust_set_game_state_bits(&mut game.world, *start, *end, *value);
                        assert_eq!(
                            rust_get_game_state_bits(&game.world, *start, *end),
                            *value,
                            "bit range {start}..={end} did not accept its value"
                        );
                    }

                    rust_init_game_state(&mut game.world);

                    for (start, end, _) in &ranges {
                        assert_eq!(
                            rust_get_game_state_bits(&game.world, *start, *end),
                            0,
                            "bit range {start}..={end} survived a new game"
                        );
                    }
                }
                assert_eq!(
                    ranges.last().map(|(_, end, _)| *end),
                    Some(NUM_GAME_STATE_BITS as c_int - 1),
                    "the final state bit must be covered"
                );
            }
        }
    }

    #[test]
    #[serial]
    fn test_typed_state_keys_use_generated_ranges() {
        let mut game = crate::Game::new();
        {
            {
                rust_init_game_state(&mut game.world);

                {
                    game.world.set_story_state(StoryKey::SHOFIXTI_VISITS, 5);
                    game.world.set_story_state(StoryKey::SHOFIXTI_RECRUITED, 1);
                    game.world.set_story_state(StoryKey::SPATHI_VISITS, 3);

                    assert_eq!(game.world.story_state(StoryKey::SHOFIXTI_VISITS), 5);
                    assert_eq!(game.world.story_state(StoryKey::SHOFIXTI_RECRUITED), 1);
                    assert_eq!(game.world.story_state(StoryKey::SPATHI_VISITS), 3);
                    assert!(StoryKey::from_name("NOT_A_REAL_STATE").is_none());
                }
            }
        }
    }

    #[test]
    #[serial]
    fn test_rust_get_set_game_state_bits() {
        let mut game = crate::Game::new();
        {
            {
                rust_init_game_state(&mut game.world);
                rust_set_game_state_bits(&mut game.world, 0, 2, 5);
                let result = rust_get_game_state_bits(&game.world, 0, 2);
                assert_eq!(result, 5);
            }
        }
    }

    #[test]
    #[serial]
    fn test_rust_get_set_game_state_32() {
        let mut game = crate::Game::new();
        {
            {
                rust_init_game_state(&mut game.world);

                let test_value = 0xDEADBEEF;
                rust_set_game_state_32(&mut game.world, 0, test_value);

                let result = rust_get_game_state_32(&game.world, 0);
                assert_eq!(result, test_value);
            }
        }
    }

    #[test]
    #[serial]
    fn test_rust_reset_game_state() {
        let mut game = crate::Game::new();
        {
            {
                rust_init_game_state(&mut game.world);

                rust_set_game_state_bits(&mut game.world, 0, 7, 0xFF);
                rust_reset_game_state(&mut game.world);

                let result = rust_get_game_state_bits(&game.world, 0, 7);
                assert_eq!(result, 0);
            }
        }
    }
}
