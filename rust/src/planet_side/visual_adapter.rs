//! Production visual selection for generated surface nodes.

use std::ffi::c_void;

use super::assembly::{EntityVisual, SurfaceVisualPort, WorldVisualPort};
use super::creatures::CreatureKind;
use super::entities::{SurfaceEntity, SurfaceEntityKind};
use super::generation::{GeneratedEntity, ScanType};
use super::graphics_adapter::SurfaceFrame;
use super::hazards::HazardKind;
use super::mask_adapter::extract_frame_mask;
use super::resources::{LanderGraphic, PlanetSideAssetAccess};
use super::runtime::AdapterError;

const NUM_SCANDOT_TRANSITIONS: u16 = 8;
const MINERAL_FRAME_BASE: u16 = NUM_SCANDOT_TRANSITIONS * 2;
const MINERAL_FRAMES_PER_CATEGORY: u16 = 5;

/// Captured production frame sources needed by generated surface nodes.
pub struct CffiSurfaceVisuals {
    misc_data: *mut c_void,
    energy: *mut c_void,
    life: [*mut c_void; 3],
    life_types: [Option<u8>; 3],
    #[cfg(feature = "linked_c_archive")]
    owned_life: [bool; 3],
}

impl CffiSurfaceVisuals {
    pub fn new(
        misc_data: *mut c_void,
        energy: *mut c_void,
        life: [*mut c_void; 3],
    ) -> Result<Self, AdapterError> {
        if misc_data.is_null() {
            return Err(AdapterError::new("misc_data_frame"));
        }
        Ok(Self {
            misc_data,
            energy,
            life,
            life_types: [None; 3],
            #[cfg(feature = "linked_c_archive")]
            owned_life: [false; 3],
        })
    }

    fn life_frame(&mut self, creature: u8) -> Result<*mut c_void, AdapterError> {
        if let Some(index) = self
            .life_types
            .iter()
            .position(|assigned| *assigned == Some(creature))
        {
            return Ok(self.life[index]);
        }
        let Some(index) = self.life_types.iter().position(Option::is_none) else {
            return Err(AdapterError::new("life_variation_overflow"));
        };
        if self.life[index].is_null() {
            #[cfg(feature = "linked_c_archive")]
            unsafe {
                self.life[index] = load_life_form(creature);
                self.owned_life[index] = !self.life[index].is_null();
            }
            #[cfg(not(feature = "linked_c_archive"))]
            return Err(AdapterError::new("life_frame_missing"));
        }
        if self.life[index].is_null() {
            return Err(AdapterError::new("life_frame_missing"));
        }
        self.life_types[index] = Some(creature);
        Ok(self.life[index])
    }
}

#[cfg(feature = "linked_c_archive")]
extern "C" {
    fn SetAbsFrameIndex(frame: *mut c_void, index: u16) -> *mut c_void;
    fn load_life_form(selector: u8) -> *mut c_void;
    fn ReleaseDrawable(frame: *mut c_void) -> *mut c_void;
    fn DestroyDrawable(frame: *mut c_void);
}

impl Drop for CffiSurfaceVisuals {
    fn drop(&mut self) {
        #[cfg(feature = "linked_c_archive")]
        unsafe {
            for (frame, owned) in self.life.iter_mut().zip(self.owned_life) {
                if owned && !frame.is_null() {
                    *frame = ReleaseDrawable(*frame);
                    DestroyDrawable(*frame);
                }
            }
        }
    }
}

impl SurfaceVisualPort for CffiSurfaceVisuals {
    fn visual_for(
        &mut self,
        generated: GeneratedEntity,
        entity: &SurfaceEntity,
    ) -> Result<EntityVisual, AdapterError> {
        let (base, index) = match entity.kind {
            SurfaceEntityKind::MineralNode { category, amount } => {
                let category =
                    u16::try_from(category).map_err(|_| AdapterError::new("mineral_category"))?;
                if category >= 8 {
                    return Err(AdapterError::new("mineral_category"));
                }
                (
                    self.misc_data,
                    MINERAL_FRAME_BASE
                        + category * MINERAL_FRAMES_PER_CATEGORY
                        + amount
                            .saturating_add(2)
                            .min(MINERAL_FRAMES_PER_CATEGORY - 1),
                )
            }
            SurfaceEntityKind::EnergyNode { .. } => {
                if self.energy.is_null() {
                    return Err(AdapterError::new("energy_frame_missing"));
                }
                (self.energy, 0)
            }
            SurfaceEntityKind::LiveCreature { kind, .. } => (self.life_frame(kind.index())?, 0),
            _ => return Err(AdapterError::new("generated_visual_kind")),
        };
        if generated.scan == ScanType::Energy && base.is_null() {
            return Err(AdapterError::new("energy_frame_missing"));
        }

        #[cfg(feature = "linked_c_archive")]
        let selected = unsafe { SetAbsFrameIndex(base, index) };
        #[cfg(not(feature = "linked_c_archive"))]
        let selected = base;
        if selected.is_null() {
            return Err(AdapterError::new("surface_frame_missing"));
        }
        let mask = unsafe { extract_frame_mask(selected)? };
        Ok(EntityVisual {
            frame: SurfaceFrame { base, index },
            mask,
        })
    }
}

/// Production visual port for dynamically-spawned world entities.
///
/// Provides frames and collision masks for lander stun-bolt shots, canned
/// creatures, and environmental hazards using the globally loaded PlanetSide
/// asset set.
pub struct CffiWorldVisuals<'a, A: PlanetSideAssetAccess> {
    assets: &'a A,
    /// Reuses the life-frame slot table from [`CffiSurfaceVisuals`] for canned
    /// creature visuals.
    surface_visuals: &'a mut CffiSurfaceVisuals,
}

