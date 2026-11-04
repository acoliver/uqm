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

    use crate::battle::build_native;

    use crate::time::clock_abi::live::add_event;

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
            // SAFETY: response callbacks run with the game-thread context published.
            unsafe { crate::app::bridge::enter(|cx| build_native::check_alliance(cx.world, race)) }
        }
        fn escort_feasibility(&self, race: u16) -> u16 {
            // SAFETY: response callbacks run with the game-thread context published.
            unsafe {
                crate::app::bridge::enter(|cx| build_native::escort_feasibility(cx.world, race))
            }
        }
        fn add_escorts(&mut self, race: u16, count: i16) {
            unsafe {
                let added = crate::app::bridge::enter(|cx| {
                    build_native::add_escort_ships(
                        cx.world,
                        race,
                        count,
                        build_native::captain_name,
                    )
                });
                if added.is_some() {
                    build_native::refresh_escorts();
                }
            }
        }
        fn fwiffo_crew(&mut self) {
            use crate::ships::queue::{NAME_OFFSET, NUM_CAPTAINS_NAMES};
            // SAFETY: response callbacks run with the game-thread context published.
            unsafe {
                crate::app::bridge::enter(|cx| {
                    build_native::set_escort_crew(cx.world, 6, 1, NAME_OFFSET + NUM_CAPTAINS_NAMES)
                });
            }
        }
        fn advance_ilwrath(&mut self) {
            unsafe {
                add_event(1, 0, 0, 0, 12);
            }
        }
    }
}

#[cfg(test)]
pub(super) mod testing;
