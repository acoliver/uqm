use super::Handler;
use crate::comm::races::alliance::AllianceApi;

pub(super) trait TalkpetApi: AllianceApi<Handler> {
    fn flags(&self) -> u8;
    fn npc_ships(&self) -> bool;
    fn umgah_alive(&mut self) -> bool;
    fn strobe(&mut self);
}
#[cfg(not(test))]
impl TalkpetApi for super::super::script::live::LiveScript<Handler> {
    fn flags(&self) -> u8 {
        extern "C" {
            static mut GlobData: crate::comm::locdata::CGlobData;
        }
        unsafe { GlobData.game_state.glob_flags }
    }
    fn npc_ships(&self) -> bool {
        extern "C" {
            static mut GlobData: crate::comm::locdata::CGlobData;
        }
        unsafe { !GlobData.game_state.npc_built_ship_q.head.is_null() }
    }
    fn umgah_alive(&mut self) -> bool {
        use crate::battle::build_native::engine::StartSphereTracking;
        unsafe { StartSphereTracking(crate::comm::dispatch::ship::UMGAH_SHIP as u16) != 0 }
    }
    fn strobe(&mut self) {
        unsafe {
            crate::comm::locdata::COMM_DATA.alien_ambient_array[16].anim_flags &=
                !super::super::script::ANIM_DISABLED;
        }
    }
}
#[cfg(test)]
#[derive(Default)]
pub(super) struct World {
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
