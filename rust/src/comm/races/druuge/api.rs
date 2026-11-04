use super::Handler;
use crate::comm::locdata::CSisState;
use crate::comm::races::encounter::EncounterApi;
#[cfg(not(test))]
use crate::engine::{World, WorldData};

pub(super) trait DruugeApi: EncounterApi<Handler> {
    fn ship(&self) -> CSisState;
    fn has_escorts(&self) -> bool;
    fn crew_cost(&self) -> u8;
    fn set_crew_cost(&mut self, value: u8);
    fn number(&mut self, value: i32);
}

#[cfg(not(test))]
impl DruugeApi for crate::comm::races::script::live::LiveScript<'_, '_, Handler> {
    fn ship(&self) -> CSisState {
        self.cx.borrow_mut().with_legacy(|| unsafe {
            let world = WorldData::current();
            std::ptr::addr_of!((**world).flagship)
                .cast::<CSisState>()
                .read()
        })
    }
    fn has_escorts(&self) -> bool {
        self.cx.borrow_mut().with_legacy(|| {
            {
                World::current().read(|world| world.built_ships_ref().head().is_some())
            }
        })
    }
    fn crew_cost(&self) -> u8 {
        self.cx
            .borrow_mut()
            .with_legacy(|| WorldData::current().read(|owner| owner.game.crew_cost))
    }
    fn set_crew_cost(&mut self, value: u8) {
        self.cx.borrow_mut().with_legacy(|| unsafe {
            (**WorldData::current()).game.crew_cost = value;
        })
    }
    fn number(&mut self, value: i32) {
        self.cx.borrow_mut().with_legacy(|| {
            use crate::comm::commglue_abi::runtime::NPCNumber;
            unsafe {
                NPCNumber(value, std::ptr::null());
            }
        })
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
