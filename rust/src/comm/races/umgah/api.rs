use super::Handler;
use crate::comm::races::encounter::EncounterApi;
pub(super) trait UmgahApi: EncounterApi<Handler> {
    fn add_biomass(&mut self, amount: u16);
}
#[cfg(not(test))]
impl UmgahApi for super::super::script::live::LiveScript<Handler> {
    fn add_biomass(&mut self, amount: u16) {
        extern "C" {
            static mut GlobData: crate::ships::native::abi::GLOBDATA;
        }
        unsafe {
            GlobData.SIS_state.TotalBioMass = GlobData.SIS_state.TotalBioMass.wrapping_add(amount);
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
