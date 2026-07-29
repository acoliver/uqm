//! Assembly of generated nodes into one Rust-owned surface runtime.

use std::cell::RefCell;
use std::rc::Rc;

use super::collision_adapter::SurfaceMasks;
use super::entities::{SurfaceEntity, SurfaceEntityId, SurfaceWorld};
use super::generation::{populate_surface, GeneratedEntity, ScanPersistence, SurfaceGenerator};
use super::geometry::CollisionMask;
use super::graphics_adapter::{SurfaceFrame, SurfaceFrameRegistry};
use super::runtime::AdapterError;

/// Visual data required by both drawing and pixel collision.
#[derive(Clone)]
pub struct EntityVisual {
    pub frame: SurfaceFrame,
    pub mask: CollisionMask,
}

/// Production-specific visual selection remains outside deterministic entities.
pub trait SurfaceVisualPort {
    fn visual_for(
        &mut self,
        generated: GeneratedEntity,
        entity: &SurfaceEntity,
    ) -> Result<EntityVisual, AdapterError>;
}

/// Visual selection for dynamically-spawned world entities (shots, hazards,
/// canned creatures).
///
/// Each method returns the [`EntityVisual`] (frame + collision mask) that the
/// world step should register in the surface assembly's frame and mask maps.
pub trait WorldVisualPort {
    /// Visual for a lander stun-bolt shot.
    fn shot_visual(&mut self, facing: u8) -> Result<EntityVisual, AdapterError>;

    /// Visual for a creature that was canned (hit points reached zero).
    fn canned_creature_visual(
        &mut self,
        kind: super::creatures::CreatureKind,
    ) -> Result<EntityVisual, AdapterError>;

    /// Visual for a spawned hazard (earthquake, lightning, lava).
    fn hazard_visual(
        &mut self,
        kind: super::hazards::HazardKind,
    ) -> Result<EntityVisual, AdapterError>;
}

/// Complete Rust ownership assembled before the active frame loop starts.
pub struct SurfaceAssembly {
    pub world: SurfaceWorld,
    pub generated: Vec<GeneratedEntity>,
    pub frames: SurfaceFrameRegistry,
    pub masks: SurfaceMasks,
}

/// Single-threaded shared ownership used by concrete graphics and interaction
/// adapters during one synchronous PlanetSide session.
pub type SharedSurface = Rc<RefCell<SurfaceAssembly>>;

#[must_use]
pub fn share_surface(assembly: SurfaceAssembly) -> SharedSurface {
    Rc::new(RefCell::new(assembly))
}

pub fn assemble_surface(
    generator: &mut impl SurfaceGenerator,
    persistence: ScanPersistence,
    visuals: &mut impl SurfaceVisualPort,
    lander_masks: Vec<CollisionMask>,
) -> Result<SurfaceAssembly, AdapterError> {
    let mut world = SurfaceWorld::new();
    let generated = populate_surface(generator, persistence, &mut world)?;
    let mut frames = SurfaceFrameRegistry::default();
    let mut masks = SurfaceMasks::new(lander_masks)?;

    for mapping in &generated {
        let entity = world
            .get(mapping.entity)
            .ok_or(AdapterError::new("generated_entity_missing"))?;
        let visual = visuals.visual_for(*mapping, entity)?;
        frames.insert(mapping.entity, visual.frame);
        masks.insert_entity(mapping.entity, visual.mask);
    }

    Ok(SurfaceAssembly {
        world,
        generated,
        frames,
        masks,
    })
}

/// Remove one retrieved entity from every runtime registry atomically.
pub fn remove_surface_entity(
    assembly: &mut SurfaceAssembly,
    entity: SurfaceEntityId,
) -> Result<SurfaceEntity, AdapterError> {
    let removed = assembly
        .world
        .remove(entity)
        .ok_or(AdapterError::new("surface_entity_missing"))?;
    assembly.frames.remove(entity);
    assembly.masks.remove_entity(entity);
    assembly.generated.retain(|entry| entry.entity != entity);
    Ok(removed)
}

