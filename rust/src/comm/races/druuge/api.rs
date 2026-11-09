use crate::engine::StoryKey;

pub(crate) const EGG_CASES: [StoryKey; 3] = [
    StoryKey::EGG_CASE0_ON_SHIP,
    StoryKey::EGG_CASE1_ON_SHIP,
    StoryKey::EGG_CASE2_ON_SHIP,
];

use super::Handler;
use crate::comm::locdata::CSisState;
use crate::comm::races::encounter::EncounterApi;

pub(crate) trait DruugeApi: EncounterApi<Handler> {
    fn ship(&self) -> CSisState;
    fn has_escorts(&self) -> bool;
    fn crew_cost(&self) -> u8;
    fn set_crew_cost(&mut self, value: u8);
    fn number(&mut self, value: i32);
}

#[cfg(test)]
pub(crate) struct World {
    pub ship: CSisState,
    pub escorts: bool,
    pub crew_cost: u8,
    pub numbers: Vec<i32>,
}
#[cfg(test)]
impl Default for World {
    fn default() -> Self {
        Self {
            ship: unsafe { std::mem::zeroed() },
            escorts: false,
            crew_cost: 3,
            numbers: Vec::new(),
        }
    }
}
#[cfg(test)]
impl DruugeApi for super::super::encounter::testing::Fake<Handler, World> {
    fn ship(&self) -> CSisState {
        self.world.ship
    }
    fn has_escorts(&self) -> bool {
        self.world.escorts
    }
    fn crew_cost(&self) -> u8 {
        self.world.crew_cost
    }
    fn set_crew_cost(&mut self, value: u8) {
        self.world.crew_cost = value;
    }
    fn number(&mut self, value: i32) {
        self.world.numbers.push(value);
    }
}
