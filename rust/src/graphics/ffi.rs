//! C FFI bindings for Rust graphics driver
//!
//! Rust owns SDL initialization - window, renderer.
//! We use REAL SDL_Surface objects created via SDL2 C API for compatibility.
//! C code gets real SDL_Surface pointers for drawing operations.

use std::cell::UnsafeCell;
use std::ffi::{c_char, c_int, c_void};
use std::ptr;

use sdl2::pixels::PixelFormatEnum;
use sdl2::render::BlendMode;

use crate::bridge_log::rust_bridge_log_msg;
use crate::graphics::pixmap::{Pixmap, PixmapFormat};
use crate::graphics::scaling::{Hq2xScaler, ScaleMode as RustScaleMode, ScaleParams, Scaler};
use xbrz::scale_rgba;

/// Number of screens (Main, Extra, Transition)
const TFB_GFX_NUMSCREENS: usize = 3;
// Base game resolution - the C code uses ScreenWidth/ScreenHeight global vars
// but those are set to 320x240 for the logical game resolution
const SCREEN_WIDTH: u32 = 320;
const SCREEN_HEIGHT: u32 = 240;

// RGBX8888 masks for screen surfaces - MUST match sdl2_pure.c on little-endian
// The C code (sdl2_pure.c lines 48-52) uses these masks on little-endian (Mac):
//   A_MASK = 0x000000ff, B_MASK = 0x0000ff00, G_MASK = 0x00ff0000, R_MASK = 0xff000000
// Screen surfaces MUST be non-alpha for DRAW_ALPHA support (gfxlib.h)
const R_MASK: u32 = 0xFF000000;  // R in bits 24-31
const G_MASK: u32 = 0x00FF0000;  // G in bits 16-23
const B_MASK: u32 = 0x0000FF00;  // B in bits 8-15
const A_MASK_SCREEN: u32 = 0x00000000; // no alpha on screen surfaces
const A_MASK_ALPHA: u32 = 0x000000FF;  // alpha mask for format_conv_surf (font backing)

// Real SDL_Surface structure layout from SDL2
#[repr(C)]
pub struct SDL_Surface {
    pub flags: u32,
    pub format: *mut c_void, // SDL_PixelFormat*
    pub w: c_int,
    pub h: c_int,
    pub pitch: c_int,
    pub pixels: *mut c_void,
    pub userdata: *mut c_void,
    pub locked: c_int,
    pub list_blitmap: *mut c_void,
    pub clip_rect: SDL_Rect,
    pub map: *mut c_void,
    pub refcount: c_int,
}

/// SDL_Rect for C interop
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct SDL_Rect {
    pub x: c_int,
    pub y: c_int,
    pub w: c_int,
    pub h: c_int,
}

// Import SDL2 C functions for creating real surfaces
extern "C" {
    fn SDL_CreateRGBSurface(
        flags: u32,
        width: c_int,
        height: c_int,
        depth: c_int,
        Rmask: u32,
        Gmask: u32,
        Bmask: u32,
        Amask: u32,
    ) -> *mut SDL_Surface;
    fn SDL_FreeSurface(surface: *mut SDL_Surface);
}

/// Thread-local graphics state wrapper
struct GraphicsStateCell(UnsafeCell<Option<RustGraphicsState>>);

// Safety: Graphics is only accessed from main thread
unsafe impl Sync for GraphicsStateCell {}

/// The Rust graphics state - owns everything
struct RustGraphicsState {
    /// SDL2 context - dropped last
    sdl_context: sdl2::Sdl,
    /// Video subsystem - dropped after canvas
    video: sdl2::VideoSubsystem,
    /// Renderer/Canvas (owns the window) - dropped first
    canvas: sdl2::render::Canvas<sdl2::video::Window>,
    /// Event pump
    event_pump: sdl2::EventPump,
    /// Real SDL_Surface pointers created via SDL_CreateRGBSurface
    surfaces: [*mut SDL_Surface; TFB_GFX_NUMSCREENS],
    /// Format conversion surface
    format_conv_surf: *mut SDL_Surface,
    /// Soft-scaled buffers (HQ2x or xBRZ), if enabled
    scaled_buffers: [Option<Vec<u8>>; TFB_GFX_NUMSCREENS],
    /// Rust HQ2x scaler
    hq2x: Hq2xScaler,
    /// Whether we've logged that HQ2x is active
    hq2x_logged: bool,
    /// Whether we've logged that xBRZ is active
    xbrz_logged: bool,
    /// Init flags passed from C
    flags: c_int,
    /// Window dimensions
    width: u32,
    height: u32,
    /// Fullscreen state
    fullscreen: bool,
}

