use super::Handler;
use crate::comm::races::encounter::EncounterApi;
pub(super) trait UmgahApi: EncounterApi<Handler> {
    fn add_biomass(&mut self, amount: u16);
}
#[cfg(not(test))]
impl UmgahApi for super::super::script::live::LiveScript<'_, '_, Handler> {
    fn add_biomass(&mut self, amount: u16) {
        let mut cx = self.cx.borrow_mut();
        let sis = &mut cx.world.data.flagship;
        sis.total_bio_mass = sis.total_bio_mass.wrapping_add(amount);
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