/// Insert a dynamically-spawned entity (shot, hazard, canned creature) into all
/// runtime registries atomically.
///
/// The visual port supplies the frame and collision mask. Returns the new
/// entity ID.
pub fn insert_surface_entity(
    assembly: &mut SurfaceAssembly,
    entity: SurfaceEntity,
    visual: EntityVisual,
) -> SurfaceEntityId {
    let id = assembly.world.insert(entity);
    assembly.frames.insert(id, visual.frame);
    assembly.masks.insert_entity(id, visual.mask);
    id
}

/// Transform a live creature into a canned creature, updating its visual in all
/// registries atomically.
///
/// The creature's hit points have already reached zero. The visual port selects
/// the canned-creature frame. The entity's position is preserved.
pub fn transform_creature_to_canned(
    assembly: &mut SurfaceAssembly,
    entity: SurfaceEntityId,
    value: u16,
    visual: EntityVisual,
) -> Result<(), AdapterError> {
    let Some(target) = assembly.world.get_mut(entity) else {
        return Err(AdapterError::new("canned_entity_missing"));
    };
    target.kind = super::entities::SurfaceEntityKind::CannedCreature { value };
    assembly.frames.insert(entity, visual.frame);
    assembly.masks.insert_entity(entity, visual.mask);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::planet_side::generation::{GeneratedNode, GeneratedNodeKind, ScanNodeId, ScanType};
    use crate::planet_side::model::SurfacePoint;

    struct Generator;
    impl SurfaceGenerator for Generator {
        fn node_count(&mut self, scan: ScanType) -> Result<u8, AdapterError> {
            Ok(u8::from(scan == ScanType::Mineral))
        }
        fn generate(
            &mut self,
            _scan: ScanType,
            _node: ScanNodeId,
        ) -> Result<GeneratedNode, AdapterError> {
            Ok(GeneratedNode {
                position: SurfacePoint { x: 4, y: 8 },
                kind: GeneratedNodeKind::Mineral {
                    category: 2,
                    amount: 3,
                },
            })
        }
        fn pickup(&mut self, _scan: ScanType, _node: ScanNodeId) -> Result<bool, AdapterError> {
            Ok(true)
        }
    }

    struct Visuals;
    impl SurfaceVisualPort for Visuals {
        fn visual_for(
            &mut self,
            _generated: GeneratedEntity,
            _entity: &SurfaceEntity,
        ) -> Result<EntityVisual, AdapterError> {
            Ok(EntityVisual {
                frame: SurfaceFrame {
                    base: std::ptr::NonNull::<u8>::dangling().as_ptr().cast(),
                    index: 7,
                },
                mask: solid(),
            })
        }
    }

    fn solid() -> CollisionMask {
        CollisionMask::from_occupancy(1, 1, SurfacePoint::default(), &[1]).unwrap()
    }

    #[test]
    fn assembly_keeps_world_frames_masks_and_generation_mapping_together() {
        let mut generator = Generator;
        let mut visuals = Visuals;
        let mut assembly = assemble_surface(
            &mut generator,
            ScanPersistence::default(),
            &mut visuals,
            (0..16).map(|_| solid()).collect(),
        )
        .unwrap();
        assert_eq!(assembly.world.len(), 1);
        assert_eq!(assembly.generated.len(), 1);
        let id = assembly.generated[0].entity;
        assert_eq!(assembly.frames.get(id).map(|frame| frame.index), Some(7));

        let removed = remove_surface_entity(&mut assembly, id).unwrap();
        assert_eq!(removed.position, SurfacePoint { x: 4, y: 8 });
        assert!(assembly.world.is_empty());
        assert!(assembly.generated.is_empty());
        assert!(assembly.frames.get(id).is_none());
    }

    #[test]
    fn invalid_lander_mask_set_stops_assembly_before_visual_selection() {
        let mut generator = Generator;
        let mut visuals = Visuals;
        assert!(matches!(
            assemble_surface(
                &mut generator,
                ScanPersistence::default(),
                &mut visuals,
                vec![solid()]
            ),
            Err(AdapterError {
                operation: "lander_collision_masks"
            })
        ));
    }
}
