use super::Handler;
use crate::comm::races::alliance::AllianceApi;
#[cfg(not(test))]
use crate::engine::World;
#[cfg(not(test))]
use crate::engine::WorldData;

pub(super) trait TalkpetApi: AllianceApi<Handler> {
    fn flags(&self) -> u8;
    fn npc_ships(&self) -> bool;
    fn umgah_alive(&mut self) -> bool;
    fn strobe(&mut self);
}
#[cfg(not(test))]
impl TalkpetApi for super::super::script::live::LiveScript<'_, '_, Handler> {
    fn flags(&self) -> u8 {
        self.cx.borrow_mut().with_legacy(|| {
            {
                WorldData::current().read(|owner| owner.game.glob_flags)
            }
        })
    }
    fn npc_ships(&self) -> bool {
        self.cx.borrow_mut().with_legacy(|| {
            {
                World::current().read(|world| world.npc_built_ships_ref().head().is_some())
            }
        })
    }
    fn umgah_alive(&mut self) -> bool {
        self.cx.borrow_mut().with_legacy(|| {
            use crate::engine::world::fleet::start_sphere_tracking;
            World::current().write(|world| {
                start_sphere_tracking(world, crate::comm::dispatch::ship::UMGAH_SHIP as u16)
            }) != 0
        })
    }
    fn strobe(&mut self) {
        self.cx.borrow_mut().with_legacy(|| unsafe {
            crate::comm::locdata::COMM_DATA.alien_ambient_array[16].anim_flags &=
                !super::super::script::ANIM_DISABLED;
        })
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
