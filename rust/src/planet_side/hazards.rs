//! Deterministic hazard and crew-damage rules from the legacy lander runtime.

use super::model::{CrewCount, ShieldSet};

/// Planet-side hazards and biological attacks that can affect crew.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HazardKind {
    Biological,
    Earthquake,
    Lightning,
    Lava,
}

impl HazardKind {
    #[must_use]
    pub const fn shield_bit(self) -> u8 {
        1 << self as u8
    }
}

/// Typed audio requests emitted by the deterministic gameplay core.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SoundCue {
    BiologicalDisaster,
    Earthquake,
    Lightning,
    Lava,
    LanderInjured,
    LanderShoots,
    LanderHits,
    LifeformCanned,
    Pickup,
    Full,
    Departs,
    Returns,
    Destroyed,
}

/// Result of applying one point of crew damage.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrewDamage {
    pub crew: CrewCount,
    pub shield_hit: bool,
    pub damage_flash_frames: u8,
    pub sounds: Vec<SoundCue>,
}

const TECTONICS_CHANCE: [u8; 8] = [0, 0, 3, 6, 12, 24, 48, 96];
const WEATHER_CHANCE: [u8; 8] = [0, 0, 3, 6, 9, 18, 36, 72];
const FIRE_CHANCE: [u8; 8] = [0, 0, 3, 6, 12, 36, 72, 144];
const TEMPERATURE_BREAKPOINTS: [i32; 7] = [50, 100, 150, 250, 350, 550, 800];
const DAMAGE_CYCLE: u8 = 6;

/// Map a surface temperature to the original 0..=7 thermal hazard rating.
#[must_use]
pub fn thermal_hazard_rating(temperature: i32) -> u8 {
    TEMPERATURE_BREAKPOINTS
        .iter()
        .position(|breakpoint| temperature < *breakpoint)
        .map_or(TEMPERATURE_BREAKPOINTS.len() as u8, |index| index as u8)
}

/// Return the chance, out of 256, that a rated hazard is generated.
#[must_use]
pub fn hazard_chance(kind: HazardKind, rating: u8) -> u8 {
    let table = match kind {
        HazardKind::Earthquake => &TECTONICS_CHANCE,
        HazardKind::Lightning => &WEATHER_CHANCE,
        HazardKind::Lava => &FIRE_CHANCE,
        HazardKind::Biological => return 0,
    };
    table.get(usize::from(rating)).copied().unwrap_or(0)
}

/// Apply the legacy lander shield and crew-loss rule.
///
/// `shield_roll` is interpreted modulo 100, exactly like `TFB_Random() % 100`.
#[must_use]
pub fn apply_crew_damage(
    mut crew: CrewCount,
    shields: ShieldSet,
    hazard: HazardKind,
    shield_roll: u32,
) -> CrewDamage {
    if crew.get() == 0 {
        return CrewDamage {
            crew,
            shield_hit: false,
            damage_flash_frames: 0,
            sounds: Vec::new(),
        };
    }

    let shield_hit = shields.contains(hazard.shield_bit()) && shield_roll % 100 < 95;
    let mut sounds = Vec::new();
    if hazard == HazardKind::Biological {
        sounds.push(SoundCue::BiologicalDisaster);
    }
    if !shield_hit && crew.lose_one() {
        sounds.push(SoundCue::LanderInjured);
    }

    CrewDamage {
        crew,
        shield_hit,
        damage_flash_frames: DAMAGE_CYCLE,
        sounds,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thermal_rating_matches_all_c_boundaries() {
        let cases = [
            (-100, 0),
            (49, 0),
            (50, 1),
            (99, 1),
            (100, 2),
            (149, 2),
            (150, 3),
            (249, 3),
            (250, 4),
            (349, 4),
            (350, 5),
            (549, 5),
            (550, 6),
            (799, 6),
            (800, 7),
            (10_000, 7),
        ];
        for (temperature, expected) in cases {
            assert_eq!(thermal_hazard_rating(temperature), expected);
        }
    }

    #[test]
    fn hazard_chance_tables_match_c() {
        assert_eq!(
            (0..8)
                .map(|rating| hazard_chance(HazardKind::Earthquake, rating))
                .collect::<Vec<_>>(),
            TECTONICS_CHANCE
        );
        assert_eq!(
            (0..8)
                .map(|rating| hazard_chance(HazardKind::Lightning, rating))
                .collect::<Vec<_>>(),
            WEATHER_CHANCE
        );
        assert_eq!(
            (0..8)
                .map(|rating| hazard_chance(HazardKind::Lava, rating))
                .collect::<Vec<_>>(),
            FIRE_CHANCE
        );
        assert_eq!(hazard_chance(HazardKind::Earthquake, 8), 0);
    }

    #[test]
    fn unshielded_biological_damage_emits_collision_and_injury_sounds() {
        let result = apply_crew_damage(
            CrewCount::new(12),
            ShieldSet::default(),
            HazardKind::Biological,
            0,
        );
        assert_eq!(result.crew, CrewCount::new(11));
        assert!(!result.shield_hit);
        assert_eq!(result.damage_flash_frames, 6);
        assert_eq!(
            result.sounds,
            [SoundCue::BiologicalDisaster, SoundCue::LanderInjured]
        );
    }

    #[test]
    fn matching_shield_blocks_damage_for_rolls_below_95() {
        let result = apply_crew_damage(
            CrewCount::new(8),
            ShieldSet::from_bits(HazardKind::Lightning.shield_bit()),
            HazardKind::Lightning,
            94,
        );
        assert_eq!(result.crew, CrewCount::new(8));
        assert!(result.shield_hit);
        assert_eq!(result.damage_flash_frames, 6);
        assert!(result.sounds.is_empty());
    }

    #[test]
    fn matching_shield_fails_at_95_percent_boundary() {
        let result = apply_crew_damage(
            CrewCount::new(8),
            ShieldSet::from_bits(HazardKind::Earthquake.shield_bit()),
            HazardKind::Earthquake,
            95,
        );
        assert_eq!(result.crew, CrewCount::new(7));
        assert!(!result.shield_hit);
        assert_eq!(result.sounds, [SoundCue::LanderInjured]);
    }
}
