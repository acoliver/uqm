use super::Handler;
use crate::comm::races::encounter::EncounterApi;
use crate::engine::StoryKey;

pub(crate) trait UrquanApi: EncounterApi<Handler> {
    fn group_battle(&self, state_key: StoryKey) -> bool;
    fn intro_mode(&mut self, mode: i32, duration: u32);
}

#[cfg(test)]
#[derive(Default)]
pub(crate) struct World {
    pub group_battles: std::collections::HashSet<StoryKey>,
    pub intro_modes: Vec<(i32, u32)>,
}

#[cfg(test)]
impl UrquanApi for crate::comm::races::encounter::testing::Fake<Handler, World> {
    fn group_battle(&self, state_key: StoryKey) -> bool {
        self.world.group_battles.contains(&state_key)
    }

    fn intro_mode(&mut self, mode: i32, duration: u32) {
        self.world.intro_modes.push((mode, duration));
    }
}
