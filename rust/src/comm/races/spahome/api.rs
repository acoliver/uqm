use super::Handler;
use crate::comm::races::alliance::AllianceApi;

pub(super) trait SpahomeApi: AllianceApi<Handler> {
    fn have_spathi_escort(&self) -> bool;
}
#[cfg(not(test))]
impl SpahomeApi for crate::comm::races::script::live::LiveScript<Handler> {
    fn have_spathi_escort(&self) -> bool {
        use crate::battle::build_native::count_escort_ships;
        // SAFETY: response callbacks run with the game-thread context published.
        unsafe {
            crate::app::bridge::enter(|cx| {
                count_escort_ships(cx.world, crate::comm::dispatch::ship::SPATHI_SHIP as u16) != 0
            })
        }
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
