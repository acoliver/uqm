#![cfg(not(test))]
#![allow(non_snake_case)]

use crate::memory::{rust_hfree as HFree, rust_hmalloc as HMalloc};
use crate::navigation::abi::*;
use std::ptr;

const HIGHLIGHT: u32 = 0;
const TRANSITION: u32 = 1;
const OVERLAY: u32 = 2;
const FADE_IN: u32 = 0;
const ON: u32 = 1;
const FADE_OUT: u32 = 2;
const OFF: u32 = 3;
const DEFAULT_CACHE_SIZE: u16 = 9;
const DRAW_REPLACE: u8 = 0;
const DRAW_ADDITIVE: u8 = 1;
const DRAW_ALPHA: u8 = 2;
const DRAW_FACTOR_1: i32 = 255;
const WANT_PIXMAP: u8 = 2;

pub use crate::navigation::abi::FlashContext;

unsafe fn cache_at(ctx: &FlashContext, i: usize) -> FRAME {
    *ctx.cache.add(i)
}
unsafe fn set_cache(ctx: &mut FlashContext, i: usize, frame: FRAME) {
    *ctx.cache.add(i) = frame;
}
unsafe fn make_mode(mode: u8, factor: i32) -> DrawMode {
    DrawMode {
        kind: mode,
        factor: factor as i16,
    }
}

