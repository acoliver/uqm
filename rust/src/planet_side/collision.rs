//! Typed collision outcomes for lander, biological, cargo, and hazard contacts.

use super::cargo::{BioCargo, CargoPickup, MineralCargo};
use super::creatures::CreatureDanger;
use super::hazards::{apply_crew_damage, CrewDamage, HazardKind};
use super::model::{CrewCount, ShieldSet};

const BIOLOGICAL_ATTACK_CHANCE: [u8; 4] = [0, 6, 13, 26];

/// A surface object that can overlap the lander.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LanderCollision {
    NaturalHazard(HazardKind),
    LiveCreature { danger: CreatureDanger },
    Mineral { category: usize, amount: u16 },
    CannedBiological { value: u16 },
    Energy { node: u8 },
}

/// Explicit random values consumed by collision resolution.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CollisionRolls {
    /// Roll used for the 25-percent earthquake/lava injury gate.
    pub hazard_injury: u32,
    /// Roll used for the danger-dependent biological attack gate.
    pub biological_attack: u32,
    /// Roll used for the 95-percent matching shield gate.
    pub shield: u32,
}

/// Effect of resolving one overlapping surface object.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CollisionOutcome {
    NoEffect,
    CrewDamage(CrewDamage),
    Cargo {
        pickup: CargoPickup,
        remove_node: bool,
    },
    EnergyPickupRequested {
        node: u8,
    },
}

/// Mutable values owned by the active trip and affected by collisions.
pub struct CollisionState<'a> {
    pub crew: &'a mut CrewCount,
    pub shields: ShieldSet,
    pub minerals: &'a mut MineralCargo,
    pub biological: &'a mut BioCargo,
}

/// Resolve one lander/object collision without invoking rendering, audio, or C callbacks.
pub fn resolve_lander_collision(
    state: &mut CollisionState<'_>,
    collision: LanderCollision,
    rolls: CollisionRolls,
) -> CollisionOutcome {
    match collision {
        LanderCollision::NaturalHazard(hazard) => {
            if !matches!(hazard, HazardKind::Earthquake | HazardKind::Lava)
                || rolls.hazard_injury % 100 >= 25
            {
                return CollisionOutcome::NoEffect;
            }
            damage(state, hazard, rolls.shield)
        }
        LanderCollision::LiveCreature { danger } => {
            let threshold = BIOLOGICAL_ATTACK_CHANCE[danger as usize];
            if rolls.biological_attack % 128 >= u32::from(threshold) {
                CollisionOutcome::NoEffect
            } else {
                damage(state, HazardKind::Biological, rolls.shield)
            }
        }
        LanderCollision::Mineral { category, amount } => {
            let pickup = state.minerals.collect(category, amount);
            CollisionOutcome::Cargo {
                remove_node: matches!(pickup, CargoPickup::Collected { complete: true, .. }),
                pickup,
            }
        }
        LanderCollision::CannedBiological { value } => {
            let pickup = state.biological.collect(value);
            CollisionOutcome::Cargo {
                remove_node: matches!(pickup, CargoPickup::Collected { complete: true, .. }),
                pickup,
            }
        }
        LanderCollision::Energy { node } => CollisionOutcome::EnergyPickupRequested { node },
    }
}

