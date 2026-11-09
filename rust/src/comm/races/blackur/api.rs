use super::Handler;
use crate::comm::races::encounter::EncounterApi;
use crate::engine::StoryKey;

pub(crate) trait BlackurApi: EncounterApi<Handler> {
    fn battle_group(&self, key: StoryKey) -> (u32, u32);

    fn in_samatra_battle(&self) -> bool {
        let (battle_group, samatra_group) = self.battle_group(StoryKey::SAMATRA_GRPOFFS0);
        battle_group != 0 && battle_group == samatra_group
    }
}

#[cfg(test)]
impl BlackurApi for crate::comm::races::encounter::testing::Fake<Handler, (u32, u32)> {
    fn battle_group(&self, _key: StoryKey) -> (u32, u32) {
        self.world
    }
}
