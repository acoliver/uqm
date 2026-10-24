//! Live draw command queue, ported from dcqueue.c (GPL-2.0-or-later).
pub use self::{
    tfb_batch_graphics as TFB_BatchGraphics, tfb_batch_reset as TFB_BatchReset,
    tfb_get_batch_depth as TFB_GetBatchDepth, tfb_unbatch_graphics as TFB_UnbatchGraphics,
};
use super::native_abi::*;
use crate::memory::rust_hfree;
use std::ptr;
const MAX: i32 = 16384;
const LIMIT: i32 = 4096;
static mut DCQ_MUTEX: *mut crate::threading::RustMutex = ptr::null_mut();
#[no_mangle]
pub static mut RenderingCond: *mut crate::threading::RustCondVar = ptr::null_mut();
pub static mut DCQ: [TFB_DrawCommand; MAX as usize] = unsafe { std::mem::zeroed() };
#[no_mangle]
pub static mut DrawCommandQueue: TFB_DrawCommandQueue = unsafe { std::mem::zeroed() };
pub static mut RENDERED_FRAMES: i32 = 0;
static mut SCISSOR: RECT = unsafe { std::mem::zeroed() };
static mut LAST_FADE: i32 = 255;
static mut LAST_TRANSITION: i32 = 255;
static mut LAST_TIME: u32 = 0;
static mut FPS_COUNTER: u32 = 0;

use crate::threading::legacy_abi::TaskSwitch;
use crate::time::counter_abi::GetTimeCounter;

use crate::threading::legacy_abi::CreateRecursiveMutex_Core;

use crate::threading::legacy_abi::DestroyRecursiveMutex;

use crate::threading::legacy_abi::LockRecursiveMutex;

use crate::threading::legacy_abi::UnlockRecursiveMutex;

use crate::threading::legacy_abi::GetRecursiveMutexDepth;

use crate::threading::legacy_abi::CreateCondVar_Core;

use crate::threading::legacy_abi::DestroyCondVar;

use crate::threading::legacy_abi::BroadcastCondVar;

use crate::threading::legacy_abi::LockMutex;

use crate::threading::legacy_abi::UnlockMutex;

use crate::threading::legacy_abi::ClearSemaphore;

use crate::graphics::geometry_ffi::TFB_BBox_Init;

use crate::graphics::geometry_ffi::TFB_BBox_Reset;

use crate::graphics::geometry_ffi::TFB_BBox_SetClipRect;

use crate::graphics::geometry_ffi::TFB_BBox_RegisterPoint;

use crate::graphics::geometry_ffi::TFB_BBox_RegisterRect;

use crate::graphics::geometry_ffi::TFB_BBox_RegisterCanvas;

use crate::graphics::cmap_abi::TFB_ReturnColorMap;

use crate::graphics::cmap_abi::GetFadeAmount;

