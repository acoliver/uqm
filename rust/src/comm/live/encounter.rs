use crate::comm::live::script::LiveScript;
use crate::comm::races::encounter::*;

use crate::engine::world::fleet;

impl<H> EncounterApi<H> for LiveScript<'_, '_, H> {
    fn segue(&mut self, segue: u32) {
        crate::comm::commglue_abi::set_segue(
            &mut self.comm.borrow_mut(),
            &mut self.cx.borrow_mut(),
            segue,
        );
    }
    fn get_segue(&self) -> u32 {
        crate::comm::commglue_abi::get_segue(&self.cx.borrow())
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
        let added =
            fleet::add_escort_ships(self.cx.borrow_mut().world, race, count, fleet::captain_name);
        if added.is_some() {
            // SAFETY: the campaign borrow ends before the game-thread display refresh.
            {
                let cx = &mut *self.cx.borrow_mut();
                crate::ui::sis::delta_sis_gauges(cx.kernel, cx.world, 32767, 32767, 32767)
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
        crate::time::clock_abi::add_event(&mut self.cx.borrow_mut(), 1, 0, 0, 0, 12);
    }
}