static RUST_GFX: GraphicsStateCell = GraphicsStateCell(UnsafeCell::new(None));

fn get_gfx_state() -> Option<&'static mut RustGraphicsState> {
    unsafe { (*RUST_GFX.0.get()).as_mut() }
}

pub(crate) fn with_gfx_state<F, R>(f: F) -> Option<R>
where
    F: FnOnce(&mut sdl2::render::Canvas<sdl2::video::Window>, u32, u32) -> R,
{
    get_gfx_state().map(|state| f(&mut state.canvas, state.width, state.height))
}

fn set_gfx_state(state: Option<RustGraphicsState>) {
    unsafe {
        *RUST_GFX.0.get() = state;
    }
}

// ============================================================================
// Initialization - Rust takes over all SDL
// ============================================================================

/// Initialize Rust graphics - creates window, renderer, real SDL surfaces.
/// Returns 0 on success, -1 on failure.
///
/// @plan PLAN-20260223-GFX-FULL-PORT.P03
/// @requirement REQ-INIT-095, REQ-INIT-015, REQ-INIT-020, REQ-INIT-030,
///              REQ-INIT-040, REQ-INIT-050, REQ-INIT-055, REQ-INIT-060,
///              REQ-INIT-080, REQ-INIT-090, REQ-INIT-100, REQ-FMT-030
#[no_mangle]
pub extern "C" fn rust_gfx_init(
    _driver: c_int,
    flags: c_int,
    _renderer: *const c_char,
    width: c_int,
    height: c_int,
) -> c_int {
    // REQ-INIT-095: Already-initialized guard — prevent double-init
    if get_gfx_state().is_some() {
        rust_bridge_log_msg("RUST_GFX_INIT: Already initialized, returning -1");
        return -1;
    }

    rust_bridge_log_msg(&format!(
        "RUST_GFX_INIT: flags=0x{:x} ({}) width={} height={}",
        flags, flags, width, height
    ));
    
    // Log scaler flags for debugging
    if (flags & (1 << 3)) != 0 { rust_bridge_log_msg("  SCALE_BILINEAR set"); }
    if (flags & (1 << 4)) != 0 { rust_bridge_log_msg("  SCALE_BIADAPT set"); }
    if (flags & (1 << 5)) != 0 { rust_bridge_log_msg("  SCALE_BIADAPTADV set"); }
    if (flags & (1 << 6)) != 0 { rust_bridge_log_msg("  SCALE_TRISCAN set"); }
    if (flags & (1 << 7)) != 0 { rust_bridge_log_msg("  SCALE_HQXX set"); }
    if (flags & (1 << 8)) != 0 { rust_bridge_log_msg("  SCALE_XBRZ3 set"); }
    if (flags & (1 << 9)) != 0 { rust_bridge_log_msg("  SCALE_XBRZ4 set"); }

    let fullscreen = (flags & 0x01) != 0;

    // Initialize SDL2 via rust-sdl2
    let sdl_context = match sdl2::init() {
        Ok(ctx) => ctx,
        Err(e) => {
            rust_bridge_log_msg(&format!("RUST_GFX_INIT: SDL2 init failed: {}", e));
            return -1;
        }
    };

    let video = match sdl_context.video() {
        Ok(v) => v,
        Err(e) => {
            rust_bridge_log_msg(&format!("RUST_GFX_INIT: Video subsystem failed: {}", e));
            return -1;
        }
    };

    rust_bridge_log_msg(&format!(
        "RUST_GFX_INIT: SDL2 video driver: {}",
        video.current_video_driver()
    ));

    // Create window
    let mut window_builder =
        video.window("The Ur-Quan Masters v0.8.0 (Rust)", width as u32, height as u32);
    window_builder.position_centered();

    let window = match window_builder.build() {
        Ok(w) => w,
        Err(e) => {
            rust_bridge_log_msg(&format!("RUST_GFX_INIT: Window creation failed: {}", e));
            return -1;
        }
    };

    // Create canvas/renderer (software renderer avoids GPU format surprises)
    let mut canvas = match window.into_canvas().software().present_vsync().build() {
        Ok(c) => c,
        Err(e) => {
            rust_bridge_log_msg(&format!("RUST_GFX_INIT: Renderer creation failed: {}", e));
            return -1;
        }
    };

    // Force nearest-neighbor scaling for crisp pixels (matches SDL2 pure path)
    let _ = sdl2::hint::set("SDL_HINT_RENDER_SCALE_QUALITY", "0");

    // Set logical size for scaling
    if let Err(e) = canvas.set_logical_size(SCREEN_WIDTH, SCREEN_HEIGHT) {
        rust_bridge_log_msg(&format!("RUST_GFX_INIT: set_logical_size failed: {}", e));
        return -1;
    }

    rust_bridge_log_msg(&format!(
        "RUST_GFX_INIT: Logical size {}x{} (window {}x{})",
        SCREEN_WIDTH,
        SCREEN_HEIGHT,
        width,
        height
    ));

    rust_bridge_log_msg(&format!("RUST_GFX_INIT: Renderer: {}", canvas.info().name));

    // Get event pump
    let event_pump = match sdl_context.event_pump() {
        Ok(ep) => ep,
        Err(e) => {
            rust_bridge_log_msg(&format!("RUST_GFX_INIT: Event pump failed: {}", e));
            return -1;
        }
    };

    // Create REAL SDL surfaces via SDL_CreateRGBSurface
    let mut surfaces: [*mut SDL_Surface; TFB_GFX_NUMSCREENS] =
        [ptr::null_mut(); TFB_GFX_NUMSCREENS];

    for i in 0..TFB_GFX_NUMSCREENS {
        let surface = unsafe {
            SDL_CreateRGBSurface(
                0, // flags
                SCREEN_WIDTH as c_int,
                SCREEN_HEIGHT as c_int,
                32, // depth
                R_MASK,
                G_MASK,
                B_MASK,
                A_MASK_SCREEN,
            )
        };

        if surface.is_null() {
            rust_bridge_log_msg(&format!("RUST_GFX_INIT: Failed to create surface {}", i));
            // Clean up already created surfaces
            for j in 0..i {
                if !surfaces[j].is_null() {
                    unsafe { SDL_FreeSurface(surfaces[j]) };
                }
            }
            return -1;
        }

        surfaces[i] = surface;
        rust_bridge_log_msg(&format!("RUST_GFX_INIT: Created surface {}: {:p}", i, surface));
    }

    // Create format conversion surface (same format)
    let format_conv_surf = unsafe {
        SDL_CreateRGBSurface(0, 0, 0, 32, R_MASK, G_MASK, B_MASK, A_MASK_ALPHA)
    };

    if format_conv_surf.is_null() {
        rust_bridge_log_msg("RUST_GFX_INIT: Failed to create format_conv_surf");
        for i in 0..TFB_GFX_NUMSCREENS {
            if !surfaces[i].is_null() {
                unsafe { SDL_FreeSurface(surfaces[i]) };
            }
        }
        return -1;
    }

    let mut state = RustGraphicsState {
        sdl_context,
        video,
        canvas,
        event_pump,
        surfaces,
        format_conv_surf,
        scaled_buffers: [None, None, None],
        hq2x: Hq2xScaler::new(),
        hq2x_logged: false,
        xbrz_logged: false,
        flags,
        width: width as u32,
        height: height as u32,
        fullscreen,
    };

    // Configure soft scaling when requested (HQ2x or xBRZ)
    let scale_any = flags & ((1 << 3) | (1 << 4) | (1 << 5) | (1 << 6) | (1 << 7) | (1 << 8) | (1 << 9));
    let use_soft_scaler = scale_any != 0 && (flags & (1 << 3)) == 0; // SOFT_ONLY = SCALE_ANY & ~BILINEAR
    if use_soft_scaler {
        let scale_factor = if (flags & (1 << 8)) != 0 { 3 } else if (flags & (1 << 9)) != 0 { 4 } else { 2 };
        let buffer_size = (SCREEN_WIDTH * scale_factor * SCREEN_HEIGHT * scale_factor * 4) as usize;
        for i in 0..TFB_GFX_NUMSCREENS {
            state.scaled_buffers[i] = Some(vec![0u8; buffer_size]);
        }
    }

    set_gfx_state(Some(state));

    rust_bridge_log_msg("RUST_GFX_INIT: Success");
    0
}

