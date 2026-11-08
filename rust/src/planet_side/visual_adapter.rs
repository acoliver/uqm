//! Production visual selection with retained frames and collision masks.
use super::{
    assembly::{EntityVisual, SurfaceVisualPort, WorldVisualPort},
    creatures::CreatureKind,
    entities::{SurfaceEntity, SurfaceEntityKind},
    generation::GeneratedEntity,
    graphics_adapter::SurfaceFrame,
    hazards::HazardKind,
    mask_adapter::extract_frame_mask,
    resources::{LanderGraphic, PlanetSideAssets},
    runtime::AdapterError,
    scan_frames::{mineral_surface_sprite, ScanFrameError},
};
use crate::graphics::{context_abi::ContextFrame, drawable_abi::LoadedDrawable};

pub struct CffiSurfaceVisuals {
    misc_data: ContextFrame,
    energy: Option<ContextFrame>,
    life: [Option<ContextFrame>; 3],
    life_types: [Option<u8>; 3],
    canister: Option<LoadedDrawable>,
    owned_life: Vec<LoadedDrawable>,
}

impl CffiSurfaceVisuals {
    pub fn new(
        misc_data: ContextFrame,
        energy: Option<ContextFrame>,
        life: [Option<ContextFrame>; 3],
    ) -> Result<Self, AdapterError> {
        if misc_data.frame_count() == 0 {
            return Err(AdapterError::new("misc_data_frame"));
        }
        Ok(Self {
            misc_data,
            energy,
            life,
            life_types: [None; 3],
            canister: None,
            owned_life: Vec::new(),
        })
    }

    fn canister_frame(&mut self) -> Result<ContextFrame, AdapterError> {
        if self.canister.is_none() {
            self.canister = Some(
                LoadedDrawable::load(c"graphics.lifecan")
                    .ok_or(AdapterError::new("load_resource"))?,
            );
        }
        Ok(self
            .canister
            .as_ref()
            .unwrap_or_else(|| unreachable!())
            .lease()
            .context_frame(0))
    }

    fn life_frame(&mut self, creature: u8) -> Result<ContextFrame, AdapterError> {
        if let Some(index) = self
            .life_types
            .iter()
            .position(|assigned| *assigned == Some(creature))
        {
            return Ok(self.life[index]
                .as_ref()
                .unwrap_or_else(|| unreachable!())
                .clone());
        }
        let index = self
            .life_types
            .iter()
            .position(Option::is_none)
            .ok_or(AdapterError::new("life_variation_overflow"))?;
        if self.life[index].is_none() {
            let key = std::ffi::CString::new(format!("graphics.life.{creature}"))
                .unwrap_or_else(|_| unreachable!());
            let owner =
                LoadedDrawable::load(&key).ok_or(AdapterError::new("life_frame_missing"))?;
            self.life[index] = Some(owner.lease().context_frame(0));
            self.owned_life.push(owner);
        }
        self.life_types[index] = Some(creature);
        Ok(self.life[index]
            .as_ref()
            .unwrap_or_else(|| unreachable!())
            .clone())
    }
}

fn visual(base: ContextFrame, index: u16) -> Result<EntityVisual, AdapterError> {
    let mask = extract_frame_mask(&base.wrapped_frame(index))?;
    Ok(EntityVisual {
        frame: SurfaceFrame { base, index },
        mask,
    })
}

impl SurfaceVisualPort for CffiSurfaceVisuals {
    fn visual_for(
        &mut self,
        generated: GeneratedEntity,
        entity: &SurfaceEntity,
    ) -> Result<EntityVisual, AdapterError> {
        let (base, index) = match entity.kind {
            SurfaceEntityKind::MineralNode { category, size, .. } => (
                self.misc_data.clone(),
                mineral_visual_frame(category, size)?,
            ),
            SurfaceEntityKind::EnergyNode { .. } => (
                self.energy
                    .clone()
                    .ok_or(AdapterError::new("energy_frame_missing"))?,
                0,
            ),
            SurfaceEntityKind::CannedCreature { frame_index, .. } => {
                (self.canister_frame()?, frame_index)
            }
            SurfaceEntityKind::LiveCreature {
                kind, frame_index, ..
            } => (
                self.life_frame(kind.index())?,
                life_animation_frame(frame_index),
            ),
            _ => return Err(AdapterError::new("generated_visual_kind")),
        };
        let _ = generated;
        visual(base, index)
    }
}

/// Normalize the creature lifecycle frame to the four-frame life animation (like
/// `lifea.ani`..`lifez.ani`) that the visual port selects for a live
/// creature.  The stored frame_index advances one step per world step; once it
/// reaches 4 the animation wraps back to frame 0 (4 -> 0), and any other
/// cycle value maps onto its 0..3 slot.
fn life_animation_frame(frame_index: u16) -> u16 {
    frame_index % 4
}

