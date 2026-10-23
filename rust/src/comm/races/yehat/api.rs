use super::Handler;
#[cfg(test)]
use crate::comm::races::encounter::testing::Fake;
use crate::comm::races::encounter::EncounterApi;
#[cfg(not(test))]
use crate::comm::races::script::live::LiveScript;

pub(super) trait YehatApi: EncounterApi<Handler> {
    fn have_shofixti_escort(&self) -> bool;
    fn random_bit(&mut self) -> u8;
    fn schedule_rebel_event(&mut self);
}

#[cfg(not(test))]
impl YehatApi for LiveScript<Handler> {
    fn have_shofixti_escort(&self) -> bool {
        extern "C" {
            fn HaveEscortShip(race: u16) -> u32;
        }
        unsafe { HaveEscortShip(u16::from(crate::comm::dispatch::ship::SHOFIXTI_SHIP)) != 0 }
    }

    fn random_bit(&mut self) -> u8 {
        (crate::math::random() & 1) as u8
    }

    fn schedule_rebel_event(&mut self) {
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
            AddEvent(1, 0, 0, 0, 15);
        }
    }
}

#[cfg(test)]
#[derive(Default)]
pub(super) struct World {
    pub have_shofixti: bool,
    pub random_bit: u8,
    pub random_calls: usize,
}

#[cfg(test)]
impl YehatApi for Fake<Handler, World> {
    fn have_shofixti_escort(&self) -> bool {
        self.world.have_shofixti
    }

    fn random_bit(&mut self) -> u8 {
        self.world.random_calls += 1;
        self.world.random_bit
    }

    fn schedule_rebel_event(&mut self) {
        self.effects
            .push(crate::comm::races::encounter::testing::Effect::Event(
                1, 0, 0, 0, 15,
            ));
    }
}
