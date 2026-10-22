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
            static mut GlobData: crate::comm::locdata::CGlobData;
        }
        unsafe { GlobData.sis_state }
    }
    fn has_escorts(&self) -> bool {
        extern "C" {
            static mut GlobData: crate::comm::locdata::CGlobData;
        }
        unsafe { !GlobData.game_state.built_ship_q.head.is_null() }
    }
    fn crew_cost(&self) -> u8 {
        extern "C" {
            static mut GlobData: crate::comm::locdata::CGlobData;
        }
        unsafe { GlobData.game_state.crew_cost }
    }
    fn set_crew_cost(&mut self, value: u8) {
        extern "C" {
            static mut GlobData: crate::comm::locdata::CGlobData;
        }
        unsafe {
            GlobData.game_state.crew_cost = value;
        }
    }
    fn number(&mut self, value: i32) {
        extern "C" {
            fn NPCNumber(value: i32, format: *const std::ffi::c_char);
        }
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
