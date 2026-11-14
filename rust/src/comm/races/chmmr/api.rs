use super::Handler;
use crate::comm::races::alliance::AllianceApi;

pub(crate) trait ChmmrApi: AllianceApi<Handler> {
    fn refit(&mut self);
    fn intro_mode(&mut self, mode: i32, duration: i32);
}

#[cfg(test)]
#[derive(Default)]
pub(crate) struct World {
    pub refits: usize,
    pub intros: Vec<(i32, i32)>,
}

#[cfg(test)]
impl ChmmrApi for super::super::encounter::testing::Fake<Handler, World> {
    fn refit(&mut self) {
        self.world.refits += 1;
    }
    fn intro_mode(&mut self, mode: i32, duration: i32) {
        self.world.intros.push((mode, duration));
    }
}
