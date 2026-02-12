//! C FFI bindings for DukVid decoder vtable and the pure Rust video player.

use std::ffi::{c_char, c_int, c_void, CStr};
use std::ptr;
use std::sync::Mutex;

use crate::bridge_log::rust_bridge_log_msg;
use crate::io::ffi::{uio_close, uio_open, uio_read, uio_DirHandle, uio_Handle};

use super::decoder::DukVideoDecoder;
use super::player::VideoPlayer;
use super::scaler::{LanczosVideoScaler, VideoScaler};
use super::{DUCK_FPS, VideoError, VideoFrame};

pub use crate::graphics::ffi::SDL_Surface;

fn log_msg(msg: &str) {
    rust_bridge_log_msg(msg);
}

// ============================================================================
// UIO + gfx externs
// ============================================================================

extern "C" {
    // SDL screen surfaces managed by C graphics backend.
    fn TFB_DrawCanvas_Lock(canvas: *mut c_void);
    fn TFB_DrawCanvas_Unlock(canvas: *mut c_void);

    fn TFB_SwapBuffers(force_full_redraw: c_int);

    // Provided by C SDL backend.
    static SDL_Screen: *mut SDL_Surface;

    // Logical screen dimensions from gfx_common.h (internal 320x240)
    static ScreenWidth: c_int;
    static ScreenHeight: c_int;
}

// Minimal SDL types needed for direct pixel writes.
#[allow(non_camel_case_types)]
#[repr(C)]
pub struct SDL_PixelFormat {
    pub format: u32,
    pub palette: *mut c_void,
    pub BitsPerPixel: u8,
    pub BytesPerPixel: u8,
    pub padding: [u8; 2],
    pub Rmask: u32,
    pub Gmask: u32,
    pub Bmask: u32,
    pub Amask: u32,
    pub Rloss: u8,
    pub Gloss: u8,
    pub Bloss: u8,
    pub Aloss: u8,
    pub Rshift: u8,
    pub Gshift: u8,
    pub Bshift: u8,
    pub Ashift: u8,
    pub refcount: c_int,
    pub next: *mut SDL_PixelFormat,
}

// ============================================================================
// UIO helpers

// Re-export minimal canvas lock/unlock for internal modules.
#[inline]
pub unsafe fn tfb_drawcanvas_lock(canvas: *mut c_void) {
    TFB_DrawCanvas_Lock(canvas);
}

#[inline]
pub unsafe fn tfb_drawcanvas_unlock(canvas: *mut c_void) {
    TFB_DrawCanvas_Unlock(canvas);
}

// ============================================================================

unsafe fn read_uio_handle(handle: *mut uio_Handle, label: &str) -> Option<Vec<u8>> {
    let mut data = Vec::new();
    let mut buf = [0u8; 8192];
    loop {
        let n = uio_read(handle, buf.as_mut_ptr(), buf.len());
        if n <= 0 {
            if n < 0 {
                log_msg(&format!("RUST_VIDEO: uio_read returned {} for {}", n, label));
            }
            break;
        }
        data.extend_from_slice(&buf[..n as usize]);
    }

    uio_close(handle);

    if data.is_empty() {
        log_msg(&format!("RUST_VIDEO: no data read from {}", label));
        return None;
    }

    Some(data)
}

unsafe fn read_uio_file(dir: *mut uio_DirHandle, filename: &str) -> Option<Vec<u8>> {
    log_msg(&format!("RUST_VIDEO: read_uio_file start {} dir={:?}", filename, dir));

    let c_filename = std::ffi::CString::new(filename).ok()?;
    let handle = uio_open(dir, c_filename.as_ptr(), 0 /* O_RDONLY */, 0);
    log_msg(&format!("RUST_VIDEO: uio_open returned {:?} for {}", handle, filename));
    if handle.is_null() {
        log_msg(&format!("RUST_VIDEO: uio_open failed for {}", filename));

        let c_filename_abs = std::ffi::CString::new(format!("/{}", filename)).ok()?;
        let handle_abs = uio_open(dir, c_filename_abs.as_ptr(), 0 /* O_RDONLY */, 0);
        log_msg(&format!("RUST_VIDEO: uio_open returned {:?} for /{}", handle_abs, filename));
        if handle_abs.is_null() {
            log_msg(&format!("RUST_VIDEO: uio_open failed for /{}", filename));
            return None;
        }
        return read_uio_handle(handle_abs, filename);
    }

    read_uio_handle(handle, filename)
}


