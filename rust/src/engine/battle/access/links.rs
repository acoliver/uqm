#[cfg(not(test))]
use super::link_queue::*;
#[cfg(not(test))]
impl LinkQueue<'_, Starship> {
    /// # Safety
    /// The fragment's string table and icon drawable stay loaded until this ship is cleared.
    pub(crate) unsafe fn set_picker_resources(
        &mut self,
        id: LinkId<Starship>,
        strings: crate::graphics::native_abi::STRING,
        icons: crate::graphics::native_abi::FRAME,
    ) {
        let ship = self.record_mut(id);
        ship.race_strings = strings;
        ship.icons = icons;
    }
}
