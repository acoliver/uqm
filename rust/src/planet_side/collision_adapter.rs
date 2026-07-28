//! Pixel-mask collision adapter over the Rust-owned surface entity world.

use std::collections::HashMap;

use super::assembly::{remove_surface_entity, SharedSurface};
use super::collision::{CollisionOutcome, CollisionRolls, LanderCollision};
use super::creatures::CreatureCatalog;
use super::entities::{SurfaceEntityId, SurfaceEntityKind};
use super::generation::{persist_pickup, ScanPersistence, SurfaceGenerator};
use super::geometry::{masks_intersect, CollisionMask};
use super::runtime::{AdapterError, CollisionContact, PlanetSideCollision};
use super::simulation::LanderState;
use super::special_effects::{self, SpecialPickupEffects};

/// Gameplay RNG boundary. Surface generation uses a separate random context.
pub trait GameplayRandom {
    fn next(&mut self) -> u32;
}

/// Production gameplay random stream.
pub struct CffiGameplayRandom;

impl GameplayRandom for CffiGameplayRandom {
    fn next(&mut self) -> u32 {
        crate::math::TFB_Random()
    }
}

/// Collision masks associated with Rust surface entities and lander facings.
pub struct SurfaceMasks {
    lander: Vec<CollisionMask>,
    entities: HashMap<SurfaceEntityId, CollisionMask>,
}

impl SurfaceMasks {
    pub fn new(lander: Vec<CollisionMask>) -> Result<Self, AdapterError> {
        if lander.len() != 16 {
            return Err(AdapterError::new("lander_collision_masks"));
        }
        Ok(Self {
            lander,
            entities: HashMap::new(),
        })
    }

    pub fn insert_entity(&mut self, entity: SurfaceEntityId, mask: CollisionMask) {
        self.entities.insert(entity, mask);
    }

    pub fn remove_entity(&mut self, entity: SurfaceEntityId) {
        self.entities.remove(&entity);
    }
}

/// Concrete Rust collision adapter. Geometry and classification are both
/// independent of C `ELEMENT` and `IntersectControl` layouts.
pub struct SurfaceCollisionAdapter<R, G> {
    pub surface: SharedSurface,
    pub random: R,
    pub generator: G,
    pub persistence: ScanPersistence,
}

impl<R: GameplayRandom, G: SurfaceGenerator> PlanetSideCollision for SurfaceCollisionAdapter<R, G> {
    fn contacts(&mut self, lander: &LanderState) -> Result<Vec<CollisionContact>, AdapterError> {
        let surface = self.surface.borrow();
        let lander_mask = &surface.masks.lander[usize::from(lander.facing) % 16];
        let mut contacts = Vec::new();
        for (id, entity) in surface.world.iter() {
            let Some(entity_mask) = surface.masks.entities.get(&id) else {
                continue;
            };
            if !masks_intersect(lander.position, lander_mask, entity.position, entity_mask) {
                continue;
            }
            let Some(collision) = classify(&entity.kind) else {
                continue;
            };
            let rolls = rolls_for(&mut self.random, collision);
            contacts.push(CollisionContact {
                entity: Some(id),
                collision,
                rolls,
            });
        }
        Ok(contacts)
    }

    fn commit(
        &mut self,
        contact: CollisionContact,
        outcome: &CollisionOutcome,
        crew: u8,
    ) -> Result<SpecialPickupEffects, AdapterError> {
        let Some(entity_id) = contact.entity else {
            return Ok(SpecialPickupEffects::default());
        };
        let generated = {
            let surface = self.surface.borrow();
            surface
                .generated
                .iter()
                .find(|entry| entry.entity == entity_id)
                .copied()
        };
        let mut effects = SpecialPickupEffects::default();
        let should_remove = match outcome {
            CollisionOutcome::Cargo {
                remove_node: true, ..
            } => match generated {
                Some(mapping) => {
                    special_effects::begin(crew);
                    let result =
                        persist_pickup(&mut self.generator, &mut self.persistence, mapping);
                    effects = special_effects::finish();
                    result?
                }
                None => true,
            },
            CollisionOutcome::EnergyPickupRequested { .. } => {
                let mapping = generated.ok_or(AdapterError::new("energy_generation_mapping"))?;
                special_effects::begin(crew);
                let result = persist_pickup(&mut self.generator, &mut self.persistence, mapping);
                effects = special_effects::finish();
                result?
            }
            CollisionOutcome::NoEffect
            | CollisionOutcome::CrewDamage(_)
            | CollisionOutcome::Cargo {
                remove_node: false, ..
            } => false,
        };
        if should_remove {
            remove_surface_entity(&mut self.surface.borrow_mut(), entity_id)?;
        }
        Ok(effects)
    }
}

