//! TFB draw API and image lifecycle ported from `tfb_draw.c`.
use super::native_abi::{
    Color, DrawMode, Mutex, TFB_Char, TFB_ColorMap, TFB_DrawCommand, TFB_DrawCommandQueue,
    TFB_DrawCommand_Callback, TFB_DrawCommand_Copy, TFB_DrawCommand_CopyToImage,
    TFB_DrawCommand_FilledImage, TFB_DrawCommand_FontChar, TFB_DrawCommand_Image,
    TFB_DrawCommand_Line, TFB_DrawCommand_Rect, TFB_DrawCommand_ReinitVideo,
    TFB_DrawCommand_SetMipmap, TFB_Image, BOOLEAN, EXTENT, HOT_SPOT, POINT, RECT, SCREEN,
    TFB_DRAWCOMMANDTYPE_CALLBACK, TFB_DRAWCOMMANDTYPE_COPY, TFB_DRAWCOMMANDTYPE_COPYTOIMAGE,
    TFB_DRAWCOMMANDTYPE_DELETEDATA, TFB_DRAWCOMMANDTYPE_DELETEIMAGE,
    TFB_DRAWCOMMANDTYPE_FILLEDIMAGE, TFB_DRAWCOMMANDTYPE_FONTCHAR, TFB_DRAWCOMMANDTYPE_IMAGE,
    TFB_DRAWCOMMANDTYPE_LINE, TFB_DRAWCOMMANDTYPE_RECTANGLE, TFB_DRAWCOMMANDTYPE_REINITVIDEO,
    TFB_DRAWCOMMANDTYPE_SETMIPMAP,
};
use super::native_canvas::{
    drawing::*, lifecycle::*, pixels::TFB_DrawCanvas_Intersect, rotation::*, scaling::*,
    transparency::*, trilinear::*, GSCALE_IDENTITY, TFB_SCALE_BILINEAR, TFB_SCALE_NEAREST,
};
use crate::memory::{rust_hfree, rust_hmalloc};
use std::{
    ffi::{c_char, c_void},
    ptr,
};

type Image = TFB_Image;
type Canvas = *mut c_void;

extern "C" {
    static mut DrawCommandQueue: TFB_DrawCommandQueue;
    static ScreenWidth: i32;
    static ScreenHeight: i32;
    fn TFB_EnqueueDrawCommand(command: *mut TFB_DrawCommand);
    fn TFB_FlushGraphics();
    fn TFB_FlushGraphicsEx(skip_swap: BOOLEAN);
    fn CreateMutex_Core(name: *const c_char, sync_class: u32) -> Mutex;
    fn LockMutex(mutex: Mutex);
    fn UnlockMutex(mutex: Mutex);
    fn DestroyMutex(mutex: Mutex);

}
use crate::graphics::gfx_common_abi::{BatchGraphics, UnbatchGraphics};

unsafe fn enqueue(command: &mut TFB_DrawCommand) {
    TFB_EnqueueDrawCommand(command);
}
unsafe fn empty_command(kind: u32) -> TFB_DrawCommand {
    let mut c: TFB_DrawCommand = std::mem::zeroed();
    c.Type = kind as i32;
    c
}
unsafe fn image_lock(image: *mut Image) {
    LockMutex((*image).mutex);
}
unsafe fn image_unlock(image: *mut Image) {
    UnlockMutex((*image).mutex);
}
unsafe fn make_mutex() -> Mutex {
    let name = b"image lock\0";
    CreateMutex_Core(name.as_ptr().cast(), 1 << 2)
}
unsafe fn init_image(image: *mut Image) {
    (*image).ScaledImg = ptr::null_mut();
    (*image).MipmapImg = ptr::null_mut();
    (*image).FilledImg = ptr::null_mut();
    (*image).colormap_index = -1;
    (*image).colormap_version = 0;
    (*image).NormalHs = HOT_SPOT { x: 0, y: 0 };
    (*image).MipmapHs = HOT_SPOT { x: 0, y: 0 };
    (*image).last_scale_hs = HOT_SPOT { x: 0, y: 0 };
    (*image).last_scale_type = -1;
    (*image).last_scale = 0;
    (*image).dirty = 0;
    (*image).mutex = make_mutex();
}

