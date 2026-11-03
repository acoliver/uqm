//! Typed access to the existing packed story-state schema.
pub use super::game_state::{StoryKey, STATE_KEYS};

pub fn bit_range(name: &str) -> Option<(u32, u32)> {
    let key = StoryKey::from_name(name)?;
    Some((key.start() as u32, key.end() as u32))
}

#[must_use]
pub fn get_game_state(key: StoryKey) -> u32 {
    use crate::engine::World as GameWorld;
    // SAFETY: legacy story access is synchronous on the published game thread.
    unsafe { (*GameWorld::current()).story_state(key) }
}

pub fn set_game_state(key: StoryKey, value: u32) {
    use crate::engine::World as GameWorld;
    // SAFETY: legacy story access is synchronous on the published game thread.
    unsafe { (*GameWorld::current()).set_story_state(key, value) }
}
