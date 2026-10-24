//! Alliance and mission effects used by the Supox, Utwig and Zoq-Fot-Pik scripts.
use super::encounter::EncounterApi;
use crate::comm::dispatch::ship;
pub(super) const SUPOX_SHIP: u16 = ship::SUPOX_SHIP as u16;
pub(super) const UTWIG_SHIP: u16 = ship::UTWIG_SHIP as u16;
pub(super) const ZOQFOTPIK_SHIP: u16 = ship::ZOQFOTPIK_SHIP as u16;

pub(super) trait AllianceApi<H>: EncounterApi<H> {
    fn set_allied(&mut self, race: u16, allied: bool);
    fn track(&mut self, race: u16);
    fn event(&mut self, kind: i32, month: u16, day: u16, year: u16, selector: u8);
}

#[cfg(not(test))]
mod live {
    use super::*;
    use crate::comm::races::script::live::LiveScript;

    use crate::battle::build_native::engine::SetRaceAllied;

    use crate::battle::build_native::engine::StartSphereTracking;

    use crate::time::clock_abi::live::AddEvent;
    impl<H> AllianceApi<H> for LiveScript<H> {
        fn set_allied(&mut self, race: u16, allied: bool) {
            unsafe {
                SetRaceAllied(race, u32::from(allied));
            }
        }
        fn track(&mut self, race: u16) {
            unsafe {
                StartSphereTracking(race);
            }
        }
        fn event(&mut self, kind: i32, month: u16, day: u16, year: u16, selector: u8) {
            unsafe {
                AddEvent((kind) as u32, month, day, year, selector);
            }
        }
    }
}

#[cfg(test)]
impl<H: Copy, E> AllianceApi<H> for super::encounter::testing::Fake<H, E> {
    fn set_allied(&mut self, race: u16, allied: bool) {
        self.alliances[usize::from(race)] = if allied { 1 } else { 2 };
        self.effects
            .push(super::encounter::testing::Effect::Allied(race, allied));
    }
    fn track(&mut self, race: u16) {
        self.effects
            .push(super::encounter::testing::Effect::Track(race));
    }
    fn event(&mut self, kind: i32, month: u16, day: u16, year: u16, selector: u8) {
        self.effects.push(super::encounter::testing::Effect::Event(
            kind, month, day, year, selector,
        ));
    }
}
