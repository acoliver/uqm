//! Pixel-mask collision adapter over the Rust-owned surface entity world.

use std::collections::HashMap;

use super::assembly::{
    insert_surface_entity, remove_surface_entity, transform_creature_to_canned, SharedSurface,
    SurfaceAssembly, WorldVisualPort,
};
use super::collision::{CollisionOutcome, CollisionRolls, LanderCollision};
use super::creatures::CreatureCatalog;
use super::entities::{SurfaceEntity, SurfaceEntityId, SurfaceEntityKind};
use super::generation::{persist_pickup, ScanPersistence, SurfaceGenerator};
use super::geometry::{masks_intersect, CollisionMask};
use super::hazards::{HazardKind, SoundCue};
use super::model::SurfacePoint;
use super::runtime::{AdapterError, CollisionContact, PlanetSideCollision, WorldStepResult};
use super::simulation::{LanderState, Shot};
use super::special_effects::{self, SpecialPickupEffects};
use super::world::{self, HazardChances, WorldStepInputs};

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

/// Blanket bridge: any `GameplayRandom` is automatically a `WorldRandom`.
/// This lets the collision adapter use the same random stream for both lander
/// collision rolls and world simulation without a separate wrapper struct.
impl<R: GameplayRandom + ?Sized> world::WorldRandom for R {
    fn next(&mut self) -> u32 {
        GameplayRandom::next(self)
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

    /// Return the collision mask for the given entity, if registered.
    #[must_use]
    pub fn entity_mask(&self, entity: SurfaceEntityId) -> Option<&CollisionMask> {
        self.entities.get(&entity)
    }
}

/// Adapter implementing [`world::MaskLookup`] over a [`SurfaceMasks`] registry.
///
/// This bridges the shared collision-mask map to the world simulation's mask
/// lookup trait without copying.
pub struct SurfaceMaskLookup<'a> {
    masks: &'a SurfaceMasks,
}

impl<'a> SurfaceMaskLookup<'a> {
    #[must_use]
    pub const fn new(masks: &'a SurfaceMasks) -> Self {
        Self { masks }
    }
}

impl world::MaskLookup for SurfaceMaskLookup<'_> {
    fn mask(&self, id: SurfaceEntityId) -> Option<&CollisionMask> {
        self.masks.entity_mask(id)
    }
}

/// Concrete Rust collision adapter. Geometry and classification are both
/// independent of C `ELEMENT` and `IntersectControl` layouts.
pub struct SurfaceCollisionAdapter<R, G, V> {
    pub surface: SharedSurface,
    pub random: R,
    pub generator: G,
    pub persistence: ScanPersistence,
    pub world_visuals: V,
    /// Frame count for earthquake entity life_span calculation. Sourced from the
    /// earthquake graphic's frame count at session initialization.
    pub earthquake_frame_count: u16,
    /// Frame count for lava entity life_span calculation. Sourced from the lava
    /// graphic's frame count at session initialization.
    pub lava_frame_count: u16,
}

