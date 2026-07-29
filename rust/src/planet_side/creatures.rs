//! Immutable creature attributes used by planet-side biological entities.

pub const CREATURE_COUNT: usize = 26;

const BEHAVIOR_MASK: u8 = 0x03;
const AWARENESS_MASK: u8 = 0x0c;
const SPEED_MASK: u8 = 0x30;
const DANGER_MASK: u8 = 0xc0;

/// Creature behavior encoded in the low two attribute bits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreatureBehavior {
    Hunt,
    Flee,
    Unpredictable,
    Reserved,
}

/// Creature awareness range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Awareness {
    Low = 0,
    Medium = 1,
    High = 2,
    Reserved = 3,
}

/// Creature movement speed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CreatureSpeed {
    Motionless = 0,
    Slow = 1,
    Medium = 2,
    Fast = 3,
}

/// Creature collision danger.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreatureDanger {
    Harmless,
    Weak,
    Normal,
    Monstrous,
}

/// Fully decoded creature properties.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreatureStats {
    pub behavior: CreatureBehavior,
    pub awareness: Awareness,
    pub speed: CreatureSpeed,
    pub danger: CreatureDanger,
    pub value: u8,
    pub hit_points: u8,
}

/// Valid creature table index.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreatureKind(u8);

impl CreatureKind {
    pub fn new(index: u8) -> Option<Self> {
        (usize::from(index) < CREATURE_COUNT).then_some(Self(index))
    }

    #[must_use]
    pub const fn index(self) -> u8 {
        self.0
    }

    #[must_use]
    pub const fn is_brainbox_bulldozer(self) -> bool {
        self.0 == 24
    }
}

/// C-parity creature catalog translated from `lander.c::CreatureData`.
pub struct CreatureCatalog;

impl CreatureCatalog {
    #[must_use]
    pub fn stats(kind: CreatureKind) -> CreatureStats {
        let (attributes, packed) = RAW_CREATURE_DATA[usize::from(kind.index())];
        CreatureStats {
            behavior: match attributes & BEHAVIOR_MASK {
                0 => CreatureBehavior::Hunt,
                1 => CreatureBehavior::Flee,
                2 => CreatureBehavior::Unpredictable,
                _ => CreatureBehavior::Reserved,
            },
            awareness: match (attributes & AWARENESS_MASK) >> 2 {
                0 => Awareness::Low,
                1 => Awareness::Medium,
                2 => Awareness::High,
                _ => Awareness::Reserved,
            },
            speed: match (attributes & SPEED_MASK) >> 4 {
                0 => CreatureSpeed::Motionless,
                1 => CreatureSpeed::Slow,
                2 => CreatureSpeed::Medium,
                _ => CreatureSpeed::Fast,
            },
            danger: match (attributes & DANGER_MASK) >> 6 {
                0 => CreatureDanger::Harmless,
                1 => CreatureDanger::Weak,
                2 => CreatureDanger::Normal,
                _ => CreatureDanger::Monstrous,
            },
            value: packed >> 4,
            hit_points: packed & 0x0f,
        }
    }
}

const HUNT: u8 = 0;
const FLEE: u8 = 1;
const UNPREDICTABLE: u8 = 2;
const AWARE_MEDIUM: u8 = 1 << 2;
const AWARE_HIGH: u8 = 2 << 2;
const SLOW: u8 = 1 << 4;
const MEDIUM: u8 = 2 << 4;
const FAST: u8 = 3 << 4;
const WEAK: u8 = 1 << 6;
const NORMAL: u8 = 2 << 6;
const MONSTROUS: u8 = 3 << 6;

const fn packed(value: u8, hit_points: u8) -> u8 {
    (value << 4) | hit_points
}

const RAW_CREATURE_DATA: [(u8, u8); CREATURE_COUNT] = [
    (0, packed(1, 1)),
    (0, packed(6, 1)),
    (WEAK, packed(3, 1)),
    (NORMAL, packed(5, 3)),
    (0, packed(2, 10)),
    (UNPREDICTABLE | SLOW, packed(1, 2)),
    (FLEE | AWARE_MEDIUM | SLOW, packed(8, 5)),
    (HUNT | SLOW | WEAK, packed(2, 2)),
    (UNPREDICTABLE | SLOW | NORMAL, packed(3, 8)),
    (HUNT | AWARE_MEDIUM | SLOW | MONSTROUS, packed(10, 15)),
    (HUNT | AWARE_MEDIUM | MEDIUM | WEAK, packed(3, 3)),
    (FLEE | AWARE_MEDIUM | MEDIUM, packed(2, 1)),
    (UNPREDICTABLE | MEDIUM | WEAK, packed(2, 2)),
    (HUNT | AWARE_HIGH | MEDIUM | NORMAL, packed(4, 6)),
    (UNPREDICTABLE | MEDIUM | MONSTROUS, packed(9, 12)),
    (HUNT | AWARE_HIGH | FAST | WEAK, packed(3, 1)),
    (FLEE | AWARE_HIGH | FAST, packed(1, 1)),
    (HUNT | FAST | NORMAL, packed(7, 8)),
    (FLEE | AWARE_HIGH | FAST | WEAK, packed(15, 2)),
    (FLEE | FAST | WEAK, packed(1, 1)),
    (UNPREDICTABLE | SLOW | WEAK, packed(6, 2)),
    (FLEE | AWARE_HIGH | SLOW | WEAK, packed(4, 2)),
    (WEAK, packed(8, 5)),
    (MONSTROUS, packed(1, 1)),
    (UNPREDICTABLE | SLOW, packed(0, 1)),
    (HUNT | AWARE_HIGH | FAST | MONSTROUS, packed(15, 15)),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_out_of_range_creature_ids() {
        assert_eq!(CreatureKind::new(25).map(CreatureKind::index), Some(25));
        assert_eq!(CreatureKind::new(26), None);
    }

    #[test]
    fn evil_one_and_brainbox_match_c_special_entries() {
        let evil_one = CreatureCatalog::stats(CreatureKind::new(23).unwrap());
        assert_eq!(evil_one.speed, CreatureSpeed::Motionless);
        assert_eq!(evil_one.danger, CreatureDanger::Monstrous);
        assert_eq!((evil_one.value, evil_one.hit_points), (1, 1));

        let brainbox = CreatureCatalog::stats(CreatureKind::new(24).unwrap());
        assert_eq!(brainbox.behavior, CreatureBehavior::Unpredictable);
        assert_eq!(brainbox.speed, CreatureSpeed::Slow);
        assert_eq!(brainbox.danger, CreatureDanger::Harmless);
        assert_eq!((brainbox.value, brainbox.hit_points), (0, 1));
    }

    #[test]
    fn zex_beauty_matches_c_special_entry() {
        let beauty = CreatureCatalog::stats(CreatureKind::new(25).unwrap());
        assert_eq!(beauty.behavior, CreatureBehavior::Hunt);
        assert_eq!(beauty.awareness, Awareness::High);
        assert_eq!(beauty.speed, CreatureSpeed::Fast);
        assert_eq!(beauty.danger, CreatureDanger::Monstrous);
        assert_eq!((beauty.value, beauty.hit_points), (15, 15));
    }

    #[test]
    fn all_catalog_entries_decode_valid_nibbles() {
        for index in 0..CREATURE_COUNT as u8 {
            let stats = CreatureCatalog::stats(CreatureKind::new(index).unwrap());
            assert!(stats.value <= 15);
            assert!(stats.hit_points <= 15);
        }
    }
}
