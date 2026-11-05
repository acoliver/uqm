//! Escort display updates for campaign screens.

/// # Safety
/// The game thread owns the initialized SIS display, with no owner borrow retained.
#[cfg(not(test))]
pub unsafe fn refresh_escorts() {
    // SAFETY: the legacy caller has released its campaign mutation before drawing.
    unsafe { crate::ui::sis::DeltaSISGauges(32767, 32767, 32767) }
}
