//! Capture and restore of the live campaign sections used by SaveGame and
//! LoadGame: global/story state, queues, state files, summary and location.
//!
//! Location preparation runs before capture. The menu ABI coordinates these
//! owners on the game thread and installs the saved activity after loading.

pub use uqm_engine::save::events;
pub use uqm_engine::save::global;
pub use uqm_engine::save::groups;
pub use uqm_engine::save::hyper_save;
pub use uqm_engine::save::ip_group_save;
pub mod live;
pub mod location;
pub use uqm_engine::save::queues;
pub mod solar;
pub mod star;
pub use uqm_engine::save::statefiles;
pub use uqm_engine::save::summary;

use super::types::Summary;
pub use uqm_engine::save::{
    capture_runtime, restore_runtime, EventError, Restored, RuntimeError, RuntimeSections,
};
#[cfg(test)]
use uqm_modes::comm::locdata::CGameState;

/// Preserve the live writer's historical `write_a16` element-amount layout.
/// The general codec must retain distinct amounts when re-encoding a save.
pub fn prepare_summary_for_write(summary: &mut Summary) {
    let first_amount = summary.sis.element_amounts[0];
    summary.sis.element_amounts.fill(first_amount);
}

#[cfg(test)]
mod group_tests;
#[cfg(test)]
mod hyper_save_tests;
#[cfg(test)]
mod ip_group_save_tests;
#[cfg(test)]
mod location_tests;
#[cfg(test)]
mod queue_tests;
#[cfg(test)]
mod solar_tests;
#[cfg(test)]
mod statefile_tests;
#[cfg(test)]
mod summary_tests;
#[cfg(test)]
mod tests;

#[cfg(test)]
mod campaign_tests;
