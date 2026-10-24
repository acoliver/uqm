//! Pixel-mask collision adapter over the Rust-owned surface entity world.

use std::collections::HashMap;

use super::assembly::{
    advance_creature_animation_frame, insert_surface_entity, remove_surface_entity,
    transform_creature_to_canned, SharedSurface, SurfaceAssembly, WorldVisualPort,
};
use super::cargo::CargoPickup;
use super::collision::{CollisionOutcome, CollisionRolls, LanderCollision};
use super::creatures::CreatureCatalog;
use super::entities::{SurfaceEntity, SurfaceEntityId, SurfaceEntityKind};
use super::generation::{persist_pickup, ScanPersistence, SurfaceGenerator};
use super::geometry::{masks_intersect, masks_intersect_wrapped, CollisionMask};
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
        crate::math::random()
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

    /// Number of registered entity masks.
    #[must_use]
    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.entities.len()
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

/// Publish the mineral deposits still on the surface for read-only automation.
fn publish_deposits(surface: &SurfaceAssembly, near: SurfacePoint) {
    let deposits = surface.world.iter().filter_map(|(_, entity)| {
        let SurfaceEntityKind::MineralNode { category, .. } = &entity.kind else {
            return None;
        };
        Some(super::telemetry::MineralDeposit {
            x: entity.position.x,
            y: entity.position.y,
            category: u8::try_from(*category).ok()?,
        })
    });
    super::telemetry::publish_deposits(deposits, near);
    super::telemetry::publish_energy(
        surface
            .world
            .iter()
            .filter_map(|(_, entity)| {
                if let SurfaceEntityKind::EnergyNode { node } = &entity.kind {
                    Some((*node, entity.position.x, entity.position.y))
                } else {
                    None
                }
            })
            .collect(),
    );
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
        publish_deposits(&surface, lander.position);
        let lander_mask = &surface.masks.lander[usize::from(lander.facing) % 16];
        let mut contacts = Vec::new();
        for (id, entity) in surface.world.iter() {
            let Some(entity_mask) = surface.masks.entities.get(&id) else {
                continue;
            };
            let wrapped = masks_intersect_wrapped(
                lander.position,
                lander_mask,
                entity.position,
                entity_mask,
                super::world::WORLD_WIDTH,
            );
            if !wrapped {
                continue;
            }
            let Some(collision) = classify(&entity.kind) else {
                continue;
            };
            if !masks_intersect(lander.position, lander_mask, entity.position, entity_mask) {
                super::telemetry::seam_hit();
            }
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
        // Count a mineral pickup only when a deposit was actually collected:
        // canned biological cargo, a full hold, and zero-amount outcomes are
        // not mineral pickups.
        if let CollisionOutcome::Cargo {
            pickup: CargoPickup::Collected { amount, .. },
            ..
        } = outcome
        {
            if matches!(contact.collision, LanderCollision::Mineral { .. }) && *amount > 0 {
                super::telemetry::mineral_pickup();
            }
        }
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
                let retrieved = result?;
                if retrieved {
                    super::telemetry::energy_retrieved(mapping.node.get());
                }
                retrieved
            }
            CollisionOutcome::NoEffect
            | CollisionOutcome::CrewDamage(_)
            | CollisionOutcome::Cargo {
                remove_node: false, ..
            } => false,
        };
        if should_remove {
            let mut surface = self.surface.borrow_mut();
            remove_surface_entity(&mut surface, entity_id)?;
            publish_deposits(&surface, super::telemetry::lander_position());
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
        // Play world sounds in source order. Every LanderHits cue is a
        // stun-bolt that connected with a live creature on a presented frame.
        //
        // The world-step carries an explicit hit verdict, so a hit on an
        // already-full slot (AnyCanned) does not double the prior lander-hit
        // when two bolts pass in the same frame.  Count each non-full verdict
        // once; the sound is played per cue as the C control does.
        for cue in &result.effects.sounds {
            audio.play(*cue)?;
        }
        for _ in &result.effects.landed_shot {
            super::telemetry::creature_hit();
        }

        // Apply living animation before the hit's canister transition.
        for (entity_id, kind, animation_frame) in &result.effects.creature_frames {
            advance_creature_animation_frame(
                &mut self.surface.borrow_mut(),
                *entity_id,
                *kind,
                *animation_frame,
                &mut self.world_visuals,
            )?;
        }
        // Persist dead-but-flashing nodes too, so takeoff cannot revive them.
        let pending: Vec<_> = {
            let surface = self.surface.borrow();
            surface
                .generated
                .iter()
                .filter_map(|entry| {
                    let entity = surface
                        .world
                        .get(entry.entity)
                        .expect("generated surface entity");
                    match entity.kind {
                        SurfaceEntityKind::LiveCreature {
                            kind,
                            hit_points: 0,
                            ..
                        } => Some((
                            entry.node,
                            entity.position,
                            u16::from(CreatureCatalog::stats(kind).value),
                        )),
                        _ => None,
                    }
                })
                .collect()
        };
        for (node, position, value) in pending {
            self.generator.can_node(node, position, value)?;
        }
        for (entity_id, animation_frame) in &result.effects.canister_frames {
            let visual = self
                .world_visuals
                .canned_creature_visual(*animation_frame)?;
            let mut surface = self.surface.borrow_mut();
            surface.frames.insert(*entity_id, visual.frame);
            surface.masks.insert_entity(*entity_id, visual.mask);
        }
        for (entity_id, value) in &result.effects.canned {
            let pending = {
                let surface = self.surface.borrow();
                surface
                    .generated
                    .iter()
                    .find(|entry| entry.entity == *entity_id)
                    .map(|entry| {
                        (
                            entry.node,
                            surface
                                .world
                                .get(*entity_id)
                                .expect("canned surface entity")
                                .position,
                        )
                    })
            };
            if let Some((node, position)) = pending {
                self.generator.can_node(node, position, *value)?;
            }
            let visual = self.world_visuals.canned_creature_visual(0)?;
            transform_creature_to_canned(
                &mut self.surface.borrow_mut(),
                *entity_id,
                *value,
                visual,
            )?;
        }

        // A stun bolt on a value-0 creature skips `shotCreature` and runs the
        // node pickup: persist the retrieved node, then drop the entity. A node
        // the callback declines stays on the surface, as in C.
        for entity_id in &result.effects.retrieved {
            let mapping = {
                let surface = self.surface.borrow();
                surface
                    .generated
                    .iter()
                    .find(|entry| entry.entity == *entity_id)
                    .copied()
            };
            let should_remove = match mapping {
                Some(mapping) => {
                    persist_pickup(&mut self.generator, &mut self.persistence, mapping)?
                }
                None => true,
            };
            if should_remove {
                remove_surface_entity(&mut self.surface.borrow_mut(), *entity_id)?;
            }
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
        SurfaceEntityKind::MineralNode {
            category, quantity, ..
        } => Some(LanderCollision::Mineral {
            category: *category,
            amount: *quantity,
        }),
        SurfaceEntityKind::EnergyNode { node } => Some(LanderCollision::Energy { node: *node }),
        SurfaceEntityKind::LiveCreature {
            kind,
            hit_points: 0,
            ..
        } => Some(LanderCollision::CannedBiological {
            value: u16::from(CreatureCatalog::stats(*kind).value),
        }),
        SurfaceEntityKind::LiveCreature { kind, .. } => Some(LanderCollision::LiveCreature {
            danger: CreatureCatalog::stats(*kind).danger,
        }),
        SurfaceEntityKind::CannedCreature { value, .. } => {
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

    /// Stands in for a selected-system `pickupEnergy` callback that requests
    /// takeoff, the way `GenerateSol_pickupEnergy` does for Luna's moon base.
    #[derive(Default)]
    struct TakeoffRequestingGenerator;

    impl SurfaceGenerator for TakeoffRequestingGenerator {
        fn can_node(
            &mut self,
            _: crate::planet_side::generation::ScanNodeId,
            _: crate::planet_side::model::SurfacePoint,
            _: u16,
        ) -> Result<(), AdapterError> {
            Ok(())
        }

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
            super::super::special_effects::set_lander_takeoff();
            Ok(true)
        }
    }

    #[derive(Default)]
    struct Generator;

    impl SurfaceGenerator for Generator {
        fn can_node(
            &mut self,
            _: crate::planet_side::generation::ScanNodeId,
            _: crate::planet_side::model::SurfacePoint,
            _: u16,
        ) -> Result<(), AdapterError> {
            Ok(())
        }

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

    /// The telemetry counters are process-global; each test takes the trip mutex.
    fn telemetry_guard() -> std::sync::MutexGuard<'static, ()> {
        super::super::telemetry::tests::exclusive()
    }

    /// A shared surface with one entity registered in all its registries.
    fn shared_surface_with_entity(entity: SurfaceEntity) -> SharedSurface {
        let mut world = SurfaceWorld::new();
        let id = world.insert(entity);
        let mut masks = SurfaceMasks::new((0..16).map(|_| solid()).collect()).unwrap();
        masks.insert_entity(id, solid());
        super::super::assembly::share_surface(super::super::assembly::SurfaceAssembly {
            world,
            generated: Vec::new(),
            frames: super::super::graphics_adapter::SurfaceFrameRegistry::default(),
            masks,
        })
    }

    /// A collision adapter whose world has a single registered entity.
    fn test_adapter(
        surface: SharedSurface,
    ) -> SurfaceCollisionAdapter<Random, Generator, TestWorldVisuals> {
        SurfaceCollisionAdapter {
            surface,
            random: Random {
                values: Default::default(),
                calls: 0,
            },
            generator: Generator,
            persistence: ScanPersistence::default(),
            world_visuals: TestWorldVisuals,
            earthquake_frame_count: 13,
            lava_frame_count: 7,
        }
    }

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
            animation_frame: u16,
        ) -> Result<EntityVisual, AdapterError> {
            Ok(EntityVisual {
                frame: SurfaceFrame {
                    base: 2_usize as *mut _,
                    index: animation_frame,
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

        fn creature_animation_visual(
            &mut self,
            _kind: super::super::creatures::CreatureKind,
            animation_frame: u16,
        ) -> Result<EntityVisual, AdapterError> {
            Ok(EntityVisual {
                frame: SurfaceFrame {
                    base: std::ptr::NonNull::<u8>::dangling().as_ptr().cast(),
                    index: animation_frame,
                },
                mask: solid(),
            })
        }
    }

    #[test]
    fn stunned_creature_keeps_canister_visual_and_yields_bio_on_pickup() {
        let _guard = telemetry_guard();
        let mut creature = brainbox_entity(1);
        if let SurfaceEntityKind::LiveCreature {
            kind, frame_index, ..
        } = &mut creature.kind
        {
            *kind = super::super::creatures::CreatureKind::new(0).unwrap();
            *frame_index = 2;
        }
        let surface = shared_surface_with_entity(creature);
        let id = surface.borrow().world.ids()[0];
        let mut adapter = test_adapter(surface.clone());
        adapter
            .register_shot(Shot {
                position: SurfacePoint::default(),
                facing: 0,
                velocity_x: 0,
                velocity_y: 0,
                life: 1,
            })
            .unwrap();
        for _ in 0..8 {
            let result = adapter
                .step_world(&lander(), HazardChances::default())
                .unwrap();
            adapter
                .apply_world_step(&result, &mut RecordingAudio)
                .unwrap();
            assert_eq!(
                surface.borrow().frames.get(id).unwrap().base,
                2_usize as *mut _,
                "living animation must never overwrite the canister sprite"
            );
        }
        let start = surface.borrow().world.get(id).unwrap().position;
        let result = adapter
            .step_world(&lander(), HazardChances::default())
            .unwrap();
        adapter
            .apply_world_step(&result, &mut RecordingAudio)
            .unwrap();
        assert_eq!(surface.borrow().world.get(id).unwrap().position, start);
        let contact = adapter
            .contacts(&lander())
            .unwrap()
            .into_iter()
            .next()
            .unwrap();
        assert_eq!(
            contact.collision,
            LanderCollision::CannedBiological { value: 1 }
        );
        let mut crew = CrewCount::new(12);
        let mut minerals = super::super::cargo::MineralCargo::new(100, 0, false);
        let mut biological = super::super::cargo::BioCargo::default();
        let outcome = super::super::collision::resolve_lander_collision(
            &mut super::super::collision::CollisionState {
                crew: &mut crew,
                shields: Default::default(),
                minerals: &mut minerals,
                biological: &mut biological,
            },
            contact.collision,
            contact.rolls,
        );
        adapter.commit(contact, &outcome, 12).unwrap();
        assert_eq!(biological.level(), 1);
        assert!(surface.borrow().world.get(id).is_none());
        assert!(surface.borrow().frames.get(id).is_none());
    }

    struct PersistentGenerator {
        files: std::rc::Rc<std::cell::RefCell<crate::state::state_file::StateFileManager>>,
    }
    impl SurfaceGenerator for PersistentGenerator {
        fn node_count(
            &mut self,
            scan: super::super::generation::ScanType,
        ) -> Result<u8, AdapterError> {
            Ok(u8::from(
                scan == super::super::generation::ScanType::Biological,
            ))
        }
        fn generate(
            &mut self,
            _: super::super::generation::ScanType,
            node: super::super::generation::ScanNodeId,
        ) -> Result<super::super::generation::GeneratedNode, AdapterError> {
            use super::super::generation::*;
            Ok(
                match self.files.borrow().canned_node((1, 2, 1, node.get())) {
                    Some(canned) => GeneratedNode::from_canister(canned),
                    None => GeneratedNode {
                        position: SurfacePoint::default(),
                        kind: GeneratedNodeKind::Biological {
                            creature: super::super::creatures::CreatureKind::new(0).unwrap(),
                            hit_points: 1,
                            initial_frame: 0,
                        },
                    },
                },
            )
        }
        fn pickup(
            &mut self,
            _: super::super::generation::ScanType,
            node: super::super::generation::ScanNodeId,
        ) -> Result<bool, AdapterError> {
            self.files
                .borrow_mut()
                .remove_canned_node((1, 2, 1, node.get()));
            Ok(true)
        }
        fn can_node(
            &mut self,
            node: super::super::generation::ScanNodeId,
            position: SurfacePoint,
            value: u16,
        ) -> Result<(), AdapterError> {
            self.files
                .borrow_mut()
                .can_node(crate::state::planet_info::CannedNode {
                    star: 1,
                    planet: 2,
                    moon: 1,
                    node: node.get(),
                    value: value as u8,
                    x: position.x as i16,
                    y: position.y as i16,
                });
            Ok(())
        }
    }

    #[test]
    fn lethal_shot_survives_relanding_save_load_and_collection_retires_node() {
        use super::super::generation::*;
        use crate::state::state_file::StateFileManager;
        let _guard = telemetry_guard();
        let files = std::rc::Rc::new(std::cell::RefCell::new(StateFileManager::new()));
        let mut generator = PersistentGenerator {
            files: files.clone(),
        };
        let mut world = SurfaceWorld::new();
        let generated =
            populate_surface(&mut generator, ScanPersistence::default(), &mut world).unwrap();
        let id = generated[0].entity;
        let mut masks = SurfaceMasks::new((0..16).map(|_| solid()).collect()).unwrap();
        masks.insert_entity(id, solid());
        let surface = super::super::assembly::share_surface(SurfaceAssembly {
            world,
            generated,
            masks,
            frames: Default::default(),
        });
        let mut adapter = SurfaceCollisionAdapter {
            surface: surface.clone(),
            generator,
            random: Random {
                values: Default::default(),
                calls: 0,
            },
            persistence: ScanPersistence::default(),
            world_visuals: TestWorldVisuals,
            earthquake_frame_count: 13,
            lava_frame_count: 7,
        };
        adapter
            .register_shot(Shot {
                position: SurfacePoint::default(),
                facing: 0,
                velocity_x: 0,
                velocity_y: 0,
                life: 1,
            })
            .unwrap();
        let result = adapter
            .step_world(&lander(), HazardChances::default())
            .unwrap();
        adapter
            .apply_world_step(&result, &mut RecordingAudio)
            .unwrap();
        // Departure during the damage flash must already be persistent.
        let canned = files.borrow().canned_node((1, 2, 1, 0)).unwrap();
        assert_eq!(canned.value, 1);
        assert!(files.borrow().canned_node((1, 2, 0, 0)).is_none());
        let mut save = crate::savegame::decode_save(include_bytes!(
            "../../tests/fixtures/savegame/legacy-save.00"
        ))
        .unwrap();
        assert!(save.canned_nodes.is_empty());
        save.canned_nodes.push(canned);
        let loaded =
            crate::savegame::decode_save(&crate::savegame::encode_save(&save).unwrap()).unwrap();
        let files = std::rc::Rc::new(std::cell::RefCell::new(StateFileManager::new()));
        for node in loaded.canned_nodes {
            files.borrow_mut().can_node(node);
        }
        let mut generator = PersistentGenerator {
            files: files.clone(),
        };
        let mut world = SurfaceWorld::new();
        let generated =
            populate_surface(&mut generator, ScanPersistence::default(), &mut world).unwrap();
        let id = generated[0].entity;
        assert!(matches!(
            world.get(id).unwrap().kind,
            SurfaceEntityKind::CannedCreature { value: 1, .. }
        ));
        let position = world.get(id).unwrap().position;
        let mut masks = SurfaceMasks::new((0..16).map(|_| solid()).collect()).unwrap();
        masks.insert_entity(id, solid());
        let surface = super::super::assembly::share_surface(SurfaceAssembly {
            world,
            generated,
            masks,
            frames: Default::default(),
        });
        let mut adapter = SurfaceCollisionAdapter {
            surface: surface.clone(),
            generator,
            random: Random {
                values: Default::default(),
                calls: 0,
            },
            persistence: ScanPersistence::default(),
            world_visuals: TestWorldVisuals,
            earthquake_frame_count: 13,
            lava_frame_count: 7,
        };
        for _ in 0..20 {
            let result = adapter
                .step_world(&lander(), HazardChances::default())
                .unwrap();
            adapter
                .apply_world_step(&result, &mut RecordingAudio)
                .unwrap();
            assert_eq!(surface.borrow().world.get(id).unwrap().position, position);
        }
        let contact = adapter.contacts(&lander()).unwrap()[0];
        let mut crew = CrewCount::new(12);
        let mut minerals = super::super::cargo::MineralCargo::new(100, 0, false);
        let mut biological = super::super::cargo::BioCargo::default();
        let outcome = super::super::collision::resolve_lander_collision(
            &mut super::super::collision::CollisionState {
                crew: &mut crew,
                shields: Default::default(),
                minerals: &mut minerals,
                biological: &mut biological,
            },
            contact.collision,
            contact.rolls,
        );
        adapter.commit(contact, &outcome, 12).unwrap();
        assert_eq!(biological.level(), 1);
        assert!(adapter
            .persistence
            .is_retrieved(ScanType::Biological, ScanNodeId::new(0).unwrap()));
        assert!(files.borrow().canned_node(canned.key()).is_none());
        assert!(surface.borrow().world.is_empty());
        let persistence = adapter.persistence;
        let mut generator = PersistentGenerator { files };
        assert!(
            populate_surface(&mut generator, persistence, &mut SurfaceWorld::new())
                .unwrap()
                .is_empty()
        );
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
                frame_index: 1,
                animation_wait: 4,
                damage_wait: 0,
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

    fn brainbox_entity(hit_points: u8) -> SurfaceEntity {
        SurfaceEntity {
            kind: SurfaceEntityKind::LiveCreature {
                kind: super::super::creatures::CreatureKind::new(24).unwrap(),
                hit_points,
                aware: false,
                velocity: crate::battle::velocity::VelocityDesc::new(),
                thrust_wait: 0,
                frame_index: 0,
                animation_wait: 4,
                damage_wait: 0,
            },
            position: SurfacePoint::default(),
            finite_life: None,
        }
    }

    struct RecordingAudio;

    impl super::super::runtime::PlanetSideAudio for RecordingAudio {
        fn play(&mut self, _cue: SoundCue) -> Result<(), AdapterError> {
            Ok(())
        }
    }

    /// C `lander.c` creature table: Brainbox Bulldozer is `DANGER_HARMLESS`, and
    /// the contact roll `TFB_Random() % 128 < danger_vals[0]` can never succeed,
    /// yet the random value is still drawn on every contact frame.
    #[test]
    fn brainbox_contact_never_bites_but_still_draws_one_random_per_frame() {
        use super::super::collision::{resolve_lander_collision, CollisionState};
        use super::super::model::ShieldSet;

        let surface = shared_surface_with_entity(brainbox_entity(1));
        let mut adapter = test_adapter(surface);
        let mut crew = CrewCount::new(12);
        let mut minerals = super::super::cargo::MineralCargo::new(100, 0, false);
        let mut biological = super::super::cargo::BioCargo::default();
        const FRAMES: usize = 64;
        for _ in 0..FRAMES {
            // A roll of 0 would bite at every danger level above Harmless.
            adapter.random.values.extend([0, 0]);
            let contacts = adapter.contacts(&lander()).unwrap();
            let [contact] = contacts.as_slice() else {
                panic!("expected exactly one contact");
            };
            resolve_lander_collision(
                &mut CollisionState {
                    crew: &mut crew,
                    shields: ShieldSet::default(),
                    minerals: &mut minerals,
                    biological: &mut biological,
                },
                contact.collision,
                contact.rolls,
            );
        }
        assert_eq!(crew, CrewCount::new(12), "a Brainbox must never cost crew");
        assert_eq!(
            adapter.random.calls, FRAMES,
            "one random draw per contact frame, no shield draw"
        );
    }

    /// C `lander.c` ~786-803: a stun bolt on a creature whose catalog value is 0
    /// skips `shotCreature`, so the first bolt retires the node through
    /// `callPickupForScanType`/`set_node_retrieved` whatever its hit points were.
    #[test]
    fn first_stun_bolt_retires_a_multi_hit_point_brainbox_node() {
        use super::super::generation::{GeneratedEntity, ScanNodeId, ScanType};

        let _telemetry = telemetry_guard();
        let mut world = SurfaceWorld::new();
        let id = world.insert(brainbox_entity(3));
        let mut masks = SurfaceMasks::new((0..16).map(|_| solid()).collect()).unwrap();
        masks.insert_entity(id, solid());
        let node = ScanNodeId::new(4).unwrap();
        let surface =
            super::super::assembly::share_surface(super::super::assembly::SurfaceAssembly {
                world,
                generated: vec![GeneratedEntity {
                    entity: id,
                    scan: ScanType::Biological,
                    node,
                }],
                frames: super::super::graphics_adapter::SurfaceFrameRegistry::default(),
                masks,
            });
        let mut adapter = test_adapter(surface.clone());
        adapter
            .register_shot(Shot {
                position: SurfacePoint::default(),
                facing: 0,
                velocity_x: 0,
                velocity_y: 0,
                life: 12,
            })
            .unwrap();
        let before = super::super::telemetry::observation();

        let result = adapter
            .step_world(&lander(), super::super::world::HazardChances::default())
            .unwrap();
        adapter
            .apply_world_step(&result, &mut RecordingAudio)
            .unwrap();

        assert!(
            adapter.persistence.is_retrieved(ScanType::Biological, node),
            "the Brainbox node must be recorded as retrieved"
        );
        let surface = surface.borrow();
        assert!(
            surface.world.get(id).is_none(),
            "the Brainbox must be removed, not canned"
        );
        assert!(
            surface.generated.is_empty(),
            "the retired node mapping must be dropped"
        );
        assert!(
            !result.effects.sounds.contains(&SoundCue::LanderHits),
            "shotCreature, which plays LANDER_HITS, is not reached for a value-0 creature"
        );
        assert_eq!(
            super::super::telemetry::observation().creature_hits,
            before.creature_hits + 1,
            "the bolt still counts as a creature hit"
        );
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
                size: 1,
                quantity: 1,
            },
            position: SurfacePoint::default(),
            finite_life: None,
        });
        world.insert(SurfaceEntity {
            kind: SurfaceEntityKind::MineralNode {
                category: 1,
                size: 2,
                quantity: 2,
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
                animation_wait: 0,
                damage_wait: 0,
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
            "no hazard should spawn with zero chances"
        );
    }

    #[test]
    fn energy_pickup_reports_the_takeoff_its_callback_requested() {
        // Luna's moon base is an energy node whose selected-system callback
        // calls set_lander_takeoff() before returning "picked up". The adapter
        // must surface that request, otherwise the lander never lifts off.
        let mut world = SurfaceWorld::new();
        let id = world.insert(SurfaceEntity {
            kind: SurfaceEntityKind::EnergyNode { node: 0 },
            position: SurfacePoint::default(),
            finite_life: None,
        });
        let mut masks = SurfaceMasks::new((0..16).map(|_| solid()).collect()).unwrap();
        masks.insert_entity(id, solid());
        let surface =
            super::super::assembly::share_surface(super::super::assembly::SurfaceAssembly {
                world,
                generated: vec![super::super::generation::GeneratedEntity {
                    entity: id,
                    scan: super::super::generation::ScanType::Energy,
                    node: super::super::generation::ScanNodeId::new(0).unwrap(),
                }],
                frames: super::super::graphics_adapter::SurfaceFrameRegistry::default(),
                masks,
            });
        let mut adapter = SurfaceCollisionAdapter {
            surface,
            random: Random {
                values: Default::default(),
                calls: 0,
            },
            generator: TakeoffRequestingGenerator,
            persistence: ScanPersistence::default(),
            world_visuals: TestWorldVisuals,
            earthquake_frame_count: 13,
            lava_frame_count: 7,
        };

        let contacts = adapter.contacts(&lander()).unwrap();
        let [contact] = contacts.as_slice() else {
            panic!("expected exactly one energy contact");
        };
        let effects = adapter
            .commit(
                *contact,
                &CollisionOutcome::EnergyPickupRequested { node: 0 },
                12,
            )
            .unwrap();

        assert!(
            effects.takeoff_requested,
            "takeoff requested by the pickup callback must reach the session"
        );
    }

    #[test]
    fn commit_counts_a_collected_mineral_pickup() {
        let _guard = telemetry_guard();
        let surface = shared_surface_with_entity(SurfaceEntity {
            kind: SurfaceEntityKind::MineralNode {
                category: 0,
                size: 1,
                quantity: 4,
            },
            position: SurfacePoint::default(),
            finite_life: None,
        });
        let mut adapter = test_adapter(surface);
        let contacts = adapter.contacts(&lander()).unwrap();
        let [contact] = contacts.as_slice() else {
            panic!("expected exactly one mineral contact");
        };
        let outcome = CollisionOutcome::Cargo {
            pickup: CargoPickup::Collected {
                amount: 4,
                complete: true,
            },
            remove_node: false,
        };
        let before = crate::planet_side::telemetry::observation();
        adapter.commit(*contact, &outcome, 12).unwrap();
        assert_eq!(
            crate::planet_side::telemetry::observation().mineral_pickups,
            before.mineral_pickups + 1,
            "a collected mineral increments the pickup counter"
        );
    }

    #[test]
    fn commit_skips_full_or_zero_mineral_pickups() {
        let _guard = telemetry_guard();
        let surface = shared_surface_with_entity(SurfaceEntity {
            kind: SurfaceEntityKind::MineralNode {
                category: 0,
                size: 1,
                quantity: 0,
            },
            position: SurfacePoint::default(),
            finite_life: None,
        });
        let mut adapter = test_adapter(surface);
        let contacts = adapter.contacts(&lander()).unwrap();
        let [contact] = contacts.as_slice() else {
            panic!("expected exactly one mineral contact");
        };
        for outcome in [
            CollisionOutcome::Cargo {
                pickup: CargoPickup::Full,
                remove_node: false,
            },
            CollisionOutcome::Cargo {
                pickup: CargoPickup::Collected {
                    amount: 0,
                    complete: true,
                },
                remove_node: false,
            },
        ] {
            let before = crate::planet_side::telemetry::observation();
            adapter.commit(*contact, &outcome, 12).unwrap();
            assert_eq!(
                crate::planet_side::telemetry::observation().mineral_pickups,
                before.mineral_pickups,
                "a full or zero pickup is not a collected mineral"
            );
        }
    }

    #[test]
    fn commit_does_not_count_canned_biological_cargo() {
        let _guard = telemetry_guard();
        let surface = shared_surface_with_entity(SurfaceEntity {
            kind: SurfaceEntityKind::CannedCreature {
                value: 7,
                animation_wait: 4,
                frame_index: 0,
            },
            position: SurfacePoint::default(),
            finite_life: None,
        });
        let mut adapter = test_adapter(surface);
        let contacts = adapter.contacts(&lander()).unwrap();
        let [contact] = contacts.as_slice() else {
            panic!("expected exactly one canned-biological contact");
        };
        let outcome = CollisionOutcome::Cargo {
            pickup: CargoPickup::Collected {
                amount: 7,
                complete: true,
            },
            remove_node: false,
        };
        let before = crate::planet_side::telemetry::observation();
        adapter.commit(*contact, &outcome, 12).unwrap();
        assert_eq!(
            crate::planet_side::telemetry::observation().mineral_pickups,
            before.mineral_pickups,
            "biological cargo is not a mineral pickup"
        );
    }

    #[test]
    fn contacts_does_not_count_wrapped_overlap_of_an_unclassifiable_entity() {
        let _guard = telemetry_guard();
        // A shot overlaps the lander only through a wrapped copy; because a
        // shot is unclassifiable it must not count as a seam hit.
        let surface = shared_surface_with_entity(SurfaceEntity {
            kind: SurfaceEntityKind::Shot(super::super::simulation::Shot {
                position: SurfacePoint { x: 4, y: 0 },
                facing: 0,
                velocity_x: 0,
                velocity_y: 0,
                life: 12,
            }),
            position: SurfacePoint {
                x: super::super::world::WORLD_WIDTH,
                y: 0,
            },
            finite_life: Some(12),
        });
        let mut adapter = test_adapter(surface);
        let before = crate::planet_side::telemetry::observation();
        let contacts = adapter.contacts(&lander()).unwrap();
        assert!(contacts.is_empty(), "shots never produce a lander contact");
        assert_eq!(
            crate::planet_side::telemetry::observation().seam_hits,
            before.seam_hits,
            "an unclassified wrapped overlap must not count as a seam hit"
        );
    }

    #[test]
    fn contacts_counts_the_wrapped_overlap_of_a_classified_entity() {
        let _guard = telemetry_guard();
        // The deposit's raw position touches only across the seam.
        let surface = shared_surface_with_entity(SurfaceEntity {
            kind: SurfaceEntityKind::MineralNode {
                category: 3,
                size: 1,
                quantity: 4,
            },
            position: SurfacePoint {
                x: super::super::world::WORLD_WIDTH,
                y: 0,
            },
            finite_life: None,
        });
        let mut adapter = test_adapter(surface);
        let before = crate::planet_side::telemetry::observation();
        let contacts = adapter.contacts(&lander()).unwrap();
        assert_eq!(contacts.len(), 1, "the wrapped copy of a mineral connects");
        assert_eq!(
            crate::planet_side::telemetry::observation().seam_hits,
            before.seam_hits + 1,
            "a classified wrapped-only contact counts as a seam hit"
        );
    }
}
