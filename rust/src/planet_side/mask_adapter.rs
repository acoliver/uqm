//! Production collision-mask extraction from captured graphics frames.
//!
//! This module uses a dedicated source-matched frame prefix. It deliberately
//! does not reuse the battle `IntersectControl` mirror, whose field order is
//! incompatible with PlanetSide's legacy geometry ABI.

use std::ffi::c_void;

use super::geometry::CollisionMask;
#[cfg(feature = "linked_c_archive")]
use super::model::SurfacePoint;
use super::runtime::AdapterError;

#[cfg(feature = "linked_c_archive")]
#[repr(C)]
#[derive(Clone, Copy)]
struct CPoint {
    x: i16,
    y: i16,
}

#[cfg(feature = "linked_c_archive")]
#[repr(C)]
#[derive(Clone, Copy)]
struct CExtent {
    width: i16,
    height: i16,
}

/// Exact `FRAME_DESC` layout from `libs/graphics/drawable.h`.
#[cfg(feature = "linked_c_archive")]
#[repr(C)]
struct CFrameDesc {
    frame_type: u16,
    index: u16,
    hot_spot: CPoint,
    bounds: CExtent,
    image: *mut c_void,
    parent: *mut c_void,
}

#[cfg(feature = "linked_c_archive")]
#[repr(C)]
#[derive(Clone, Copy)]
struct CColor {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}

#[cfg(feature = "linked_c_archive")]
extern "C" {
    fn SetAbsFrameIndex(frame: *mut c_void, index: u16) -> *mut c_void;
    fn TFB_DrawCanvas_Lock(canvas: *mut c_void);
    fn TFB_DrawCanvas_Unlock(canvas: *mut c_void);
    fn TFB_DrawCanvas_GetPixel(canvas: *mut c_void, x: i32, y: i32) -> CColor;
}

#[cfg(feature = "linked_c_archive")]
struct CanvasLock(*mut c_void);

#[cfg(feature = "linked_c_archive")]
impl Drop for CanvasLock {
    fn drop(&mut self) {
        unsafe { TFB_DrawCanvas_Unlock(self.0) };
    }
}

/// Extract a one-bit mask from a captured production frame.
///
/// # Safety
///
/// `frame` must be a live captured `FRAME_DESC` whose image owns a live normal
/// canvas for the duration of the call.
pub unsafe fn extract_frame_mask(frame: *mut c_void) -> Result<CollisionMask, AdapterError> {
    #[cfg(feature = "linked_c_archive")]
    {
        if frame.is_null() {
            return Err(AdapterError::new("collision_frame_null"));
        }
        let descriptor = &*frame.cast::<CFrameDesc>();
        if descriptor.image.is_null()
            || descriptor.bounds.width <= 0
            || descriptor.bounds.height <= 0
        {
            return Err(AdapterError::new("collision_frame_invalid"));
        }

        // `NormalImg` is the first field of `TFB_Image` in tfb_draw.h.
        let canvas = descriptor.image.cast::<*mut c_void>().read();
        if canvas.is_null() {
            return Err(AdapterError::new("collision_canvas_missing"));
        }

        let width = u16::try_from(descriptor.bounds.width)
            .map_err(|_| AdapterError::new("collision_frame_extent"))?;
        let height = u16::try_from(descriptor.bounds.height)
            .map_err(|_| AdapterError::new("collision_frame_extent"))?;
        TFB_DrawCanvas_Lock(canvas);
        let _lock = CanvasLock(canvas);
        let mut occupancy = Vec::with_capacity(usize::from(width) * usize::from(height));
        for y in 0..i32::from(height) {
            for x in 0..i32::from(width) {
                occupancy.push(u8::from(TFB_DrawCanvas_GetPixel(canvas, x, y).a != 0));
            }
        }
        CollisionMask::from_occupancy(
            width,
            height,
            SurfacePoint {
                x: i32::from(descriptor.hot_spot.x),
                y: i32::from(descriptor.hot_spot.y),
            },
            &occupancy,
        )
        .map_err(|_| AdapterError::new("collision_mask_build"))
    }
    #[cfg(not(feature = "linked_c_archive"))]
    {
        let _ = frame;
        Err(AdapterError::new("collision_mask_unlinked"))
    }
}

/// Extract all 16 directional lander masks from one captured drawable.
///
/// # Safety
///
/// `base` must be a live captured lander frame handle with at least 16 frames.
pub unsafe fn extract_lander_masks(base: *mut c_void) -> Result<Vec<CollisionMask>, AdapterError> {
    #[cfg(feature = "linked_c_archive")]
    {
        if base.is_null() {
            return Err(AdapterError::new("lander_collision_frame_null"));
        }
        let mut masks = Vec::with_capacity(16);
        for index in 0..16 {
            let frame = SetAbsFrameIndex(base, index);
            masks.push(extract_frame_mask(frame)?);
        }
        Ok(masks)
    }
    #[cfg(not(feature = "linked_c_archive"))]
    {
        let _ = base;
        Err(AdapterError::new("collision_mask_unlinked"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unlinked_extraction_is_a_typed_error() {
        assert_eq!(
            unsafe { extract_frame_mask(std::ptr::null_mut()) },
            Err(AdapterError::new("collision_mask_unlinked"))
        );
        assert_eq!(
            unsafe { extract_lander_masks(std::ptr::null_mut()) },
            Err(AdapterError::new("collision_mask_unlinked"))
        );
    }

    #[cfg(all(feature = "linked_c_archive", target_pointer_width = "64"))]
    #[test]
    fn frame_descriptor_layout_matches_64_bit_c_abi() {
        assert_eq!(std::mem::size_of::<CFrameDesc>(), 32);
        assert_eq!(std::mem::align_of::<CFrameDesc>(), 8);
    }
}
