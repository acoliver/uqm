use super::Handler;
use crate::comm::races::alliance::AllianceApi;

pub(crate) trait TalkpetApi: AllianceApi<Handler> {
    fn flags(&self) -> u8;
    fn npc_ships(&self) -> bool;
    fn umgah_alive(&mut self) -> bool;
    fn strobe(&mut self);
}

#[cfg(test)]
#[derive(Default)]
pub(crate) struct World {
    pub flags: u8,
    pub npc_ships: bool,
    pub umgah_alive: bool,
    pub strobes: usize,
    pub tracking_calls: usize,
}
#[cfg(test)]
impl TalkpetApi for super::super::encounter::testing::Fake<Handler, World> {
    fn flags(&self) -> u8 {
        self.world.flags
    }
    fn npc_ships(&self) -> bool {
        self.world.npc_ships
    }
    fn umgah_alive(&mut self) -> bool {
        self.world.tracking_calls += 1;
        self.world.umgah_alive
    }
    fn strobe(&mut self) {
        self.world.strobes += 1;
    }
}
