//! Typed surface-generation and scan-persistence boundary.

use crate::battle::velocity::VelocityDesc;

use super::creatures::CreatureKind;
use super::entities::{SurfaceEntity, SurfaceEntityId, SurfaceEntityKind, SurfaceWorld};
use super::model::SurfacePoint;
use super::runtime::AdapterError;

pub const MAX_SCAN_NODES: u8 = 32;

/// Scan type ordering preserved from `planets.h`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum ScanType {
    Mineral = 0,
    Energy = 1,
    Biological = 2,
}

/// Validated identity of a scan node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ScanNodeId(u8);

impl ScanNodeId {
    pub fn new(node: u8) -> Result<Self, GenerationError> {
        if node < MAX_SCAN_NODES {
            Ok(Self(node))
        } else {
            Err(GenerationError::NodeOutOfRange(node))
        }
    }

    #[must_use]
    pub const fn get(self) -> u8 {
        self.0
    }
}

/// Retrieval bits for one scan category.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RetrievalMask(u32);

impl RetrievalMask {
    #[must_use]
    pub fn contains(self, node: ScanNodeId) -> bool {
        self.0 & (1_u32 << node.get()) != 0
    }

    pub fn insert(&mut self, node: ScanNodeId) {
        self.0 |= 1_u32 << node.get();
    }

    #[must_use]
    pub const fn bits(self) -> u32 {
        self.0
    }
}

/// Complete persisted retrieval state for one world.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ScanPersistence {
    masks: [RetrievalMask; 3],
}

impl ScanPersistence {
    #[must_use]
    pub const fn from_masks(masks: [u32; 3]) -> Self {
        Self {
            masks: [
                RetrievalMask(masks[0]),
                RetrievalMask(masks[1]),
                RetrievalMask(masks[2]),
            ],
        }
    }

    #[must_use]
    pub const fn to_masks(self) -> [u32; 3] {
        [
            self.masks[0].bits(),
            self.masks[1].bits(),
            self.masks[2].bits(),
        ]
    }

    #[must_use]
    pub fn is_retrieved(self, scan: ScanType, node: ScanNodeId) -> bool {
        self.masks[scan as usize].contains(node)
    }

    pub fn mark_retrieved(&mut self, scan: ScanType, node: ScanNodeId) {
        self.masks[scan as usize].insert(node);
    }

    #[must_use]
    pub fn mask(self, scan: ScanType) -> u32 {
        self.masks[scan as usize].bits()
    }
}

/// Data returned by the selected system's generation implementation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeneratedNodeKind {
    Mineral {
        category: usize,
        gross_size: u16,
        fine_quantity: u16,
    },
    Energy,
    Biological {
        creature: CreatureKind,
        hit_points: u8,
        /// Normalized 0..3 index into the 4-frame life animation, chosen at
        /// node-generation time, mirroring the legacy `generateBioNode` stamp
        /// `SetAbsFrameIndex(life_form, (COUNT)TFB_Random())` on the 4-frame
        /// `lifea.ani`..`lifez.ani`.  The first world step then advances this
        /// frame ordinarily (N -> N + 1).
        initial_frame: u8,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GeneratedNode {
    pub position: SurfacePoint,
    pub kind: GeneratedNodeKind,
}

/// Special-system generation and pickup hooks selected by `GenerateFunctions`.
pub trait SurfaceGenerator {
    fn node_count(&mut self, scan: ScanType) -> Result<u8, AdapterError>;
    fn generate(&mut self, scan: ScanType, node: ScanNodeId)
        -> Result<GeneratedNode, AdapterError>;
    fn pickup(&mut self, scan: ScanType, node: ScanNodeId) -> Result<bool, AdapterError>;
}

/// Mapping retained alongside runtime entities for deterministic persistence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GeneratedEntity {
    pub entity: SurfaceEntityId,
    pub scan: ScanType,
    pub node: ScanNodeId,
}

/// Generation validation error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenerationError {
    NodeOutOfRange(u8),
    TooManyNodes { scan: ScanType, count: u8 },
}

/// Populate active entities in C parity order: biological, energy, mineral,
/// with each category's node indices descending.
pub fn populate_surface(
    generator: &mut impl SurfaceGenerator,
    persistence: ScanPersistence,
    world: &mut SurfaceWorld,
) -> Result<Vec<GeneratedEntity>, AdapterError> {
    let mut generated = Vec::new();
    for scan in [ScanType::Biological, ScanType::Energy, ScanType::Mineral] {
        let count = generator.node_count(scan)?;
        if count > MAX_SCAN_NODES {
            return Err(AdapterError::new("surface_generator_too_many_nodes"));
        }
        for raw_node in (0..count).rev() {
            let node = ScanNodeId(raw_node);
            if persistence.is_retrieved(scan, node) {
                continue;
            }
            let data = generator.generate(scan, node)?;
            let kind = match data.kind {
                GeneratedNodeKind::Mineral {
                    category,
                    gross_size,
                    fine_quantity,
                } => SurfaceEntityKind::MineralNode {
                    category,
                    size: gross_size,
                    quantity: fine_quantity,
                },
                GeneratedNodeKind::Energy => SurfaceEntityKind::EnergyNode { node: raw_node },
                GeneratedNodeKind::Biological {
                    creature,
                    hit_points,
                    initial_frame,
                } => SurfaceEntityKind::LiveCreature {
                    kind: creature,
                    hit_points,
                    aware: false,
                    velocity: VelocityDesc::new(),
                    thrust_wait: 0,
                    frame_index: u16::from(initial_frame),
                },
            };
            let entity = world.insert(SurfaceEntity {
                kind,
                position: data.position,
                finite_life: None,
            });
            generated.push(GeneratedEntity { entity, scan, node });
        }
    }
    Ok(generated)
}

