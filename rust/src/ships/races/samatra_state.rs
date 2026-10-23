//! Boss lifecycle observations. Only the live boss callbacks record these events.

use parking_lot::Mutex;

pub const OFFSETS: [(i32, i32); 8] = [
    (-136, -35),
    (-47, -70),
    (35, -67),
    (118, -42),
    (115, 46),
    (64, 79),
    (-96, 76),
    (-145, 47),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Generator {
    pub id: u8,
    pub x: i32,
    pub y: i32,
    pub hit_points: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Snapshot {
    pub remaining: u8,
    pub generators: Vec<Generator>,
    pub gate_open: bool,
    pub gate: (i32, i32),
    pub bomb_carrier: bool,
    pub selected_species: Option<u32>,
    pub player: Option<crate::automation::battle_fight::ShipState>,
    pub arena: (i32, i32),
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct Events {
    pub generation: u64,
    pub destroyed: u8,
    pub gate_entered: bool,
}

static EVENTS: Mutex<Events> = Mutex::new(Events {
    generation: 0,
    destroyed: 0,
    gate_entered: false,
});

pub fn events() -> Events {
    *EVENTS.lock()
}

// Identity keys are never dereferenced. Battle view scrolling translates every
// element, so neither current position nor queue order can identify a generator.
#[derive(Default)]
struct GeneratorRegistry(Vec<usize>);

impl GeneratorRegistry {
    fn register(&mut self, identity: usize) {
        assert!(
            self.0.len() < 8,
            "at most eight generators per boss generation"
        );
        self.0.push(identity);
    }

    fn id(&self, identity: usize) -> usize {
        self.0
            .iter()
            .position(|key| *key == identity)
            .expect("generator was registered by its owning spawn callback")
    }
}

static GENERATORS: Mutex<GeneratorRegistry> = Mutex::new(GeneratorRegistry(Vec::new()));

pub(super) fn register_generator(identity: usize) {
    GENERATORS.lock().register(identity);
}

pub(super) fn begin() {
    GENERATORS.lock().0.clear();
    let mut events = EVENTS.lock();
    *events = Events {
        generation: events.generation + 1,
        ..Events::default()
    };
}

pub(super) fn generator_destroyed(id: usize) {
    EVENTS.lock().destroyed |= 1 << id;
}

pub(super) fn gate_entered() {
    EVENTS.lock().gate_entered = true;
}

pub(super) fn generator_id(identity: usize) -> usize {
    GENERATORS.lock().id(identity)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lifecycle_counts_each_identity_once_and_retains_gate_entry() {
        begin();
        let generation = events().generation;
        for id in 0..8 {
            register_generator(0x100 + id);
            assert_eq!(generator_id(0x100 + id), id);
            generator_destroyed(id);
            generator_destroyed(id);
        }
        gate_entered();
        assert_eq!(
            events(),
            Events {
                generation,
                destroyed: 255,
                gate_entered: true
            }
        );
        begin();
        assert_eq!(
            events(),
            Events {
                generation: generation + 1,
                ..Events::default()
            }
        );
    }

    #[test]
    fn identity_does_not_depend_on_scrolling_or_survivor_order() {
        let mut registry = GeneratorRegistry::default();
        for identity in [0x10, 0x20, 0x30] {
            registry.register(identity);
        }
        // The surviving elements may be visited in any order, and a scrolled
        // position is deliberately not an input to this identity lookup.
        for (identity, expected) in [(0x30, 2), (0x10, 0), (0x20, 1)] {
            assert_eq!(registry.id(identity), expected);
        }
    }
}
