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

    use crate::engine::world::fleet;

    impl<H> EncounterApi<H> for LiveScript<'_, '_, H> {
        fn segue(&mut self, segue: u32) {
            self.set(crate::engine::StoryKey::BATTLE_SEGUE, u8::from(segue == 1));
            crate::comm::ffi::rust_set_segue(segue)
        }
        fn get_segue(&self) -> u32 {
            self.cx
                .borrow_mut()
                .with_legacy(crate::comm::ffi::rust_get_segue)
        }
        fn activity(&self) -> u16 {
            self.cx.borrow().world.activity()
        }
        fn alliance(&self, race: u16) -> u16 {
            fleet::check_alliance(self.cx.borrow().world, race)
        }
        fn escort_feasibility(&self, race: u16) -> u16 {
            fleet::escort_feasibility(self.cx.borrow().world, race)
        }
        fn add_escorts(&mut self, race: u16, count: i16) {
            let added = fleet::add_escort_ships(
                self.cx.borrow_mut().world,
                race,
                count,
                fleet::captain_name,
            );
            if added.is_some() {
                // SAFETY: the campaign borrow ends before the game-thread display refresh.
                unsafe {
                    self.cx
                        .borrow_mut()
                        .with_legacy(|| crate::ui::sis::DeltaSISGauges(32767, 32767, 32767))
                };
            }
        }
        fn fwiffo_crew(&mut self) {
            use crate::ships::queue::{NAME_OFFSET, NUM_CAPTAINS_NAMES};
            fleet::set_escort_crew(
                self.cx.borrow_mut().world,
                6,
                1,
                NAME_OFFSET + NUM_CAPTAINS_NAMES,
            );
        }
        fn advance_ilwrath(&mut self) {
            unsafe {
                crate::time::clock_abi::live::add_event(self.cx.borrow_mut().world, 1, 0, 0, 0, 12);
            }
        }
    }
}

#[cfg(test)]
pub(super) mod testing;
