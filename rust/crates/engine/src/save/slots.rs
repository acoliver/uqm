//! The save-slot service the game options menu uses: list the slot
//! summaries, write the running game to a slot and restore one. The
//! application supplies it, because slots live in the save directory it
//! mounts.

use super::summary_record::SummaryDesc;
use crate::battle::Cx;
use crate::world::generation::SolarSnapshot;
use uqm_save::MAX_SAVED_GAMES;

#[derive(Clone, Copy)]
pub struct SaveSlots {
    /// Fill every slot's summary; unreadable slots get `year_index` 0.
    pub describe: fn(&mut [SummaryDesc; MAX_SAVED_GAMES]),
    /// Write the running game to `slot` and update its summary.
    pub save:
        fn(&mut Cx<'_>, Option<SolarSnapshot>, u16, &mut SummaryDesc, &std::ffi::CStr) -> bool,
    /// Restore the game in `slot` onto the supplied owners.
    pub load: fn(&mut Cx<'_>, u16) -> bool,
}

impl crate::Kernel {
    pub fn install_save_slots(&mut self, slots: SaveSlots) {
        self.services.save_slots = Some(slots);
    }

    pub fn save_slots(&self) -> SaveSlots {
        self.services
            .save_slots
            .unwrap_or_else(|| panic!("save slots were not installed"))
    }
}