fn calculate_video_scale(_src_w: u32, _src_h: u32) -> u32 {
    1
}

// ============================================================================
// DukVid decoder vtable (kept for C decoder integration)
// ============================================================================

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct TFB_PixelFormat {
    pub BitsPerPixel: u32,
    pub BytesPerPixel: u32,
    pub Rmask: u32,
    pub Gmask: u32,
    pub Bmask: u32,
    pub Amask: u32,
    pub Rshift: u32,
    pub Gshift: u32,
    pub Bshift: u32,
    pub Ashift: u32,
    pub Rloss: u32,
    pub Gloss: u32,
    pub Bloss: u32,
    pub Aloss: u32,
}

#[repr(C)]
pub struct TFB_VideoCallbacks {
    pub BeginFrame: Option<unsafe extern "C" fn(*mut TFB_VideoDecoder)>,
    pub EndFrame: Option<unsafe extern "C" fn(*mut TFB_VideoDecoder)>,
    pub GetCanvasLine: Option<unsafe extern "C" fn(*mut TFB_VideoDecoder, u32) -> *mut c_void>,
    pub GetTicks: Option<unsafe extern "C" fn(*mut TFB_VideoDecoder) -> u32>,
    pub SetTimer: Option<unsafe extern "C" fn(*mut TFB_VideoDecoder, u32) -> bool>,
}

#[repr(C)]
pub struct TFB_VideoDecoder {
    pub funcs: *const TFB_VideoDecoderFuncs,
    pub format: *const TFB_PixelFormat,
    pub w: u32,
    pub h: u32,
    pub length: f32,
    pub frame_count: u32,
    pub interframe_wait: u32,
    pub audio_synced: bool,
    pub callbacks: TFB_VideoCallbacks,
    pub looping: bool,
    pub data: *mut c_void,
    pub error: i32,
    pub pos: f32,
    pub cur_frame: u32,
    pub dir: *mut uio_DirHandle,
    pub filename: *mut c_char,
}

#[repr(C)]
pub struct TFB_VideoDecoderFuncs {
    pub GetName: extern "C" fn() -> *const c_char,
    pub InitModule: extern "C" fn(flags: c_int) -> bool,
    pub TermModule: extern "C" fn(),
    pub GetStructSize: extern "C" fn() -> u32,
    pub GetError: extern "C" fn(*mut TFB_VideoDecoder) -> c_int,
    pub Init: extern "C" fn(*mut TFB_VideoDecoder, *mut TFB_PixelFormat) -> bool,
    pub Term: extern "C" fn(*mut TFB_VideoDecoder),
    pub Open: extern "C" fn(*mut TFB_VideoDecoder, *mut uio_DirHandle, *const c_char) -> bool,
    pub Close: extern "C" fn(*mut TFB_VideoDecoder),
    pub DecodeNext: extern "C" fn(*mut TFB_VideoDecoder) -> c_int,
    pub SeekFrame: extern "C" fn(*mut TFB_VideoDecoder, u32) -> u32,
    pub SeekTime: extern "C" fn(*mut TFB_VideoDecoder, f32) -> f32,
    pub GetFrame: extern "C" fn(*mut TFB_VideoDecoder) -> u32,
    pub GetTime: extern "C" fn(*mut TFB_VideoDecoder) -> f32,
}

#[repr(C)]
pub struct TFB_RustDukVideoDecoder {
    pub base: TFB_VideoDecoder,
    pub rust_decoder: *mut c_void,
    pub scaler: *mut c_void,
    pub last_error: i32,
}

