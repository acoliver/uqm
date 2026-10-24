use super::Handler;
use crate::comm::races::alliance::AllianceApi;

pub(super) trait OrzApi: AllianceApi<Handler> {
    fn remove_escorts(&mut self, race: u16);
    fn talking(&mut self, enabled: bool);
}

#[cfg(not(test))]
impl OrzApi for crate::comm::races::script::live::LiveScript<Handler> {
    fn remove_escorts(&mut self, race: u16) {
        use crate::battle::build_native::engine::RemoveEscortShips;
        unsafe {
            RemoveEscortShips(race);
        }
    }
    fn talking(&mut self, enabled: bool) {
        use crate::comm::comm_abi::EnableTalkingAnim;
        unsafe {
            EnableTalkingAnim((u8::from(enabled)) as i32);
        }
    }
}

#[cfg(test)]
impl OrzApi for super::super::encounter::testing::Fake<Handler> {
    fn remove_escorts(&mut self, race: u16) {
        self.effects
            .push(super::super::encounter::testing::Effect::RemoveEscorts(
                race,
            ));
    }
    fn talking(&mut self, enabled: bool) {
        self.effects
            .push(super::super::encounter::testing::Effect::Talking(enabled));
    }
}