unsafe fn create(gfx: CONTEXT) -> *mut FlashContext {
    let ctx = HMalloc(std::mem::size_of::<FlashContext>()).cast::<FlashContext>();
    ptr::write(ctx, std::mem::zeroed());
    (*ctx).gfxContext = gfx;
    (*ctx).endNumer = 1;
    (*ctx).denom = 1;
    (*ctx).onTime = ONE_SECOND / 8;
    (*ctx).offTime = ONE_SECOND / 8;
    (*ctx).state = OFF;
    (*ctx).cacheSize = DEFAULT_CACHE_SIZE;
    (*ctx).lastFrameIndex = u16::MAX;
    let _ = get_work_context();
    ctx
}
unsafe fn clear_cache(ctx: &mut FlashContext) {
    for i in 0..ctx.cacheSize as usize {
        let frame = cache_at(ctx, i);
        if !frame.is_null() {
            DestroyDrawable(ReleaseDrawable(frame));
            set_cache(ctx, i, ptr::null_mut());
        }
    }
}
unsafe fn init_cache(ctx: &mut FlashContext) {
    ctx.cache = HMalloc(ctx.cacheSize as usize * std::mem::size_of::<FRAME>()).cast();
    for i in 0..ctx.cacheSize as usize {
        set_cache(ctx, i, ptr::null_mut());
    }
}
unsafe fn grab_original(ctx: &mut FlashContext) {
    if !ctx.original.is_null() {
        DestroyDrawable(ReleaseDrawable(ctx.original));
    }
    let old = SetContext(ctx.gfxContext);
    ctx.original = CaptureDrawable(CopyContextRect(&ctx.rect));
    SetContext(old);
    FlushGraphics();
}
unsafe fn draw_frame(ctx: &FlashContext, frame: FRAME) {
    let old = SetContext(ctx.gfxContext);
    let mut stamp = STAMP {
        origin: ctx.rect.corner,
        frame,
    };
    DrawStamp(&mut stamp);
    SetContext(old);
}
unsafe fn blend(ctx: &FlashContext, n: i32, d: i32) -> (i32, i32) {
    assert!(n >= 0 && n <= d);
    super::flash_math::blend_fraction(n, d, ctx.startNumer, ctx.endNumer, ctx.denom)
}
unsafe fn make_frame(ctx: &FlashContext, dest: FRAME, n: i32, d: i32) {
    let (bn, bd) = blend(ctx, n, d);
    let old = SetContext(get_work_context());
    SetContextFGFrame(dest);
    let mut stamp = STAMP {
        origin: POINT { x: 0, y: 0 },
        frame: ptr::null_mut(),
    };
    match ctx.type_ {
        HIGHLIGHT => {
            SetContextBackGroundColor(Color {
                r: 0,
                g: 0,
                b: 0,
                a: 0,
            });
            ClearDrawable();
            SetContextDrawMode(make_mode(DRAW_ADDITIVE, DRAW_FACTOR_1 * bn / bd));
            stamp.frame = ctx.original;
            DrawStamp(&mut stamp);
        }
        TRANSITION => {
            let u = ctx.u.transition;
            SetContextDrawMode(make_mode(DRAW_REPLACE, 0));
            stamp.frame = if u.first.is_null() {
                ctx.original
            } else {
                u.first
            };
            DrawStamp(&mut stamp);
            SetContextDrawMode(make_mode(DRAW_ALPHA, DRAW_FACTOR_1 * bn / bd));
            stamp.frame = if u.final_.is_null() {
                ctx.original
            } else {
                u.final_
            };
            DrawStamp(&mut stamp);
        }
        OVERLAY => {
            SetContextDrawMode(make_mode(DRAW_REPLACE, 0));
            stamp.frame = ctx.original;
            DrawStamp(&mut stamp);
            SetContextDrawMode(make_mode(DRAW_ADDITIVE, DRAW_FACTOR_1 * bn / bd));
            stamp.frame = ctx.u.overlay.frame;
            let origin = SetContextOrigin(GetFrameHot(stamp.frame));
            DrawStamp(&mut stamp);
            SetContextOrigin(origin);
        }
        _ => unreachable!(),
    }
    SetContext(old);
}
static mut WORK_CONTEXT: CONTEXT = ptr::null_mut();
unsafe fn get_work_context() -> CONTEXT {
    if WORK_CONTEXT.is_null() {
        #[cfg(feature = "debug-process")]
        {
            WORK_CONTEXT = CreateContextAux(c"Flash.workGfxContext".as_ptr());
        }
        #[cfg(not(feature = "debug-process"))]
        {
            WORK_CONTEXT = CreateContextAux();
        }
    }
    WORK_CONTEXT
}
unsafe fn prepare_cache(ctx: &mut FlashContext, i: usize) {
    if !cache_at(ctx, i).is_null() {
        return;
    }
    let frame = CaptureDrawable(CreateDrawable(
        WANT_PIXMAP,
        ctx.rect.extent.width,
        ctx.rect.extent.height,
        1,
    ));
    set_cache(ctx, i, frame);
    make_frame(ctx, frame, i as i32, ctx.cacheSize as i32 - 1);
}
unsafe fn draw_current(ctx: &mut FlashContext) {
    let (n, d) = match ctx.state {
        OFF => (0, 1),
        ON => (1, 1),
        state => {
            let now = GetTimeCounter();
            let d = if state == FADE_IN {
                ctx.fadeInTime as i32
            } else {
                ctx.fadeOutTime as i32
            };
            let mut n = now.wrapping_sub(ctx.lastStateTime) as i32;
            if n > d {
                n = d;
            }
            if state == FADE_OUT {
                n = ctx.fadeOutTime as i32 - n;
            }
            (n, d)
        }
    };
    if ctx.cacheSize == 0 {
        let work = CaptureDrawable(CreateDrawable(
            WANT_PIXMAP,
            ctx.rect.extent.width,
            ctx.rect.extent.height,
            1,
        ));
        make_frame(ctx, work, n, d);
        draw_frame(ctx, work);
        DestroyDrawable(ReleaseDrawable(work));
    } else {
        let pos = super::flash_math::cache_position(ctx.cacheSize, n, d);
        let i = pos as usize;
        prepare_cache(ctx, i);
        if ctx.lastFrameIndex != i as u16 {
            draw_frame(ctx, cache_at(ctx, i));
            ctx.lastFrameIndex = i as u16;
        }
    }
}
unsafe fn fix_state(ctx: &mut FlashContext) {
    loop {
        let duration = match ctx.state {
            FADE_IN => ctx.fadeInTime,
            ON => ctx.onTime,
            FADE_OUT => ctx.fadeOutTime,
            _ => ctx.offTime,
        };
        if duration != 0 {
            break;
        }
        ctx.state = (ctx.state + 1) & 3;
    }
}
unsafe fn next_state(ctx: &mut FlashContext) {
    ctx.state = (ctx.state + 1) & 3;
    fix_state(ctx);
}