static RUST_DUKV_FORMAT: Mutex<Option<TFB_PixelFormat>> = Mutex::new(None);
static RUST_DUKV_NAME: &[u8] = b"Rust DukVid\0";

extern "C" fn rust_dukv_GetName() -> *const c_char {
    RUST_DUKV_NAME.as_ptr() as *const c_char
}

extern "C" fn rust_dukv_InitModule(_flags: c_int) -> bool {
    true
}

extern "C" fn rust_dukv_TermModule() {
    if let Ok(mut guard) = RUST_DUKV_FORMAT.lock() {
        *guard = None;
    }
}

extern "C" fn rust_dukv_GetStructSize() -> u32 {
    std::mem::size_of::<TFB_RustDukVideoDecoder>() as u32
}

extern "C" fn rust_dukv_GetError(decoder: *mut TFB_VideoDecoder) -> c_int {
    if decoder.is_null() {
        return -1;
    }
    unsafe {
        let rust_dec = decoder as *mut TFB_RustDukVideoDecoder;
        (*rust_dec).last_error
    }
}

extern "C" fn rust_dukv_Init(decoder: *mut TFB_VideoDecoder, fmt: *mut TFB_PixelFormat) -> bool {
    if decoder.is_null() {
        return false;
    }

    unsafe {
        let rust_dec = decoder as *mut TFB_RustDukVideoDecoder;
        (*rust_dec).rust_decoder = ptr::null_mut();
        (*rust_dec).scaler = ptr::null_mut();
        (*rust_dec).last_error = 0;

        if !fmt.is_null() {
            (*decoder).format = fmt;
            if let Ok(mut guard) = RUST_DUKV_FORMAT.lock() {
                *guard = Some(*fmt);
            }
        }
    }

    true
}

extern "C" fn rust_dukv_Term(decoder: *mut TFB_VideoDecoder) {
    if decoder.is_null() {
        return;
    }

    unsafe {
        let rust_dec = decoder as *mut TFB_RustDukVideoDecoder;
        if !(*rust_dec).rust_decoder.is_null() {
            let _ = Box::from_raw((*rust_dec).rust_decoder as *mut DukVideoDecoder);
            (*rust_dec).rust_decoder = ptr::null_mut();
        }
        if !(*rust_dec).scaler.is_null() {
            let _ = Box::from_raw((*rust_dec).scaler as *mut VideoScaler);
            (*rust_dec).scaler = ptr::null_mut();
        }
    }
}

extern "C" fn rust_dukv_Open(decoder: *mut TFB_VideoDecoder, dir: *mut uio_DirHandle, filename: *const c_char) -> bool {
    if decoder.is_null() || filename.is_null() {
        return false;
    }

    unsafe {
        let rust_dec = decoder as *mut TFB_RustDukVideoDecoder;

        let filename_cstr = CStr::from_ptr(filename);
        let filename_str = match filename_cstr.to_str() {
            Ok(s) => s,
            Err(_) => {
                (*rust_dec).last_error = -1;
                return false;
            }
        };

        let basename = filename_str.trim_end_matches(".duk");

        let duk_file = format!("{}.duk", basename);
        let frm_file = format!("{}.frm", basename);
        let hdr_file = format!("{}.hdr", basename);
        let tbl_file = format!("{}.tbl", basename);

        let duk_data = match read_uio_file(dir, &duk_file) {
            Some(d) => d,
            None => {
                (*rust_dec).last_error = -1;
                return false;
            }
        };
        let frm_data = match read_uio_file(dir, &frm_file) {
            Some(d) => d,
            None => {
                (*rust_dec).last_error = -1;
                return false;
            }
        };
        let hdr_data = match read_uio_file(dir, &hdr_file) {
            Some(d) => d,
            None => {
                (*rust_dec).last_error = -1;
                return false;
            }
        };
        let tbl_data = match read_uio_file(dir, &tbl_file) {
            Some(d) => d,
            None => {
                (*rust_dec).last_error = -1;
                return false;
            }
        };

        match DukVideoDecoder::open_from_data(&hdr_data, &tbl_data, &frm_data, &duk_data) {
            Ok(duk_decoder) => {
                let src_w = duk_decoder.width();
                let src_h = duk_decoder.height();

                (*decoder).w = src_w;
                (*decoder).h = src_h;
                (*decoder).frame_count = duk_decoder.frame_count();
                (*decoder).length = duk_decoder.duration();
                (*decoder).interframe_wait = (1000.0 / DUCK_FPS) as u32;
                (*decoder).audio_synced = false;
                (*decoder).pos = 0.0;
                (*decoder).cur_frame = 0;
                (*decoder).error = 0;
                (*decoder).dir = dir;

                let boxed = Box::new(duk_decoder);
                (*rust_dec).rust_decoder = Box::into_raw(boxed) as *mut c_void;

                // Optional scaler (decoder vtable path currently reports native size to C).
                let scale = calculate_video_scale(src_w, src_h);
                if scale > 1 {
                    let dst_w = src_w * scale;
                    let dst_h = src_h * scale;
                    let scaler = Box::new(VideoScaler::new(src_w, src_h, dst_w, dst_h));
                    (*rust_dec).scaler = Box::into_raw(scaler) as *mut c_void;
                }

                true
            }
            Err(e) => {
                (*rust_dec).last_error = match e {
                    VideoError::BadFile(_) => -2,
                    VideoError::Eof => -5,
                    VideoError::OutOfBuffer => -4,
                    VideoError::NotInitialized => -1,
                    VideoError::IoError(_) => -1,
                    VideoError::BadArg(_) => -3,
                };
                false
            }
        }
    }
}