impl<R, G, V> PlanetSideCollision for SurfaceCollisionAdapter<R, G, V>
where
    R: GameplayRandom,
    G: SurfaceGenerator,
    V: WorldVisualPort,
{
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

    fn register_shot(&mut self, shot: Shot) -> Result<(), AdapterError> {
        let visual = self.world_visuals.shot_visual(shot.facing)?;
        let entity = SurfaceEntity {
            kind: SurfaceEntityKind::Shot(shot),
            position: shot.position,
            finite_life: Some(u16::from(shot.life)),
        };
        let mut surface = self.surface.borrow_mut();
        insert_surface_entity(&mut surface, entity, visual);
        Ok(())
    }

    fn step_world(
        &mut self,
        lander: &LanderState,
        chances: HazardChances,
    ) -> Result<WorldStepResult, AdapterError> {
        let lander_point = SurfacePoint {
            x: lander.position.x,
            y: lander.position.y,
        };

        // Roll hazard spawns before stepping the world so new hazards are
        // inserted into the entity list and processed next frame, matching C.
        let hazard_spawns = world::hazard_spawns_for_frame(
            &mut self.random,
            chances,
            lander_point,
            self.earthquake_frame_count,
            self.lava_frame_count,
        );

        // Step every entity: creature AI, velocity integration, shot-creature
        // collisions, and lifetime expiry.
        let effects = {
            let mut surface = self.surface.borrow_mut();
            let SurfaceAssembly { world, masks, .. } = &mut *surface;
            let lookup = SurfaceMaskLookup::new(masks);
            world::step_world(
                world,
                WorldStepInputs {
                    lander_position: lander_point,
                    shot_masks: &lookup,
                    creature_masks: &lookup,
                    random: &mut self.random,
                },
            )
        };

        Ok(WorldStepResult {
            effects,
            hazard_spawns,
            lightning_kills: 0,
        })
    }

    fn apply_world_step(
        &mut self,
        result: &WorldStepResult,
        audio: &mut impl super::runtime::PlanetSideAudio,
    ) -> Result<(), AdapterError> {
        // Play world sounds in source order.
        for cue in &result.effects.sounds {
            audio.play(*cue)?;
        }

        // Transform canned creatures: swap live creature → canned creature and
        // update the visual.
        for (entity_id, value) in &result.effects.canned {
            let kind = {
                let surface = self.surface.borrow();
                surface.world.get(*entity_id).and_then(|e| match &e.kind {
                    SurfaceEntityKind::LiveCreature { kind, .. } => Some(*kind),
                    _ => None,
                })
            };
            let Some(kind) = kind else {
                continue;
            };
            let visual = self.world_visuals.canned_creature_visual(kind)?;
            transform_creature_to_canned(
                &mut self.surface.borrow_mut(),
                *entity_id,
                *value,
                visual,
            )?;
        }

        {
            let mut surface = self.surface.borrow_mut();
            let SurfaceAssembly { world, frames, .. } = &mut *surface;
            frames.advance_hazard_frames(world);
        }

        // Insert hazard spawns with synchronized visuals.
        for spawn in &result.hazard_spawns {
            let visual = self.world_visuals.hazard_visual(spawn.kind)?;
            let entity = SurfaceEntity {
                kind: SurfaceEntityKind::Hazard(spawn.kind),
                position: spawn.position,
                finite_life: Some(spawn.life_span),
            };
            match spawn.kind {
                HazardKind::Earthquake => audio.play(SoundCue::Earthquake)?,
                HazardKind::Lightning => audio.play(SoundCue::Lightning)?,
                HazardKind::Lava => audio.play(SoundCue::Lava)?,
                HazardKind::Biological => {}
            }
            insert_surface_entity(&mut self.surface.borrow_mut(), entity, visual);
        }

        // Remove expired entities from all registries.
        for entity_id in &result.effects.expired {
            let _ = remove_surface_entity(&mut self.surface.borrow_mut(), *entity_id);
        }

        Ok(())
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
            danger: if kind.is_brainbox_bulldozer() {
                super::creatures::CreatureDanger::Monstrous
            } else {
                CreatureCatalog::stats(*kind).danger
            },
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
    use crate::planet_side::assembly::EntityVisual;
    use crate::planet_side::entities::{SurfaceEntity, SurfaceWorld};
    use crate::planet_side::graphics_adapter::SurfaceFrame;
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

    /// Test visual port that returns a solid 1×1 mask and dangling frame pointer.
    struct TestWorldVisuals;

    impl WorldVisualPort for TestWorldVisuals {
        fn shot_visual(&mut self, _facing: u8) -> Result<EntityVisual, AdapterError> {
            Ok(EntityVisual {
                frame: SurfaceFrame {
                    base: std::ptr::NonNull::<u8>::dangling().as_ptr().cast(),
                    index: 0,
                },
                mask: solid(),
            })
        }

        fn canned_creature_visual(
            &mut self,
            _kind: super::super::creatures::CreatureKind,
        ) -> Result<EntityVisual, AdapterError> {
            Ok(EntityVisual {
                frame: SurfaceFrame {
                    base: std::ptr::NonNull::<u8>::dangling().as_ptr().cast(),
                    index: 1,
                },
                mask: solid(),
            })
        }

        fn hazard_visual(&mut self, _kind: HazardKind) -> Result<EntityVisual, AdapterError> {
            Ok(EntityVisual {
                frame: SurfaceFrame {
                    base: std::ptr::NonNull::<u8>::dangling().as_ptr().cast(),
                    index: 2,
                },
                mask: solid(),
            })
        }
    }

    #[test]
    fn overlapping_creature_is_classified_from_catalog() {
        let mut world = SurfaceWorld::new();
        let id = world.insert(SurfaceEntity {
            kind: SurfaceEntityKind::LiveCreature {
                kind: super::super::creatures::CreatureKind::new(23).unwrap(),
                hit_points: 1,
                aware: false,
                velocity: crate::battle::velocity::VelocityDesc::new(),
                thrust_wait: 0,
                frame_index: 0,
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
            world_visuals: TestWorldVisuals,
            earthquake_frame_count: 13,
            lava_frame_count: 7,
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
            world_visuals: TestWorldVisuals,
            earthquake_frame_count: 13,
            lava_frame_count: 7,
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
            world_visuals: TestWorldVisuals,
            earthquake_frame_count: 13,
            lava_frame_count: 7,
        };
        assert!(adapter.contacts(&lander()).unwrap().is_empty());
        assert_eq!(adapter.random.calls, 0);
    }

    #[test]
    fn register_shot_inserts_entity_into_world_frames_and_masks() {
        let world = SurfaceWorld::new();
        let mut masks = SurfaceMasks::new((0..16).map(|_| solid()).collect()).unwrap();
        let frames = super::super::graphics_adapter::SurfaceFrameRegistry::default();
        let _ = &mut masks; // ensure masks is used
        let surface =
            super::super::assembly::share_surface(super::super::assembly::SurfaceAssembly {
                world,
                generated: Vec::new(),
                frames,
                masks,
            });
        let mut adapter = SurfaceCollisionAdapter {
            surface: surface.clone(),
            random: Random {
                values: std::collections::VecDeque::new(),
                calls: 0,
            },
            generator: Generator,
            persistence: ScanPersistence::default(),
            world_visuals: TestWorldVisuals,
            earthquake_frame_count: 13,
            lava_frame_count: 7,
        };

        let shot = super::super::simulation::Shot {
            position: SurfacePoint { x: 10, y: 10 },
            facing: 0,
            velocity_x: 1,
            velocity_y: 0,
            life: 12,
        };
        adapter.register_shot(shot).unwrap();

        let surface = surface.borrow();
        assert_eq!(surface.world.len(), 1, "shot entity must be in the world");
        let (id, entity) = surface.world.iter().next().unwrap();
        assert!(matches!(entity.kind, SurfaceEntityKind::Shot(_)));
        assert!(surface.frames.get(id).is_some(), "shot must have a frame");
        assert!(
            surface.masks.entity_mask(id).is_some(),
            "shot must have a mask"
        );
    }

    #[test]
    fn step_world_advances_creatures_and_spawns_hazards() {
        use super::super::world::{HazardChances, WORLD_HEIGHT};
        use crate::battle::velocity::VelocityDesc;
        use crate::planet_side::creatures::CreatureKind;
        use crate::planet_side::entities::{SurfaceEntity, SurfaceEntityKind};
        use crate::planet_side::model::SurfacePoint;

        let mut world = SurfaceWorld::new();
        // Insert a hunting creature near the lander so AI runs.
        world.insert(SurfaceEntity {
            kind: SurfaceEntityKind::LiveCreature {
                kind: CreatureKind::new(9).unwrap(),
                hit_points: 15,
                aware: true,
                velocity: VelocityDesc::new(),
                thrust_wait: 0,
                frame_index: 3,
            },
            position: SurfacePoint {
                x: 100,
                y: WORLD_HEIGHT / 2,
            },
            finite_life: None,
        });
        let masks = SurfaceMasks::new((0..16).map(|_| solid()).collect()).unwrap();
        let surface =
            super::super::assembly::share_surface(super::super::assembly::SurfaceAssembly {
                world,
                generated: Vec::new(),
                frames: super::super::graphics_adapter::SurfaceFrameRegistry::default(),
                masks,
            });
        let mut adapter = SurfaceCollisionAdapter {
            surface: surface.clone(),
            random: Random {
                // Provide random values for hazard gate + creature AI.
                values: [0; 8].into(),
                calls: 0,
            },
            generator: Generator,
            persistence: ScanPersistence::default(),
            world_visuals: TestWorldVisuals,
            earthquake_frame_count: 13,
            lava_frame_count: 7,
        };

        let lander = LanderState::new(
            SurfacePoint { x: 100, y: 100 },
            0,
            CrewCount::new(12),
            LanderUpgrades::default(),
        );
        let result = adapter
            .step_world(&lander, HazardChances::default())
            .unwrap();

        // World step ran: creature frame_index advanced from 3→4.
        let surface = surface.borrow();
        let (_, entity) = surface.world.iter().next().unwrap();
        let SurfaceEntityKind::LiveCreature { frame_index, .. } = &entity.kind else {
            panic!("expected live creature");
        };
        assert_eq!(*frame_index, 4, "creature frame_index must advance");

        // No hazard spawns because chances are all zero.
        assert!(
            result.hazard_spawns.is_empty(),
            "no hazards should spawn with zero chances"
        );
    }
}