/// # Safety
/// Game-thread access only; the context and all supplied graphics resources must be valid.
#[no_mangle]
pub unsafe extern "C" fn Flash_createHighlight(
    gfx: CONTEXT,
    rect: *const RECT,
) -> *mut FlashContext {
    let c = create(gfx);
    (*c).type_ = HIGHLIGHT;
    (*c).rect = if rect.is_null() {
        std::mem::zeroed()
    } else {
        *rect
    };
    c
}
/// # Safety
/// Game-thread access only; the context and all supplied graphics resources must be valid.
#[no_mangle]
pub unsafe extern "C" fn Flash_createTransition(
    gfx: CONTEXT,
    origin: *const POINT,
    first: FRAME,
    final_frame: FRAME,
) -> *mut FlashContext {
    let c = create(gfx);
    (*c).type_ = TRANSITION;
    (*c).u.transition.first = first;
    (*c).u.transition.final_ = final_frame;
    GetFrameRect(final_frame, &mut (*c).rect);
    (*c).rect.corner = *origin;
    c
}
/// # Safety
/// Game-thread access only; the context and all supplied graphics resources must be valid.
#[no_mangle]
pub unsafe extern "C" fn Flash_createOverlay(
    gfx: CONTEXT,
    origin: *const POINT,
    overlay: FRAME,
) -> *mut FlashContext {
    let c = create(gfx);
    (*c).type_ = OVERLAY;
    if origin.is_null() || overlay.is_null() {
        (*c).rect = std::mem::zeroed();
    } else {
        Flash_setOverlay(c, origin, overlay);
    }
    c
}
/// # Safety
/// Game-thread access only; the context and all supplied graphics resources must be valid.
#[no_mangle]
pub unsafe extern "C" fn Flash_setState(c: *mut FlashContext, state: u32, spent: TimeCount) {
    let now = GetTimeCounter();
    (*c).state = state;
    fix_state(&mut *c);
    (*c).lastStateTime = now.wrapping_sub(spent);
    (*c).lastFrameTime = now;
    if (*c).started != 0 {
        draw_current(&mut *c);
    }
}
/// # Safety
/// Game-thread access only; the context and all supplied graphics resources must be valid.
#[no_mangle]
pub unsafe extern "C" fn Flash_start(c: *mut FlashContext) {
    if (*c).started != 0 {
        log::warn!("Flash_start() called on already started FlashContext.");
        return;
    }
    init_cache(&mut *c);
    (*c).started = 1;
    (*c).paused = 0;
    grab_original(&mut *c);
    (*c).lastFrameIndex = 0;
    fix_state(&mut *c);
    draw_current(&mut *c);
}
/// # Safety
/// Game-thread access only; the context and all supplied graphics resources must be valid.
#[no_mangle]
pub unsafe extern "C" fn Flash_terminate(c: *mut FlashContext) {
    if (*c).started != 0 {
        draw_frame(&*c, (*c).original);
        clear_cache(&mut *c);
        HFree((*c).cache.cast());
        DestroyDrawable(ReleaseDrawable((*c).original));
    }
    HFree(c.cast());
}
/// # Safety
/// Game-thread access only; the context and all supplied graphics resources must be valid.
#[no_mangle]
pub unsafe extern "C" fn Flash_pause(c: *mut FlashContext) {
    (*c).paused = 1;
}
/// # Safety
/// Game-thread access only; the context and all supplied graphics resources must be valid.
#[no_mangle]
pub unsafe extern "C" fn Flash_continue(c: *mut FlashContext) {
    (*c).paused = 0;
}
/// # Safety
/// Game-thread access only; the context and all supplied graphics resources must be valid.
#[no_mangle]
pub unsafe extern "C" fn Flash_process(c: *mut FlashContext) {
    if (*c).started == 0 || (*c).paused != 0 {
        return;
    }
    let now = GetTimeCounter();
    match (*c).state {
        FADE_IN => {
            if now >= (*c).lastStateTime.wrapping_add((*c).fadeInTime) {
                next_state(&mut *c);
                (*c).lastStateTime = now;
            }
            (*c).lastFrameTime = now;
        }
        ON => {
            if now < (*c).lastStateTime.wrapping_add((*c).onTime) {
                return;
            }
            next_state(&mut *c);
            (*c).lastStateTime = now;
        }
        FADE_OUT => {
            if now >= (*c).lastStateTime.wrapping_add((*c).fadeOutTime) {
                next_state(&mut *c);
                (*c).lastStateTime = now;
            }
            (*c).lastFrameTime = now;
        }
        OFF => {
            if now < (*c).lastStateTime.wrapping_add((*c).offTime) {
                return;
            }
            next_state(&mut *c);
            (*c).lastStateTime = now;
        }
        _ => unreachable!(),
    }
    draw_current(&mut *c);
}
/// # Safety
/// Game-thread access only; the context and all supplied graphics resources must be valid.
#[no_mangle]
pub unsafe extern "C" fn Flash_setSpeed(
    c: *mut FlashContext,
    fi: TimeCount,
    on: TimeCount,
    fo: TimeCount,
    off: TimeCount,
) {
    (*c).fadeInTime = fi;
    (*c).onTime = on;
    (*c).fadeOutTime = fo;
    (*c).offTime = off;
}
/// # Safety
/// Game-thread access only; the context and all supplied graphics resources must be valid.
#[no_mangle]
pub unsafe extern "C" fn Flash_setMergeFactors(
    c: *mut FlashContext,
    start: i32,
    end: i32,
    denom: i32,
) {
    if (*c).started != 0 {
        draw_frame(&*c, (*c).original);
        clear_cache(&mut *c);
    }
    (*c).startNumer = start;
    (*c).endNumer = end;
    (*c).denom = denom;
    if (*c).started != 0 {
        grab_original(&mut *c);
        draw_current(&mut *c);
    }
}
/// # Safety
/// Game-thread access only; the context and all supplied graphics resources must be valid.
#[no_mangle]
pub unsafe extern "C" fn Flash_setFrameTime(c: *mut FlashContext, t: TimeCount) {
    (*c).frameTime = t;
}
/// # Safety
/// Game-thread access only; the context and all supplied graphics resources must be valid.
#[no_mangle]
pub unsafe extern "C" fn Flash_nextTime(c: *mut FlashContext) -> TimeCount {
    if (*c).started == 0 || (*c).paused != 0 {
        return TimeCount::MAX;
    }
    if (*c).state == FADE_IN || (*c).state == FADE_OUT {
        (*c).lastFrameTime.wrapping_add((*c).frameTime)
    } else if (*c).state == ON {
        (*c).lastStateTime.wrapping_add((*c).onTime)
    } else {
        (*c).lastStateTime.wrapping_add((*c).offTime)
    }
}
/// # Safety
/// Game-thread access only; the context and all supplied graphics resources must be valid.
#[no_mangle]
pub unsafe extern "C" fn Flash_setRect(c: *mut FlashContext, rect: *const RECT) {
    assert_eq!((*c).type_, HIGHLIGHT);
    if (*c).started != 0 {
        draw_frame(&*c, (*c).original);
        clear_cache(&mut *c);
    }
    (*c).rect = *rect;
    (*c).lastFrameIndex = u16::MAX;
    if (*c).started != 0 {
        grab_original(&mut *c);
        draw_current(&mut *c);
    }
}
/// # Safety
/// Game-thread access only; the context and all supplied graphics resources must be valid.
#[no_mangle]
pub unsafe extern "C" fn Flash_getRect(c: *mut FlashContext, rect: *mut RECT) {
    assert_ne!((*c).type_, HIGHLIGHT);
    *rect = (*c).rect;
}
/// # Safety
/// Game-thread access only; the context and all supplied graphics resources must be valid.
#[no_mangle]
pub unsafe extern "C" fn Flash_setOverlay(
    c: *mut FlashContext,
    origin: *const POINT,
    overlay: FRAME,
) {
    assert_eq!((*c).type_, OVERLAY);
    if (*c).started != 0 {
        draw_frame(&*c, (*c).original);
        clear_cache(&mut *c);
    }
    (*c).u.overlay.frame = overlay;
    GetFrameRect(overlay, &mut (*c).rect);
    (*c).rect.corner.x += (*origin).x;
    (*c).rect.corner.y += (*origin).y;
    if (*c).started != 0 {
        grab_original(&mut *c);
        draw_current(&mut *c);
    }
}
/// # Safety
/// Game-thread access only; the context and all supplied graphics resources must be valid.
#[no_mangle]
pub unsafe extern "C" fn Flash_preUpdate(c: *mut FlashContext) {
    if (*c).started != 0 {
        draw_frame(&*c, (*c).original);
        clear_cache(&mut *c);
    }
}
/// # Safety
/// Game-thread access only; the context and all supplied graphics resources must be valid.
#[no_mangle]
pub unsafe extern "C" fn Flash_postUpdate(c: *mut FlashContext) {
    if (*c).started != 0 {
        grab_original(&mut *c);
        draw_current(&mut *c);
    }
}
/// # Safety
/// Game-thread access only; the context and all supplied graphics resources must be valid.
#[no_mangle]
pub unsafe extern "C" fn Flash_setCacheSize(c: *mut FlashContext, size: u16) {
    assert!(size == 0 || size >= 2);
    if !(*c).cache.is_null() {
        clear_cache(&mut *c);
        HFree((*c).cache.cast());
        (*c).cache = ptr::null_mut();
    }
    (*c).cacheSize = size;
    if size != 0 {
        init_cache(&mut *c);
    }
}
/// # Safety
/// Game-thread access only; the context and all supplied graphics resources must be valid.
#[no_mangle]
pub unsafe extern "C" fn Flash_getCacheSize(c: *const FlashContext) -> u16 {
    (*c).cacheSize
}
