use super::Handler;
use crate::comm::races::alliance::AllianceApi;

pub(crate) trait OrzApi: AllianceApi<Handler> {
    fn remove_escorts(&mut self, race: u16);
    fn talking(&mut self, enabled: bool);
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