/// Invoke special pickup behavior and persist the scan bit only when accepted.
pub fn persist_pickup(
    generator: &mut impl SurfaceGenerator,
    persistence: &mut ScanPersistence,
    generated: GeneratedEntity,
) -> Result<bool, AdapterError> {
    if generator.pickup(generated.scan, generated.node)? {
        persistence.mark_retrieved(generated.scan, generated.node);
        Ok(true)
    } else {
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct Generator {
        calls: Vec<(ScanType, u8)>,
        pickup_result: bool,
    }

    impl SurfaceGenerator for Generator {
        fn node_count(&mut self, scan: ScanType) -> Result<u8, AdapterError> {
            Ok(if scan == ScanType::Energy { 0 } else { 2 })
        }

        fn generate(
            &mut self,
            scan: ScanType,
            node: ScanNodeId,
        ) -> Result<GeneratedNode, AdapterError> {
            self.calls.push((scan, node.get()));
            let kind = match scan {
                ScanType::Mineral => GeneratedNodeKind::Mineral {
                    category: 3,
                    gross_size: 2,
                    fine_quantity: 5,
                },
                ScanType::Energy => GeneratedNodeKind::Energy,
                ScanType::Biological => GeneratedNodeKind::Biological {
                    creature: CreatureKind::new(0).ok_or(AdapterError::new("creature"))?,
                    hit_points: 1,
                    initial_frame: 2,
                },
            };
            Ok(GeneratedNode {
                position: SurfacePoint {
                    x: i32::from(node.get()),
                    y: scan as i32,
                },
                kind,
            })
        }

        fn pickup(&mut self, scan: ScanType, node: ScanNodeId) -> Result<bool, AdapterError> {
            self.calls.push((scan, node.get()));
            Ok(self.pickup_result)
        }
    }

    #[test]
    fn scan_node_ids_reject_bits_outside_u32_mask() {
        assert_eq!(ScanNodeId::new(31).map(ScanNodeId::get), Ok(31));
        assert_eq!(
            ScanNodeId::new(32),
            Err(GenerationError::NodeOutOfRange(32))
        );
    }

    #[test]
    fn population_keeps_gross_size_and_fine_quantity_typed() {
        let mut generator = Generator::default();
        let mut world = SurfaceWorld::new();
        let generated =
            populate_surface(&mut generator, ScanPersistence::default(), &mut world).unwrap();
        let mineral = generated
            .iter()
            .find(|g| g.scan == ScanType::Mineral)
            .unwrap();
        let SurfaceEntityKind::MineralNode {
            category,
            size,
            quantity,
        } = &world.get(mineral.entity).unwrap().kind
        else {
            panic!("expected mineral");
        };
        assert_eq!((*category, *size, *quantity), (3, 2, 5));
    }

    #[test]
    fn population_skips_retrieved_nodes_and_preserves_c_order() {
        let mut generator = Generator::default();
        let mut persistence = ScanPersistence::default();
        persistence.mark_retrieved(ScanType::Biological, ScanNodeId::new(0).unwrap());
        let mut world = SurfaceWorld::new();
        let generated = populate_surface(&mut generator, persistence, &mut world).unwrap();
        assert_eq!(
            generator.calls,
            [
                (ScanType::Biological, 1),
                (ScanType::Mineral, 1),
                (ScanType::Mineral, 0)
            ]
        );
        assert_eq!(generated.len(), 3);
        assert_eq!(world.len(), 3);
    }

    #[test]
    fn persistence_round_trips_c_scan_order() {
        let persistence = ScanPersistence::from_masks([0x01, 0x20, 0x8000_0000]);
        assert_eq!(persistence.to_masks(), [0x01, 0x20, 0x8000_0000]);
        assert!(persistence.is_retrieved(ScanType::Biological, ScanNodeId::new(31).unwrap()));
    }

    #[test]
    fn rejected_special_pickup_does_not_set_retrieval_bit() {
        let mut generator = Generator::default();
        let mut persistence = ScanPersistence::default();
        let mut world = SurfaceWorld::new();
        let entity = world.insert(SurfaceEntity {
            kind: SurfaceEntityKind::EnergyNode { node: 4 },
            position: SurfacePoint::default(),
            finite_life: None,
        });
        let generated = GeneratedEntity {
            entity,
            scan: ScanType::Energy,
            node: ScanNodeId::new(4).unwrap(),
        };
        assert_eq!(
            persist_pickup(&mut generator, &mut persistence, generated),
            Ok(false)
        );
        assert_eq!(persistence.mask(ScanType::Energy), 0);
        generator.pickup_result = true;
        assert_eq!(
            persist_pickup(&mut generator, &mut persistence, generated),
            Ok(true)
        );
        assert_eq!(persistence.mask(ScanType::Energy), 1 << 4);
    }
}
