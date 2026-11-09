use super::Handler;
#[cfg(test)]
use crate::comm::races::encounter::testing::Fake;
use crate::comm::races::encounter::EncounterApi;

pub(crate) trait YehatApi: EncounterApi<Handler> {
    fn have_shofixti_escort(&self) -> bool;
    fn random_bit(&mut self) -> u8;
    fn schedule_rebel_event(&mut self);
}

#[cfg(test)]
#[derive(Default)]
pub(crate) struct World {
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