impl<'a, A: PlanetSideAssetAccess> CffiWorldVisuals<'a, A> {
    pub fn new(assets: &'a A, surface_visuals: &'a mut CffiSurfaceVisuals) -> Self {
        Self {
            assets,
            surface_visuals,
        }
    }

    fn lander_shot_frame(&self, facing: u8) -> Result<(*mut c_void, u16), AdapterError> {
        let base = self.assets.graphic(LanderGraphic::Lander);
        if base.is_null() {
            return Err(AdapterError::new("lander_frame_not_loaded"));
        }
        // Shot images immediately follow the 16 directional lander images.
        Ok((base, 16 + u16::from(facing % 16)))
    }

    fn hazard_base(&self, kind: HazardKind) -> Result<*mut c_void, AdapterError> {
        let graphic = match kind {
            HazardKind::Earthquake => LanderGraphic::Earthquake,
            HazardKind::Lightning => LanderGraphic::Lightning,
            HazardKind::Lava => LanderGraphic::Lava,
            HazardKind::Biological => return Err(AdapterError::new("biological_hazard_visual")),
        };
        let base = self.assets.graphic(graphic);
        if base.is_null() {
            return Err(AdapterError::new("hazard_frame_not_loaded"));
        }
        Ok(base)
    }
}

impl<A: PlanetSideAssetAccess> WorldVisualPort for CffiWorldVisuals<'_, A> {
    fn shot_visual(&mut self, facing: u8) -> Result<EntityVisual, AdapterError> {
        let (base, index) = self.lander_shot_frame(facing)?;
        #[cfg(feature = "linked_c_archive")]
        let selected = unsafe { SetAbsFrameIndex(base, index) };
        #[cfg(not(feature = "linked_c_archive"))]
        let selected = base;
        if selected.is_null() {
            return Err(AdapterError::new("shot_frame_missing"));
        }
        let mask = unsafe { extract_frame_mask(selected)? };
        Ok(EntityVisual {
            frame: SurfaceFrame { base, index },
            mask,
        })
    }

    fn canned_creature_visual(&mut self, kind: CreatureKind) -> Result<EntityVisual, AdapterError> {
        let base = self.surface_visuals.life_frame(kind.index())?;
        #[cfg(feature = "linked_c_archive")]
        let selected = unsafe { SetAbsFrameIndex(base, 0) };
        #[cfg(not(feature = "linked_c_archive"))]
        let selected = base;
        if selected.is_null() {
            return Err(AdapterError::new("canned_frame_missing"));
        }
        let mask = unsafe { extract_frame_mask(selected)? };
        Ok(EntityVisual {
            frame: SurfaceFrame { base, index: 0 },
            mask,
        })
    }

    fn hazard_visual(&mut self, kind: HazardKind) -> Result<EntityVisual, AdapterError> {
        let base = self.hazard_base(kind)?;
        #[cfg(feature = "linked_c_archive")]
        let selected = unsafe { SetAbsFrameIndex(base, 0) };
        #[cfg(not(feature = "linked_c_archive"))]
        let selected = base;
        if selected.is_null() {
            return Err(AdapterError::new("hazard_frame_missing"));
        }
        let mask = unsafe { extract_frame_mask(selected)? };
        Ok(EntityVisual {
            frame: SurfaceFrame { base, index: 0 },
            mask,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::planet_side::entities::{SurfaceEntity, SurfaceWorld};
    use crate::planet_side::generation::ScanNodeId;
    use crate::planet_side::model::SurfacePoint;

    fn pointer(value: usize) -> *mut c_void {
        value as *mut c_void
    }

    #[test]
    fn visual_context_requires_misc_frame() {
        assert!(matches!(
            CffiSurfaceVisuals::new(std::ptr::null_mut(), pointer(2), [pointer(3); 3]),
            Err(AdapterError {
                operation: "misc_data_frame"
            })
        ));
    }

    #[test]
    fn life_variations_are_stable_and_limited_to_three() {
        let mut visuals =
            CffiSurfaceVisuals::new(pointer(1), pointer(2), [pointer(3), pointer(4), pointer(5)])
                .unwrap();
        assert_eq!(visuals.life_frame(7), Ok(pointer(3)));
        assert_eq!(visuals.life_frame(8), Ok(pointer(4)));
        assert_eq!(visuals.life_frame(7), Ok(pointer(3)));
        assert_eq!(visuals.life_frame(9), Ok(pointer(5)));
        assert_eq!(
            visuals.life_frame(10),
            Err(AdapterError::new("life_variation_overflow"))
        );
    }

    #[test]
    fn shot_frame_follows_lander_facing() {
        let assets = crate::planet_side::resources::BorrowedPlanetSideAssets::for_test(
            [pointer(1); 8],
            pointer(2),
        );
        let mut surface = CffiSurfaceVisuals::new(pointer(3), pointer(4), [pointer(5); 3]).unwrap();
        let visuals = CffiWorldVisuals::new(&assets, &mut surface);
        assert_eq!(visuals.lander_shot_frame(0), Ok((pointer(1), 16)));
        assert_eq!(visuals.lander_shot_frame(7), Ok((pointer(1), 23)));
        assert_eq!(visuals.lander_shot_frame(15), Ok((pointer(1), 31)));
    }

    #[test]
    fn invalid_mineral_category_is_rejected_before_mask_extraction() {
        let mut visuals = CffiSurfaceVisuals::new(pointer(1), pointer(2), [pointer(3); 3]).unwrap();
        let entity = SurfaceEntity {
            kind: SurfaceEntityKind::MineralNode {
                category: 8,
                amount: 1,
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
                operation: "mineral_category"
            })
        ));
    }
}
