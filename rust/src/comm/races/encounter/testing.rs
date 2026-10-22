use super::*;
use crate::comm::races::script::Supplies;
use std::collections::{HashMap, HashSet};

#[derive(Debug, PartialEq, Eq)]
pub(in crate::comm::races) enum Effect<H> {
    Npc(i32),
    Offer(u16, H, Vec<i32>),
    Set(String, u8),
    Disable(u16),
    Segue(u32),
    Allied(u16, bool),
    Track(u16),
    Event(i32, u16, u16, u16, u8),
    Speaker(u16, bool),
    AddEscorts(u16, i16),
    FwiffoCrew,
    AdvanceIlwrath,
    Talk(u16),
    Gauges(i16, i16, i32),
    Talking(bool),
    Remove(usize, u16),
    DrawCargo(u8, u8),
    Capacity,
    Clear,
    Now,
    Sleep(u32),
    Until(u32),
    Switch,
    Skip,
}

pub(in crate::comm::races) struct Fake<H, E = ()> {
    pub state: HashMap<String, u8>,
    pub disabled: HashSet<u16>,
    pub effects: Vec<Effect<H>>,
    pub segue: u32,
    pub activity: u16,
    pub alliances: [u16; 24],
    pub feasibility: u16,
    pub world: E,
}
impl<H, E: Default> Default for Fake<H, E> {
    fn default() -> Self {
        Self {
            state: HashMap::new(),
            disabled: HashSet::new(),
            effects: Vec::new(),
            segue: 0,
            activity: 0,
            alliances: [2; 24],
            feasibility: 1,
            world: E::default(),
        }
    }
}
impl<H: Copy, E> Fake<H, E> {
    pub fn spoken(&self) -> Vec<i32> {
        self.effects
            .iter()
            .filter_map(|e| {
                if let Effect::Npc(p) = e {
                    Some(*p)
                } else {
                    None
                }
            })
            .collect()
    }
    pub fn offers(&self) -> Vec<(u16, H)> {
        self.effects
            .iter()
            .filter_map(|e| {
                if let Effect::Offer(p, h, _) = e {
                    Some((*p, *h))
                } else {
                    None
                }
            })
            .collect()
    }
    pub fn put(&mut self, key: &str, value: u8) {
        self.state.insert(key.into(), value);
    }
}
impl<H: Copy, E> ScriptApi<H> for Fake<H, E> {
    fn get(&self, key: &str) -> u8 {
        assert!(
            crate::state::game_state_keys::bit_range(key).is_some(),
            "unknown state {key}"
        );
        *self.state.get(key).unwrap_or(&0)
    }
    fn set(&mut self, key: &str, value: u8) {
        assert!(
            crate::state::game_state_keys::bit_range(key).is_some(),
            "unknown state {key}"
        );
        self.put(key, value);
        self.effects.push(Effect::Set(key.into(), value));
    }
    fn npc(&mut self, phrase: i32) {
        self.effects.push(Effect::Npc(phrase));
    }
    fn response(&mut self, phrase: u16, handler: H, fragments: &[i32]) {
        self.effects
            .push(Effect::Offer(phrase, handler, fragments.to_vec()));
    }
    fn enabled(&self, phrase: u16) -> bool {
        !self.disabled.contains(&phrase)
    }
    fn disable(&mut self, phrase: u16) {
        self.disabled.insert(phrase);
        self.effects.push(Effect::Disable(phrase));
    }
    fn supplies(&self) -> Supplies {
        panic!("unexpected commander supply access")
    }
    fn add_lander(&mut self) {
        panic!("unexpected commander lander effect")
    }
    fn gauges(&mut self, crew: i16, fuel: i16, units: i32) {
        self.effects.push(Effect::Gauges(crew, fuel, units));
    }
    fn palette(&mut self, _: u16) {
        panic!("unexpected palette selection")
    }
    fn talk_segue(&mut self, track: u16) {
        self.effects.push(Effect::Talk(track));
    }
    fn disable_animation(&mut self, _: usize) {
        panic!("unexpected ambient animation effect")
    }
    fn fade_palette(&mut self, _: u16, _: i16) {
        panic!("unexpected palette fade")
    }
}
impl<H: Copy, E> EncounterApi<H> for Fake<H, E> {
    fn segue(&mut self, segue: u32) {
        self.segue = segue;
        self.effects.push(Effect::Segue(segue));
    }
    fn get_segue(&self) -> u32 {
        self.segue
    }
    fn activity(&self) -> u16 {
        self.activity
    }
    fn alliance(&self, race: u16) -> u16 {
        self.alliances[usize::from(race)]
    }
    fn escort_feasibility(&self, _: u16) -> u16 {
        self.feasibility
    }
    fn add_escorts(&mut self, race: u16, count: i16) {
        self.effects.push(Effect::AddEscorts(race, count));
    }
    fn fwiffo_crew(&mut self) {
        self.effects.push(Effect::FwiffoCrew);
    }
    fn advance_ilwrath(&mut self) {
        self.effects.push(Effect::AdvanceIlwrath);
    }
}
