use super::Handler;
use crate::comm::locdata::CSisState;
use crate::comm::races::encounter::EncounterApi;

pub(super) trait DruugeApi: EncounterApi<Handler> {
    fn ship(&self) -> CSisState;
    fn has_escorts(&self) -> bool;
    fn crew_cost(&self) -> u8;
    fn set_crew_cost(&mut self, value: u8);
    fn number(&mut self, value: i32);
}

#[cfg(not(test))]
impl DruugeApi for crate::comm::races::script::live::LiveScript<Handler> {
    fn ship(&self) -> CSisState {
        unsafe {
            let world = WorldFields::current();
            std::ptr::addr_of!((**world).flagship)
                .cast::<CSisState>()
                .read()
        }
    }
    fn has_escorts(&self) -> bool {
        unsafe {
            !WorldFields::current()
                .read(|owner| owner.game.built_ship_q.head)
                .is_null()
        }
    }
    fn crew_cost(&self) -> u8 {
        unsafe { WorldFields::current().read(|owner| owner.game.crew_cost) }
    }
    fn set_crew_cost(&mut self, value: u8) {
        unsafe {
            (**WorldFields::current()).game.crew_cost = value;
        }
    }
    fn number(&mut self, value: i32) {
        use crate::comm::commglue_abi::runtime::NPCNumber;
        unsafe {
            NPCNumber(value, std::ptr::null());
        }
    }
}

#[cfg(test)]
pub(super) struct World {
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

use crate::engine::WorldData as WorldFields;
