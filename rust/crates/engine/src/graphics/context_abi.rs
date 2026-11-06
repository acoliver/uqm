//! Graphics context ownership and API, ported from context.c (GPL-2.0-or-later).
use super::drawable_abi::{CopyFrameRect, SetFrameHot};
use super::loaddisp::LoadDisplayPixmap;
use super::native_abi::*;
pub use super::native_context::{ContextFrame, ContextLease};
use crate::memory::{rust_hcalloc, rust_hfree};
#[cfg(feature = "debug-process")]
use std::ffi::c_char;
use std::ptr;

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
pub static mut _locPrim: PRIMITIVE = PRIMITIVE {
    Links: 0,
    Type: 0,
    color: Color {
        r: 0,
        g: 0,
        b: 0,
        a: 0,
    },
    Object: PRIM_DESC {
        Point: POINT { x: 0, y: 0 },
    },
};
#[no_mangle]
pub static mut _CurFontPtr: *mut font_desc = ptr::null_mut();
#[cfg(feature = "debug-process")]
pub static mut FIRST_CONTEXT: *mut context_desc = ptr::null_mut();
#[cfg(feature = "debug-process")]
pub static mut CONTEXT_END: *mut *mut context_desc = ptr::addr_of_mut!(FIRST_CONTEXT);

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
        super::native_context::register(
            std::ptr::NonNull::new(context).unwrap_or_else(|| unreachable!()),
        );
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
        *CONTEXT_END = context;
        CONTEXT_END = ptr::addr_of_mut!((*context).next);
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
    super::native_context::release(
        std::ptr::NonNull::new(context).unwrap_or_else(|| unreachable!()),
    );
    1
}

