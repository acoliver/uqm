use crate::engine::StoryKey;
pub(crate) const STARBASE_BULLETS: [StoryKey; 4] = [
    StoryKey::STARBASE_BULLETS0,
    StoryKey::STARBASE_BULLETS1,
    StoryKey::STARBASE_BULLETS2,
    StoryKey::STARBASE_BULLETS3,
];
pub(crate) const ZOQFOT_GRPOFFS: [StoryKey; 4] = [
    StoryKey::ZOQFOT_GRPOFFS0,
    StoryKey::ZOQFOT_GRPOFFS1,
    StoryKey::ZOQFOT_GRPOFFS2,
    StoryKey::ZOQFOT_GRPOFFS3,
];

use super::{EncounterApi, Handler};

#[derive(Clone, Copy, Default, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Date {
    pub year: u16,
    pub month: u8,
    pub day: u8,
}

#[derive(Clone, Copy, Default)]
pub(crate) struct Ship {
    pub drives: [u8; 11],
    pub jets: [u8; 8],
    pub modules: [u8; 16],
    pub crew: u16,
    pub fuel: u32,
    pub landers: u8,
    pub units: u32,
    pub mineral_mass: u16,
}

pub(crate) trait StarbaseApi: EncounterApi<Handler> {
    fn date(&self) -> Date;
    fn ship(&self) -> Ship;
    fn escorts_worth(&self) -> u16;
    fn using_speech(&self) -> bool;
    fn random(&mut self) -> u32;
    fn talking_animation(&mut self, enabled: bool);
    fn mineral_amount(&self, category: usize) -> u16;
    fn mineral_worth(&self, category: usize) -> u8;
    fn remove_minerals(&mut self, category: usize, amount: u16);
    fn draw_cargo(&mut self, old: u8, new: u8);
    fn show_capacity(&mut self);
    fn clear_display(&mut self);
    fn now(&mut self) -> u32;
    fn sleep(&mut self, ticks: u32);
    fn sleep_until(&mut self, time: u32);
    fn task_switch(&mut self);
    fn skip_sale(&mut self) -> bool;
}
