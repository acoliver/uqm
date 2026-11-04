use super::Handler;
use crate::comm::races::alliance::AllianceApi;

pub(super) trait SpahomeApi: AllianceApi<Handler> {
    fn have_spathi_escort(&self) -> bool;
}
#[cfg(not(test))]
impl SpahomeApi for crate::comm::races::script::live::LiveScript<'_, '_, Handler> {
    fn have_spathi_escort(&self) -> bool {
        crate::engine::world::fleet::count_escort_ships(
            self.cx.borrow().world,
            u16::from(crate::comm::dispatch::ship::SPATHI_SHIP),
        ) != 0
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
