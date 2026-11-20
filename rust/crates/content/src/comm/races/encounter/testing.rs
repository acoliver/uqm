use super::*;
use crate::comm::races::script::Supplies;
use crate::engine::StoryKey;
use std::collections::{HashMap, HashSet};

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Effect<H> {
    Npc(i32),
    Offer(u16, H, Vec<i32>),
    Set(StoryKey, u8),
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
    RemoveEscorts(u16),
    Palette(u16),
    FadePalette(u16, i16),
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

/// Story values for keys no test has set, drawn once per key from a seeded stream.
pub(crate) struct Walk {
    stream: u64,
    memo: HashMap<StoryKey, u8>,
}
impl Walk {
    const VALUES: [u8; 10] = [0, 0, 1, 1, 2, 3, 4, 64, 128, 192];
    fn new(seed: u64) -> Self {
        Self {
            stream: seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1,
            memo: HashMap::new(),
        }
    }
    pub fn draw(&mut self) -> u64 {
        self.stream ^= self.stream << 13;
        self.stream ^= self.stream >> 7;
        self.stream ^= self.stream << 17;
        self.stream >> 11
    }
    fn value(&mut self, key: StoryKey) -> u8 {
        if let Some(value) = self.memo.get(&key) {
            return *value;
        }
        let value = Self::VALUES[self.draw() as usize % Self::VALUES.len()];
        self.memo.insert(key, value);
        value
    }
}

pub(crate) struct Fake<H, E = ()> {
    pub walk: Option<std::cell::RefCell<Walk>>,
    pub state: HashMap<StoryKey, u8>,
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
            walk: None,
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
    pub fn put(&mut self, key: StoryKey, value: u8) {
        self.state.insert(key, value);
    }
}
impl<H: Copy, E> DialogueHost<H> for Fake<H, E> {
    fn get(&self, key: StoryKey) -> u8 {
        match (self.state.get(&key), &self.walk) {
            (Some(value), _) => *value,
            (None, Some(walk)) => walk.borrow_mut().value(key),
            (None, None) => 0,
        }
    }
    fn set(&mut self, key: StoryKey, value: u8) {
        self.put(key, value);
        self.effects.push(Effect::Set(key, value));
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
    fn palette(&mut self, index: u16) {
        self.effects.push(Effect::Palette(index))
    }
    fn talk_segue(&mut self, track: u16) {
        self.effects.push(Effect::Talk(track));
    }
    fn disable_animation(&mut self, _: usize) {
        panic!("unexpected ambient animation effect")
    }
    fn fade_palette(&mut self, index: u16, duration: i16) {
        self.effects.push(Effect::FadePalette(index, duration))
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

/// Walks seeded random conversations: every unset story value is drawn once per walk and each
/// step answers one of the responses the previous step offered. A response that an offered
/// handler ignores, or a handler that panics, fails the walk with its seed.
pub(crate) fn walk<H: Copy + std::fmt::Debug, E: Default>(
    seeds: std::ops::Range<u64>,
    steps: usize,
    mut prepare: impl FnMut(&mut Fake<H, E>, u64),
    mut start: impl FnMut(&mut Fake<H, E>),
    mut handle: impl FnMut(&mut Fake<H, E>, H, u16),
) {
    for seed in seeds {
        let mut f = Fake::<H, E> {
            walk: Some(std::cell::RefCell::new(Walk::new(seed))),
            ..Fake::default()
        };
        let mut choices = Walk::new(!seed);
        let first = choices.draw();
        prepare(&mut f, first);
        start(&mut f);
        for step in 0..steps {
            let offers = f.offers();
            if offers.is_empty() {
                break;
            }
            let (phrase, handler) = offers[choices.draw() as usize % offers.len()];
            let next = choices.draw();
            f.effects.clear();
            prepare(&mut f, next);
            handle(&mut f, handler, phrase);
            assert!(
                !f.effects.is_empty(),
                "seed {seed} step {step}: {handler:?} ignored response {phrase}"
            );
        }
    }
}
