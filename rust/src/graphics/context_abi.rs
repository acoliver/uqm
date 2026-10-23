//! Graphics context ownership and API, ported from context.c (GPL-2.0-or-later).
use super::native_abi::*;
use crate::memory::{rust_hcalloc, rust_hfree};
use std::{ffi::c_char, ptr};

const FORE: Color = Color {
    r: 255,
    g: 255,
    b: 255,
    a: 255,
};
const BACK: Color = Color {
    r: 0,
    g: 0,
    b: 0,
    a: 255,
};
const MODE: DrawMode = DrawMode {
    kind: 0,
    factor: 255,
};

#[no_mangle]
pub static mut _GraphicsStatusFlags: u8 = 0;
#[no_mangle]
pub static mut _pCurContext: *mut context_desc = ptr::null_mut();
#[no_mangle]
pub static mut _locPrim: PRIMITIVE = unsafe { std::mem::zeroed() };
#[no_mangle]
pub static mut _CurFontPtr: *mut font_desc = ptr::null_mut();
#[cfg(feature = "debug-process")]
#[no_mangle]
pub static mut firstContext: *mut context_desc = ptr::null_mut();
#[cfg(feature = "debug-process")]
#[no_mangle]
pub static mut contextEnd: *mut *mut context_desc = ptr::addr_of_mut!(firstContext);

extern "C" {
    fn SetFrameHot(frame: *mut frame_desc, hot: HOT_SPOT) -> HOT_SPOT;
    fn LoadDisplayPixmap(area: *const RECT, frame: *mut frame_desc) -> *mut drawable_desc;
    fn CopyFrameRect(frame: *mut frame_desc, area: *const RECT) -> *mut drawable_desc;
}

/// # Safety
/// Context must be null or live, and context operations must be serialized.
#[no_mangle]
pub unsafe extern "C" fn SetContext(context: *mut context_desc) -> *mut context_desc {
    let last = _pCurContext;
    if context != last {
        if !last.is_null() {
            (*last).Flags &= !(9 << 8);
            (*last).Flags |= u16::from(_GraphicsStatusFlags & 9) << 8;
            _GraphicsStatusFlags &= !4;
        }
        _pCurContext = context;
        if !context.is_null() {
            _GraphicsStatusFlags |= 4;
            _GraphicsStatusFlags &= !9;
            _GraphicsStatusFlags |= ((*context).Flags >> 8) as u8;
            _locPrim.color = (*context).ForeGroundColor;
            _CurFramePtr = (*context).ForeGroundFrame;
            _CurFontPtr = (*context).Font;
        }
    }
    last
}
unsafe fn create_context() -> *mut context_desc {
    let context = rust_hcalloc(std::mem::size_of::<context_desc>()).cast::<context_desc>();
    if !context.is_null() {
        (*context).Mode = MODE;
        (*context).ForeGroundColor = FORE;
        (*context).BackGroundColor = BACK;
    }
    context
}
/// # Safety
/// Name must remain live until context destruction; operations must be serialized.
#[cfg(feature = "debug-process")]
#[no_mangle]
pub unsafe extern "C" fn CreateContextAux(name: *const c_char) -> *mut context_desc {
    let context = create_context();
    if !context.is_null() {
        (*context).name = name;
        (*context).next = ptr::null_mut();
        *contextEnd = context;
        contextEnd = ptr::addr_of_mut!((*context).next);
    }
    context
}
/// # Safety
/// Context allocation must be serialized with other graphics operations.
#[cfg(not(feature = "debug-process"))]
#[no_mangle]
pub unsafe extern "C" fn CreateContextAux() -> *mut context_desc {
    create_context()
}