pub(super) unsafe fn free_context(allocation: std::ptr::NonNull<context_desc>) {
    let context = allocation.as_ptr();
    // SetContext mutates the outgoing descriptor, so deselect before borrowing it.
    if _pCurContext == context {
        SetContext(ptr::null_mut());
    }
    let descriptor = allocation.as_ref();
    #[cfg(feature = "debug-process")]
    {
        let mut link = ptr::addr_of_mut!(FIRST_CONTEXT);
        while !(*link).is_null() && *link != context {
            link = ptr::addr_of_mut!((**link).next);
        }
        if CONTEXT_END == ptr::from_ref(&descriptor.next).cast_mut() {
            CONTEXT_END = link;
        }
        *link = descriptor.next;
    }
    if !descriptor.FontBacking.is_null() {
        TFB_DrawScreen_DeleteImage(descriptor.FontBacking);
    }
    rust_hfree(context.cast());
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
    FIRST_CONTEXT
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

    fn allocated_context() -> *mut context_desc {
        // SAFETY: the serial context tests exclusively own context allocation and destruction.
        unsafe {
            #[cfg(feature = "debug-process")]
            {
                CreateContextAux(c"lease-test".as_ptr())
            }
            #[cfg(not(feature = "debug-process"))]
            {
                CreateContextAux()
            }
        }
    }

    #[test]
    #[serial_test::serial]
    fn retained_context_survives_native_owner_release() {
        let raw = allocated_context();
        let context = ContextLease::retain(std::ptr::NonNull::new(raw).unwrap());
        // SAFETY: this consumes the allocation's native owner; the typed lease remains live.
        assert_eq!(unsafe { DestroyContext(raw) }, 1);
        context.set_background(Color {
            r: 17,
            g: 31,
            b: 47,
            a: 255,
        });
        // SAFETY: context still owns the registered allocation.
        assert_eq!(unsafe { (*raw).BackGroundColor.r }, 17);
        drop(context);
        // SAFETY: the serial test owns the native context state.
        assert!(unsafe { _pCurContext }.is_null());
    }

    #[test]
    #[serial_test::serial]
    fn retained_batch_restores_the_previous_context_and_depth_after_unwinding() {
        let previous_raw = allocated_context();
        let current_raw = allocated_context();
        let previous = ContextLease::retain(
            std::ptr::NonNull::new(previous_raw)
                .unwrap_or_else(|| panic!("fixture context allocation failed")),
        );
        let current = ContextLease::retain(
            std::ptr::NonNull::new(current_raw)
                .unwrap_or_else(|| panic!("fixture context allocation failed")),
        );
        previous.select();
        // SAFETY: this serial test exclusively owns native context and batch state.
        let state = || unsafe {
            (
                std::ptr::addr_of!(_pCurContext).read(),
                super::super::gfx_common_abi::get_batch_depth(),
            )
        };
        let depth = state().1;
        assert_eq!(
            current.draw_batch(|| {
                assert_eq!(state(), (current_raw, depth + 1));
                previous.draw_batch(|| assert_eq!(state(), (previous_raw, depth + 2)));
                assert_eq!(state(), (current_raw, depth + 1));
                {
                    let _pause = current.pause_batch();
                    assert_eq!(state(), (current_raw, depth));
                }
                assert_eq!(state(), (current_raw, depth + 1));
                let rejected = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    let _pause = current.pause_batch();
                    panic!("paused drawing failure");
                }));
                assert!(rejected.is_err());
                assert_eq!(state(), (current_raw, depth + 1));
                17
            }),
            17
        );
        assert_eq!(state(), (previous_raw, depth));
        let rejected = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            current.draw_batch(|| panic!("fixture drawing failure"));
        }));
        assert!(rejected.is_err());
        assert_eq!(state(), (previous_raw, depth));
        // SAFETY: both fixture contexts remain owned while clearing the selection.
        unsafe { SetContext(std::ptr::null_mut()) };
        assert_eq!(current.draw_batch(|| 21), 21);
        assert_eq!(state(), (std::ptr::null_mut(), depth));
        // SAFETY: the serial test consumes both fixture allocation owners exactly once.
        unsafe {
            assert_eq!(DestroyContext(current_raw), 1);
            assert_eq!(DestroyContext(previous_raw), 1);
        }
        drop(current);
        drop(previous);
    }

    #[test]
    #[serial_test::serial]
    fn scoped_clip_and_origin_restore_after_unwind_and_target_changes() {
        let raw = allocated_context();
        let context = ContextLease::retain(std::ptr::NonNull::new(raw).unwrap());
        // SAFETY: the serial fixture owns this new two-frame drawable and its context.
        let drawable = unsafe { super::super::drawable_abi::AllocDrawable(2) };
        // SAFETY: both frames belong to this initialized allocation.
        let (first, second) = unsafe {
            let first = super::super::pixmap_abi::CaptureDrawable(drawable);
            (
                first,
                super::super::drawable_abi::retain_frame_allocations(&[first])[0]
                    .frame(1)
                    .unwrap(),
            )
        };
        let first = ContextFrame::retain(std::ptr::NonNull::new(first));
        let second = ContextFrame::retain(std::ptr::NonNull::new(second));
        first.replace_hotspot(POINT { x: -3, y: 7 });
        second.replace_hotspot(POINT { x: 9, y: 8 });
        context.set_frame(first.clone());
        context.clear_clip();
        let clip = RECT {
            corner: POINT { x: 1, y: 2 },
            extent: EXTENT {
                width: 3,
                height: 4,
            },
        };
        let rejected = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            context.with_clip(clip, || {
                assert_eq!(context.clip_rect().extent.width, 3);
                context.with_origin(POINT { x: 4, y: 5 }, || {
                    assert_eq!((first.rect().corner.x, first.rect().corner.y), (-4, -5));
                    context.set_frame(second.clone());
                    panic!("fixture drawing failure");
                });
            });
        }));
        assert!(rejected.is_err());
        assert_eq!((first.rect().corner.x, first.rect().corner.y), (3, -7));
        assert_eq!((second.rect().corner.x, second.rect().corner.y), (-9, -8));
        // SAFETY: inspect the retained context, then release only the fixture's native owners.
        unsafe {
            assert_eq!((*raw).ClipRect.extent.width, 0);
            assert_eq!(super::super::drawable_abi::DestroyDrawable(drawable), 1);
            assert_eq!(DestroyContext(raw), 1);
        }
    }

    #[test]
    #[serial_test::serial]
    fn context_retention_rejects_unowned_and_retired_addresses() {
        assert!(
            std::panic::catch_unwind(|| ContextLease::retain(std::ptr::NonNull::dangling()))
                .is_err()
        );
        let raw = allocated_context();
        // SAFETY: the native owner is consumed without any retained lease.
        assert_eq!(unsafe { DestroyContext(raw) }, 1);
        assert!(std::panic::catch_unwind(|| ContextLease::retain(
            std::ptr::NonNull::new(raw).unwrap()
        ))
        .is_err());
    }

    #[test]
    #[serial_test::serial]
    fn a_context_target_retains_its_frame_after_native_drawable_release() {
        let raw = allocated_context();
        let context = ContextLease::retain(std::ptr::NonNull::new(raw).unwrap());
        // SAFETY: the serial test exclusively owns the newly allocated drawable and context.
        let drawable = unsafe { super::super::drawable_abi::AllocDrawable(1) };
        // SAFETY: AllocDrawable initialized the drawable's first frame.
        let frame = unsafe { super::super::pixmap_abi::CaptureDrawable(drawable) };
        context.set_frame(ContextFrame::retain(std::ptr::NonNull::new(frame)));
        assert_eq!(
            // SAFETY: the context retains the allocation after its native owner is retired.
            unsafe { super::super::drawable_abi::DestroyDrawable(drawable) },
            1
        );
        context.set_clip(RECT {
            corner: POINT { x: 1, y: 2 },
            extent: EXTENT {
                width: 3,
                height: 4,
            },
        });
        let source = ContextFrame::retain(std::ptr::NonNull::new(frame));
        context.draw_stamp(&source, POINT { x: 7, y: 11 });
        // SAFETY: the context and its target remain retained by the lease.
        assert_eq!(unsafe { (*raw).ClipRect.extent.width }, 3);
        // SAFETY: consume the native context owner before releasing its retained lease.
        assert_eq!(unsafe { DestroyContext(raw) }, 1);
        drop(context);
        assert_eq!(
            super::super::drawable_abi::retain_frame_allocations(&[frame])[0].frame(0),
            Some(frame)
        );
        drop(source);
        assert!(std::panic::catch_unwind(|| {
            super::super::drawable_abi::retain_frame_allocations(&[frame])
        })
        .is_err());
    }

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
