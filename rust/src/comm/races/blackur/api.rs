use super::Handler;
use crate::comm::races::encounter::EncounterApi;

pub(super) trait BlackurApi: EncounterApi<Handler> {
    fn battle_group(&self) -> (u32, u32);

    fn in_samatra_battle(&self) -> bool {
        let (battle_group, samatra_group) = self.battle_group();
        battle_group != 0 && battle_group == samatra_group
    }
}
#[cfg(not(test))]
impl BlackurApi for super::super::script::live::LiveScript<Handler> {
    fn battle_group(&self) -> (u32, u32) {
        let (start, _) = crate::state::game_state_keys::bit_range("SAMATRA_GRPOFFS0")
            .expect("missing Samatra group-offset state");
        let offset = unsafe {
            crate::engine::World::current()
                .read(|world| crate::state::ffi::rust_get_game_state_32(world, start as i32))
        };
        (
            unsafe { WorldFields::current().read(|owner| owner.game.battle_group_ref) },
            offset,
        )
    }
}
#[cfg(test)]
impl BlackurApi for crate::comm::races::encounter::testing::Fake<Handler, (u32, u32)> {
    fn battle_group(&self) -> (u32, u32) {
        self.world
    }
}

use crate::engine::WorldData as WorldFields;
