use super::Handler;
use crate::comm::races::alliance::AllianceApi;

pub(crate) trait SpahomeApi: AllianceApi<Handler> {
    fn have_spathi_escort(&self) -> bool;
}

#[cfg(test)]
#[derive(Default)]
pub(crate) struct World {
    pub have_escort: bool,
}
#[cfg(test)]
impl SpahomeApi for crate::comm::races::encounter::testing::Fake<Handler, World> {
    fn have_spathi_escort(&self) -> bool {
        self.world.have_escort
    }
}