extern "C" fn rust_dukv_Close(decoder: *mut TFB_VideoDecoder) {
    rust_dukv_Term(decoder);
}

unsafe fn render_frame_to_canvas(
    decoder: *mut TFB_VideoDecoder,
    frame: &VideoFrame,
    fmt: &TFB_PixelFormat,
    get_canvas_line: unsafe extern "C" fn(*mut TFB_VideoDecoder, u32) -> *mut c_void,
) {
    let w = frame.width as usize;
    let h = frame.height as usize;

    match fmt.BytesPerPixel {
        2 => {
            for y in 0..h {
                let dst = get_canvas_line(decoder, y as u32) as *mut u16;
                if dst.is_null() {
                    continue;
                }
                for x in 0..w {
                    let pixel = frame.data[y * w + x];
                    let r = (pixel & 0xFF) as u32;
                    let g = ((pixel >> 8) & 0xFF) as u32;
                    let b = ((pixel >> 16) & 0xFF) as u32;
                    let out = ((r >> fmt.Rloss) << fmt.Rshift)
                        | ((g >> fmt.Gloss) << fmt.Gshift)
                        | ((b >> fmt.Bloss) << fmt.Bshift);
                    *dst.add(x) = out as u16;
                }
            }
        }
        4 => {
            for y in 0..h {
                let dst = get_canvas_line(decoder, y as u32) as *mut u32;
                if dst.is_null() {
                    continue;
                }
                for x in 0..w {
                    let pixel = frame.data[y * w + x];
                    let r = (pixel & 0xFF) as u32;
                    let g = ((pixel >> 8) & 0xFF) as u32;
                    let b = ((pixel >> 16) & 0xFF) as u32;
                    let a = ((pixel >> 24) & 0xFF) as u32;
                    let out = ((r >> fmt.Rloss) << fmt.Rshift)
                        | ((g >> fmt.Gloss) << fmt.Gshift)
                        | ((b >> fmt.Bloss) << fmt.Bshift)
                        | ((a >> fmt.Aloss) << fmt.Ashift);
                    *dst.add(x) = out;
                }
            }
        }
        _ => {}
    }
}

