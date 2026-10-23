//! Live draw command queue, ported from dcqueue.c (GPL-2.0-or-later).
use super::native_abi::*;
use crate::memory::rust_hfree;
use std::{
    ffi::{c_char, c_void},
    ptr,
};
const MAX: i32 = 16384;
const LIMIT: i32 = 4096;
static mut DCQ_MUTEX: *mut c_void = ptr::null_mut();
#[no_mangle]
pub static mut RenderingCond: *mut c_void = ptr::null_mut();
#[no_mangle]
pub static mut DCQ: [TFB_DrawCommand; MAX as usize] = unsafe { std::mem::zeroed() };
#[no_mangle]
pub static mut DrawCommandQueue: TFB_DrawCommandQueue = unsafe { std::mem::zeroed() };
#[no_mangle]
pub static mut RenderedFrames: i32 = 0;
static mut SCISSOR: RECT = unsafe { std::mem::zeroed() };
static mut LAST_FADE: i32 = 255;
static mut LAST_TRANSITION: i32 = 255;
static mut LAST_TIME: u32 = 0;
static mut FPS_COUNTER: u32 = 0;
extern "C" {
    fn CreateRecursiveMutex_Core(name: *const c_char, class: u32) -> *mut c_void;
    fn DestroyRecursiveMutex(mutex: *mut c_void);
    fn LockRecursiveMutex(mutex: *mut c_void);
    fn UnlockRecursiveMutex(mutex: *mut c_void);
    fn GetRecursiveMutexDepth(mutex: *mut c_void) -> i32;
    fn CreateCondVar_Core(name: *const c_char, class: u32) -> *mut c_void;
    fn DestroyCondVar(cond: *mut c_void);
    fn BroadcastCondVar(cond: *mut c_void);
    fn LockMutex(mutex: *mut c_void);
    fn UnlockMutex(mutex: *mut c_void);
    fn ClearSemaphore(sem: *mut c_void);
    fn TaskSwitch();
    fn GetTimeCounter() -> u32;
    static ScreenWidth: i32;
    static ScreenHeight: i32;
    static ScreenWidthActual: i32;
    static ScreenHeightActual: i32;
    static GraphicsDriver: i32;
    static GfxFlags: i32;
    static TFB_DEBUG_HALT: i32;
    static TransitionAmount: i32;
    fn TFB_BBox_Init(width: i32, height: i32);
    fn TFB_BBox_Reset();
    fn TFB_BBox_SetClipRect(rect: *const RECT);
    fn TFB_BBox_RegisterPoint(x: i32, y: i32);
    fn TFB_BBox_RegisterRect(rect: *const RECT);
    fn TFB_BBox_RegisterCanvas(canvas: *mut c_void, x: i32, y: i32);
    fn TFB_ReturnColorMap(map: *mut TFB_ColorMap);
    fn GetFadeAmount() -> i32;
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
#[no_mangle]
pub unsafe extern "C" fn Lock_DCQ(slots: i32) {
    LockRecursiveMutex(DCQ_MUTEX);
    while DrawCommandQueue.FullSize >= MAX - slots {
        wait_for_space();
    }
}
/// # Safety
/// Caller must hold the draw queue lock.
#[no_mangle]
pub unsafe extern "C" fn Unlock_DCQ() {
    UnlockRecursiveMutex(DCQ_MUTEX);
}
/// # Safety
/// Queue must be initialized; called by serialized graphics producer.
#[no_mangle]
pub unsafe extern "C" fn TFB_BatchGraphics() {
    LockRecursiveMutex(DCQ_MUTEX);
    DrawCommandQueue.Batching += 1;
    UnlockRecursiveMutex(DCQ_MUTEX);
}
/// # Safety
/// Queue must be initialized; called by serialized graphics producer.
#[no_mangle]
pub unsafe extern "C" fn TFB_UnbatchGraphics() {
    LockRecursiveMutex(DCQ_MUTEX);
    if DrawCommandQueue.Batching != 0 {
        DrawCommandQueue.Batching -= 1;
    }
    synchronize(&mut *ptr::addr_of_mut!(DrawCommandQueue));
    UnlockRecursiveMutex(DCQ_MUTEX);
}
/// # Safety
/// Queue must be initialized.
#[no_mangle]
pub unsafe extern "C" fn TFB_GetBatchDepth() -> i32 {
    LockRecursiveMutex(DCQ_MUTEX);
    let depth = DrawCommandQueue.Batching;
    UnlockRecursiveMutex(DCQ_MUTEX);
    depth
}
/// # Safety
/// Queue must be initialized.
#[no_mangle]
pub unsafe extern "C" fn TFB_BatchReset() {
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
    Lock_DCQ(1);
    *ptr::addr_of_mut!(DCQ)
        .cast::<TFB_DrawCommand>()
        .add(DrawCommandQueue.InsertionPoint as usize) = *command;
    DrawCommandQueue.InsertionPoint = (DrawCommandQueue.InsertionPoint + 1) % MAX;
    DrawCommandQueue.FullSize += 1;
    synchronize(&mut *ptr::addr_of_mut!(DrawCommandQueue));
    Unlock_DCQ();
}
/// # Safety
/// Target must be writable and queue initialized.
#[no_mangle]
pub unsafe extern "C" fn TFB_DrawCommandQueue_Pop(target: *mut TFB_DrawCommand) -> i32 {
    LockRecursiveMutex(DCQ_MUTEX);
    if DrawCommandQueue.Size == 0 {
        Unlock_DCQ();
        return 0;
    }
    if DrawCommandQueue.Front == DrawCommandQueue.Back && DrawCommandQueue.Size != MAX {
        log::debug!("Front == Back, Size != DCQ_MAX");
        DrawCommandQueue.Size = 0;
        Unlock_DCQ();
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
/// # Safety
/// Queue must be initialized; caller owns the disposition of discarded resources.
#[no_mangle]
pub unsafe extern "C" fn TFB_DrawCommandQueue_Clear() {
    LockRecursiveMutex(DCQ_MUTEX);
    DrawCommandQueue = std::mem::zeroed();
    UnlockRecursiveMutex(DCQ_MUTEX);
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
            840.0f32 * RenderedFrames as f32 / FPS_COUNTER as f32
        );
        FPS_COUNTER = 0;
        RenderedFrames = 0;
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
        Lock_DCQ(-1);
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
            Lock_DCQ(-1);
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
        Unlock_DCQ();
    }
    if skip_swap == 0 {
        TFB_SwapBuffers(0);
    }
    RenderedFrames += 1;
    BroadcastCondVar(RenderingCond);
}
/// # Safety
/// Called by main thread during shutdown while queue remains initialized.
#[no_mangle]
pub unsafe extern "C" fn TFB_PurgeDanglingGraphics() {
    Lock_DCQ(-1);
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
                    TFB_ReturnColorMap(map);
                }
            }
            12 => ClearSemaphore(c.data.sendsignal.sem.cast()),
            _ => {}
        }
    }
    Unlock_DCQ();
}
#[cfg(test)]
mod tests {
    use super::*;
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
