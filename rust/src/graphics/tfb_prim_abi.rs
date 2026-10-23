//! Primitive dispatch, ported from tfb_prim.c (GPL-2.0-or-later).
use super::native_abi::*;
use std::ptr;
extern "C" {
    fn GetGraphicScale() -> i32;
    fn GetGraphicScaleMode() -> i32;
    fn LockMutex(mutex: *mut std::ffi::c_void);
    fn UnlockMutex(mutex: *mut std::ffi::c_void);
}
fn scaled_extent(extent: i16, scale: i32) -> i32 {
    (i32::from(extent) * scale + (256 >> 1)) / 256
}
unsafe fn draw_rect(rect: &mut RECT, color: Color, mode: DrawMode) {
    if (*_CurFramePtr).Type == SCREEN_DRAWABLE as u16 {
        TFB_DrawScreen_Rect(rect, color, mode, 0);
    } else {
        TFB_DrawImage_Rect(rect, color, mode, (*_CurFramePtr).image);
    }
}
/// # Safety
/// Point and active frame/context must be live on the serialized graphics thread.
#[no_mangle]
pub unsafe extern "C" fn TFB_Prim_Point(
    point: *mut POINT,
    color: Color,
    mode: DrawMode,
    origin: POINT,
) {
    let mut r = RECT {
        corner: POINT {
            x: (*point).x.wrapping_add(origin.x),
            y: (*point).y.wrapping_add(origin.y),
        },
        extent: EXTENT {
            width: 1,
            height: 1,
        },
    };
    draw_rect(&mut r, color, mode);
}
/// # Safety
/// Rectangle and active frame/context must be live on the serialized graphics thread.
#[no_mangle]
pub unsafe extern "C" fn TFB_Prim_Rect(
    rect: *mut RECT,
    color: Color,
    mode: DrawMode,
    origin: POINT,
) {
    let scale = GetGraphicScale();
    let r = *rect;
    let mut arm = r;
    arm.extent.height = 1;
    TFB_Prim_FillRect(&mut arm, color, mode, origin);
    arm.extent.height = r.extent.height;
    arm.extent.width = 1;
    TFB_Prim_FillRect(&mut arm, color, mode, origin);
    arm.corner.x = (i32::from(arm.corner.x) + scaled_extent(r.extent.width, scale) - 1) as i16;
    TFB_Prim_FillRect(&mut arm, color, mode, origin);
    arm.corner.x = r.corner.x;
    arm.corner.y = (i32::from(arm.corner.y) + scaled_extent(r.extent.height, scale) - 1) as i16;
    arm.extent.width = r.extent.width;
    arm.extent.height = 1;
    TFB_Prim_FillRect(&mut arm, color, mode, origin);
}
/// # Safety
/// Rectangle and active frame/context must be live on the serialized graphics thread.
#[no_mangle]
pub unsafe extern "C" fn TFB_Prim_FillRect(
    rect: *mut RECT,
    color: Color,
    mode: DrawMode,
    origin: POINT,
) {
    let mut r = *rect;
    r.corner.x = r.corner.x.wrapping_add(origin.x);
    r.corner.y = r.corner.y.wrapping_add(origin.y);
    let scale = GetGraphicScale();
    if scale != 256 {
        r.extent.width = scaled_extent(r.extent.width, scale) as i16;
        r.extent.height = scaled_extent(r.extent.height, scale) as i16;
    }
    draw_rect(&mut r, color, mode);
}
/// # Safety
/// Line and active frame/context must be live on the serialized graphics thread.
#[no_mangle]
pub unsafe extern "C" fn TFB_Prim_Line(
    line: *mut LINE,
    color: Color,
    mode: DrawMode,
    origin: POINT,
) {
    let x1 = i32::from((*line).first.x) + i32::from(origin.x);
    let y1 = i32::from((*line).first.y) + i32::from(origin.y);
    let x2 = i32::from((*line).second.x) + i32::from(origin.x);
    let y2 = i32::from((*line).second.y) + i32::from(origin.y);
    if (*_CurFramePtr).Type == SCREEN_DRAWABLE as u16 {
        TFB_DrawScreen_Line(x1, y1, x2, y2, color, mode, 0);
    } else {
        TFB_DrawImage_Line(x1, y1, x2, y2, color, mode, (*_CurFramePtr).image);
    }
}
/// # Safety
/// Stamp, source image and active frame/context must be live on graphics thread.
#[no_mangle]
pub unsafe extern "C" fn TFB_Prim_Stamp(stamp: *mut STAMP, mode: DrawMode, origin: POINT) {
    let frame = (*stamp).frame;
    if frame.is_null() {
        log::warn!("TFB_Prim_Stamp: Tried to draw a NULL frame");
        return;
    }
    let image = (*frame).image;
    if image.is_null() {
        log::warn!("Non-existent image to TFB_Prim_Stamp()");
        return;
    }
    LockMutex((*image).mutex.cast());
    (*image).NormalHs = (*frame).HotSpot;
    let x = i32::from((*stamp).origin.x) + i32::from(origin.x);
    let y = i32::from((*stamp).origin.y) + i32::from(origin.y);
    let cmap =
        if TFB_DrawCanvas_IsPaletted((*image).NormalImg) != 0 && (*image).colormap_index != -1 {
            TFB_GetColorMap((*image).colormap_index)
        } else {
            ptr::null_mut()
        };
    UnlockMutex((*image).mutex.cast());
    if (*_CurFramePtr).Type == SCREEN_DRAWABLE as u16 {
        TFB_DrawScreen_Image(
            image,
            x,
            y,
            GetGraphicScale(),
            GetGraphicScaleMode(),
            cmap,
            mode,
            0,
        );
    } else {
        TFB_DrawImage_Image(
            image,
            x,
            y,
            GetGraphicScale(),
            GetGraphicScaleMode(),
            cmap,
            mode,
            (*_CurFramePtr).image,
        );
    }
}
/// # Safety
/// Stamp, source image and active frame/context must be live on graphics thread.
#[no_mangle]
pub unsafe extern "C" fn TFB_Prim_StampFill(
    stamp: *mut STAMP,
    color: Color,
    mode: DrawMode,
    origin: POINT,
) {
    let frame = (*stamp).frame;
    if frame.is_null() {
        log::warn!("TFB_Prim_StampFill: Tried to draw a NULL frame");
        return;
    }
    let image = (*frame).image;
    if image.is_null() {
        log::warn!("Non-existent image to TFB_Prim_StampFill()");
        return;
    }
    LockMutex((*image).mutex.cast());
    (*image).NormalHs = (*frame).HotSpot;
    let x = i32::from((*stamp).origin.x) + i32::from(origin.x);
    let y = i32::from((*stamp).origin.y) + i32::from(origin.y);
    UnlockMutex((*image).mutex.cast());
    if (*_CurFramePtr).Type == SCREEN_DRAWABLE as u16 {
        TFB_DrawScreen_FilledImage(
            image,
            x,
            y,
            GetGraphicScale(),
            GetGraphicScaleMode(),
            color,
            mode,
            0,
        );
    } else {
        TFB_DrawImage_FilledImage(
            image,
            x,
            y,
            GetGraphicScale(),
            GetGraphicScaleMode(),
            color,
            mode,
            (*_CurFramePtr).image,
        );
    }
}
/// # Safety
/// Glyph/backing and active frame/context must be live on graphics thread.
#[no_mangle]
pub unsafe extern "C" fn TFB_Prim_FontChar(
    char_origin: POINT,
    glyph: *mut TFB_Char,
    backing: *mut TFB_Image,
    mode: DrawMode,
    origin: POINT,
) {
    let x = i32::from(char_origin.x) + i32::from(origin.x);
    let y = i32::from(char_origin.y) + i32::from(origin.y);
    if (*_CurFramePtr).Type == SCREEN_DRAWABLE as u16 {
        TFB_DrawScreen_FontChar(glyph, backing, x, y, mode, 0);
    } else {
        TFB_DrawImage_FontChar(glyph, backing, x, y, mode, (*_CurFramePtr).image);
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fixed_point_rounding_promotes_before_multiplication() {
        assert_eq!(scaled_extent(320, 256), 320);
        assert_eq!(scaled_extent(321, 128), 161);
        assert_eq!(scaled_extent(-3, 128), -1);
        assert_eq!(scaled_extent(32767, 512), 65534);
    }
}
