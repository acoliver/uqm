use crate::comm::live::script::LiveScript;
use crate::comm::races::alliance::*;

use crate::engine::world::fleet::set_race_allied;

use crate::engine::world::fleet::start_sphere_tracking;

impl<H> AllianceApi<H> for LiveScript<'_, '_, H> {
    fn set_allied(&mut self, race: u16, allied: bool) {
        set_race_allied(self.cx.borrow_mut().world, race, u32::from(allied));
    }
    fn track(&mut self, race: u16) {
        start_sphere_tracking(self.cx.borrow_mut().world, race);
    }
    fn event(&mut self, kind: i32, month: u16, day: u16, year: u16, selector: u8) {
        // SAFETY: the borrowed World owns the event queue and remains live throughout insertion.
        unsafe {
            crate::time::clock_abi::live::add_event(
                self.cx.borrow_mut().world,
                kind as u32,
                month,
                day,
                year,
                selector,
            );
        }
    }
}
