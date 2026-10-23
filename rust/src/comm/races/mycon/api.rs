use super::Handler;
use crate::comm::races::alliance::AllianceApi;

pub(super) trait MyconApi: AllianceApi<Handler> {
    fn random(&mut self) -> u32;
}

#[cfg(not(test))]
impl MyconApi for super::super::script::live::LiveScript<Handler> {
    fn random(&mut self) -> u32 {
        crate::math::random()
    }
}

#[cfg(test)]
#[derive(Default)]
pub(super) struct World {
    pub random: std::collections::VecDeque<u32>,
    pub random_calls: usize,
}
#[cfg(test)]
impl MyconApi for super::super::encounter::testing::Fake<Handler, World> {
    fn random(&mut self) -> u32 {
        self.world.random_calls += 1;
        self.world
            .random
            .pop_front()
            .expect("unexpected random call")
    }
}