fn classify(kind: &SurfaceEntityKind) -> Option<LanderCollision> {
    match kind {
        SurfaceEntityKind::MineralNode { category, amount } => Some(LanderCollision::Mineral {
            category: *category,
            amount: *amount,
        }),
        SurfaceEntityKind::EnergyNode { node } => Some(LanderCollision::Energy { node: *node }),
        SurfaceEntityKind::LiveCreature { kind, .. } => Some(LanderCollision::LiveCreature {
            danger: CreatureCatalog::stats(*kind).danger,
        }),
        SurfaceEntityKind::CannedCreature { value } => {
            Some(LanderCollision::CannedBiological { value: *value })
        }
        SurfaceEntityKind::Hazard(hazard) => Some(LanderCollision::NaturalHazard(*hazard)),
        SurfaceEntityKind::Shot(_) | SurfaceEntityKind::Explosion => None,
    }
}

fn rolls_for(random: &mut impl GameplayRandom, collision: LanderCollision) -> CollisionRolls {
    match collision {
        LanderCollision::NaturalHazard(_) => {
            let hazard_injury = random.next();
            let shield = if hazard_injury % 100 < 25 {
                random.next()
            } else {
                0
            };
            CollisionRolls {
                hazard_injury,
                shield,
                ..CollisionRolls::default()
            }
        }
        LanderCollision::LiveCreature { danger } => {
            const CHANCE: [u32; 4] = [0, 6, 13, 26];
            let biological_attack = random.next();
            let shield = if biological_attack % 128 < CHANCE[danger as usize] {
                random.next()
            } else {
                0
            };
            CollisionRolls {
                biological_attack,
                shield,
                ..CollisionRolls::default()
            }
        }
        LanderCollision::Mineral { .. }
        | LanderCollision::CannedBiological { .. }
        | LanderCollision::Energy { .. } => CollisionRolls::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::planet_side::entities::{SurfaceEntity, SurfaceWorld};
    use crate::planet_side::hazards::HazardKind;
    use crate::planet_side::model::{CrewCount, LanderUpgrades, SurfacePoint};

    #[derive(Default)]
    struct Generator;

    impl SurfaceGenerator for Generator {
        fn node_count(
            &mut self,
            _scan: super::super::generation::ScanType,
        ) -> Result<u8, AdapterError> {
            Ok(0)
        }

        fn generate(
            &mut self,
            _scan: super::super::generation::ScanType,
            _node: super::super::generation::ScanNodeId,
        ) -> Result<super::super::generation::GeneratedNode, AdapterError> {
            Err(AdapterError::new("unexpected_generate"))
        }

        fn pickup(
            &mut self,
            _scan: super::super::generation::ScanType,
            _node: super::super::generation::ScanNodeId,
        ) -> Result<bool, AdapterError> {
            Ok(true)
        }
    }
    struct Random {
        values: std::collections::VecDeque<u32>,
        calls: usize,
    }

    impl GameplayRandom for Random {
        fn next(&mut self) -> u32 {
            self.calls += 1;
            self.values.pop_front().unwrap_or_default()
        }
    }

    fn solid() -> CollisionMask {
        CollisionMask::from_occupancy(1, 1, SurfacePoint::default(), &[1]).unwrap()
    }

    fn lander() -> LanderState {
        LanderState::new(
            SurfacePoint::default(),
            0,
            CrewCount::new(12),
            LanderUpgrades::default(),
        )
    }

    #[test]
    fn overlapping_creature_is_classified_from_catalog() {
        let mut world = SurfaceWorld::new();
        let id = world.insert(SurfaceEntity {
            kind: SurfaceEntityKind::LiveCreature {
                kind: super::super::creatures::CreatureKind::new(23).unwrap(),
                hit_points: 1,
                aware: false,
            },
            position: SurfacePoint::default(),
            finite_life: None,
        });
        let mut masks = SurfaceMasks::new((0..16).map(|_| solid()).collect()).unwrap();
        masks.insert_entity(id, solid());
        let surface =
            super::super::assembly::share_surface(super::super::assembly::SurfaceAssembly {
                world,
                generated: Vec::new(),
                frames: super::super::graphics_adapter::SurfaceFrameRegistry::default(),
                masks,
            });
        let mut adapter = SurfaceCollisionAdapter {
            surface,
            random: Random {
                values: [0, 99].into(),
                calls: 0,
            },
            generator: Generator,
            persistence: ScanPersistence::default(),
        };
        let contacts = adapter.contacts(&lander()).unwrap();
        assert!(matches!(
            contacts.as_slice(),
            [CollisionContact {
                collision: LanderCollision::LiveCreature { .. },
                rolls: CollisionRolls {
                    biological_attack: 0,
                    shield: 99,
                    ..
                },
                ..
            }]
        ));
        assert_eq!(adapter.random.calls, 2);
    }

    #[test]
    fn missed_hazard_gate_does_not_consume_shield_roll() {
        let mut world = SurfaceWorld::new();
        let id = world.insert(SurfaceEntity {
            kind: SurfaceEntityKind::Hazard(HazardKind::Earthquake),
            position: SurfacePoint::default(),
            finite_life: Some(1),
        });
        let mut masks = SurfaceMasks::new((0..16).map(|_| solid()).collect()).unwrap();
        masks.insert_entity(id, solid());
        let surface =
            super::super::assembly::share_surface(super::super::assembly::SurfaceAssembly {
                world,
                generated: Vec::new(),
                frames: super::super::graphics_adapter::SurfaceFrameRegistry::default(),
                masks,
            });
        let mut adapter = SurfaceCollisionAdapter {
            surface,
            random: Random {
                values: [25, 77].into(),
                calls: 0,
            },
            generator: Generator,
            persistence: ScanPersistence::default(),
        };
        let contacts = adapter.contacts(&lander()).unwrap();
        assert_eq!(contacts[0].rolls.hazard_injury, 25);
        assert_eq!(contacts[0].rolls.shield, 0);
        assert_eq!(adapter.random.calls, 1);
    }

    #[test]
    fn transparent_or_unregistered_entities_do_not_contact() {
        let mut world = SurfaceWorld::new();
        let registered = world.insert(SurfaceEntity {
            kind: SurfaceEntityKind::MineralNode {
                category: 0,
                amount: 1,
            },
            position: SurfacePoint::default(),
            finite_life: None,
        });
        world.insert(SurfaceEntity {
            kind: SurfaceEntityKind::MineralNode {
                category: 1,
                amount: 2,
            },
            position: SurfacePoint::default(),
            finite_life: None,
        });
        let mut masks = SurfaceMasks::new((0..16).map(|_| solid()).collect()).unwrap();
        masks.insert_entity(
            registered,
            CollisionMask::from_occupancy(1, 1, SurfacePoint::default(), &[0]).unwrap(),
        );
        let surface =
            super::super::assembly::share_surface(super::super::assembly::SurfaceAssembly {
                world,
                generated: Vec::new(),
                frames: super::super::graphics_adapter::SurfaceFrameRegistry::default(),
                masks,
            });
        let mut adapter = SurfaceCollisionAdapter {
            surface,
            random: Random {
                values: std::collections::VecDeque::new(),
                calls: 0,
            },
            generator: Generator,
            persistence: ScanPersistence::default(),
        };
        assert!(adapter.contacts(&lander()).unwrap().is_empty());
        assert_eq!(adapter.random.calls, 0);
    }
}