extern "C" fn rust_dukv_DecodeNext(decoder: *mut TFB_VideoDecoder) -> c_int {
    if decoder.is_null() {
        return -1;
    }

    unsafe {
        let rust_dec = decoder as *mut TFB_RustDukVideoDecoder;
        if (*rust_dec).rust_decoder.is_null() {
            return -1;
        }

        let duk = &mut *((*rust_dec).rust_decoder as *mut DukVideoDecoder);
        let cur_frame = (*decoder).cur_frame;

        if cur_frame >= (*decoder).frame_count {
            if (*decoder).looping {
                (*decoder).cur_frame = 0;
            } else {
                return 0;
            }
        }

        match duk.decode_frame((*decoder).cur_frame) {
            Ok(frame) => {
                if let Some(begin_frame) = (*decoder).callbacks.BeginFrame {
                    begin_frame(decoder);
                }

                if let Some(get_canvas_line) = (*decoder).callbacks.GetCanvasLine {
                    let fmt = &*(*decoder).format;
                    if !(*rust_dec).scaler.is_null() {
                        let scaler = &mut *((*rust_dec).scaler as *mut VideoScaler);
                        if let Some(scaled_pixels) = scaler.scale(&frame.data) {
                            let (dst_w, dst_h) = scaler.dst_dimensions();
                            let scaled_frame = VideoFrame {
                                width: dst_w,
                                height: dst_h,
                                data: scaled_pixels,
                                timestamp: frame.timestamp,
                            };
                            render_frame_to_canvas(decoder, &scaled_frame, fmt, get_canvas_line);
                        } else {
                            render_frame_to_canvas(decoder, &frame, fmt, get_canvas_line);
                        }
                    } else {
                        render_frame_to_canvas(decoder, &frame, fmt, get_canvas_line);
                    }
                }

                if let Some(end_frame) = (*decoder).callbacks.EndFrame {
                    end_frame(decoder);
                }

                (*decoder).pos = (*decoder).cur_frame as f32 / DUCK_FPS;
                (*decoder).cur_frame += 1;

                if let Some(set_timer) = (*decoder).callbacks.SetTimer {
                    set_timer(decoder, (*decoder).interframe_wait);
                }

                1
            }
            Err(VideoError::Eof) => 0,
            Err(_) => {
                (*rust_dec).last_error = -1;
                -1
            }
        }
    }
}

extern "C" fn rust_dukv_SeekFrame(decoder: *mut TFB_VideoDecoder, frame: u32) -> u32 {
    if decoder.is_null() {
        return 0;
    }
    unsafe {
        let frame_count = (*decoder).frame_count;
        let target = if frame >= frame_count { frame_count.saturating_sub(1) } else { frame };
        (*decoder).cur_frame = target;
        (*decoder).pos = target as f32 / DUCK_FPS;
        target
    }
}

extern "C" fn rust_dukv_SeekTime(decoder: *mut TFB_VideoDecoder, time: f32) -> f32 {
    if decoder.is_null() {
        return 0.0;
    }
    unsafe {
        let target_frame = (time * DUCK_FPS) as u32;
        let actual_frame = rust_dukv_SeekFrame(decoder, target_frame);
        actual_frame as f32 / DUCK_FPS
    }
}

extern "C" fn rust_dukv_GetFrame(decoder: *mut TFB_VideoDecoder) -> u32 {
    if decoder.is_null() {
        return 0;
    }
    unsafe { (*decoder).cur_frame }
}

extern "C" fn rust_dukv_GetTime(decoder: *mut TFB_VideoDecoder) -> f32 {
    if decoder.is_null() {
        return 0.0;
    }
    unsafe { (*decoder).pos }
}

#[no_mangle]
pub static rust_dukv_DecoderVtbl: TFB_VideoDecoderFuncs = TFB_VideoDecoderFuncs {
    GetName: rust_dukv_GetName,
    InitModule: rust_dukv_InitModule,
    TermModule: rust_dukv_TermModule,
    GetStructSize: rust_dukv_GetStructSize,
    GetError: rust_dukv_GetError,
    Init: rust_dukv_Init,
    Term: rust_dukv_Term,
    Open: rust_dukv_Open,
    Close: rust_dukv_Close,
    DecodeNext: rust_dukv_DecodeNext,
    SeekFrame: rust_dukv_SeekFrame,
    SeekTime: rust_dukv_SeekTime,
    GetFrame: rust_dukv_GetFrame,
    GetTime: rust_dukv_GetTime,
};

