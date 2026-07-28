//! Safe generational storage for deterministic surface entities.

use super::creatures::CreatureKind;
use super::hazards::HazardKind;
use super::model::SurfacePoint;
use super::simulation::Shot;

/// Stable entity identity. Reusing a slot changes its generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SurfaceEntityId {
    slot: u32,
    generation: u32,
}

/// Complete classification of an active surface entity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SurfaceEntityKind {
    MineralNode {
        category: usize,
        amount: u16,
    },
    EnergyNode {
        node: u8,
    },
    LiveCreature {
        kind: CreatureKind,
        hit_points: u8,
        aware: bool,
    },
    CannedCreature {
        value: u16,
    },
    Shot(Shot),
    Hazard(HazardKind),
    Explosion,
}

/// Entity state owned by a PlanetSide session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SurfaceEntity {
    pub kind: SurfaceEntityKind,
    pub position: SurfacePoint,
    pub finite_life: Option<u16>,
}

#[derive(Debug, Clone)]
struct Slot {
    generation: u32,
    entity: Option<SurfaceEntity>,
}

/// Deterministic entity world independent of the legacy C ELEMENT queue.
#[derive(Debug, Clone, Default)]
pub struct SurfaceWorld {
    slots: Vec<Slot>,
    free: Vec<u32>,
    order: Vec<SurfaceEntityId>,
}

impl SurfaceWorld {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, entity: SurfaceEntity) -> SurfaceEntityId {
        let id = if let Some(slot_index) = self.free.pop() {
            let slot = &mut self.slots[slot_index as usize];
            slot.entity = Some(entity);
            SurfaceEntityId {
                slot: slot_index,
                generation: slot.generation,
            }
        } else {
            let slot = self.slots.len() as u32;
            self.slots.push(Slot {
                generation: 0,
                entity: Some(entity),
            });
            SurfaceEntityId {
                slot,
                generation: 0,
            }
        };
        self.order.push(id);
        id
    }

    #[must_use]
    pub fn get(&self, id: SurfaceEntityId) -> Option<&SurfaceEntity> {
        self.slots
            .get(id.slot as usize)
            .filter(|slot| slot.generation == id.generation)
            .and_then(|slot| slot.entity.as_ref())
    }

    pub fn get_mut(&mut self, id: SurfaceEntityId) -> Option<&mut SurfaceEntity> {
        self.slots
            .get_mut(id.slot as usize)
            .filter(|slot| slot.generation == id.generation)
            .and_then(|slot| slot.entity.as_mut())
    }

    pub fn remove(&mut self, id: SurfaceEntityId) -> Option<SurfaceEntity> {
        let slot = self.slots.get_mut(id.slot as usize)?;
        if slot.generation != id.generation {
            return None;
        }
        let entity = slot.entity.take()?;
        slot.generation = slot.generation.wrapping_add(1);
        self.free.push(id.slot);
        self.order.retain(|candidate| *candidate != id);
        Some(entity)
    }

    pub fn iter(&self) -> impl Iterator<Item = (SurfaceEntityId, &SurfaceEntity)> {
        self.order
            .iter()
            .copied()
            .filter_map(|id| self.get(id).map(|entity| (id, entity)))
    }

    /// Decrement finite lifetimes in display order and remove expired entities.
    pub fn advance_lifetimes(&mut self) -> Vec<SurfaceEntityId> {
        let ids = self.order.clone();
        let mut expired = Vec::new();
        for id in ids {
            let Some(entity) = self.get_mut(id) else {
                continue;
            };
            let Some(life) = &mut entity.finite_life else {
                continue;
            };
            if *life > 0 {
                *life -= 1;
            }
            if *life == 0 {
                expired.push(id);
            }
        }
        for id in &expired {
            let _ = self.remove(*id);
        }
        expired
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.order.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.order.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mineral(amount: u16) -> SurfaceEntity {
        SurfaceEntity {
            kind: SurfaceEntityKind::MineralNode {
                category: 0,
                amount,
            },
            position: SurfacePoint::default(),
            finite_life: None,
        }
    }

    #[test]
    fn stale_id_cannot_access_reused_slot() {
        let mut world = SurfaceWorld::new();
        let old = world.insert(mineral(1));
        assert!(world.remove(old).is_some());
        let new = world.insert(mineral(2));
        assert_eq!(old.slot, new.slot);
        assert_ne!(old.generation, new.generation);
        assert!(world.get(old).is_none());
        assert!(matches!(
            world.get(new).map(|entity| &entity.kind),
            Some(SurfaceEntityKind::MineralNode { amount: 2, .. })
        ));
    }

    #[test]
    fn iteration_preserves_insertion_order_after_removal() {
        let mut world = SurfaceWorld::new();
        let one = world.insert(mineral(1));
        let two = world.insert(mineral(2));
        let three = world.insert(mineral(3));
        world.remove(two);
        assert_eq!(
            world.iter().map(|(id, _)| id).collect::<Vec<_>>(),
            [one, three]
        );
    }

    #[test]
    fn finite_lifetimes_expire_without_touching_persistent_nodes() {
        let mut world = SurfaceWorld::new();
        let persistent = world.insert(mineral(1));
        let temporary = world.insert(SurfaceEntity {
            kind: SurfaceEntityKind::Explosion,
            position: SurfacePoint::default(),
            finite_life: Some(2),
        });
        assert!(world.advance_lifetimes().is_empty());
        assert_eq!(world.advance_lifetimes(), [temporary]);
        assert!(world.get(persistent).is_some());
        assert!(world.get(temporary).is_none());
    }
}
