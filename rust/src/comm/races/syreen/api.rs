use super::Handler;
use crate::comm::races::alliance::AllianceApi;

pub(super) trait SyreenApi: AllianceApi<Handler> {
    fn using_speech(&self) -> bool;
}
#[cfg(not(test))]
impl SyreenApi for super::super::script::live::LiveScript<'_, '_, Handler> {
    fn using_speech(&self) -> bool {
        self.cx.borrow().kernel.using_speech != 0
    }
}
#[cfg(test)]
#[derive(Default)]
pub(super) struct World {
    pub speech: bool,
}
#[cfg(test)]
impl SyreenApi for super::super::encounter::testing::Fake<Handler, World> {
    fn using_speech(&self) -> bool {
        self.world.speech
    }
}
