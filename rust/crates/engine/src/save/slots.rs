//! Application-supplied save operations bound to one retained save directory.
use super::summary_record::SummaryDesc;
use crate::{battle::Cx, io::Dir, world::generation::SolarSnapshot};
use std::ffi::CStr;
use uqm_save::MAX_SAVED_GAMES;

type Describe = fn(&Dir, &mut [SummaryDesc; MAX_SAVED_GAMES]);
type Save = fn(&Dir, &mut Cx<'_>, Option<SolarSnapshot>, u16, &mut SummaryDesc, &CStr) -> bool;
type Load = fn(&Dir, &mut Cx<'_>, u16) -> bool;

#[derive(Clone)]
pub struct SaveSlots {
    directory: Dir,
    describe: Describe,
    save: Save,
    load: Load,
}

impl SaveSlots {
    pub fn new(directory: Dir, describe: Describe, save: Save, load: Load) -> Self {
        Self {
            directory,
            describe,
            save,
            load,
        }
    }

    pub fn describe(&self, summaries: &mut [SummaryDesc; MAX_SAVED_GAMES]) {
        (self.describe)(&self.directory, summaries);
    }

    pub fn save(
        &self,
        cx: &mut Cx<'_>,
        solar: Option<SolarSnapshot>,
        slot: u16,
        summary: &mut SummaryDesc,
        name: &CStr,
    ) -> bool {
        (self.save)(&self.directory, cx, solar, slot, summary, name)
    }

    pub fn load(&self, cx: &mut Cx<'_>, slot: u16) -> bool {
        (self.load)(&self.directory, cx, slot)
    }
}

impl crate::Kernel {
    pub fn install_save_slots(&mut self, slots: SaveSlots) {
        self.services.save_slots = Some(slots);
    }

    pub fn save_slots(&self) -> SaveSlots {
        self.services
            .save_slots
            .clone()
            .unwrap_or_else(|| panic!("save slots were not installed"))
    }
}
