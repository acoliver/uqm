//! Drawing front-end ABI, ported from frame.c (GPL-2.0-or-later).
use super::font_abi::{_text_blt, TextRect};
use super::native_abi::*;
use std::ptr;

pub(super) unsafe fn graphics_active() -> bool {
    _GraphicsStatusFlags & 12 == 12
}

/// # Safety
/// Called on the graphics thread with an active context and foreground frame.
/// Non-null outputs must point to writable records.
#[no_mangle]
pub unsafe extern "C" fn GetContextValidRect(rect: *mut RECT, origin: *mut POINT) -> BOOLEAN {
    let mut tmp_rect = std::mem::zeroed();
    let mut tmp_point = std::mem::zeroed();
    let rect = if rect.is_null() {
        &mut tmp_rect
    } else {
        &mut *rect
    };
    let origin = if origin.is_null() {
        &mut tmp_point
    } else {
        &mut *origin
    };
    valid_rect(
        (*_CurFramePtr).Bounds,
        (*_CurFramePtr).HotSpot,
        (*_pCurContext).ClipRect,
        rect,
        origin,
    )
}
unsafe fn valid_rect(
    bounds: EXTENT,
    hotspot: HOT_SPOT,
    mut clip: RECT,
    rect: &mut RECT,
    origin: &mut POINT,
) -> BOOLEAN {
    rect.corner = POINT { x: 0, y: 0 };
    rect.extent = bounds;
    *origin = hotspot;
    if clip.extent.width != 0 {
        let output = ptr::from_mut(rect);
        if BoxIntersect(&mut clip, output, output) == 0 {
            return 0;
        }
        origin.x = origin.x.wrapping_add(clip.corner.x);
        origin.y = origin.y.wrapping_add(clip.corner.y);
    }
    1
}
unsafe fn clear_background(clip: &RECT) {
    let mut r = RECT {
        corner: POINT { x: 0, y: 0 },
        extent: clip.extent,
    };
    TFB_Prim_FillRect(
        &mut r,
        (*_pCurContext).BackGroundColor,
        DrawMode { kind: 0, factor: 0 },
        clip.corner,
    );
}
/// # Safety
/// Primitives and their linked indices must be live and valid on the graphics thread.
#[no_mangle]
pub unsafe extern "C" fn DrawBatch(base: *mut PRIMITIVE, links: u32, flags: u8) {
    let mut rect = std::mem::zeroed();
    let mut origin = std::mem::zeroed();
    if !graphics_active() || GetContextValidRect(&mut rect, &mut origin) == 0 {
        return;
    }
    let mode = (*_pCurContext).Mode;
    BatchGraphics();
    if flags & 1 != 0 {
        clear_background(&rect);
    }
    let mut index = links as u16;
    while index != u16::MAX {
        let p = base.add(index as usize);
        match (*p).Type {
            0 => TFB_Prim_Point(&mut (*p).Object.Point, (*p).color, mode, origin),
            1 => TFB_Prim_Stamp(&mut (*p).Object.Stamp, mode, origin),
            2 => TFB_Prim_StampFill(&mut (*p).Object.Stamp, (*p).color, mode, origin),
            3 => TFB_Prim_Line(&mut (*p).Object.Line, (*p).color, mode, origin),
            4 => {
                let mut clip = std::mem::zeroed();
                if TextRect(&mut (*p).Object.Text, &mut clip, ptr::null_mut()) != 0 {
                    _text_blt(&mut clip, &mut (*p).Object.Text, origin);
                }
            }
            5 => TFB_Prim_Rect(&mut (*p).Object.Rect, (*p).color, mode, origin),
            6 => TFB_Prim_FillRect(&mut (*p).Object.Rect, (*p).color, mode, origin),
            _ => {}
        }
        index = ((*p).Links >> 16) as u16;
    }
    UnbatchGraphics();
}
/// # Safety
/// Graphics-thread caller must maintain active context/frame invariants.
#[no_mangle]
pub unsafe extern "C" fn ClearDrawable() {
    let mut rect = std::mem::zeroed();
    if graphics_active() && GetContextValidRect(&mut rect, ptr::null_mut()) != 0 {
        clear_background(&rect);
    }
}
macro_rules! draw {
    ($name:ident, $ty:ty, $prim:ident) => {
        /// # Safety
        /// Argument must be live; caller must obey the graphics-thread context contract.
        #[no_mangle]
        pub unsafe extern "C" fn $name(value: *mut $ty) {
            let mut origin = std::mem::zeroed();
            if graphics_active() && GetContextValidRect(ptr::null_mut(), &mut origin) != 0 {
                $prim(value, _locPrim.color, (*_pCurContext).Mode, origin);
            }
        }
    };
}
draw!(DrawPoint, POINT, TFB_Prim_Point);
draw!(DrawRectangle, RECT, TFB_Prim_Rect);
draw!(DrawFilledRectangle, RECT, TFB_Prim_FillRect);
draw!(DrawLine, LINE, TFB_Prim_Line);
draw!(DrawFilledStamp, STAMP, TFB_Prim_StampFill);
/// # Safety
/// Stamp and graphics context must be live on the graphics thread.
#[no_mangle]
pub unsafe extern "C" fn DrawStamp(stamp: *mut STAMP) {
    let mut origin = std::mem::zeroed();
    if graphics_active() && GetContextValidRect(ptr::null_mut(), &mut origin) != 0 {
        TFB_Prim_Stamp(stamp, (*_pCurContext).Mode, origin);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn clipping_keeps_context_origin_separate_from_intersection() {
        unsafe {
            let mut rect = std::mem::zeroed();
            let mut origin = std::mem::zeroed();
            let bounds = EXTENT {
                width: 20,
                height: 10,
            };
            let hotspot = HOT_SPOT { x: 2, y: 3 };
            let clip = RECT {
                corner: POINT { x: -5, y: 2 },
                extent: EXTENT {
                    width: 10,
                    height: 7,
                },
            };
            assert_eq!(valid_rect(bounds, hotspot, clip, &mut rect, &mut origin), 1);
            assert_eq!(
                (
                    rect.corner.x,
                    rect.corner.y,
                    rect.extent.width,
                    rect.extent.height
                ),
                (0, 2, 5, 7)
            );
            assert_eq!((origin.x, origin.y), (-3, 5));
            let outside = RECT {
                corner: POINT { x: 30, y: 0 },
                extent: EXTENT {
                    width: 2,
                    height: 2,
                },
            };
            assert_eq!(
                valid_rect(bounds, hotspot, outside, &mut rect, &mut origin),
                0
            );
            let disabled = RECT {
                corner: POINT { x: 30, y: 30 },
                extent: EXTENT {
                    width: 0,
                    height: 2,
                },
            };
            assert_eq!(
                valid_rect(bounds, hotspot, disabled, &mut rect, &mut origin),
                1
            );
            assert_eq!(
                (rect.extent.width, rect.extent.height, origin.x, origin.y),
                (20, 10, 2, 3)
            );
        }
    }
}
