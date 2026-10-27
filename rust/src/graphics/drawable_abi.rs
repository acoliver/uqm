//! Drawable allocation and frame operations, ported from drawable.c (GPL-2.0-or-later).
use super::native_abi::*;
use super::pixmap_abi::{CaptureDrawable, ReleaseDrawable};
use crate::memory::{rust_hcalloc, rust_hfree, rust_hmalloc};
use std::ptr;

#[no_mangle]
pub static mut _CurFramePtr: *mut frame_desc = ptr::null_mut();

use crate::threading::legacy_abi::LockMutex;

use crate::threading::legacy_abi::UnlockMutex;

use crate::graphics::tfb_draw_abi::TFB_DrawImage_New;

use crate::graphics::native_canvas::scaling::TFB_DrawCanvas_Rescale_Nearest;

extern "C" {
    static mut ScreenWidth: i32;
    static mut ScreenHeight: i32;
}
/// # Safety
/// Frame must be null or live, and graphics operations must be serialized.
#[no_mangle]
pub unsafe extern "C" fn SetContextFGFrame(frame: *mut frame_desc) -> *mut frame_desc {
    let last = _CurFramePtr;
    if frame != last {
        if !last.is_null() {
            _GraphicsStatusFlags &= !8;
        }
        _CurFramePtr = frame;
        if !frame.is_null() {
            _GraphicsStatusFlags |= 8;
        }
        if _GraphicsStatusFlags & 4 != 0 {
            (*_pCurContext).ForeGroundFrame = frame;
        }
    }
    last
}
/// # Safety
/// Called on the serialized graphics thread.
#[no_mangle]
pub unsafe extern "C" fn GetContextFGFrame() -> *mut frame_desc {
    _CurFramePtr
}

