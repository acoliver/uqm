use super::Handler;
use crate::comm::races::encounter::EncounterApi;
pub(super) trait ShofixtApi: EncounterApi<Handler> {
    fn shofixti_name(&self) -> i32 {
        if self.get("SHOFIXTI_KIA") != 0 {
            super::phrases::KATANA
        } else {
            super::phrases::TANAKA
        }
    }
    fn schedule_return(&mut self);
}
#[cfg(not(test))]
impl ShofixtApi for super::super::script::live::LiveScript<Handler> {
    fn schedule_return(&mut self) {
        extern "C" {
            fn AddEvent(
                kind: i32,
                month: u16,
                day: u16,
                year: u16,
                selector: u8,
            ) -> *mut std::ffi::c_void;
        }
        unsafe {
            AddEvent(1, 2, 0, 0, 8);
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