fn damage(
    state: &mut CollisionState<'_>,
    hazard: HazardKind,
    shield_roll: u32,
) -> CollisionOutcome {
    let result = apply_crew_damage(*state.crew, state.shields, hazard, shield_roll);
    *state.crew = result.crew;
    CollisionOutcome::CrewDamage(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::planet_side::hazards::SoundCue;

    fn with_state<T>(
        shields: ShieldSet,
        f: impl FnOnce(&mut CollisionState<'_>) -> T,
    ) -> (T, CrewCount, MineralCargo, BioCargo) {
        let mut crew = CrewCount::new(12);
        let mut minerals = MineralCargo::new(100, 0, false);
        let mut biological = BioCargo::default();
        let result = f(&mut CollisionState {
            crew: &mut crew,
            shields,
            minerals: &mut minerals,
            biological: &mut biological,
        });
        (result, crew, minerals, biological)
    }

    #[test]
    fn monstrous_creature_can_damage_with_both_collision_sounds() {
        let (outcome, crew, _, _) = with_state(ShieldSet::default(), |state| {
            resolve_lander_collision(
                state,
                LanderCollision::LiveCreature {
                    danger: CreatureDanger::Monstrous,
                },
                CollisionRolls {
                    biological_attack: 25,
                    shield: 99,
                    ..CollisionRolls::default()
                },
            )
        });
        let CollisionOutcome::CrewDamage(damage) = outcome else {
            panic!("expected crew damage");
        };
        assert_eq!(crew, CrewCount::new(11));
        assert_eq!(
            damage.sounds,
            [SoundCue::BiologicalDisaster, SoundCue::LanderInjured]
        );
    }

    #[test]
    fn biological_attack_miss_has_no_damage_or_sound() {
        let (outcome, crew, _, _) = with_state(ShieldSet::default(), |state| {
            resolve_lander_collision(
                state,
                LanderCollision::LiveCreature {
                    danger: CreatureDanger::Monstrous,
                },
                CollisionRolls {
                    biological_attack: 26,
                    ..CollisionRolls::default()
                },
            )
        });
        assert_eq!(outcome, CollisionOutcome::NoEffect);
        assert_eq!(crew, CrewCount::new(12));
    }

    #[test]
    fn harmless_creature_never_attacks() {
        let (outcome, _, _, _) = with_state(ShieldSet::default(), |state| {
            resolve_lander_collision(
                state,
                LanderCollision::LiveCreature {
                    danger: CreatureDanger::Harmless,
                },
                CollisionRolls::default(),
            )
        });
        assert_eq!(outcome, CollisionOutcome::NoEffect);
    }

    #[test]
    fn natural_hazard_injury_gate_matches_twenty_five_percent_boundary() {
        let (hit, hit_crew, _, _) = with_state(ShieldSet::default(), |state| {
            resolve_lander_collision(
                state,
                LanderCollision::NaturalHazard(HazardKind::Lava),
                CollisionRolls {
                    hazard_injury: 24,
                    shield: 99,
                    ..CollisionRolls::default()
                },
            )
        });
        assert!(matches!(hit, CollisionOutcome::CrewDamage(_)));
        assert_eq!(hit_crew, CrewCount::new(11));

        let (miss, miss_crew, _, _) = with_state(ShieldSet::default(), |state| {
            resolve_lander_collision(
                state,
                LanderCollision::NaturalHazard(HazardKind::Lava),
                CollisionRolls {
                    hazard_injury: 25,
                    ..CollisionRolls::default()
                },
            )
        });
        assert_eq!(miss, CollisionOutcome::NoEffect);
        assert_eq!(miss_crew, CrewCount::new(12));
    }

    #[test]
    fn shielded_biological_hit_emits_disaster_sound_without_injury() {
        let shields = ShieldSet::from_bits(HazardKind::Biological.shield_bit());
        let (outcome, crew, _, _) = with_state(shields, |state| {
            resolve_lander_collision(
                state,
                LanderCollision::LiveCreature {
                    danger: CreatureDanger::Weak,
                },
                CollisionRolls {
                    biological_attack: 0,
                    shield: 94,
                    ..CollisionRolls::default()
                },
            )
        });
        let CollisionOutcome::CrewDamage(damage) = outcome else {
            panic!("expected shielded collision");
        };
        assert_eq!(crew, CrewCount::new(12));
        assert!(damage.shield_hit);
        assert_eq!(damage.sounds, [SoundCue::BiologicalDisaster]);
    }

    #[test]
    fn mineral_and_biological_pickups_mutate_distinct_holds() {
        let (mineral_outcome, _, minerals, biological) =
            with_state(ShieldSet::default(), |state| {
                resolve_lander_collision(
                    state,
                    LanderCollision::Mineral {
                        category: 2,
                        amount: 4,
                    },
                    CollisionRolls::default(),
                )
            });
        assert_eq!(minerals.categories()[2], 4);
        assert_eq!(biological.level(), 0);
        assert_eq!(
            mineral_outcome,
            CollisionOutcome::Cargo {
                pickup: CargoPickup::Collected {
                    amount: 4,
                    complete: true
                },
                remove_node: true
            }
        );
    }

    #[test]
    fn partial_mineral_pickup_keeps_node_on_surface() {
        let mut crew = CrewCount::new(12);
        let mut minerals = MineralCargo::new(3, 0, false);
        let mut biological = BioCargo::default();
        let outcome = resolve_lander_collision(
            &mut CollisionState {
                crew: &mut crew,
                shields: ShieldSet::default(),
                minerals: &mut minerals,
                biological: &mut biological,
            },
            LanderCollision::Mineral {
                category: 2,
                amount: 5,
            },
            CollisionRolls::default(),
        );
        assert_eq!(
            outcome,
            CollisionOutcome::Cargo {
                pickup: CargoPickup::Collected {
                    amount: 3,
                    complete: false
                },
                remove_node: false
            }
        );
    }

    #[test]
    fn energy_collision_is_deferred_to_typed_generation_hook() {
        let (outcome, _, _, _) = with_state(ShieldSet::default(), |state| {
            resolve_lander_collision(
                state,
                LanderCollision::Energy { node: 7 },
                CollisionRolls::default(),
            )
        });
        assert_eq!(outcome, CollisionOutcome::EnergyPickupRequested { node: 7 });
    }
}