unsafe fn request_drawable(
    count: u16,
    kind: u16,
    flags: u8,
    width: i16,
    height: i16,
) -> *mut drawable_desc {
    let drawable = AllocDrawable(count);
    if drawable.is_null() {
        return drawable;
    }
    (*drawable).Flags = flags;
    (*drawable).MaxIndex = count.wrapping_sub(1);
    for i in 0..count {
        let frame = &mut *(*drawable).Frame.add(i as usize);
        if kind == RAM_DRAWABLE as u16 && width > 0 && height > 0 {
            frame.image = TFB_DrawImage_New(TFB_DrawCanvas_New_TrueColor(
                i32::from(width),
                i32::from(height),
                u32::from(flags & 8 != 0),
            ))
            .cast();
        }
        frame.Type = kind;
        frame.Index = i;
        frame.Bounds = EXTENT { width, height };
    }
    drawable
}
/// # Safety
/// Outputs must be writable; called on the graphics thread.
#[no_mangle]
pub unsafe extern "C" fn CreateDisplay(
    flags: u8,
    width: *mut i16,
    height: *mut i16,
) -> *mut drawable_desc {
    let drawable = request_drawable(
        1,
        SCREEN_DRAWABLE as u16,
        flags & 3,
        ScreenWidth as i16,
        ScreenHeight as i16,
    );
    if !drawable.is_null() {
        let frame = CaptureDrawable(drawable);
        if frame.is_null() {
            DestroyDrawable(drawable);
        } else {
            *width = (*frame).Bounds.width;
            *height = (*frame).Bounds.height;
            ReleaseDrawable(frame);
            return drawable;
        }
    }
    *width = 0;
    *height = 0;
    ptr::null_mut()
}
/// # Safety
/// Allocation must be serialized with drawable use.
#[no_mangle]
pub unsafe extern "C" fn AllocDrawable(count: u16) -> *mut drawable_desc {
    let drawable = rust_hcalloc(std::mem::size_of::<drawable_desc>()).cast::<drawable_desc>();
    if !drawable.is_null() {
        (*drawable).Frame =
            rust_hmalloc(std::mem::size_of::<frame_desc>() * usize::from(count)).cast();
        if (*drawable).Frame.is_null() {
            rust_hfree(drawable.cast());
            return ptr::null_mut();
        }
        crate::collections::observation::record(crate::collections::observation::Event::Drawable(
            drawable as usize,
        ));
        for i in 0..count {
            let frame = (*drawable).Frame.add(i as usize);
            ptr::write(frame, std::mem::zeroed());
            (*frame).parent = drawable;
            crate::collections::observation::record(
                crate::collections::observation::Event::Frame {
                    address: frame as usize,
                    owner: drawable as usize,
                    index: i,
                },
            );
        }
    }
    drawable
}
/// # Safety
/// Called on the serialized graphics thread.
#[no_mangle]
pub unsafe extern "C" fn CreateDrawable(
    flags: u8,
    width: i16,
    height: i16,
    count: u16,
) -> *mut drawable_desc {
    let drawable = request_drawable(count, RAM_DRAWABLE as u16, flags & 15, width, height);
    if !drawable.is_null() {
        let frame = CaptureDrawable(drawable);
        if !frame.is_null() {
            ReleaseDrawable(frame);
            return drawable;
        }
    }
    ptr::null_mut()
}
/// # Safety
/// Drawable must be null or owned and live, consumed on the graphics thread.
#[no_mangle]
pub unsafe extern "C" fn DestroyDrawable(drawable: *mut drawable_desc) -> BOOLEAN {
    if !_CurFramePtr.is_null() && drawable == (*_CurFramePtr).parent {
        SetContextFGFrame(ptr::null_mut());
    }
    if drawable.is_null() {
        return 0;
    }
    crate::collections::observation::record(crate::collections::observation::Event::DrawableFree(
        drawable as usize,
    ));
    super::gfxload::release_cel_data(drawable.cast());
    1
}
/// # Safety
/// Frame must be null or live; output must be writable.
#[no_mangle]
pub unsafe extern "C" fn GetFrameRect(frame: *mut frame_desc, rect: *mut RECT) -> BOOLEAN {
    if frame.is_null() {
        return 0;
    }
    *rect = RECT {
        corner: POINT {
            x: (*frame).HotSpot.x.wrapping_neg(),
            y: (*frame).HotSpot.y.wrapping_neg(),
        },
        extent: (*frame).Bounds,
    };
    1
}
/// # Safety
/// Frame must be null or live with exclusive graphics-thread access.
#[no_mangle]
pub unsafe extern "C" fn SetFrameHot(frame: *mut frame_desc, hot: HOT_SPOT) -> HOT_SPOT {
    if frame.is_null() {
        return HOT_SPOT { x: 0, y: 0 };
    }
    let old = (*frame).HotSpot;
    (*frame).HotSpot = hot;
    old
}
/// # Safety
/// Frame must be null or live on graphics thread.
#[no_mangle]
pub unsafe extern "C" fn GetFrameHot(frame: *mut frame_desc) -> HOT_SPOT {
    if frame.is_null() {
        HOT_SPOT { x: 0, y: 0 }
    } else {
        (*frame).HotSpot
    }
}
fn rotate_hotspot(hot: HOT_SPOT, bounds: EXTENT, rotated: EXTENT, angle: i32) -> HOT_SPOT {
    let radians = f64::from(angle) * std::f64::consts::PI / 180.0;
    let mut dx = f64::from(i32::from(hot.x) - i32::from(bounds.width) / 2);
    let mut dy = f64::from(i32::from(hot.y) - i32::from(bounds.height) / 2);
    let distance = (dx * dx + dy * dy).sqrt();
    if distance as i32 != 0 {
        let original = (-dy).atan2(dx);
        dx = (original + radians).cos() * distance;
        dy = -(original + radians).sin() * distance;
    }
    HOT_SPOT {
        x: (i32::from(rotated.width) / 2 + dx as i32) as i16,
        y: (i32::from(rotated.height) / 2 + dy as i32) as i16,
    }
}
/// # Safety
/// Frame must be null or a live non-screen frame on graphics thread.
#[no_mangle]
pub unsafe extern "C" fn RotateFrame(frame: *mut frame_desc, angle: i32) -> *mut drawable_desc {
    if frame.is_null() {
        return ptr::null_mut();
    }
    assert_ne!((*frame).Type, SCREEN_DRAWABLE as u16);
    let drawable = request_drawable(1, RAM_DRAWABLE as u16, 2, 0, 0);
    if drawable.is_null() {
        return drawable;
    }
    let rotated = CaptureDrawable(drawable);
    if rotated.is_null() {
        crate::collections::observation::record(
            crate::collections::observation::Event::DrawableFree(drawable as usize),
        );
        super::gfxload::release_cel_data(drawable.cast());
        return ptr::null_mut();
    }
    (*rotated).image = TFB_DrawImage_New_Rotated((*frame).image, angle);
    (*rotated).Bounds = (*(*rotated).image).extent;
    (*rotated).HotSpot =
        rotate_hotspot((*frame).HotSpot, (*frame).Bounds, (*rotated).Bounds, angle);
    ReleaseDrawable(rotated);
    drawable
}
/// # Safety
/// Frame must be null or a live non-screen frame.
#[no_mangle]
pub unsafe extern "C" fn SetFrameTransparentColor(frame: *mut frame_desc, color: Color) {
    if frame.is_null() {
        return;
    }
    assert_ne!((*frame).Type, SCREEN_DRAWABLE as u16);
    let image = &*(*frame).image;
    LockMutex(image.mutex.cast());
    TFB_DrawCanvas_SetTransparentColor(image.NormalImg, color, 0);
    UnlockMutex(image.mutex.cast());
}
/// # Safety
/// Frame must be null or a live non-screen frame; point must be in bounds.
#[no_mangle]
pub unsafe extern "C" fn GetFramePixel(frame: *mut frame_desc, point: POINT) -> Color {
    if frame.is_null() {
        return Color {
            r: 0,
            g: 0,
            b: 0,
            a: 0,
        };
    }
    assert_ne!((*frame).Type, SCREEN_DRAWABLE as u16);
    let image = &*(*frame).image;
    LockMutex(image.mutex.cast());
    let color = TFB_DrawCanvas_GetPixel(image.NormalImg, i32::from(point.x), i32::from(point.y));
    UnlockMutex(image.mutex.cast());
    color
}
unsafe fn matching_frame(frame: *mut frame_desc, width: i32, height: i32) -> *mut frame_desc {
    let drawable = CreateDrawable((*(*frame).parent).Flags, width as i16, height as i16, 1);
    if drawable.is_null() {
        return ptr::null_mut();
    }
    let new = CaptureDrawable(drawable);
    if new.is_null() {
        crate::collections::observation::record(
            crate::collections::observation::Event::DrawableFree(drawable as usize),
        );
        super::gfxload::release_cel_data(drawable.cast());
    }
    new
}
/// # Safety
/// Source must be null or a live non-screen frame and area must be readable.
#[no_mangle]
pub unsafe extern "C" fn CopyFrameRect(
    frame: *mut frame_desc,
    area: *const RECT,
) -> *mut drawable_desc {
    if frame.is_null() {
        return ptr::null_mut();
    }
    assert_ne!((*frame).Type, SCREEN_DRAWABLE as u16);
    let new = matching_frame(
        frame,
        i32::from((*area).extent.width),
        i32::from((*area).extent.height),
    );
    if new.is_null() {
        return ptr::null_mut();
    }
    TFB_DrawImage_CopyRect((*frame).image, area, (*new).image, POINT { x: 0, y: 0 });
    ReleaseDrawable(new)
}
/// # Safety
/// Source must be null or a live non-screen frame on graphics thread.
#[no_mangle]
pub unsafe extern "C" fn CloneFrame(frame: *mut frame_desc) -> *mut drawable_desc {
    if frame.is_null() {
        return ptr::null_mut();
    }
    assert_ne!((*frame).Type, SCREEN_DRAWABLE as u16);
    let r = RECT {
        corner: POINT { x: 0, y: 0 },
        extent: (*frame).Bounds,
    };
    let new = CaptureDrawable(CopyFrameRect(frame, &r));
    if new.is_null() {
        return ptr::null_mut();
    }
    (*new).HotSpot = (*frame).HotSpot;
    ReleaseDrawable(new)
}
/// # Safety
/// Source must be null or a live non-screen frame with nonzero bounds.
#[no_mangle]
pub unsafe extern "C" fn RescaleFrame(
    frame: *mut frame_desc,
    width: i32,
    height: i32,
) -> *mut drawable_desc {
    if frame.is_null() {
        return ptr::null_mut();
    }
    assert_ne!((*frame).Type, SCREEN_DRAWABLE as u16);
    let new = matching_frame(frame, width, height);
    if new.is_null() {
        return ptr::null_mut();
    }
    (*new).HotSpot.x =
        (i32::from((*frame).HotSpot.x) * width / i32::from((*frame).Bounds.width)) as i16;
    (*new).HotSpot.y =
        (i32::from((*frame).HotSpot.y) * height / i32::from((*frame).Bounds.height)) as i16;
    let image = &*(*frame).image;
    LockMutex(image.mutex.cast());
    TFB_DrawCanvas_Rescale_Nearest(
        image.NormalImg,
        (*(*new).image).NormalImg,
        -1,
        ptr::null_mut(),
        ptr::null_mut(),
        ptr::null_mut(),
    );
    UnlockMutex(image.mutex.cast());
    ReleaseDrawable(new)
}
/// # Safety
/// Non-screen frame must be null or live; pixel buffer must cover width * height elements.
#[no_mangle]
pub unsafe extern "C" fn ReadFramePixelColors(
    frame: *mut frame_desc,
    pixels: *mut Color,
    width: i32,
    height: i32,
) -> BOOLEAN {
    if frame.is_null() {
        return 0;
    }
    assert_ne!((*frame).Type, SCREEN_DRAWABLE as u16);
    TFB_DrawCanvas_GetPixelColors((*(*frame).image).NormalImg, pixels, width, height)
}