/// Select the lander surface sprite for a mineral deposit from its category
/// and gross size (never the fine collectible quantity).  Shared by the
/// generated-node path and the fixture mineral path so validation and frame
/// selection cannot drift.
fn mineral_visual_frame(category: usize, size: u16) -> Result<u16, AdapterError> {
    mineral_surface_sprite(category, size).map_err(|error| match error {
        ScanFrameError::Category => AdapterError::new("mineral_category"),
        ScanFrameError::GrossSize => AdapterError::new("mineral_gross_size"),
    })
}

pub struct CffiWorldVisuals<'a> {
    assets: &'a PlanetSideAssets,
    surface_visuals: &'a mut CffiSurfaceVisuals,
}

impl<'a> CffiWorldVisuals<'a> {
    pub fn new(assets: &'a PlanetSideAssets, surface_visuals: &'a mut CffiSurfaceVisuals) -> Self {
        Self {
            assets,
            surface_visuals,
        }
    }
}

impl WorldVisualPort for CffiWorldVisuals<'_> {
    fn shot_visual(&mut self, facing: u8) -> Result<EntityVisual, AdapterError> {
        visual(
            self.assets.graphic(LanderGraphic::Lander),
            16 + u16::from(facing % 16),
        )
    }
    fn canned_creature_visual(
        &mut self,
        animation_frame: u16,
    ) -> Result<EntityVisual, AdapterError> {
        visual(self.surface_visuals.canister_frame()?, animation_frame)
    }
    fn hazard_visual(&mut self, kind: HazardKind) -> Result<EntityVisual, AdapterError> {
        let graphic = match kind {
            HazardKind::Earthquake => LanderGraphic::Earthquake,
            HazardKind::Lightning => LanderGraphic::Lightning,
            HazardKind::Lava => LanderGraphic::Lava,
            HazardKind::Biological => return Err(AdapterError::new("biological_hazard_visual")),
        };
        visual(self.assets.graphic(graphic), 0)
    }
    fn creature_animation_visual(
        &mut self,
        kind: CreatureKind,
        animation_frame: u16,
    ) -> Result<EntityVisual, AdapterError> {
        visual(
            self.surface_visuals.life_frame(kind.index())?,
            animation_frame,
        )
    }
}

