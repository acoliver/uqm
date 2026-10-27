//! Battle progress observation, shared with the migration trace.
pub use crate::collections::observation::{assert_progress, current_frame};
#[no_mangle]
pub extern "C" fn rust_battle_frame_reset() {
    crate::collections::observation::reset();
}
#[no_mangle]
pub extern "C" fn rust_battle_frame_advance() {
    crate::collections::observation::advance();
    super::Coordinator::parity_checkpoint("battle");
}
