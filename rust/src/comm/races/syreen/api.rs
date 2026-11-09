use super::Handler;
use crate::comm::races::alliance::AllianceApi;

pub(crate) trait SyreenApi: AllianceApi<Handler> {
    fn using_speech(&self) -> bool;
}

#[cfg(test)]
#[derive(Default)]
pub(crate) struct World {
    pub speech: bool,
}
#[cfg(test)]
impl SyreenApi for super::super::encounter::testing::Fake<Handler, World> {
    fn using_speech(&self) -> bool {
        self.world.speech
    }
}
