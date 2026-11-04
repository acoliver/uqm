use super::Handler;
use crate::comm::races::alliance::AllianceApi;

pub(super) trait OrzApi: AllianceApi<Handler> {
    fn remove_escorts(&mut self, race: u16);
    fn talking(&mut self, enabled: bool);
}

#[cfg(not(test))]
impl OrzApi for crate::comm::races::script::live::LiveScript<'_, '_, Handler> {
    fn remove_escorts(&mut self, race: u16) {
        use crate::engine::world::fleet;
        let remaining = fleet::remove_escort_ships(self.cx.borrow_mut().world, race, u16::MAX);
        if remaining > 0 {
            // SAFETY: the campaign borrow ends before the game-thread display refresh.
            unsafe {
                self.cx
                    .borrow_mut()
                    .with_legacy(|| crate::battle::build_native::refresh_escorts())
            };
        }
    }
    fn talking(&mut self, enabled: bool) {
        self.cx.borrow_mut().with_legacy(|| {
            use crate::comm::comm_abi::EnableTalkingAnim;
            // SAFETY: this synchronous callback owns the initialized communication display.
            unsafe { EnableTalkingAnim(i32::from(u8::from(enabled))) };
        })
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
