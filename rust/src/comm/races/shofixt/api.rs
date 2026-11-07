use super::Handler;
use crate::comm::races::encounter::EncounterApi;
use crate::engine::StoryKey;
pub(super) trait ShofixtApi: EncounterApi<Handler> {
    fn shofixti_name(&self) -> i32 {
        if self.get(StoryKey::SHOFIXTI_KIA) != 0 {
            super::phrases::KATANA
        } else {
            super::phrases::TANAKA
        }
    }
    fn schedule_return(&mut self);
}
#[cfg(not(test))]
impl ShofixtApi for super::super::script::live::LiveScript<'_, '_, Handler> {
    fn schedule_return(&mut self) {
        unsafe {
            crate::time::clock_abi::live::add_event(self.cx.borrow_mut().world, 1, 2, 0, 0, 8);
        }
    }
}
#[cfg(test)]
impl ShofixtApi for crate::comm::races::encounter::testing::Fake<Handler, ()> {
    fn schedule_return(&mut self) {
        self.effects
            .push(crate::comm::races::encounter::testing::Effect::Event(
                1, 2, 0, 0, 8,
            ));
    }
}