/// # Safety
/// Pointer arguments and referenced image/canvas resources must remain live through this serialized graphics operation.
#[no_mangle]
pub unsafe extern "C" fn TFB_DrawScreen_Line(
    x1: i32,
    y1: i32,
    x2: i32,
    y2: i32,
    color: Color,
    mode: DrawMode,
    dest: SCREEN,
) {
    let mut c = empty_command(TFB_DRAWCOMMANDTYPE_LINE);
    c.data.line = TFB_DrawCommand_Line {
        x1,
        y1,
        x2,
        y2,
        color,
        drawMode: mode,
        destBuffer: dest,
    };
    enqueue(&mut c);
}
/// # Safety
/// Pointer arguments and referenced image/canvas resources must remain live through this serialized graphics operation.
#[no_mangle]
pub unsafe extern "C" fn TFB_DrawScreen_Rect(
    rect: *mut RECT,
    color: Color,
    mode: DrawMode,
    dest: SCREEN,
) {
    let full = RECT {
        corner: POINT { x: 0, y: 0 },
        extent: EXTENT {
            width: ScreenWidth as i16,
            height: ScreenHeight as i16,
        },
    };
    let mut c = empty_command(TFB_DRAWCOMMANDTYPE_RECTANGLE);
    c.data.rect = TFB_DrawCommand_Rect {
        rect: if rect.is_null() { full } else { *rect },
        color,
        drawMode: mode,
        destBuffer: dest,
    };
    enqueue(&mut c);
}
/// # Safety
/// Pointer arguments and referenced image/canvas resources must remain live through this serialized graphics operation.
#[no_mangle]
pub unsafe extern "C" fn TFB_DrawScreen_Image(
    img: *mut Image,
    x: i32,
    y: i32,
    scale: i32,
    scale_mode: i32,
    cmap: *mut TFB_ColorMap,
    mode: DrawMode,
    dest: SCREEN,
) {
    let mut c = empty_command(TFB_DRAWCOMMANDTYPE_IMAGE);
    c.data.image = TFB_DrawCommand_Image {
        image: img,
        x,
        y,
        destBuffer: dest,
        colormap: cmap,
        drawMode: mode,
        scale: if scale == GSCALE_IDENTITY { 0 } else { scale },
        scaleMode: scale_mode,
    };
    enqueue(&mut c);
}
/// # Safety
/// Pointer arguments and referenced image/canvas resources must remain live through this serialized graphics operation.
#[no_mangle]
pub unsafe extern "C" fn TFB_DrawScreen_FilledImage(
    img: *mut Image,
    x: i32,
    y: i32,
    scale: i32,
    scale_mode: i32,
    color: Color,
    mode: DrawMode,
    dest: SCREEN,
) {
    let mut c = empty_command(TFB_DRAWCOMMANDTYPE_FILLEDIMAGE);
    c.data.filledimage = TFB_DrawCommand_FilledImage {
        image: img,
        x,
        y,
        color,
        destBuffer: dest,
        drawMode: mode,
        scale: if scale == GSCALE_IDENTITY { 0 } else { scale },
        scaleMode: scale_mode,
    };
    enqueue(&mut c);
}
/// # Safety
/// Pointer arguments and referenced image/canvas resources must remain live through this serialized graphics operation.
#[no_mangle]
pub unsafe extern "C" fn TFB_DrawScreen_FontChar(
    ch: *mut TFB_Char,
    backing: *mut Image,
    x: i32,
    y: i32,
    mode: DrawMode,
    dest: SCREEN,
) {
    let mut c = empty_command(TFB_DRAWCOMMANDTYPE_FONTCHAR);
    c.data.fontchar = TFB_DrawCommand_FontChar {
        fontchar: ch,
        backing,
        x,
        y,
        drawMode: mode,
        destBuffer: dest,
    };
    enqueue(&mut c);
}
/// # Safety
/// Pointer arguments and referenced image/canvas resources must remain live through this serialized graphics operation.
#[no_mangle]
pub unsafe extern "C" fn TFB_DrawScreen_CopyToImage(img: *mut Image, r: *const RECT, src: SCREEN) {
    let mut c = empty_command(TFB_DRAWCOMMANDTYPE_COPYTOIMAGE);
    c.data.copytoimage = TFB_DrawCommand_CopyToImage {
        image: img,
        rect: *r,
        srcBuffer: src,
    };
    enqueue(&mut c);
}
/// # Safety
/// Pointer arguments and referenced image/canvas resources must remain live through this serialized graphics operation.
#[no_mangle]
pub unsafe extern "C" fn TFB_DrawScreen_Copy(r: *const RECT, src: SCREEN, dest: SCREEN) {
    let full = RECT {
        corner: POINT { x: 0, y: 0 },
        extent: EXTENT {
            width: ScreenWidth as i16,
            height: ScreenHeight as i16,
        },
    };
    let mut c = empty_command(TFB_DRAWCOMMANDTYPE_COPY);
    c.data.copy = TFB_DrawCommand_Copy {
        rect: if r.is_null() { full } else { *r },
        srcBuffer: src,
        destBuffer: dest,
    };
    enqueue(&mut c);
}
/// # Safety
/// Pointer arguments and referenced image/canvas resources must remain live through this serialized graphics operation.
#[no_mangle]
pub unsafe extern "C" fn TFB_DrawScreen_SetMipmap(
    img: *mut Image,
    mip: *mut Image,
    hotx: i32,
    hoty: i32,
) {
    let mut c = empty_command(TFB_DRAWCOMMANDTYPE_SETMIPMAP);
    c.data.setmipmap = TFB_DrawCommand_SetMipmap {
        image: img,
        mipmap: mip,
        hotx,
        hoty,
    };
    enqueue(&mut c);
}
/// # Safety
/// Pointer arguments and referenced image/canvas resources must remain live through this serialized graphics operation.
#[no_mangle]
pub unsafe extern "C" fn TFB_DrawScreen_DeleteImage(img: *mut Image) {
    if !img.is_null() {
        let mut c = empty_command(TFB_DRAWCOMMANDTYPE_DELETEIMAGE);
        c.data.deleteimage.image = img;
        enqueue(&mut c);
    }
}
/// # Safety
/// Pointer arguments and referenced image/canvas resources must remain live through this serialized graphics operation.
#[no_mangle]
pub unsafe extern "C" fn TFB_DrawScreen_DeleteData(data: *mut c_void) {
    if !data.is_null() {
        let mut c = empty_command(TFB_DRAWCOMMANDTYPE_DELETEDATA);
        c.data.deletedata.data = data;
        enqueue(&mut c);
    }
}
/// # Safety
/// Pointer arguments and referenced image/canvas resources must remain live through this serialized graphics operation.
#[no_mangle]
pub unsafe extern "C" fn TFB_DrawScreen_WaitForSignal() {
    if DrawCommandQueue.Batching > 0 {
        UnbatchGraphics();
        TFB_FlushGraphicsEx(1);
        BatchGraphics();
    } else {
        TFB_FlushGraphics();
    }
}
/// # Safety
/// Pointer arguments and referenced image/canvas resources must remain live through this serialized graphics operation.
#[no_mangle]
pub unsafe extern "C" fn TFB_DrawScreen_ReinitVideo(
    driver: i32,
    flags: i32,
    width: i32,
    height: i32,
) {
    let mut c = empty_command(TFB_DRAWCOMMANDTYPE_REINITVIDEO);
    c.data.reinitvideo = TFB_DrawCommand_ReinitVideo {
        driver,
        flags,
        width,
        height,
    };
    enqueue(&mut c);
}
/// # Safety
/// Pointer arguments and referenced image/canvas resources must remain live through this serialized graphics operation.
#[no_mangle]
pub unsafe extern "C" fn TFB_DrawScreen_Callback(
    callback: Option<unsafe extern "C" fn(*mut c_void)>,
    arg: *mut c_void,
) {
    let mut c = empty_command(TFB_DRAWCOMMANDTYPE_CALLBACK);
    c.data.callback = TFB_DrawCommand_Callback { callback, arg };
    enqueue(&mut c);
}

