use super::Handler;
use crate::comm::races::alliance::AllianceApi;

pub(crate) trait PkunkApi: AllianceApi<Handler> {
    fn date(&self) -> (u16, u16, u16);
    fn gift_ships(&mut self) -> u16;
}

#[cfg(test)]
#[derive(Default)]
pub(crate) struct World {
    pub date: (u16, u16, u16),
    pub ships_added: u16,
}

#[cfg(test)]
impl PkunkApi for super::super::encounter::testing::Fake<Handler, World> {
    fn date(&self) -> (u16, u16, u16) {
        self.world.date
    }
    fn gift_ships(&mut self) -> u16 {
        self.effects
            .push(super::super::encounter::testing::Effect::AddEscorts(
                crate::comm::dispatch::ship::PKUNK_SHIP as u16,
                4,
            ));
        self.world.ships_added
    }
}