/// # Safety
/// Context must be null or an owned context, consumed by this call.
#[no_mangle]
pub unsafe extern "C" fn DestroyContext(context: *mut context_desc) -> BOOLEAN {
    if context.is_null() {
        return 0;
    }
    if _pCurContext == context {
        SetContext(ptr::null_mut());
    }
    #[cfg(feature = "debug-process")]
    {
        let mut link = ptr::addr_of_mut!(firstContext);
        while !(*link).is_null() && *link != context {
            link = ptr::addr_of_mut!((**link).next);
        }
        if contextEnd == ptr::addr_of_mut!((*context).next) {
            contextEnd = link;
        }
        *link = (*context).next;
    }
    if !(*context).FontBacking.is_null() {
        TFB_DrawImage_Delete((*context).FontBacking);
    }
    rust_hfree(context.cast());
    1
}
/// # Safety
/// Called on the serialized graphics thread.
#[no_mangle]
pub unsafe extern "C" fn SetContextForeGroundColor(color: Color) -> Color {
    if _GraphicsStatusFlags & 4 == 0 {
        return FORE;
    }
    let old = (*_pCurContext).ForeGroundColor;
    if (old.r, old.g, old.b, old.a) != (color.r, color.g, color.b, color.a) {
        (*_pCurContext).ForeGroundColor = color;
        if (*_pCurContext).BackingFlags & 2 == 0 {
            (*_pCurContext).BackingFlags |= 1;
        }
    }
    _locPrim.color = color;
    old
}
/// # Safety
/// Called on the serialized graphics thread.
#[no_mangle]
pub unsafe extern "C" fn GetContextForeGroundColor() -> Color {
    if _GraphicsStatusFlags & 4 == 0 {
        FORE
    } else {
        (*_pCurContext).ForeGroundColor
    }
}
/// # Safety
/// Called on the serialized graphics thread.
#[no_mangle]
pub unsafe extern "C" fn SetContextBackGroundColor(color: Color) -> Color {
    if _GraphicsStatusFlags & 4 == 0 {
        return BACK;
    }
    let old = (*_pCurContext).BackGroundColor;
    (*_pCurContext).BackGroundColor = color;
    old
}
/// # Safety
/// Called on the serialized graphics thread.
#[no_mangle]
pub unsafe extern "C" fn GetContextBackGroundColor() -> Color {
    if _GraphicsStatusFlags & 4 == 0 {
        BACK
    } else {
        (*_pCurContext).BackGroundColor
    }
}
/// # Safety
/// Called on the serialized graphics thread.
#[no_mangle]
pub unsafe extern "C" fn SetContextDrawMode(mode: DrawMode) -> DrawMode {
    if _GraphicsStatusFlags & 4 == 0 {
        return MODE;
    }
    let old = (*_pCurContext).Mode;
    (*_pCurContext).Mode = mode;
    old
}
/// # Safety
/// Called on the serialized graphics thread.
#[no_mangle]
pub unsafe extern "C" fn GetContextDrawMode() -> DrawMode {
    if _GraphicsStatusFlags & 4 == 0 {
        MODE
    } else {
        (*_pCurContext).Mode
    }
}
unsafe fn foreground_rect() -> RECT {
    RECT {
        corner: POINT { x: 0, y: 0 },
        extent: if _CurFramePtr.is_null() {
            EXTENT {
                width: 0,
                height: 0,
            }
        } else {
            (*_CurFramePtr).Bounds
        },
    }
}
fn set_clip(context: &mut context_desc, rect: Option<RECT>, foreground: RECT) {
    if let Some(r) = rect {
        if (r.corner.x, r.corner.y, r.extent.width, r.extent.height)
            != (
                foreground.corner.x,
                foreground.corner.y,
                foreground.extent.width,
                foreground.extent.height,
            )
        {
            context.ClipRect = r;
            return;
        }
    }
    context.ClipRect.extent.width = 0;
}
/// # Safety
/// Optional rectangle must be readable; called on graphics thread.
#[no_mangle]
pub unsafe extern "C" fn SetContextClipRect(rect: *mut RECT) -> BOOLEAN {
    if _GraphicsStatusFlags & 4 == 0 {
        return 0;
    }
    set_clip(
        &mut *_pCurContext,
        rect.as_ref().copied(),
        foreground_rect(),
    );
    1
}
/// # Safety
/// Rectangle must be writable; called on graphics thread.
#[no_mangle]
pub unsafe extern "C" fn GetContextClipRect(rect: *mut RECT) -> BOOLEAN {
    if _GraphicsStatusFlags & 4 == 0 {
        return 0;
    }
    *rect = (*_pCurContext).ClipRect;
    let explicit = (*rect).extent.width != 0;
    if !explicit {
        *rect = foreground_rect();
    }
    u32::from(explicit)
}
/// # Safety
/// Called on the serialized graphics thread.
#[no_mangle]
pub unsafe extern "C" fn SetContextOrigin(origin: POINT) -> POINT {
    SetFrameHot(_CurFramePtr, origin)
}
/// # Safety
/// Effect must be null or a live frame on the graphics thread.
#[no_mangle]
pub unsafe extern "C" fn SetContextFontEffect(effect: *mut frame_desc) -> *mut frame_desc {
    if _GraphicsStatusFlags & 4 == 0 {
        return ptr::null_mut();
    }
    let old = (*_pCurContext).FontEffect;
    if effect != old {
        (*_pCurContext).FontEffect = effect;
        (*_pCurContext).BackingFlags |= 1;
        if effect.is_null() {
            (*_pCurContext).BackingFlags &= !2;
        } else {
            (*_pCurContext).BackingFlags |= 2;
        }
    }
    old
}
/// # Safety
/// Called on graphics thread with live font and context records.
#[no_mangle]
pub unsafe extern "C" fn FixContextFontEffect() {
    if _GraphicsStatusFlags & 4 == 0
        || (!(*_pCurContext).FontBacking.is_null() && (*_pCurContext).BackingFlags & 1 == 0)
    {
        return;
    }
    let (mut w, mut h) = (0, 0);
    if super::font_abi::GetContextFontLeading(&mut h) == 0
        || super::font_abi::GetContextFontLeadingWidth(&mut w) == 0
    {
        return;
    }
    if !(*_pCurContext).FontBacking.is_null() {
        TFB_DrawScreen_DeleteImage((*_pCurContext).FontBacking);
    }
    let image = TFB_DrawImage_CreateForScreen(w as i32, h as i32, 1);
    if (*_pCurContext).BackingFlags & 2 != 0 {
        let effect = (*_pCurContext).FontEffect;
        TFB_DrawImage_Image(
            (*effect).image,
            -i32::from((*effect).HotSpot.x),
            -i32::from((*effect).HotSpot.y),
            0,
            0,
            ptr::null_mut(),
            DrawMode { kind: 0, factor: 0 },
            image,
        );
    } else {
        let mut rect = RECT {
            corner: POINT { x: 0, y: 0 },
            extent: EXTENT {
                width: w,
                height: h,
            },
        };
        TFB_DrawImage_Rect(
            &mut rect,
            (*_pCurContext).ForeGroundColor,
            DrawMode { kind: 0, factor: 0 },
            image,
        );
    }
    (*_pCurContext).FontBacking = image;
    (*_pCurContext).BackingFlags &= !1;
}
/// # Safety
/// Optional area must be readable; active frame must be live on graphics thread.
#[no_mangle]
pub unsafe extern "C" fn CopyContextRect(area: *const RECT) -> *mut drawable_desc {
    if _GraphicsStatusFlags & 4 == 0 || _CurFramePtr.is_null() {
        return ptr::null_mut();
    }
    let mut foreground = foreground_rect();
    let mut r = std::mem::zeroed();
    GetContextClipRect(&mut r);
    if let Some(area) = area.as_ref() {
        r.corner.x = r.corner.x.wrapping_add(area.corner.x);
        r.corner.y = r.corner.y.wrapping_add(area.corner.y);
        r.extent = area.extent;
    }
    let output = ptr::addr_of_mut!(r);
    if BoxIntersect(output, &mut foreground, output) == 0 {
        return ptr::null_mut();
    }
    if (*_CurFramePtr).Type == SCREEN_DRAWABLE as u16 {
        LoadDisplayPixmap(&r, ptr::null_mut())
    } else {
        CopyFrameRect(_CurFramePtr, &r)
    }
}
/// # Safety
/// Context must be live on the serialized graphics thread.
#[cfg(feature = "debug-process")]
#[no_mangle]
pub unsafe extern "C" fn GetContextName(context: *mut context_desc) -> *const c_char {
    (*context).name
}
/// # Safety
/// Called on the serialized graphics thread.
#[cfg(feature = "debug-process")]
#[no_mangle]
pub unsafe extern "C" fn GetFirstContext() -> *mut context_desc {
    firstContext
}
/// # Safety
/// Context must be live on the serialized graphics thread.
#[cfg(feature = "debug-process")]
#[no_mangle]
pub unsafe extern "C" fn GetNextContext(context: *mut context_desc) -> *mut context_desc {
    (*context).next
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn full_frame_clip_is_undefined_and_partial_clip_is_preserved() {
        let mut context: context_desc = unsafe { std::mem::zeroed() };
        let full = RECT {
            corner: POINT { x: 0, y: 0 },
            extent: EXTENT {
                width: 7,
                height: 5,
            },
        };
        let partial = RECT {
            corner: POINT { x: -2, y: 1 },
            extent: EXTENT {
                width: 4,
                height: 3,
            },
        };
        set_clip(&mut context, Some(partial), full);
        assert_eq!(
            (context.ClipRect.corner.x, context.ClipRect.extent.width),
            (-2, 4)
        );
        set_clip(&mut context, Some(full), full);
        assert_eq!(context.ClipRect.extent.width, 0);
        assert_eq!(context.ClipRect.corner.x, -2);
        set_clip(&mut context, Some(partial), full);
        set_clip(&mut context, None, full);
        assert_eq!(context.ClipRect.extent.width, 0);
    }
}
