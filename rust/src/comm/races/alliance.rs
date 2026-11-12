//! Alliance and mission effects used by the Supox, Utwig and Zoq-Fot-Pik scripts.
pub(crate) use super::encounter::EncounterApi;
pub(crate) use crate::engine::species::ship;
pub(crate) const SUPOX_SHIP: u16 = ship::SUPOX_SHIP as u16;
pub(crate) const UTWIG_SHIP: u16 = ship::UTWIG_SHIP as u16;
pub(crate) const ZOQFOTPIK_SHIP: u16 = ship::ZOQFOTPIK_SHIP as u16;

pub(crate) trait AllianceApi<H>: EncounterApi<H> {
    fn set_allied(&mut self, race: u16, allied: bool);
    fn track(&mut self, race: u16);
    fn event(&mut self, kind: i32, month: u16, day: u16, year: u16, selector: u8);
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