/// # Safety
/// Pointer arguments and referenced image/canvas resources must remain live through this serialized graphics operation.
#[no_mangle]
pub unsafe extern "C" fn TFB_DrawImage_Line(
    x1: i32,
    y1: i32,
    x2: i32,
    y2: i32,
    color: Color,
    mode: DrawMode,
    target: *mut Image,
) {
    image_lock(target);
    TFB_DrawCanvas_Line(x1, y1, x2, y2, color, mode, (*target).NormalImg);
    (*target).dirty = 1;
    image_unlock(target);
}
/// # Safety
/// Pointer arguments and referenced image/canvas resources must remain live through this serialized graphics operation.
#[no_mangle]
pub unsafe extern "C" fn TFB_DrawImage_Rect(
    r: *mut RECT,
    color: Color,
    mode: DrawMode,
    target: *mut Image,
) {
    image_lock(target);
    TFB_DrawCanvas_Rect(r, color, mode, (*target).NormalImg);
    (*target).dirty = 1;
    image_unlock(target);
}
/// # Safety
/// Pointer arguments and referenced image/canvas resources must remain live through this serialized graphics operation.
#[no_mangle]
pub unsafe extern "C" fn TFB_DrawImage_Image(
    img: *mut Image,
    x: i32,
    y: i32,
    scale: i32,
    sm: i32,
    cmap: *mut TFB_ColorMap,
    mode: DrawMode,
    target: *mut Image,
) {
    image_lock(target);
    TFB_DrawCanvas_Image(img, x, y, scale, sm, cmap, mode, (*target).NormalImg);
    (*target).dirty = 1;
    image_unlock(target);
}
/// # Safety
/// Pointer arguments and referenced image/canvas resources must remain live through this serialized graphics operation.
#[no_mangle]
pub unsafe extern "C" fn TFB_DrawImage_FilledImage(
    img: *mut Image,
    x: i32,
    y: i32,
    scale: i32,
    sm: i32,
    color: Color,
    mode: DrawMode,
    target: *mut Image,
) {
    image_lock(target);
    TFB_DrawCanvas_FilledImage(img, x, y, scale, sm, color, mode, (*target).NormalImg);
    (*target).dirty = 1;
    image_unlock(target);
}
/// # Safety
/// Pointer arguments and referenced image/canvas resources must remain live through this serialized graphics operation.
#[no_mangle]
pub unsafe extern "C" fn TFB_DrawImage_FontChar(
    ch: *mut TFB_Char,
    backing: *mut Image,
    x: i32,
    y: i32,
    mode: DrawMode,
    target: *mut Image,
) {
    image_lock(target);
    TFB_DrawCanvas_FontChar(ch, backing, x, y, mode, (*target).NormalImg);
    (*target).dirty = 1;
    image_unlock(target);
}
/// # Safety
/// Pointer arguments and referenced image/canvas resources must remain live through this serialized graphics operation.
#[no_mangle]
pub unsafe extern "C" fn TFB_DrawImage_New(canvas: Canvas) -> *mut Image {
    let image = rust_hmalloc(std::mem::size_of::<Image>()).cast::<Image>();
    init_image(image);
    let mut extent = EXTENT {
        width: 0,
        height: 0,
    };
    TFB_DrawCanvas_GetExtent(canvas, &mut extent);
    (*image).extent = extent;
    (*image).NormalImg = if TFB_DrawCanvas_IsPaletted(canvas) != 0 {
        canvas
    } else {
        TFB_DrawCanvas_ToScreenFormat(canvas)
    };
    image
}
/// # Safety
/// Pointer arguments and referenced image/canvas resources must remain live through this serialized graphics operation.
#[no_mangle]
pub unsafe extern "C" fn TFB_DrawImage_CreateForScreen(
    w: i32,
    h: i32,
    alpha: BOOLEAN,
) -> *mut Image {
    let image = rust_hmalloc(std::mem::size_of::<Image>()).cast::<Image>();
    init_image(image);
    (*image).extent = EXTENT {
        width: w as i16,
        height: h as i16,
    };
    (*image).NormalImg = TFB_DrawCanvas_New_ForScreen(w, h, alpha);
    image
}
/// # Safety
/// Pointer arguments and referenced image/canvas resources must remain live through this serialized graphics operation.
#[no_mangle]
pub unsafe extern "C" fn TFB_DrawImage_New_Rotated(img: *mut Image, angle: i32) -> *mut Image {
    if (*img).NormalImg.is_null() {
        log(b"TFB_DrawImage_New_Rotated: source canvas is NULL! Failing.");
        return ptr::null_mut();
    }
    let mut size = EXTENT {
        width: 0,
        height: 0,
    };
    TFB_DrawCanvas_GetRotatedExtent((*img).NormalImg, angle, &mut size);
    let dst = TFB_DrawCanvas_New_RotationTarget((*img).NormalImg, angle);
    if dst.is_null() {
        log(b"TFB_DrawImage_New_Rotated: rotation target canvas not created! Failing.");
        return ptr::null_mut();
    }
    TFB_DrawCanvas_Rotate((*img).NormalImg, dst, angle, size);
    TFB_DrawImage_New(dst)
}
unsafe fn log(message: &[u8]) {
    log::warn!("{}", String::from_utf8_lossy(message));
}
/// # Safety
/// Pointer arguments and referenced image/canvas resources must remain live through this serialized graphics operation.
#[no_mangle]
pub unsafe extern "C" fn TFB_DrawImage_SetMipmap(
    img: *mut Image,
    mip: *mut Image,
    hotx: i32,
    hoty: i32,
) {
    if img.is_null() || mip.is_null() {
        return;
    }
    image_lock(img);
    image_lock(mip);
    let ip = TFB_DrawCanvas_IsPaletted((*img).NormalImg) != 0;
    let mp = TFB_DrawCanvas_IsPaletted((*mip).NormalImg) != 0;
    if !mp || (ip && (*img).colormap_index == (*mip).colormap_index) {
        (*img).MipmapImg = (*mip).NormalImg;
        (*img).MipmapHs = HOT_SPOT {
            x: hotx as i16,
            y: hoty as i16,
        };
    } else {
        (*img).MipmapImg = ptr::null_mut();
    }
    image_unlock(mip);
    image_unlock(img);
}
/// # Safety
/// Pointer arguments and referenced image/canvas resources must remain live through this serialized graphics operation.
#[no_mangle]
pub unsafe extern "C" fn TFB_DrawImage_Delete(image: *mut Image) {
    if image.is_null() {
        log(b"INTERNAL ERROR: Tried to delete a null image!");
        return;
    }
    image_lock(image);
    TFB_DrawCanvas_Delete((*image).NormalImg);
    if !(*image).ScaledImg.is_null() {
        TFB_DrawCanvas_Delete((*image).ScaledImg);
        (*image).ScaledImg = ptr::null_mut();
    }
    if !(*image).FilledImg.is_null() {
        TFB_DrawCanvas_Delete((*image).FilledImg);
        (*image).FilledImg = ptr::null_mut();
    }
    let mutex = (*image).mutex;
    UnlockMutex(mutex);
    DestroyMutex(mutex);
    rust_hfree(image.cast());
}
/// # Safety
/// Pointer arguments and referenced image/canvas resources must remain live through this serialized graphics operation.
#[no_mangle]
pub unsafe extern "C" fn TFB_DrawImage_FixScaling(image: *mut Image, target: i32, ty: i32) {
    if (*image).dirty != 0
        || (*image).ScaledImg.is_null()
        || target != (*image).last_scale
        || ty != (*image).last_scale_type
    {
        (*image).dirty = 0;
        (*image).ScaledImg = TFB_DrawCanvas_New_ScaleTarget(
            (*image).NormalImg,
            (*image).ScaledImg,
            ty,
            (*image).last_scale_type,
        );
        if ty == TFB_SCALE_NEAREST {
            TFB_DrawCanvas_Rescale_Nearest(
                (*image).NormalImg,
                (*image).ScaledImg,
                target,
                &mut (*image).NormalHs,
                &mut (*image).extent,
                &mut (*image).last_scale_hs,
            )
        } else if ty == TFB_SCALE_BILINEAR {
            TFB_DrawCanvas_Rescale_Bilinear(
                (*image).NormalImg,
                (*image).ScaledImg,
                target,
                &mut (*image).NormalHs,
                &mut (*image).extent,
                &mut (*image).last_scale_hs,
            )
        } else {
            TFB_DrawCanvas_Rescale_Trilinear(
                (*image).NormalImg,
                (*image).MipmapImg,
                (*image).ScaledImg,
                target,
                &mut (*image).NormalHs,
                &mut (*image).MipmapHs,
                &mut (*image).extent,
                &mut (*image).last_scale_hs,
            )
        }
        (*image).last_scale_type = ty;
        (*image).last_scale = target;
    }
}
/// # Safety
/// Pointer arguments and referenced image/canvas resources must remain live through this serialized graphics operation.
#[no_mangle]
pub unsafe extern "C" fn TFB_DrawImage_Intersect(
    a: *mut Image,
    ao: POINT,
    b: *mut Image,
    bo: POINT,
    r: *const RECT,
) -> BOOLEAN {
    image_lock(a);
    image_lock(b);
    let result = TFB_DrawCanvas_Intersect((*a).NormalImg, ao, (*b).NormalImg, bo, r);
    image_unlock(b);
    image_unlock(a);
    result
}
/// # Safety
/// Pointer arguments and referenced image/canvas resources must remain live through this serialized graphics operation.
#[no_mangle]
pub unsafe extern "C" fn TFB_DrawImage_CopyRect(
    src: *mut Image,
    r: *const RECT,
    dst: *mut Image,
    pt: POINT,
) {
    image_lock(src);
    image_lock(dst);
    TFB_DrawCanvas_CopyRect((*src).NormalImg, r, (*dst).NormalImg, pt);
    (*dst).dirty = 1;
    image_unlock(dst);
    image_unlock(src);
}
