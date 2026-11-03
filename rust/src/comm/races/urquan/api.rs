use super::Handler;
use crate::comm::races::encounter::EncounterApi;
use crate::engine::World as WorldOwner;
#[cfg(not(test))]
use crate::engine::WorldData as WorldFields;

pub(super) trait UrquanApi: EncounterApi<Handler> {
    fn group_battle(&self, state_key: &str) -> bool;
    fn intro_mode(&mut self, mode: i32, duration: u32);
}

#[cfg(not(test))]
impl UrquanApi for super::super::script::live::LiveScript<Handler> {
    fn group_battle(&self, state_key: &str) -> bool {
        let (start, _) = crate::state::game_state_keys::bit_range(state_key)
            .expect("Ur-Quan group offset state is missing");
        let offset = unsafe {
            WorldOwner::current()
                .read(|world| crate::state::ffi::rust_get_game_state_32(world, start as i32))
        };
        {
            WorldFields::current().read(|owner| owner.game.battle_group_ref) != 0
                && WorldFields::current().read(|owner| owner.game.battle_group_ref) == offset
        }
    }

    fn intro_mode(&mut self, mode: i32, duration: u32) {
        use crate::comm::comm_abi::SetCommIntroMode;
        SetCommIntroMode(mode as u32, duration)
    }
}

#[cfg(test)]
#[derive(Default)]
pub(super) struct World {
    pub group_battles: std::collections::HashSet<String>,
    pub intro_modes: Vec<(i32, u32)>,
}

#[cfg(test)]
impl UrquanApi for crate::comm::races::encounter::testing::Fake<Handler, World> {
    fn group_battle(&self, state_key: &str) -> bool {
        self.world.group_battles.contains(state_key)
    }

    fn intro_mode(&mut self, mode: i32, duration: u32) {
        self.world.intro_modes.push((mode, duration));
    }
}
