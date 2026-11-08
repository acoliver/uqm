//! Collision masks from retained native frames.

use super::{geometry::CollisionMask, model::SurfacePoint, runtime::AdapterError};
use crate::graphics::context_abi::ContextFrame;

pub fn extract_frame_mask(frame: &ContextFrame) -> Result<CollisionMask, AdapterError> {
    let occupancy = frame
        .alpha_occupancy()
        .ok_or(AdapterError::new("collision_frame_invalid"))?;
    let rect = frame.rect();
    CollisionMask::from_occupancy(
        rect.extent.width as u16,
        rect.extent.height as u16,
        SurfacePoint {
            x: -i32::from(rect.corner.x),
            y: -i32::from(rect.corner.y),
        },
        &occupancy,
    )
    .map_err(|_| AdapterError::new("collision_mask_build"))
}

pub fn extract_lander_masks(base: &ContextFrame) -> Result<Vec<CollisionMask>, AdapterError> {
    (0..16)
        .map(|index| extract_frame_mask(&base.wrapped_frame(index)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graphics::{
        drawable_abi::{LoadedDrawable, WriteFramePixelColors},
        native_abi::{Color, HOT_SPOT},
    };

    #[test]
    #[serial_test::serial]
    fn absent_frame_does_not_enter_another_threads_live_registry() {
        if !crate::native_test_process::own_graphics_process(concat!(
            module_path!(),
            "::absent_frame_does_not_enter_another_threads_live_registry"
        )) {
            return;
        }
        let mut game = crate::engine::Game::new();
        let display = crate::campaign_display_test::CampaignDisplay::install(&mut game).unwrap();
        let frame = ContextFrame::retain(std::ptr::NonNull::new(game.kernel.screen));
        assert!(
            std::thread::spawn(|| {
                assert!(ContextFrame::retain(None).alpha_occupancy().is_none());
            })
            .join()
            .is_ok(),
            "an absent frame must not access another thread's graphics allocation"
        );
        assert!(frame.rect().extent.width > 0);
        drop(frame);
        display.finish(game);
    }

    #[test]
    #[serial_test::serial]
    fn retained_mask_uses_normal_alpha_and_hotspot_after_owner_release() {
        let mut game = crate::engine::Game::new();
        let display = crate::campaign_display_test::CampaignDisplay::install(&mut game).unwrap();
        let owner = LoadedDrawable::create(8, 2, 2, 16).unwrap();
        let lease = owner.lease();
        lease.set_hotspot(0, HOT_SPOT { x: 1, y: -2 });
        let pixels = [0, 1, 127, 255].map(|a| Color {
            r: 0,
            g: 0,
            b: 0,
            a,
        });
        // SAFETY: this serialized fixture owns the four-pixel destination and complete source buffer.
        assert_ne!(
            unsafe { WriteFramePixelColors(lease.frame(0).unwrap(), pixels.as_ptr(), 2, 2) },
            0
        );
        let frame = lease.context_frame(0);
        drop(lease);
        drop(owner);
        assert_eq!(frame.alpha_occupancy().unwrap(), [0, 1, 1, 1]);
        let mask = extract_frame_mask(&frame).unwrap();
        assert_eq!(
            mask,
            CollisionMask::from_occupancy(2, 2, SurfacePoint { x: 1, y: -2 }, &[0, 1, 1, 1])
                .unwrap()
        );
        assert_eq!(extract_lander_masks(&frame).unwrap().len(), 16);
        assert_eq!(
            extract_frame_mask(&ContextFrame::retain(None)),
            Err(AdapterError::new("collision_frame_invalid"))
        );
        drop(frame);
        display.finish(game);
    }
}