/// Uninitialize graphics
#[no_mangle]
pub extern "C" fn rust_gfx_uninit() {
    rust_bridge_log_msg("RUST_GFX_UNINIT");

    // Take ownership of the state so we can control drop order
    let state_opt = unsafe { (*RUST_GFX.0.get()).take() };
    
    if let Some(mut state) = state_opt {
        // Free SDL surfaces BEFORE dropping the SDL context
        // The surfaces must be freed while SDL is still initialized
        for i in 0..TFB_GFX_NUMSCREENS {
            state.scaled_buffers[i] = None;
        }
        for i in 0..TFB_GFX_NUMSCREENS {
            if !state.surfaces[i].is_null() {
                unsafe { SDL_FreeSurface(state.surfaces[i]) };
                state.surfaces[i] = std::ptr::null_mut();
            }
        }
        if !state.format_conv_surf.is_null() {
            unsafe { SDL_FreeSurface(state.format_conv_surf) };
            state.format_conv_surf = std::ptr::null_mut();
        }
        
        // Drop canvas first, then video, then sdl_context
        // We need to be explicit about drop order
        drop(state.canvas);
        drop(state.video);
        drop(state.sdl_context);
    }
    
    rust_bridge_log_msg("RUST_GFX_UNINIT: Done");
}

