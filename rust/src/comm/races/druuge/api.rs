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
        extern "C" {
            static mut GlobData: crate::ships::native::abi::GLOBDATA;
        }
        unsafe {
            std::ptr::addr_of!(GlobData.SIS_state)
                .cast::<CSisState>()
                .read()
        }
    }
    fn has_escorts(&self) -> bool {
        extern "C" {
            static mut GlobData: crate::ships::native::abi::GLOBDATA;
        }
        unsafe { !GlobData.Game_state.built_ship_q.head.is_null() }
    }
    fn crew_cost(&self) -> u8 {
        extern "C" {
            static mut GlobData: crate::ships::native::abi::GLOBDATA;
        }
        unsafe { GlobData.Game_state.CrewCost }
    }
    fn set_crew_cost(&mut self, value: u8) {
        extern "C" {
            static mut GlobData: crate::ships::native::abi::GLOBDATA;
        }
        unsafe {
            GlobData.Game_state.CrewCost = value;
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
