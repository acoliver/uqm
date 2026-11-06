#[cfg(not(test))]
use super::link_queue::*;
/// # Safety
/// Only the game thread may take a view, and it must end it before a legacy call.
#[cfg(not(test))]
pub(crate) unsafe fn ships<'a>(side: usize) -> LinkQueue<'a, Starship> {
    assert!(side < 2, "invalid ship side {side}");
    // SAFETY: callers serialize the address-stable battle ship authority.
    unsafe { LinkQueue::new(&mut *crate::engine::battle::record::ship_queue(side).as_ptr()) }
}

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
