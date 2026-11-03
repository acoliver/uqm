//! Battle progress observation, shared with the migration trace.
pub use crate::collections::observation::{assert_progress, current_frame};
#[no_mangle]
pub extern "C" fn rust_battle_frame_reset() {
    crate::app::observe::checkpoint(crate::engine::observe::CheckpointView::BattleReset);
}
#[no_mangle]
pub extern "C" fn rust_battle_frame_advance() {
    crate::app::observe::checkpoint(crate::engine::observe::CheckpointView::BattleAdvanced);
}