impl super::automation_fixture::FixtureVisualPort for CffiWorldVisuals<'_> {
    fn mineral_visual(&mut self, entity: &SurfaceEntity) -> Result<EntityVisual, AdapterError> {
        let SurfaceEntityKind::MineralNode { category, size, .. } = entity.kind else {
            return Err(AdapterError::new("fixture_mineral_visual"));
        };
        visual(
            self.surface_visuals.misc_data.clone(),
            mineral_visual_frame(category, size)?,
        )
    }
    fn creature_visual(
        &mut self,
        kind: CreatureKind,
        animation_frame: u16,
    ) -> Result<EntityVisual, AdapterError> {
        WorldVisualPort::creature_animation_visual(self, kind, animation_frame)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::planet_side::entities::{SurfaceEntity, SurfaceEntityKind, SurfaceWorld};
    use crate::planet_side::generation::{GeneratedEntity, ScanNodeId, ScanType};
    use crate::planet_side::model::SurfacePoint;

    fn frame(width: i16) -> ContextFrame {
        LoadedDrawable::create(8, width, 1, 60)
            .unwrap()
            .lease()
            .context_frame(0)
    }

    /// The production mineral kind → frame index via the shared helper, as the
    /// `SurfaceVisualPort::visual_for` match arm computes it for a typed entity.
    fn mineral_visual_frame_from_mineral(kind: SurfaceEntityKind) -> Result<u16, AdapterError> {
        match kind {
            SurfaceEntityKind::MineralNode { category, size, .. } => {
                mineral_visual_frame(category, size)
            }
            _ => unreachable!("mineral entity kind only"),
        }
    }

    const MINERAL_CATEGORY_COUNT: usize = 8;
    const MINERAL_GROSS_SIZE_COUNT: u16 = 3;
    const MINERAL_FINE_DENSITY_MAX: u16 = 255;

    fn mineral_entity(category: usize, size: u16, quantity: u16) -> SurfaceEntity {
        SurfaceEntity {
            kind: SurfaceEntityKind::MineralNode {
                category,
                size,
                quantity,
            },
            position: SurfacePoint::default(),
            finite_life: None,
        }
    }

    #[test]
    fn mineral_matrix_fine_quantity_never_selects_the_frame() {
        // Full matrix over every valid category (0..7), every UQM-generated
        // gross size (0..2 from surface.c), and every representable fine
        // density byte (0..=255).  Typed MineralNode entities differ only in
        // fine quantity: the selected frame must be identical for all fine
        // values.  The lander surface sprite is `8 + 5*category + gross + 2`
        // in the original C; the gross-0 sprite of each category is written
        // out literally here so the expectation does not reuse the code under
        // test.
        const GROSS_ZERO_SURFACE_SPRITE: [u16; MINERAL_CATEGORY_COUNT] =
            [10, 15, 20, 25, 30, 35, 40, 45];
        for (category, gross_zero_sprite) in GROSS_ZERO_SURFACE_SPRITE.into_iter().enumerate() {
            for size in 0..MINERAL_GROSS_SIZE_COUNT {
                let expected = gross_zero_sprite + size;
                for fine in 0..=MINERAL_FINE_DENSITY_MAX {
                    assert_eq!(
                        mineral_visual_frame_from_mineral(
                            mineral_entity(category, size, fine).kind
                        ),
                        Ok(expected)
                    );
                }
            }
        }
    }

    #[test]
    fn luna_base_metal_deposit_selects_the_small_grey_sprite() {
        // Category 2 (base metals), gross 0: C frame 20, not the noble-gas
        // cluster frame 26 the old base of 16 selected.
        assert_eq!(mineral_visual_frame(2, 0), Ok(20));
        assert_eq!(mineral_visual_frame(7, 2), Ok(47));
    }

    #[test]
    fn out_of_range_gross_size_is_rejected_not_clamped() {
        assert_eq!(
            mineral_visual_frame(2, 3),
            Err(AdapterError::new("mineral_gross_size"))
        );
    }

    #[test]
    fn invalid_mineral_category_is_rejected_through_shared_helper() {
        for category in MINERAL_CATEGORY_COUNT..2 * MINERAL_CATEGORY_COUNT {
            assert!(matches!(
                mineral_visual_frame(category, 2),
                Err(AdapterError {
                    operation: "mineral_category",
                    batch_depth_corrections: 0
                })
            ));
            assert!(matches!(
                mineral_visual_frame_from_mineral(mineral_entity(category, 2, 1).kind),
                Err(AdapterError {
                    operation: "mineral_category",
                    batch_depth_corrections: 0
                })
            ));
        }
    }

    #[test]
    fn visual_context_requires_misc_frame() {
        assert!(matches!(
            CffiSurfaceVisuals::new(
                ContextFrame::retain(None),
                None,
                std::array::from_fn(|_| None)
            ),
            Err(AdapterError {
                operation: "misc_data_frame",
                batch_depth_corrections: 0
            })
        ));
    }

    #[test]
    fn life_animation_frame_normalizes_typed_frames_to_the_four_frame_contract() {
        // The stored lifecycle frame_index driven by one world step per frame is
        // normalized to the 0..3 life animation: 0..=3 pass through, and the
        // rollover 4..=7 cycle values wrap back (4 -> 0 .. 7 -> 3).
        for frame_index in 0..=3 {
            assert_eq!(life_animation_frame(frame_index), frame_index);
        }
        for frame_index in 4..=7 {
            assert_eq!(life_animation_frame(frame_index), frame_index % 4);
        }
    }

    #[test]
    #[serial_test::serial]
    fn life_variations_are_stable_and_limited_to_three() {
        let mut game = crate::engine::Game::new();
        let display = crate::campaign_display_test::CampaignDisplay::install(&mut game).unwrap();
        let mut visuals = CffiSurfaceVisuals::new(
            frame(1),
            None,
            std::array::from_fn(|index| Some(frame(index as i16 + 3))),
        )
        .unwrap();
        assert_eq!(visuals.life_frame(7).unwrap().rect().extent.width, 3);
        assert_eq!(visuals.life_frame(8).unwrap().rect().extent.width, 4);
        assert_eq!(visuals.life_frame(7).unwrap().rect().extent.width, 3);
        assert_eq!(visuals.life_frame(9).unwrap().rect().extent.width, 5);
        assert!(matches!(
            visuals.life_frame(10),
            Err(AdapterError {
                operation: "life_variation_overflow",
                ..
            })
        ));
        drop(visuals);
        display.finish(game);
    }

    #[test]
    #[serial_test::serial]
    fn invalid_mineral_category_is_rejected_before_mask_extraction() {
        // The production entry point rejects an invalid category through the shared
        // helper before any frame or mask work, using a typed entity.
        let mut game = crate::engine::Game::new();
        let display = crate::campaign_display_test::CampaignDisplay::install(&mut game).unwrap();
        let mut visuals = CffiSurfaceVisuals::new(
            frame(1),
            None,
            std::array::from_fn(|index| Some(frame(index as i16 + 3))),
        )
        .unwrap();
        let entity = SurfaceEntity {
            kind: SurfaceEntityKind::MineralNode {
                category: MINERAL_CATEGORY_COUNT,
                size: 2,
                quantity: 1,
            },
            position: SurfacePoint::default(),
            finite_life: None,
        };
        let mut world = SurfaceWorld::new();
        let entity_id = world.insert(entity.clone());
        let result = visuals.visual_for(
            GeneratedEntity {
                entity: entity_id,
                scan: ScanType::Mineral,
                node: ScanNodeId::new(0).unwrap(),
            },
            &entity,
        );
        assert!(matches!(
            result,
            Err(AdapterError {
                operation: "mineral_category",
                batch_depth_corrections: 0
            })
        ));
        drop(visuals);
        display.finish(game);
    }
}
