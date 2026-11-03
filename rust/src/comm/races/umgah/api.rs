use super::Handler;
use crate::comm::races::encounter::EncounterApi;
#[cfg(not(test))]
use crate::engine::WorldData;
pub(super) trait UmgahApi: EncounterApi<Handler> {
    fn add_biomass(&mut self, amount: u16);
}
#[cfg(not(test))]
impl UmgahApi for super::super::script::live::LiveScript<Handler> {
    fn add_biomass(&mut self, amount: u16) {
        unsafe {
            (**WorldData::current()).flagship.total_bio_mass = WorldData::current()
                .read(|owner| owner.flagship.total_bio_mass)
                .wrapping_add(amount);
        }
    }
}
#[cfg(test)]
#[derive(Default)]
pub(super) struct World {
    pub biomass: u16,
}
#[cfg(test)]
impl UmgahApi for super::super::encounter::testing::Fake<Handler, World> {
    fn add_biomass(&mut self, amount: u16) {
        self.world.biomass = self.world.biomass.wrapping_add(amount);
    }
}