// ============================================================================
// Pure Rust player: simple C API
// ============================================================================

static PLAYER: Mutex<Option<VideoPlayer>> = Mutex::new(None);

#[no_mangle]
pub unsafe extern "C" fn rust_play_video(
    dir: *mut uio_DirHandle,
    filename: *const c_char,
    x: i32,
    y: i32,
    looping: bool,
) -> bool {
    if filename.is_null() {
        rust_bridge_log_msg("RUST_VIDEO: rust_play_video called with null filename");
        return false;
    }

    let filename_str = match CStr::from_ptr(filename).to_str() {
        Ok(s) => s,
        Err(_) => {
            rust_bridge_log_msg("RUST_VIDEO: rust_play_video filename is not valid UTF-8");
            return false;
        }
    };

    let basename = filename_str.trim_end_matches(".duk");

    let duk_file = format!("{}.duk", basename);
    let frm_file = format!("{}.frm", basename);
    let hdr_file = format!("{}.hdr", basename);
    let tbl_file = format!("{}.tbl", basename);

    let duk_data = match read_uio_file(dir, &duk_file) {
        Some(d) => d,
        None => return false,
    };
    let frm_data = match read_uio_file(dir, &frm_file) {
        Some(d) => d,
        None => return false,
    };
    let hdr_data = match read_uio_file(dir, &hdr_file) {
        Some(d) => d,
        None => return false,
    };
    let tbl_data = match read_uio_file(dir, &tbl_file) {
        Some(d) => d,
        None => return false,
    };

    let decoder = match DukVideoDecoder::open_from_data(&hdr_data, &tbl_data, &frm_data, &duk_data) {
        Ok(d) => d,
        Err(e) => {
            rust_bridge_log_msg(&format!("RUST_VIDEO: decoder init failed: {:?}", e));
            return false;
        }
    };

    let src_w = decoder.width();
    let src_h = decoder.height();
    let scale = calculate_video_scale(src_w, src_h);

    let mut player = VideoPlayer::new(decoder);
    player.set_position(x, y);
    player.set_loop(looping);

    if scale > 1 {
        let dst_w = src_w * scale;
        let dst_h = src_h * scale;
        player.set_scaler(Some(VideoScaler::new(src_w, src_h, dst_w, dst_h)));
    }

    rust_bridge_log_msg(&format!(
        "RUST_VIDEO: play {} ({}x{}), scale={}, dst=({}, {}), looping={}",
        filename_str,
        src_w,
        src_h,
        scale,
        x,
        y,
        looping
    ));

    player.play();

    if let Ok(mut guard) = PLAYER.lock() {
        *guard = Some(player);
    }

    true
}

#[no_mangle]
pub extern "C" fn rust_stop_video() {
    if let Ok(mut guard) = PLAYER.lock() {
        *guard = None;
    }
}

#[no_mangle]
pub extern "C" fn rust_video_playing() -> bool {
    if let Ok(guard) = PLAYER.lock() {
        if let Some(ref p) = *guard {
            return p.playing();
        }
    }
    false
}

#[no_mangle]
pub unsafe extern "C" fn rust_process_video_frame() -> bool {
    let screen = SDL_Screen;
    if screen.is_null() {
        rust_bridge_log_msg("RUST_VIDEO: SDL_Screen is null");
        return false;
    }

    let mut still_playing = false;

    if let Ok(mut guard) = PLAYER.lock() {
        if let Some(ref mut p) = *guard {
            still_playing = p.process_frame(screen);
            if still_playing {
                if !p.direct_window_mode() {
                    TFB_SwapBuffers(1);
                }
            } else {
                *guard = None;
            }
        } else {
            rust_bridge_log_msg("RUST_VIDEO: process_frame called with no active player");
        }
    } else {
        rust_bridge_log_msg("RUST_VIDEO: process_frame failed to lock player");
    }

    still_playing
}