/// # Safety
/// Non-screen frame must be null or live; pixel buffer must cover width * height elements.
#[no_mangle]
pub unsafe extern "C" fn WriteFramePixelColors(
    frame: *mut frame_desc,
    pixels: *const Color,
    width: i32,
    height: i32,
) -> BOOLEAN {
    if frame.is_null() {
        return 0;
    }
    assert_ne!((*frame).Type, SCREEN_DRAWABLE as u16);
    TFB_DrawCanvas_SetPixelColors((*(*frame).image).NormalImg, pixels, width, height)
}

/// # Safety
/// Non-screen frame must be null or live; pixel buffer must cover width * height elements.
#[no_mangle]
pub unsafe extern "C" fn ReadFramePixelIndexes(
    frame: *mut frame_desc,
    pixels: *mut u8,
    width: i32,
    height: i32,
) -> BOOLEAN {
    if frame.is_null() {
        return 0;
    }
    assert_ne!((*frame).Type, SCREEN_DRAWABLE as u16);
    TFB_DrawCanvas_GetPixelIndexes((*(*frame).image).NormalImg, pixels, width, height)
}

/// # Safety
/// Non-screen frame must be null or live; pixel buffer must cover width * height elements.
#[no_mangle]
pub unsafe extern "C" fn WriteFramePixelIndexes(
    frame: *mut frame_desc,
    pixels: *const u8,
    width: i32,
    height: i32,
) -> BOOLEAN {
    if frame.is_null() {
        return 0;
    }
    assert_ne!((*frame).Type, SCREEN_DRAWABLE as u16);
    TFB_DrawCanvas_SetPixelIndexes((*(*frame).image).NormalImg, pixels, width, height)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn frame_hotspot_and_bounds_preserve_signed_coordinates() {
        unsafe {
            let mut frame: frame_desc = std::mem::zeroed();
            frame.Bounds = EXTENT {
                width: 7,
                height: 5,
            };
            let previous = SetFrameHot(&mut frame, HOT_SPOT { x: -2, y: 3 });
            assert_eq!((previous.x, previous.y), (0, 0));
            let mut rect = std::mem::zeroed();
            assert_eq!(GetFrameRect(&mut frame, &mut rect), 1);
            assert_eq!(
                (
                    rect.corner.x,
                    rect.corner.y,
                    rect.extent.width,
                    rect.extent.height
                ),
                (2, -3, 7, 5)
            );
            assert_eq!(GetFrameRect(ptr::null_mut(), &mut rect), 0);
        }
    }
    #[test]
    fn rotation_truncates_hotspot_after_rotating_about_integer_center() {
        let bounds = EXTENT {
            width: 8,
            height: 6,
        };
        let hot = rotate_hotspot(
            HOT_SPOT { x: 6, y: 3 },
            bounds,
            EXTENT {
                width: 6,
                height: 8,
            },
            90,
        );
        assert_eq!((hot.x, hot.y), (3, 2));
        let center = rotate_hotspot(HOT_SPOT { x: 4, y: 3 }, bounds, bounds, 123);
        assert_eq!((center.x, center.y), (4, 3));
    }
}
