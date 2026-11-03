//! Typed access to the existing packed story-state schema.
pub use super::game_state::{StoryKey, STATE_KEYS};

pub fn bit_range(name: &str) -> Option<(u32, u32)> {
    let key = StoryKey::from_name(name)?;
    Some((key.start() as u32, key.end() as u32))
}

#[must_use]
pub fn get_game_state(world: &crate::engine::World, key: StoryKey) -> u32 {
    world.story_state(key)
}

pub fn set_game_state(world: &mut crate::engine::World, key: StoryKey, value: u32) {
    world.set_story_state(key, value)
}
