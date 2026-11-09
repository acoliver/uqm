use super::Handler;
use crate::comm::races::encounter::EncounterApi;
pub(crate) trait UmgahApi: EncounterApi<Handler> {
    fn add_biomass(&mut self, amount: u16);
}

#[cfg(test)]
#[derive(Default)]
pub(crate) struct World {
    pub biomass: u16,
}
#[cfg(test)]
impl UmgahApi for super::super::encounter::testing::Fake<Handler, World> {
    fn add_biomass(&mut self, amount: u16) {
        self.world.biomass = self.world.biomass.wrapping_add(amount);
    }
}