/// Present a video frame directly to the window, bypassing the 320x240 SDL_Screen.
/// The frame data should be RGBA format where frame.data[0] is the R component.
/// Returns true on success, false on failure.
#[no_mangle]
pub unsafe extern "C" fn rust_present_video_to_window(
    frame_data: *const u8,
    width: c_int,
    height: c_int,
    stride: c_int, // row stride in bytes, typically width * 4 for RGBA
) -> bool {
    if frame_data.is_null() || width <= 0 || height <= 0 || stride <= 0 {
        rust_bridge_log_msg(&format!(
            "RUST_VIDEO: rust_present_video_to_window invalid params data={:p} w={} h={} stride={}",
            frame_data, width, height, stride
        ));
        return false;
    }

    let frame_len = (height * stride) as usize;
    let frame_slice = std::slice::from_raw_parts(frame_data, frame_len);

    let mut presented = false;
    let mut saw_state = false;
    let mut log_line = None;

    crate::graphics::ffi::with_gfx_state(|canvas, window_width, window_height| {
        saw_state = true;
        let texture_creator = canvas.texture_creator();
        let mut texture = match texture_creator.create_texture_streaming(
            sdl2::pixels::PixelFormatEnum::RGBX8888,
            width as u32,
            height as u32,
        ) {
            Ok(tex) => tex,
            Err(e) => {
                rust_bridge_log_msg(&format!(
                    "RUST_VIDEO: Failed to create video texture: {}",
                    e
                ));
                return;
            }
        };

        let mut upload_buffer: Vec<u8>;
        let upload_slice = if stride as usize == (width as usize * 4) {
            upload_buffer = Vec::with_capacity(frame_len);
            let pixels = frame_slice.chunks_exact(4);
            for px in pixels {
                let r = px[0];
                let g = px[1];
                let b = px[2];
                upload_buffer.extend_from_slice(&[0, b, g, r]);
            }
            upload_buffer.as_slice()
        } else {
            upload_buffer = Vec::with_capacity((width as usize * height as usize) * 4);
            for row in 0..height as usize {
                let row_start = row * stride as usize;
                let row_end = row_start + (width as usize * 4);
                let row_slice = &frame_slice[row_start..row_end];
                for px in row_slice.chunks_exact(4) {
                    let r = px[0];
                    let g = px[1];
                    let b = px[2];
                    upload_buffer.extend_from_slice(&[0, b, g, r]);
                }
            }
            upload_buffer.as_slice()
        };

        if let Err(e) = texture.update(None, upload_slice, (width * 4) as usize) {
            rust_bridge_log_msg(&format!(
                "RUST_VIDEO: Failed to update video texture: {}",
                e
            ));
            return;
        }

        let (old_logical_w, old_logical_h) = canvas.logical_size();
        if let Err(e) = canvas.set_logical_size(window_width, window_height) {
            rust_bridge_log_msg(&format!(
                "RUST_VIDEO: Failed to set logical size {}x{}: {}",
                window_width, window_height, e
            ));
            return;
        }

        let video_width = width as u32;
        let video_height = height as u32;

        let scale_x = window_width as f32 / video_width as f32;
        let scale_y = window_height as f32 / video_height as f32;
        let scale = scale_x.min(scale_y);

        let scaled_width = (video_width as f32 * scale) as u32;
        let scaled_height = (video_height as f32 * scale) as u32;

        let dst_x = (window_width - scaled_width) / 2;
        let dst_y = (window_height - scaled_height) / 2;

        let dst_rect = sdl2::rect::Rect::new(dst_x as i32, dst_y as i32, scaled_width, scaled_height);

        canvas.set_draw_color(sdl2::pixels::Color::BLACK);
        canvas.clear();

        if let Err(e) = canvas.copy(&texture, None, Some(dst_rect)) {
            rust_bridge_log_msg(&format!(
                "RUST_VIDEO: Failed to copy video texture: {}",
                e
            ));
            let _ = canvas.set_logical_size(old_logical_w, old_logical_h);
            return;
        }

        canvas.present();
        let _ = canvas.set_logical_size(old_logical_w, old_logical_h);

        presented = true;
        log_line = Some(format!(
            "RUST_VIDEO: Presented frame {}x{} stride={} scaled to {}x{} at ({},{}) window={}x{}",
            width, height, stride, scaled_width, scaled_height, dst_x, dst_y, window_width, window_height
        ));
    });

    if !presented {
        if !saw_state {
            rust_bridge_log_msg("RUST_VIDEO: rust_present_video_to_window no graphics state");
        }
        return false;
    }

    if let Some(line) = log_line {
        rust_bridge_log_msg(&line);
    }

    true
}

