//! Encounter effects shared by the Starbase, Spathi and Ilwrath scripts.

use super::script::ScriptApi;

pub(super) trait EncounterApi<H>: ScriptApi<H> {
    fn segue(&mut self, segue: u32);
    fn get_segue(&self) -> u32;
    fn activity(&self) -> u16;
    fn alliance(&self, race: u16) -> u16;
    fn escort_feasibility(&self, race: u16) -> u16;
    fn add_escorts(&mut self, race: u16, count: i16);
    fn fwiffo_crew(&mut self);
    fn advance_ilwrath(&mut self);
}

#[cfg(not(test))]
mod live {
    use super::*;
    use crate::comm::races::script::live::LiveScript;
    use std::ffi::c_void;

    extern "C" {
        fn CheckAlliance(race: u16) -> u16;
        fn EscortFeasibilityStudy(race: u16) -> u16;
        fn AddEscortShips(race: u16, count: i16) -> u16;
        fn SetEscortCrewComplement(race: u16, crew: u16, captain: u8) -> i32;
        fn AddEvent(kind: i32, month: u16, day: u16, year: u16, selector: u8) -> *mut c_void;
    }

    impl<H> EncounterApi<H> for LiveScript<H> {
        fn segue(&mut self, segue: u32) {
            self.set("BATTLE_SEGUE", u8::from(segue == 1));
            crate::comm::ffi::rust_set_segue(segue)
        }
        fn get_segue(&self) -> u32 {
            crate::comm::ffi::rust_get_segue()
        }
        fn activity(&self) -> u16 {
            crate::mainloop::ffi::get_current_activity().0
        }
        fn alliance(&self, race: u16) -> u16 {
            unsafe { CheckAlliance(race) }
        }
        fn escort_feasibility(&self, race: u16) -> u16 {
            unsafe { EscortFeasibilityStudy(race) }
        }
        fn add_escorts(&mut self, race: u16, count: i16) {
            unsafe {
                AddEscortShips(race, count);
            }
        }
        fn fwiffo_crew(&mut self) {
            use crate::ships::queue::{NAME_OFFSET, NUM_CAPTAINS_NAMES};
            unsafe {
                SetEscortCrewComplement(6, 1, NAME_OFFSET + NUM_CAPTAINS_NAMES);
            }
        }
        fn advance_ilwrath(&mut self) {
            unsafe {
                AddEvent(1, 0, 0, 0, 12);
            }
        }
    }
}

#[cfg(test)]
pub(super) mod testing;