// ============================================================================
// Screen access for C code - returns real SDL_Surface pointers
// ============================================================================

/// Get SDL_Screen pointer for C code (main screen = 0)
#[no_mangle]
pub extern "C" fn rust_gfx_get_sdl_screen() -> *mut SDL_Surface {
    rust_gfx_get_screen_surface(0)
}

/// Get TransitionScreen pointer for C code (screen = 2)
#[no_mangle]
pub extern "C" fn rust_gfx_get_transition_screen() -> *mut SDL_Surface {
    rust_gfx_get_screen_surface(2)
}

/// Get SDL_Screens[i] pointer for C code
#[no_mangle]
pub extern "C" fn rust_gfx_get_screen_surface(screen: c_int) -> *mut SDL_Surface {
    if screen < 0 || screen >= TFB_GFX_NUMSCREENS as c_int {
        return ptr::null_mut();
    }

    if let Some(state) = get_gfx_state() {
        return state.surfaces[screen as usize];
    }
    ptr::null_mut()
}

/// Get format_conv_surf for C code
#[no_mangle]
pub extern "C" fn rust_gfx_get_format_conv_surf() -> *mut SDL_Surface {
    if let Some(state) = get_gfx_state() {
        return state.format_conv_surf;
    }
    ptr::null_mut()
}

// ============================================================================
// TFB_GRAPHICS_BACKEND vtable functions
// ============================================================================

/// Preprocess - called before rendering. Sets blend mode to None and clears.
///
/// @plan PLAN-20260223-GFX-FULL-PORT.P03
/// @requirement REQ-PRE-010
#[no_mangle]
pub extern "C" fn rust_gfx_preprocess(
    _force_redraw: c_int,
    _transition_amount: c_int,
    _fade_amount: c_int,
) {
    if let Some(state) = get_gfx_state() {
        // REQ-PRE-010: Reset blend mode before clearing for clean renderer state
        state.canvas.set_blend_mode(BlendMode::None);
        state.canvas.set_draw_color(sdl2::pixels::Color::BLACK);
        state.canvas.clear();
    }
}

