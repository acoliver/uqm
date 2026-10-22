use super::Handler;
use crate::comm::races::alliance::AllianceApi;

pub(super) trait SpahomeApi: AllianceApi<Handler> {
    fn have_spathi_escort(&self) -> bool;
}
#[cfg(not(test))]
impl SpahomeApi for crate::comm::races::script::live::LiveScript<Handler> {
    fn have_spathi_escort(&self) -> bool {
        extern "C" {
            fn HaveEscortShip(race: u16) -> u32;
        }
        unsafe { HaveEscortShip(crate::comm::dispatch::ship::SPATHI_SHIP as u16) != 0 }
    }
}
#[cfg(test)]
#[derive(Default)]
pub(super) struct World {
    pub have_escort: bool,
}
#[cfg(test)]
impl SpahomeApi for crate::comm::races::encounter::testing::Fake<Handler, World> {
    fn have_spathi_escort(&self) -> bool {
        self.world.have_escort
    }
}