extern "C" {
    static ScreenWidth: i32;
    static ScreenHeight: i32;
    static ScreenWidthActual: i32;
    static ScreenHeightActual: i32;
    static GraphicsDriver: i32;
    static GfxFlags: i32;
    static TFB_DEBUG_HALT: i32;
    static TransitionAmount: i32;
}
fn synchronize(queue: &mut TFB_DrawCommandQueue) {
    if queue.Batching == 0 {
        let (front, back) = (queue.Front, queue.InsertionPoint);
        queue.Back = back;
        queue.Size = if front <= back {
            back - front
        } else {
            back + MAX - front
        };
        queue.FullSize = queue.Size;
    }
}
unsafe fn wait_for_space() {
    TFB_BatchReset();
    let depth = GetRecursiveMutexDepth(DCQ_MUTEX);
    for _ in 0..depth {
        UnlockRecursiveMutex(DCQ_MUTEX);
    }
    TFB_FlushGraphicsEx(1);
    for _ in 0..depth {
        LockRecursiveMutex(DCQ_MUTEX);
    }
}
/// # Safety
/// Queue must be initialized and used according to the render-thread contract.
pub unsafe extern "C" fn lock_dcq(slots: i32) {
    LockRecursiveMutex(DCQ_MUTEX);
    while DrawCommandQueue.FullSize >= MAX - slots {
        wait_for_space();
    }
}
/// # Safety
/// Caller must hold the draw queue lock.
pub unsafe extern "C" fn unlock_dcq() {
    UnlockRecursiveMutex(DCQ_MUTEX);
}
/// # Safety
/// Queue must be initialized; called by serialized graphics producer.
pub unsafe fn tfb_batch_graphics() {
    LockRecursiveMutex(DCQ_MUTEX);
    DrawCommandQueue.Batching += 1;
    UnlockRecursiveMutex(DCQ_MUTEX);
}
/// # Safety
/// Queue must be initialized; called by serialized graphics producer.
pub unsafe fn tfb_unbatch_graphics() {
    LockRecursiveMutex(DCQ_MUTEX);
    if DrawCommandQueue.Batching != 0 {
        DrawCommandQueue.Batching -= 1;
    }
    synchronize(&mut *ptr::addr_of_mut!(DrawCommandQueue));
    UnlockRecursiveMutex(DCQ_MUTEX);
}
/// # Safety
/// Queue must be initialized.
pub unsafe fn tfb_get_batch_depth() -> i32 {
    LockRecursiveMutex(DCQ_MUTEX);
    let depth = DrawCommandQueue.Batching;
    UnlockRecursiveMutex(DCQ_MUTEX);
    depth
}
/// # Safety
/// Queue must be initialized.
pub unsafe fn tfb_batch_reset() {
    LockRecursiveMutex(DCQ_MUTEX);
    DrawCommandQueue.Batching = 0;
    synchronize(&mut *ptr::addr_of_mut!(DrawCommandQueue));
    UnlockRecursiveMutex(DCQ_MUTEX);
}
/// # Safety
/// Called once during graphics initialization on main thread.
#[no_mangle]
pub unsafe extern "C" fn Init_DrawCommandQueue() {
    DrawCommandQueue = std::mem::zeroed();
    TFB_BBox_Init(ScreenWidth, ScreenHeight);
    DCQ_MUTEX = CreateRecursiveMutex_Core(c"DCQ".as_ptr(), 5);
    RenderingCond = CreateCondVar_Core(c"DCQ empty".as_ptr(), 5);
}
/// # Safety
/// All queue users must have stopped.
#[no_mangle]
pub unsafe extern "C" fn Uninit_DrawCommandQueue() {
    if !RenderingCond.is_null() {
        DestroyCondVar(RenderingCond);
        RenderingCond = ptr::null_mut();
    }
    if !DCQ_MUTEX.is_null() {
        DestroyRecursiveMutex(DCQ_MUTEX);
        DCQ_MUTEX = ptr::null_mut();
    }
}
/// # Safety
/// Command and its referenced resources must remain live through dispatch.
#[no_mangle]
pub unsafe extern "C" fn TFB_DrawCommandQueue_Push(command: *mut TFB_DrawCommand) {
    lock_dcq(1);
    *ptr::addr_of_mut!(DCQ)
        .cast::<TFB_DrawCommand>()
        .add(DrawCommandQueue.InsertionPoint as usize) = *command;
    DrawCommandQueue.InsertionPoint = (DrawCommandQueue.InsertionPoint + 1) % MAX;
    DrawCommandQueue.FullSize += 1;
    synchronize(&mut *ptr::addr_of_mut!(DrawCommandQueue));
    unlock_dcq();
}
/// # Safety
/// Target must be writable and queue initialized.
#[no_mangle]
pub unsafe extern "C" fn TFB_DrawCommandQueue_Pop(target: *mut TFB_DrawCommand) -> i32 {
    LockRecursiveMutex(DCQ_MUTEX);
    if DrawCommandQueue.Size == 0 {
        unlock_dcq();
        return 0;
    }
    if DrawCommandQueue.Front == DrawCommandQueue.Back && DrawCommandQueue.Size != MAX {
        log::debug!("Front == Back, Size != DCQ_MAX");
        DrawCommandQueue.Size = 0;
        unlock_dcq();
        return 0;
    }
    *target = *ptr::addr_of!(DCQ)
        .cast::<TFB_DrawCommand>()
        .add(DrawCommandQueue.Front as usize);
    DrawCommandQueue.Front = (DrawCommandQueue.Front + 1) % MAX;
    DrawCommandQueue.Size -= 1;
    DrawCommandQueue.FullSize -= 1;
    UnlockRecursiveMutex(DCQ_MUTEX);
    1
}
fn equal_rect(a: RECT, b: RECT) -> bool {
    (a.corner.x, a.corner.y, a.extent.width, a.extent.height)
        == (b.corner.x, b.corner.y, b.extent.width, b.extent.height)
}
/// # Safety
/// Command resources and current frame/context must be live on graphics thread.
#[no_mangle]
pub unsafe extern "C" fn TFB_EnqueueDrawCommand(command: *mut TFB_DrawCommand) {
    if TFB_DEBUG_HALT != 0 {
        return;
    }
    if (*command).Type <= 6
        && (*_CurFramePtr).Type == SCREEN_DRAWABLE as u16
        && ((!_pCurContext.is_null() && !equal_rect(SCISSOR, (*_pCurContext).ClipRect))
            || (_pCurContext.is_null() && SCISSOR.extent.width != 0))
    {
        let mut scissor: TFB_DrawCommand = std::mem::zeroed();
        if !_pCurContext.is_null() {
            SCISSOR = (*_pCurContext).ClipRect;
        } else {
            SCISSOR.extent.width = 0;
        }
        if SCISSOR.extent.width != 0 {
            scissor.Type = 7;
            scissor.data.scissor.rect = SCISSOR;
        } else {
            scissor.Type = 8;
        }
        TFB_EnqueueDrawCommand(&mut scissor);
    }
    TFB_DrawCommandQueue_Push(command);
}
unsafe fn compute_fps() {
    let now = GetTimeCounter();
    let delta = now.wrapping_sub(LAST_TIME);
    LAST_TIME = now;
    FPS_COUNTER = FPS_COUNTER.wrapping_add(delta);
    if FPS_COUNTER > 840 / 100 {
        log::info!(
            "fps {:.2}, effective {:.2}",
            840.0f32 / delta as f32,
            840.0f32 * RENDERED_FRAMES as f32 / FPS_COUNTER as f32
        );
        FPS_COUNTER = 0;
        RENDERED_FRAMES = 0;
    }
}
unsafe fn register_image(image: *mut TFB_Image, x: i32, y: i32, scale: i32) {
    LockMutex((*image).mutex.cast());
    if scale != 0 {
        TFB_BBox_RegisterCanvas(
            (*image).ScaledImg,
            x - i32::from((*image).last_scale_hs.x),
            y - i32::from((*image).last_scale_hs.y),
        );
    } else {
        TFB_BBox_RegisterCanvas(
            (*image).NormalImg,
            x - i32::from((*image).NormalHs.x),
            y - i32::from((*image).NormalHs.y),
        );
    }
    UnlockMutex((*image).mutex.cast());
}
/// # Safety
/// Called only on the render/main thread with initialized queue and screens.
#[no_mangle]
pub unsafe extern "C" fn TFB_FlushGraphics() {
    TFB_FlushGraphicsEx(0);
}
/// # Safety
/// Called only on the render/main thread with initialized queue and screens.
#[no_mangle]
pub unsafe extern "C" fn TFB_FlushGraphicsEx(skip_swap: BOOLEAN) {
    if DrawCommandQueue.Size == 0 {
        let fade = GetFadeAmount();
        let transition = ptr::read_volatile(ptr::addr_of!(TransitionAmount));
        if skip_swap == 0
            && ((fade != 255 && fade != LAST_FADE)
                || (transition != 255 && transition != LAST_TRANSITION)
                || (fade == 255 && LAST_FADE != 255)
                || (transition == 255 && LAST_TRANSITION != 255))
        {
            TFB_SwapBuffers(1);
        } else {
            TaskSwitch();
        }
        LAST_FADE = fade;
        LAST_TRANSITION = transition;
        BroadcastCondVar(RenderingCond);
        return;
    }
    if GfxFlags & 2 != 0 {
        compute_fps();
    }
    let mut handled = 0;
    let mut deterrence = false;
    if DrawCommandQueue.FullSize > MAX {
        TFB_BatchReset();
    }
    if DrawCommandQueue.Size > LIMIT {
        lock_dcq(-1);
        deterrence = true;
    }
    TFB_BBox_Reset();
    loop {
        let mut command: TFB_DrawCommand = std::mem::zeroed();
        if TFB_DrawCommandQueue_Pop(&mut command) == 0 {
            break;
        }
        handled += 1;
        if !deterrence && handled + DrawCommandQueue.Size > LIMIT {
            deterrence = true;
            lock_dcq(-1);
        }
        match command.Type {
            9 => {
                let c = command.data.setmipmap;
                TFB_DrawImage_SetMipmap(c.image, c.mipmap, c.hotx, c.hoty);
            }
            2 => {
                let c = command.data.image;
                TFB_DrawCanvas_Image(
                    c.image,
                    c.x,
                    c.y,
                    c.scale,
                    c.scaleMode,
                    c.colormap,
                    c.drawMode,
                    TFB_GetScreenCanvas(c.destBuffer),
                );
                if c.destBuffer == 0 {
                    register_image(c.image, c.x, c.y, c.scale);
                }
            }
            3 => {
                let c = command.data.filledimage;
                TFB_DrawCanvas_FilledImage(
                    c.image,
                    c.x,
                    c.y,
                    c.scale,
                    c.scaleMode,
                    c.color,
                    c.drawMode,
                    TFB_GetScreenCanvas(c.destBuffer),
                );
                if c.destBuffer == 0 {
                    register_image(c.image, c.x, c.y, c.scale);
                }
            }
            4 => {
                let c = command.data.fontchar;
                TFB_DrawCanvas_FontChar(
                    c.fontchar,
                    c.backing,
                    c.x,
                    c.y,
                    c.drawMode,
                    TFB_GetScreenCanvas(c.destBuffer),
                );
                if c.destBuffer == 0 {
                    let r = RECT {
                        corner: POINT {
                            x: (c.x - i32::from((*c.fontchar).HotSpot.x)) as i16,
                            y: (c.y - i32::from((*c.fontchar).HotSpot.y)) as i16,
                        },
                        extent: (*c.fontchar).extent,
                    };
                    TFB_BBox_RegisterRect(&r);
                }
            }
            0 => {
                let c = command.data.line;
                if c.destBuffer == 0 {
                    TFB_BBox_RegisterPoint(c.x1, c.y1);
                    TFB_BBox_RegisterPoint(c.x2, c.y2);
                }
                TFB_DrawCanvas_Line(
                    c.x1,
                    c.y1,
                    c.x2,
                    c.y2,
                    c.color,
                    c.drawMode,
                    TFB_GetScreenCanvas(c.destBuffer),
                );
            }
            1 => {
                let mut c = command.data.rect;
                if c.destBuffer == 0 {
                    TFB_BBox_RegisterRect(&c.rect);
                }
                TFB_DrawCanvas_Rect(
                    &mut c.rect,
                    c.color,
                    c.drawMode,
                    TFB_GetScreenCanvas(c.destBuffer),
                );
            }
            7 => {
                let r = command.data.scissor.rect;
                TFB_DrawCanvas_SetClipRect(TFB_GetScreenCanvas(0), &r);
                TFB_BBox_SetClipRect(&r);
            }
            8 => {
                TFB_DrawCanvas_SetClipRect(TFB_GetScreenCanvas(0), ptr::null_mut());
                TFB_BBox_SetClipRect(ptr::null());
            }
            6 => {
                let c = command.data.copytoimage;
                if c.image.is_null() {
                    log::debug!("COPYTOIMAGE passed null image ptr");
                    continue;
                }
                LockMutex((*c.image).mutex.cast());
                TFB_DrawCanvas_CopyRect(
                    TFB_GetScreenCanvas(c.srcBuffer),
                    &c.rect,
                    (*c.image).NormalImg,
                    POINT { x: 0, y: 0 },
                );
                UnlockMutex((*c.image).mutex.cast());
            }
            5 => {
                let c = command.data.copy;
                if c.destBuffer == 0 {
                    TFB_BBox_RegisterRect(&c.rect);
                }
                TFB_DrawCanvas_CopyRect(
                    TFB_GetScreenCanvas(c.srcBuffer),
                    &c.rect,
                    TFB_GetScreenCanvas(c.destBuffer),
                    c.rect.corner,
                );
            }
            10 => TFB_DrawImage_Delete(command.data.deleteimage.image),
            11 => rust_hfree(command.data.deletedata.data),
            12 => ClearSemaphore(command.data.sendsignal.sem.cast()),
            13 => {
                let c = command.data.reinitvideo;
                let (driver, flags, width, height) = (
                    GraphicsDriver,
                    GfxFlags,
                    ScreenWidthActual,
                    ScreenHeightActual,
                );
                if TFB_ReInitGraphics(c.driver, c.flags, c.width, c.height) != 0 {
                    log::error!(
                        "Could not provide requested mode: reverting to last known driver."
                    );
                    if TFB_ReInitGraphics(driver, flags, width, height) != 0 {
                        log::error!("Video reinitialization failed");
                        std::process::exit(1);
                    }
                }
                TFB_SwapBuffers(3);
            }
            14 => {
                let c = command.data.callback;
                c.callback.expect("queued callback")(c.arg);
            }
            _ => {}
        }
    }
    if deterrence {
        unlock_dcq();
    }
    if skip_swap == 0 {
        TFB_SwapBuffers(0);
    }
    RENDERED_FRAMES += 1;
    BroadcastCondVar(RenderingCond);
}
/// # Safety
/// Called by main thread during shutdown while queue remains initialized.
#[no_mangle]
pub unsafe extern "C" fn TFB_PurgeDanglingGraphics() {
    lock_dcq(-1);
    loop {
        let mut c: TFB_DrawCommand = std::mem::zeroed();
        if TFB_DrawCommandQueue_Pop(&mut c) == 0 {
            break;
        }
        match c.Type {
            10 => TFB_DrawImage_Delete(c.data.deleteimage.image),
            11 => rust_hfree(c.data.deletedata.data),
            2 => {
                let map = c.data.image.colormap;
                if !map.is_null() {
                    TFB_ReturnColorMap((map).cast());
                }
            }
            12 => ClearSemaphore(c.data.sendsignal.sem.cast()),
            _ => {}
        }
    }
    unlock_dcq();
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn batch_controls_have_rust_signatures() {
        let _: unsafe fn() = TFB_BatchGraphics;
        let _: unsafe fn() = TFB_UnbatchGraphics;
        let _: unsafe fn() -> i32 = TFB_GetBatchDepth;
        let _: unsafe fn() = TFB_BatchReset;
    }

    #[test]
    fn nested_batches_publish_only_after_outer_commit_and_wrap_ring() {
        let mut q: TFB_DrawCommandQueue = unsafe { std::mem::zeroed() };
        q.Front = MAX - 2;
        q.InsertionPoint = 1;
        q.Batching = 2;
        q.FullSize = 3;
        synchronize(&mut q);
        assert_eq!(q.Size, 0);
        q.Batching -= 1;
        synchronize(&mut q);
        assert_eq!(q.Size, 0);
        q.Batching -= 1;
        synchronize(&mut q);
        assert_eq!((q.Back, q.Size, q.FullSize), (1, 3, 3));
        q.Front = 1;
        synchronize(&mut q);
        assert_eq!((q.Size, q.FullSize), (0, 0));
    }
}