/// Postprocess - called after rendering, does the actual display.
///
/// @plan PLAN-20260223-GFX-FULL-PORT.P03
/// @requirement REQ-POST-010, REQ-POST-020, REQ-INV-010
///
/// Per REQ-POST-020 / REQ-INV-010, the end-state for postprocess is
/// present-only (no texture creation, no surface upload, no canvas.copy).
/// The upload/scaling logic below is retained until ScreenLayer (P06-P08)
/// takes over compositing; removing it now would produce a black screen.
/// @plan remove upload/scaling block in P05 once ScreenLayer composites.
#[no_mangle]
pub extern "C" fn rust_gfx_postprocess() {
    if let Some(state) = get_gfx_state() {
        // @plan P05: Remove this texture upload block once ScreenLayer composites.
        // Get pixels from the main screen surface and upload to texture
        let texture_creator = state.canvas.texture_creator();

        // Surface is 32bpp RGBX (R=0xFF000000, G=0x00FF0000, B=0x0000FF00, A=0x00000000)
        // Use RGBX8888 texture format which matches this layout
        let use_soft_scaler = state.scaled_buffers[0].is_some();
        let scale_factor = if (state.flags & (1 << 8)) != 0 {
            3
        } else if (state.flags & (1 << 9)) != 0 {
            4
        } else {
            2
        };
        let tex_w = if use_soft_scaler {
            SCREEN_WIDTH * scale_factor
        } else {
            SCREEN_WIDTH
        };
        let tex_h = if use_soft_scaler {
            SCREEN_HEIGHT * scale_factor
        } else {
            SCREEN_HEIGHT
        };

        if let Ok(mut texture) = texture_creator.create_texture_streaming(
            PixelFormatEnum::RGBX8888,
            tex_w,
            tex_h,
        ) {
            let src_surface = state.surfaces[0];
            let mut uploaded = false;

            if use_soft_scaler {
                let scale_factor = if (state.flags & (1 << 8)) != 0 {
                    3
                } else if (state.flags & (1 << 9)) != 0 {
                    4
                } else {
                    2
                };
                let using_xbrz = (state.flags & ((1 << 8) | (1 << 9))) != 0;
                if using_xbrz {
                    // Log once per run
                } else if !state.hq2x_logged {
                    rust_bridge_log_msg("RUST_GFX: HQ2x scaler active");
                    state.hq2x_logged = true;
                }
                if let Some(buffer) = state.scaled_buffers[0].as_mut() {
                    if !src_surface.is_null() {
                        unsafe {
                            let surf = &*src_surface;
                            if !surf.pixels.is_null() && surf.pitch > 0 {
                                let src_pitch = surf.pitch as usize;
                                let src_width = SCREEN_WIDTH as usize;
                                let src_height = SCREEN_HEIGHT as usize;
                                let src_size = src_pitch * src_height;
                                let src_bytes = std::slice::from_raw_parts(
                                    surf.pixels as *const u8,
                                    src_size,
                                );

                                let mut pixmap = Pixmap::new(
                                    std::num::NonZeroU32::new(1).unwrap(),
                                    SCREEN_WIDTH,
                                    SCREEN_HEIGHT,
                                    PixmapFormat::Rgba32,
                                ).unwrap();
                                let dst_bytes = pixmap.data_mut();

                                // Source is RGBX8888 in memory: bytes are [X, B, G, R] on little-endian
                                // xBRZ expects RGBA: bytes are [R, G, B, A]
                                for y in 0..src_height {
                                    let src_row = &src_bytes[y * src_pitch..(y * src_pitch + src_width * 4)];
                                    let dst_row = &mut dst_bytes[y * src_width * 4..(y + 1) * src_width * 4];
                                    for x in 0..src_width {
                                        let s = &src_row[x * 4..x * 4 + 4];
                                        let d = &mut dst_row[x * 4..x * 4 + 4];
                                        // RGBX8888 memory [X,B,G,R] -> RGBA [R,G,B,A]
                                        d[0] = s[3]; // R
                                        d[1] = s[2]; // G
                                        d[2] = s[1]; // B
                                        d[3] = 0xFF; // A (opaque)
                                    }
                                }

                                if using_xbrz {
                                    let scaled_bytes =
                                        scale_rgba(dst_bytes, src_width, src_height, scale_factor);
                                    let dst_width = SCREEN_WIDTH as usize * scale_factor;
                                    let dst_height = SCREEN_HEIGHT as usize * scale_factor;
                                    let dst_stride = dst_width * 4;
                                    
                                    if !state.xbrz_logged {
                                        rust_bridge_log_msg(&format!("RUST_GFX: xBRZ scaler active ({}x)", scale_factor));
                                        rust_bridge_log_msg(&format!("RUST_GFX: xBRZ input size {}x{}, output size {}x{}, stride {}", 
                                            src_width, src_height, dst_width, dst_height, dst_stride));
                                        rust_bridge_log_msg(&format!("RUST_GFX: xBRZ scaled_bytes len={}, buffer len={}", 
                                            scaled_bytes.len(), buffer.len()));
                                        state.xbrz_logged = true;
                                    }

                                    // xBRZ outputs RGBA [R,G,B,A], texture is RGBX8888 [X,B,G,R] in memory
                                    for y in 0..dst_height {
                                        let src_row =
                                            &scaled_bytes[y * dst_stride..(y + 1) * dst_stride];
                                        let dst_row =
                                            &mut buffer[y * dst_stride..(y + 1) * dst_stride];
                                        for x in 0..dst_width {
                                            let s = &src_row[x * 4..x * 4 + 4];
                                            let d = &mut dst_row[x * 4..x * 4 + 4];
                                            // RGBA [R,G,B,A] -> RGBX8888 memory [X,B,G,R]
                                            d[0] = 0xFF; // X (padding)
                                            d[1] = s[2]; // B
                                            d[2] = s[1]; // G
                                            d[3] = s[0]; // R
                                        }
                                    }

                                    let _ = texture.update(None, buffer, dst_stride);
                                    uploaded = true;
                                } else {
                                    let params = ScaleParams::new(512, RustScaleMode::Hq2x);
                                    if let Ok(scaled) = state.hq2x.scale(&pixmap, params) {
                                        let scaled_bytes = scaled.data();
                                        let dst_width = SCREEN_WIDTH as usize * 2;
                                        let dst_height = SCREEN_HEIGHT as usize * 2;
                                        let dst_stride = dst_width * 4;

                                        for y in 0..dst_height {
                                            let src_row = &scaled_bytes
                                                [y * dst_stride..(y + 1) * dst_stride];
                                            let dst_row = &mut buffer
                                                [y * dst_stride..(y + 1) * dst_stride];
                                            for x in 0..dst_width {
                                                let s = &src_row[x * 4..x * 4 + 4];
                                                let d = &mut dst_row[x * 4..x * 4 + 4];
                                                d[0] = s[3]; // X
                                                d[1] = s[2]; // B
                                                d[2] = s[1]; // G
                                                d[3] = s[0]; // R
                                            }
                                        }

                                        let _ = texture.update(None, buffer, dst_stride);
                                        uploaded = true;
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if !uploaded {
                if !src_surface.is_null() {
                    unsafe {
                        let surf = &*src_surface;
                        if !surf.pixels.is_null() && surf.pitch > 0 {
                            let pitch = surf.pitch as usize;
                            let height = SCREEN_HEIGHT as usize;
                            let total_size = pitch * height;
                            let pixel_data = std::slice::from_raw_parts(
                                surf.pixels as *const u8,
                                total_size,
                            );
                            let _ = texture.update(None, pixel_data, pitch);
                        }
                    }
                }
            }
            
            let _ = state.canvas.copy(&texture, None, None);
        }

        state.canvas.present();
    }
}

/// Upload transition screen (for transition effects)
#[no_mangle]
pub extern "C" fn rust_gfx_upload_transition_screen() {
    // No-op for now
}

/// Draw a screen layer
#[no_mangle]
pub extern "C" fn rust_gfx_screen(_screen: c_int, _alpha: u8, _rect: *const SDL_Rect) {
    // The actual drawing is done by C code directly to the SDL surfaces
}

/// Draw a color layer (for fades)
#[no_mangle]
pub extern "C" fn rust_gfx_color(_r: u8, _g: u8, _b: u8, _a: u8, _rect: *const SDL_Rect) {
    // TODO: Implement fade overlay
}

// ============================================================================
// Event processing
// ============================================================================

/// Process SDL events, returns 1 if quit requested
#[no_mangle]
pub extern "C" fn rust_gfx_process_events() -> c_int {
    if let Some(state) = get_gfx_state() {
        for event in state.event_pump.poll_iter() {
            match event {
                sdl2::event::Event::Quit { .. } => return 1,
                _ => {}
            }
        }
    }
    0
}

// ============================================================================
// Utility functions
// ============================================================================

/// Toggle fullscreen
#[no_mangle]
pub extern "C" fn rust_gfx_toggle_fullscreen() -> c_int {
    if let Some(state) = get_gfx_state() {
        state.fullscreen = !state.fullscreen;
        return if state.fullscreen { 1 } else { 0 };
    }
    -1
}

/// Check if fullscreen
#[no_mangle]
pub extern "C" fn rust_gfx_is_fullscreen() -> c_int {
    if let Some(state) = get_gfx_state() {
        return if state.fullscreen { 1 } else { 0 };
    }
    0
}

/// Set gamma (not supported in software mode)
#[no_mangle]
pub extern "C" fn rust_gfx_set_gamma(_gamma: f32) -> c_int {
    -1 // Not supported
}

/// Get screen width
#[no_mangle]
pub extern "C" fn rust_gfx_get_width() -> c_int {
    SCREEN_WIDTH as c_int
}

/// Get screen height
#[no_mangle]
pub extern "C" fn rust_gfx_get_height() -> c_int {
    SCREEN_HEIGHT as c_int
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sdl_rect_size() {
        assert_eq!(std::mem::size_of::<SDL_Rect>(), 16);
    }
}