#[no_mangle]
pub extern "C" fn rust_get_video_position() -> u32 {
    if let Ok(guard) = PLAYER.lock() {
        if let Some(ref p) = *guard {
            return p.position_ms();
        }
    }
    0
}

/// Enhanced video player that uses direct window presentation
/// 
/// This function creates a VideoPlayer that scales with Lanczos to the actual
/// window size and presents directly to the window, bypassing the 320x240 surface
/// and completely avoiding xBRZ/hq2x scaling which would corrupt the video.
#[no_mangle]
pub unsafe extern "C" fn rust_play_video_direct_window(
    dir: *mut uio_DirHandle,
    filename: *const c_char,
    window_width: u32,
    window_height: u32,
    looping: bool,
) -> bool {
    if filename.is_null() {
        rust_bridge_log_msg("RUST_VIDEO: rust_play_video_direct_window called with null filename");
        return false;
    }

    let filename_str = match CStr::from_ptr(filename).to_str() {
        Ok(s) => s,
        Err(_) => {
            rust_bridge_log_msg("RUST_VIDEO: rust_play_video_direct_window filename is not valid UTF-8");
            return false;
        }
    };

    rust_bridge_log_msg(&format!(
        "RUST_VIDEO: Direct window play {} (window {}x{}), looping={} - bypassing xBRZ/hq2x completely",
        filename_str,
        window_width,
        window_height,
        looping
    ));

    if crate::graphics::ffi::with_gfx_state(|_, _, _| {}).is_none() {
        rust_bridge_log_msg("RUST_VIDEO: Direct window path unavailable (Rust gfx not initialized)");
        return false;
    }

    let basename = filename_str.trim_end_matches(".duk");

    let duk_file = format!("{}.duk", basename);
    let frm_file = format!("{}.frm", basename);
    let hdr_file = format!("{}.hdr", basename);
    let tbl_file = format!("{}.tbl", basename);

    let duk_data = match read_uio_file(dir, &duk_file) {
        Some(d) => d,
        None => return false,
    };
    let frm_data = match read_uio_file(dir, &frm_file) {
        Some(d) => d,
        None => return false,
    };
    let hdr_data = match read_uio_file(dir, &hdr_file) {
        Some(d) => d,
        None => return false,
    };
    let tbl_data = match read_uio_file(dir, &tbl_file) {
        Some(d) => d,
        None => return false,
    };

    let decoder = match DukVideoDecoder::open_from_data(&hdr_data, &tbl_data, &frm_data, &duk_data) {
        Ok(d) => d,
        Err(e) => {
            rust_bridge_log_msg(&format!("RUST_VIDEO: decoder init failed: {:?}", e));
            return false;
        }
    };

    let src_w = decoder.width();
    let src_h = decoder.height();
    
    rust_bridge_log_msg(&format!(
        "RUST_VIDEO: Source video {}x{}, Lanczos scaling to window {}x{}",
        src_w, src_h, window_width, window_height
    ));

    let lanczos_scaler = Some(LanczosVideoScaler::new(src_w, src_h, window_width, window_height));

    let mut player = VideoPlayer::new(decoder);
    player.set_position(0, 0); // Centered by renderer
    player.set_loop(looping);
    player.set_lanczos_scaler(lanczos_scaler);
    player.set_direct_window_mode(true);

    player.play();

    if let Ok(mut guard) = PLAYER.lock() {
        *guard = Some(player);
    }

    rust_bridge_log_msg("RUST_VIDEO: Direct window player initialized - video will bypass all gfx scaling");
    true
}
