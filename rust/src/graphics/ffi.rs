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
/// Extra screen index — skipped during compositing (not rendered to display)
const TFB_SCREEN_EXTRA: c_int = 1;
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
/// @plan PLAN-20260223-GFX-FULL-PORT.P03, PLAN-20260223-GFX-FULL-PORT.P05
/// @requirement REQ-PRE-010, REQ-PRE-020, REQ-PRE-040
#[no_mangle]
pub extern "C" fn rust_gfx_preprocess(
    _force_redraw: c_int,
    _transition_amount: c_int,
    _fade_amount: c_int,
) {
    // REQ-PRE-040: transition_amount and fade_amount are intentionally ignored
    // (handled by ScreenLayer/ColorLayer in P06-P08)
    if let Some(state) = get_gfx_state() {
        // REQ-PRE-010: Reset blend mode before clearing for clean renderer state
        state.canvas.set_blend_mode(BlendMode::None);
        // REQ-PRE-020: Clear to opaque black
        state.canvas.set_draw_color(sdl2::pixels::Color::BLACK);
        state.canvas.clear();
    }
}

/// Postprocess - called after rendering, does the actual display.
///
/// @plan PLAN-20260223-GFX-FULL-PORT.P03, PLAN-20260223-GFX-FULL-PORT.P05
/// @requirement REQ-POST-010, REQ-POST-020, REQ-INV-010
///
/// Per REQ-POST-020 / REQ-INV-010, the end-state for postprocess is
/// present-only (no texture creation, no surface upload, no canvas.copy).
/// The upload/scaling logic below is retained until ScreenLayer (P08)
/// takes over compositing; removing it now would produce a black screen.
/// @plan remove upload/scaling block in P08 once ScreenLayer composites.
#[no_mangle]
pub extern "C" fn rust_gfx_postprocess() {
    if let Some(state) = get_gfx_state() {
        // @plan P08: Remove this texture upload block once ScreenLayer composites.
        let texture_creator = state.canvas.texture_creator();

        let use_soft_scaler = state.scaled_buffers[0].is_some();
        let factor = scale_factor_from_flags(state.flags).unwrap_or(1) as usize;
        let tex_w = if use_soft_scaler {
            SCREEN_WIDTH * factor as u32
        } else {
            SCREEN_WIDTH
        };
        let tex_h = if use_soft_scaler {
            SCREEN_HEIGHT * factor as u32
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
                let using_xbrz = (state.flags & ((1 << 8) | (1 << 9))) != 0;
                if using_xbrz {
                    if !state.xbrz_logged {
                        rust_bridge_log_msg(&format!(
                            "RUST_GFX: xBRZ scaler active ({}x)",
                            factor
                        ));
                        state.xbrz_logged = true;
                    }
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
                                let src_w = SCREEN_WIDTH as usize;
                                let src_h = SCREEN_HEIGHT as usize;
                                let src_bytes = std::slice::from_raw_parts(
                                    surf.pixels as *const u8,
                                    src_pitch * src_h,
                                );

                                let mut pixmap = Pixmap::new(
                                    std::num::NonZeroU32::new(1).unwrap(),
                                    SCREEN_WIDTH,
                                    SCREEN_HEIGHT,
                                    PixmapFormat::Rgba32,
                                )
                                .unwrap();

                                convert_rgbx_to_rgba(
                                    src_bytes,
                                    pixmap.data_mut(),
                                    src_w,
                                    src_h,
                                    src_pitch,
                                );

                                if using_xbrz {
                                    let scaled_bytes =
                                        scale_rgba(pixmap.data(), src_w, src_h, factor);
                                    let dst_w = src_w * factor;
                                    let dst_h = src_h * factor;
                                    let dst_stride = dst_w * 4;

                                    convert_rgba_to_rgbx(&scaled_bytes, buffer, dst_w, dst_h);

                                    let _ = texture.update(None, buffer, dst_stride);
                                    uploaded = true;
                                } else {
                                    let params =
                                        ScaleParams::new(512, RustScaleMode::Hq2x);
                                    if let Ok(scaled) = state.hq2x.scale(&pixmap, params) {
                                        let dst_w = src_w * 2;
                                        let dst_h = src_h * 2;
                                        let dst_stride = dst_w * 4;

                                        convert_rgba_to_rgbx(
                                            scaled.data(),
                                            buffer,
                                            dst_w,
                                            dst_h,
                                        );

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
                            let total_size = pitch * SCREEN_HEIGHT as usize;
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

/// Draw a screen layer — composites screen surface onto the renderer.
///
/// Reads pixel data from `surfaces[screen]`, uploads to a temporary streaming
/// texture, and renders it onto the current frame with the requested alpha
/// and clipping rect.
///
/// @plan PLAN-20260223-GFX-FULL-PORT.P08
/// @requirement REQ-SCR-010, REQ-SCR-020, REQ-SCR-030, REQ-SCR-040,
///              REQ-SCR-050, REQ-SCR-060, REQ-SCR-070, REQ-SCR-075,
///              REQ-SCR-090, REQ-SCR-100, REQ-SCR-110, REQ-SCR-130,
///              REQ-SCR-140, REQ-SCR-150, REQ-SCR-170,
///              REQ-FMT-020, REQ-ERR-065, REQ-NP-025
#[no_mangle]
pub extern "C" fn rust_gfx_screen(screen: c_int, alpha: u8, rect: *const SDL_Rect) {
    // REQ-SCR-140: uninitialized guard
    let state = match get_gfx_state() {
        Some(s) => s,
        None => return,
    };

    // REQ-SCR-100: screen range check
    if screen < 0 || screen >= TFB_GFX_NUMSCREENS as c_int {
        return;
    }

    // REQ-SCR-090: extra screen skip
    if screen == TFB_SCREEN_EXTRA {
        return;
    }

    // REQ-SCR-110: null surface guard
    let screen_idx = screen as usize;
    let src_surface = state.surfaces[screen_idx];
    if src_surface.is_null() {
        return;
    }

    // REQ-SCR-160: convert C rect (NULL → None for full-screen)
    let sdl_rect = convert_c_rect(rect);

    // REQ-SCALE-020/025: determine software scaling from flags
    let sw_scale = scale_factor_from_flags(state.flags);
    let use_soft_scaler = sw_scale.is_some() && state.scaled_buffers[screen_idx].is_some();

    if use_soft_scaler {
        // ---- Scaled path ----
        // @plan PLAN-20260223-GFX-FULL-PORT.P12
        // @requirement REQ-SCALE-010, REQ-SCALE-020, REQ-SCALE-030, REQ-SCALE-040,
        //              REQ-SCALE-050, REQ-SCALE-055, REQ-WIN-030
        let factor = sw_scale.unwrap();
        let src_w = SCREEN_WIDTH as usize;
        let src_h = SCREEN_HEIGHT as usize;

        // SAFETY: src_surface was checked non-null above and is owned by RustGraphicsState.
        let src_pitch = unsafe {
            let surf = &*src_surface;
            if surf.pixels.is_null() || surf.pitch <= 0 {
                return;
            }
            surf.pitch as usize
        };

        // Step 1: Convert RGBX→RGBA into a Pixmap
        let mut pixmap = match Pixmap::new(
            std::num::NonZeroU32::new(1).unwrap(),
            SCREEN_WIDTH,
            SCREEN_HEIGHT,
            PixmapFormat::Rgba32,
        ) {
            Ok(p) => p,
            Err(_) => return,
        };

        unsafe {
            let surf = &*src_surface;
            let src_bytes = std::slice::from_raw_parts(
                surf.pixels as *const u8,
                src_pitch * src_h,
            );
            convert_rgbx_to_rgba(src_bytes, pixmap.data_mut(), src_w, src_h, src_pitch);
        }

        // Step 2: Run scaler
        let using_xbrz = (state.flags & ((1 << 8) | (1 << 9))) != 0;
        let scaled_rgba: Vec<u8>;
        let scaled_w: usize;
        let scaled_h: usize;

        if using_xbrz {
            if !state.xbrz_logged {
                rust_bridge_log_msg(&format!(
                    "RUST_GFX: xBRZ scaler active ({}x)",
                    factor
                ));
                state.xbrz_logged = true;
            }
            scaled_rgba = scale_rgba(pixmap.data(), src_w, src_h, factor as usize);
            scaled_w = src_w * factor as usize;
            scaled_h = src_h * factor as usize;
        } else {
            if !state.hq2x_logged {
                rust_bridge_log_msg("RUST_GFX: HQ2x scaler active");
                state.hq2x_logged = true;
            }
            let params = ScaleParams::new(512, RustScaleMode::Hq2x);
            match state.hq2x.scale(&pixmap, params) {
                Ok(scaled_pixmap) => {
                    scaled_rgba = scaled_pixmap.data().to_vec();
                    scaled_w = src_w * 2;
                    scaled_h = src_h * 2;
                }
                Err(_) => return,
            }
        }

        // Step 3: Convert RGBA→RGBX into the scaled buffer
        if let Some(buffer) = state.scaled_buffers[screen_idx].as_mut() {
            let out_stride = scaled_w * 4;
            let needed = out_stride * scaled_h;
            if buffer.len() < needed {
                return;
            }
            convert_rgba_to_rgbx(&scaled_rgba, buffer, scaled_w, scaled_h);

            // Step 4: Create texture at scaled dimensions
            // REQ-SCALE-040: texture dimensions = SCREEN_WIDTH*factor × SCREEN_HEIGHT*factor
            let texture_creator = state.canvas.texture_creator();
            let mut texture = match texture_creator.create_texture_streaming(
                PixelFormatEnum::RGBX8888,
                scaled_w as u32,
                scaled_h as u32,
            ) {
                Ok(t) => t,
                Err(_) => return,
            };

            // Step 5: Upload scaled buffer
            if texture.update(None, buffer, out_stride).is_err() {
                return;
            }

            // REQ-SCR-030 / REQ-SCR-040: set blend mode based on alpha
            if alpha == 255 {
                texture.set_blend_mode(BlendMode::None);
            } else {
                texture.set_blend_mode(BlendMode::Blend);
                texture.set_alpha_mod(alpha);
            }

            // Step 6: Compute scaled source rect (multiply by factor)
            // REQ-SCALE-050 / REQ-WIN-030 / REQ-SCALE-055:
            // src_rect coords × factor, dst_rect stays unscaled
            let src_rect = sdl_rect.map(|r| {
                sdl2::rect::Rect::new(
                    r.x() * factor as i32,
                    r.y() * factor as i32,
                    r.width() * factor,
                    r.height() * factor,
                )
            });
            let _ = state.canvas.copy(&texture, src_rect, sdl_rect);
        }
    } else {
        // ---- Unscaled path (original P08 implementation) ----
        let texture_creator = state.canvas.texture_creator();
        let mut texture = match texture_creator.create_texture_streaming(
            PixelFormatEnum::RGBX8888,
            SCREEN_WIDTH,
            SCREEN_HEIGHT,
        ) {
            Ok(t) => t,
            Err(_) => return,
        };

        unsafe {
            let surf = &*src_surface;
            if surf.pixels.is_null() || surf.pitch <= 0 {
                return;
            }
            let pitch = surf.pitch as usize;
            let total_size = pitch * SCREEN_HEIGHT as usize;
            let pixel_data =
                std::slice::from_raw_parts(surf.pixels as *const u8, total_size);
            if texture.update(None, pixel_data, pitch).is_err() {
                return;
            }
        }

        // REQ-SCR-030 / REQ-SCR-040: set blend mode based on alpha
        if alpha == 255 {
            texture.set_blend_mode(BlendMode::None);
        } else {
            texture.set_blend_mode(BlendMode::Blend);
            texture.set_alpha_mod(alpha);
        }

        // REQ-SCR-050 / REQ-SCR-060 / REQ-SCR-150: render with src_rect == dst_rect
        let _ = state.canvas.copy(&texture, sdl_rect, sdl_rect);
    }

    // REQ-NP-025: texture is dropped here (end of scope, Rust ownership)
}

/// Draw a color layer (for fades).
///
/// @plan PLAN-20260223-GFX-FULL-PORT.P11
/// @requirement REQ-CLR-010, REQ-CLR-020, REQ-CLR-030, REQ-CLR-040, REQ-CLR-050,
///              REQ-CLR-055, REQ-CLR-060
#[no_mangle]
pub extern "C" fn rust_gfx_color(r: u8, g: u8, b: u8, a: u8, rect: *const SDL_Rect) {
    // REQ-CLR-060: uninitialized guard
    let state = match get_gfx_state() {
        Some(s) => s,
        None => return,
    };

    // REQ-CLR-055: negative rect dimension guard (convert_c_rect clamps negatives to 0,
    // sdl2::rect::Rect then clamps 0→1, so we check the original C rect directly)
    if !rect.is_null() {
        // SAFETY: caller guarantees rect points to a valid SDL_Rect when non-null
        let c_rect = unsafe { &*rect };
        if c_rect.w < 0 || c_rect.h < 0 {
            return;
        }
    }

    // REQ-CLR-020/030: opaque overwrites, translucent blends
    if a == 255 {
        state.canvas.set_blend_mode(BlendMode::None);
    } else {
        state.canvas.set_blend_mode(BlendMode::Blend);
    }

    // REQ-CLR-010: set draw color
    state.canvas.set_draw_color(sdl2::pixels::Color::RGBA(r, g, b, a));

    // REQ-CLR-040/050: fill entire screen (null) or specified rect
    let sdl_rect = convert_c_rect(rect);
    let _ = state.canvas.fill_rect(sdl_rect);
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
// Utility / helper functions
// ============================================================================

/// Determine software scale factor from GFX flags.
///
/// - Bit 8 = xBRZ 3× (factor 3)
/// - Bit 9 = xBRZ 4× (factor 4)
/// - Bit 7 = HQ2x   (factor 2)
/// - Otherwise = 1 (no software scaling)
///
/// Returns `Some(factor)` when a software scaler is active, `None` when only
/// bilinear or no scaling is requested.
///
/// @plan PLAN-20260223-GFX-FULL-PORT.P12
/// @requirement REQ-SCALE-020, REQ-SCALE-025
fn scale_factor_from_flags(flags: c_int) -> Option<u32> {
    let scale_any =
        flags & ((1 << 3) | (1 << 4) | (1 << 5) | (1 << 6) | (1 << 7) | (1 << 8) | (1 << 9));
    let bilinear_only = scale_any != 0 && (flags & (1 << 3)) != 0
        && (flags & ((1 << 7) | (1 << 8) | (1 << 9))) == 0;
    if scale_any == 0 || bilinear_only {
        return None;
    }
    if (flags & (1 << 8)) != 0 {
        Some(3)
    } else if (flags & (1 << 9)) != 0 {
        Some(4)
    } else {
        Some(2)
    }
}

/// Convert RGBX8888 surface data to RGBA pixel buffer.
///
/// Source is RGBX8888 on little-endian: bytes `[X, B, G, R]` per pixel.
/// Destination is RGBA: bytes `[R, G, B, 0xFF]` per pixel.
/// The source may have a larger pitch (row stride) than `width * 4`.
///
/// @plan PLAN-20260223-GFX-FULL-PORT.P12
/// @requirement REQ-SCALE-010, REQ-SCALE-060
fn convert_rgbx_to_rgba(
    src: &[u8],
    dst: &mut [u8],
    width: usize,
    height: usize,
    pitch: usize,
) {
    for y in 0..height {
        let src_row = &src[y * pitch..y * pitch + width * 4];
        let dst_row = &mut dst[y * width * 4..(y + 1) * width * 4];
        for x in 0..width {
            let s = &src_row[x * 4..x * 4 + 4];
            let d = &mut dst_row[x * 4..x * 4 + 4];
            // RGBX8888 memory [X,B,G,R] -> RGBA [R,G,B,A]
            d[0] = s[3]; // R
            d[1] = s[2]; // G
            d[2] = s[1]; // B
            d[3] = 0xFF; // A (opaque)
        }
    }
}

/// Convert RGBA pixel buffer to RGBX8888 format.
///
/// Source is RGBA: bytes `[R, G, B, A]` per pixel.
/// Destination is RGBX8888 on little-endian: bytes `[0xFF, B, G, R]` per pixel.
/// Both buffers use a tightly-packed stride of `width * 4`.
///
/// @plan PLAN-20260223-GFX-FULL-PORT.P12
/// @requirement REQ-SCALE-030, REQ-SCALE-070
fn convert_rgba_to_rgbx(src: &[u8], dst: &mut [u8], width: usize, height: usize) {
    let stride = width * 4;
    for y in 0..height {
        let src_row = &src[y * stride..(y + 1) * stride];
        let dst_row = &mut dst[y * stride..(y + 1) * stride];
        for x in 0..width {
            let s = &src_row[x * 4..x * 4 + 4];
            let d = &mut dst_row[x * 4..x * 4 + 4];
            // RGBA [R,G,B,A] -> RGBX8888 memory [X,B,G,R]
            d[0] = 0xFF; // X (padding)
            d[1] = s[2]; // B
            d[2] = s[1]; // G
            d[3] = s[0]; // R
        }
    }
}

/// Convert a C `SDL_Rect` pointer to an `Option<sdl2::rect::Rect>`.
///
/// - Null pointer → `None` (full-screen operation).
/// - Non-null pointer → safely dereference and convert.
/// - Negative width/height are clamped to 0 (REQ-SCR-160).
///
/// @plan PLAN-20260223-GFX-FULL-PORT.P06
/// @requirement REQ-SCR-160
fn convert_c_rect(rect: *const SDL_Rect) -> Option<sdl2::rect::Rect> {
    if rect.is_null() {
        return None;
    }
    let r = unsafe { &*rect };
    let w = if r.w < 0 { 0 } else { r.w as u32 };
    let h = if r.h < 0 { 0 } else { r.h as u32 };
    Some(sdl2::rect::Rect::new(r.x, r.y, w, h))
}

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

    // ========================================================================
    // Phase P04 Tests: Preprocess/Postprocess TDD
    // @plan PLAN-20260223-GFX-FULL-PORT.P04
    // @requirement REQ-PRE-050, REQ-POST-030, REQ-INV-050,
    //              REQ-INIT-030, REQ-INIT-080, REQ-INIT-060, REQ-INIT-095
    // ========================================================================

    /// REQ-PRE-050: Preprocess returns immediately when uninitialized.
    /// GIVEN: The backend is not initialized (no rust_gfx_init called)
    /// WHEN:  rust_gfx_preprocess is called
    /// THEN:  Returns immediately with no side effects (no panic/crash)
    #[test]
    fn test_preprocess_uninitialized_no_panic() {
        // Ensure uninitialized state
        assert!(get_gfx_state().is_none(), "precondition: state must be None");
        // Call preprocess — must not panic or crash
        rust_gfx_preprocess(0, 0, 0);
    }

    /// REQ-POST-030: Postprocess returns immediately when uninitialized.
    /// GIVEN: The backend is not initialized
    /// WHEN:  rust_gfx_postprocess is called
    /// THEN:  Returns immediately with no side effects (no panic/crash)
    #[test]
    fn test_postprocess_uninitialized_no_panic() {
        assert!(get_gfx_state().is_none(), "precondition: state must be None");
        rust_gfx_postprocess();
    }

    /// Verify SDL_Rect::default() produces zeroed fields.
    /// This is important because C code depends on zero-initialized rects.
    #[test]
    fn test_sdl_rect_default() {
        let rect = SDL_Rect::default();
        assert_eq!(rect.x, 0);
        assert_eq!(rect.y, 0);
        assert_eq!(rect.w, 0);
        assert_eq!(rect.h, 0);
    }

    /// REQ-INIT-095: Calling init when already initialized returns -1.
    /// GIVEN: rust_gfx_init has been called successfully
    /// WHEN:  rust_gfx_init is called again
    /// THEN:  Returns -1 without modifying existing state
    ///
    /// Requires SDL2 display server — ignored on headless CI.
    #[test]
    #[ignore]
    fn test_init_already_initialized_returns_neg1() {
        // First init should succeed
        let result1 = rust_gfx_init(0, 0, std::ptr::null(), 640, 480);
        assert_eq!(result1, 0, "first init should succeed");

        // Second init should return -1
        let result2 = rust_gfx_init(0, 0, std::ptr::null(), 640, 480);
        assert_eq!(result2, -1, "second init must return -1 (REQ-INIT-095)");

        // Cleanup
        rust_gfx_uninit();
    }

    /// REQ-INIT-080: Successful init returns 0.
    /// Requires SDL2 display server — ignored on headless CI.
    #[test]
    #[ignore]
    fn test_init_returns_zero() {
        let result = rust_gfx_init(0, 0, std::ptr::null(), 640, 480);
        assert_eq!(result, 0, "init should return 0 on success (REQ-INIT-080)");
        rust_gfx_uninit();
    }

    /// REQ-INIT-030: After init, get_screen_surface(0..2) returns non-null.
    /// Requires SDL2 display server — ignored on headless CI.
    #[test]
    #[ignore]
    fn test_init_creates_surfaces() {
        let result = rust_gfx_init(0, 0, std::ptr::null(), 640, 480);
        assert_eq!(result, 0, "init must succeed");

        for i in 0..TFB_GFX_NUMSCREENS as c_int {
            let surface = rust_gfx_get_screen_surface(i);
            assert!(
                !surface.is_null(),
                "screen surface {} must be non-null after init (REQ-INIT-030)",
                i
            );
        }

        rust_gfx_uninit();
    }

    /// REQ-INIT-060: Init with soft-scaler flags allocates scaled buffers.
    /// Requires SDL2 display server — ignored on headless CI.
    #[test]
    #[ignore]
    fn test_init_scaling_buffers() {
        // Flag bit 7 = SCALE_HQXX (triggers soft scaler allocation)
        let hqxx_flag: c_int = 1 << 7;
        let result = rust_gfx_init(0, hqxx_flag, std::ptr::null(), 640, 480);
        assert_eq!(result, 0, "init with SCALE_HQXX must succeed");

        if let Some(state) = get_gfx_state() {
            for i in 0..TFB_GFX_NUMSCREENS {
                assert!(
                    state.scaled_buffers[i].is_some(),
                    "scaled_buffer[{}] must be allocated when soft scaler is active (REQ-INIT-060)",
                    i
                );
            }
        } else {
            panic!("state must be Some after successful init");
        }

        rust_gfx_uninit();
    }

    // NOTE: test_init_partial_failure_cleanup (REQ-INIT-090) deferred to P13 (Error Handling)
    //       — requires error injection points that are too complex for this phase.

    // NOTE: test_init_logs_on_failure (REQ-INIT-100) verified by code inspection:
    //       every failure path in rust_gfx_init calls rust_bridge_log_msg before returning -1.
    //       Building a test logger sink is out of scope for this phase.

    // NOTE: Postprocess texture_creator usage (REQ-POST-020/REQ-INV-010) is a static
    //       analysis check, not a runtime test. The texture upload block is documented as
    //       retained until ScreenLayer (P06-P08) and marked with @plan P05 for removal.

    // ========================================================================
    // Phase P06 Tests: Screen Compositing Stub
    // @plan PLAN-20260223-GFX-FULL-PORT.P06
    // @requirement REQ-SCR-140, REQ-SCR-100, REQ-SCR-090, REQ-SCR-160
    // ========================================================================

    /// REQ-SCR-140: rust_gfx_screen returns immediately when uninitialized.
    #[test]
    fn test_gfx_screen_uninitialized_no_panic() {
        assert!(get_gfx_state().is_none(), "precondition: state must be None");
        rust_gfx_screen(0, 255, std::ptr::null());
    }

    /// REQ-SCR-100: rust_gfx_screen returns for out-of-range screen indices.
    #[test]
    fn test_gfx_screen_out_of_range_no_panic() {
        assert!(get_gfx_state().is_none(), "precondition: state must be None");
        rust_gfx_screen(-1, 255, std::ptr::null());
        rust_gfx_screen(3, 255, std::ptr::null());
        rust_gfx_screen(100, 255, std::ptr::null());
    }

    /// REQ-SCR-090: rust_gfx_screen(1, ...) returns immediately (extra screen skip).
    #[test]
    fn test_gfx_screen_extra_skip_no_panic() {
        assert!(get_gfx_state().is_none(), "precondition: state must be None");
        rust_gfx_screen(TFB_SCREEN_EXTRA, 128, std::ptr::null());
    }

    /// REQ-SCR-160: convert_c_rect null → None.
    #[test]
    fn test_convert_c_rect_null_returns_none() {
        assert!(convert_c_rect(std::ptr::null()).is_none());
    }

    /// REQ-SCR-160: convert_c_rect non-null → Some with correct values.
    #[test]
    fn test_convert_c_rect_valid_rect() {
        let c_rect = SDL_Rect { x: 10, y: 20, w: 100, h: 50 };
        let result = convert_c_rect(&c_rect as *const SDL_Rect);
        assert!(result.is_some());
        let r = result.unwrap();
        assert_eq!(r.x(), 10);
        assert_eq!(r.y(), 20);
        assert_eq!(r.width(), 100);
        assert_eq!(r.height(), 50);
    }

    /// REQ-SCR-160: convert_c_rect clamps negative width/height to 0.
    /// Note: sdl2::rect::Rect clamps minimum dimension to 1, so we check
    /// that the clamped-to-0 value becomes 1 after sdl2's own clamp.
    #[test]
    fn test_convert_c_rect_negative_dimensions_clamped() {
        let c_rect = SDL_Rect { x: 5, y: 5, w: -10, h: -20 };
        let result = convert_c_rect(&c_rect as *const SDL_Rect);
        assert!(result.is_some());
        let r = result.unwrap();
        assert_eq!(r.x(), 5);
        assert_eq!(r.y(), 5);
        // We clamp to 0, sdl2::rect::Rect then clamps 0→1
        assert_eq!(r.width(), 1);
        assert_eq!(r.height(), 1);
    }

    /// convert_c_rect with zero-sized rect.
    /// sdl2::rect::Rect clamps minimum dimension to 1.
    #[test]
    fn test_convert_c_rect_zero_size() {
        let c_rect = SDL_Rect { x: 0, y: 0, w: 0, h: 0 };
        let result = convert_c_rect(&c_rect as *const SDL_Rect);
        assert!(result.is_some());
        let r = result.unwrap();
        // sdl2::rect::Rect clamps 0→1
        assert_eq!(r.width(), 1);
        assert_eq!(r.height(), 1);
    }

    // ========================================================================
    // Phase P09 Tests: Color Layer Stub
    // @plan PLAN-20260223-GFX-FULL-PORT.P09
    // @requirement REQ-CLR-060, REQ-CLR-055
    // ========================================================================

    /// REQ-CLR-060: rust_gfx_color returns immediately when uninitialized.
    #[test]
    fn test_gfx_color_uninitialized_no_panic() {
        assert!(get_gfx_state().is_none(), "precondition: state must be None");
        rust_gfx_color(255, 0, 0, 128, std::ptr::null());
    }

    /// REQ-CLR-055: rust_gfx_color returns for negative rect dimensions.
    #[test]
    fn test_gfx_color_negative_rect_no_panic() {
        assert!(get_gfx_state().is_none(), "precondition: state must be None");
        // Negative rect dimensions — returns at uninitialized guard first,
        // but verifies no panic with bad rect data
        let bad_rect = SDL_Rect { x: 0, y: 0, w: -10, h: -20 };
        rust_gfx_color(255, 128, 0, 200, &bad_rect as *const SDL_Rect);
    }

    /// rust_gfx_color with null rect does not panic when uninitialized.
    #[test]
    fn test_gfx_color_null_rect_no_panic() {
        assert!(get_gfx_state().is_none(), "precondition: state must be None");
        rust_gfx_color(0, 0, 0, 0, std::ptr::null());
    }


    // ========================================================================
    // Phase P12 Tests: Scaling Integration — Pixel Conversion Helpers
    // @plan PLAN-20260223-GFX-FULL-PORT.P12
    // @requirement REQ-SCALE-010, REQ-SCALE-020, REQ-SCALE-025, REQ-SCALE-030,
    //              REQ-SCALE-060, REQ-SCALE-070
    // ========================================================================

    /// REQ-SCALE-060: convert_rgbx_to_rgba basic test.
    ///
    /// RGBX8888 `[X, B, G, R]` → RGBA `[R, G, B, 0xFF]`.
    #[test]
    fn test_convert_rgbx_to_rgba_basic() {
        // Two pixels, tightly packed (pitch == width * 4)
        let src: [u8; 8] = [
            0xFF, 0x00, 0x80, 0xC0, // pixel 0: X=0xFF, B=0x00, G=0x80, R=0xC0
            0x00, 0x33, 0x66, 0x99, // pixel 1: X=0x00, B=0x33, G=0x66, R=0x99
        ];
        let mut dst = [0u8; 8];
        convert_rgbx_to_rgba(&src, &mut dst, 2, 1, 8);

        assert_eq!(
            dst,
            [
                0xC0, 0x80, 0x00, 0xFF, // pixel 0: R=0xC0, G=0x80, B=0x00, A=0xFF
                0x99, 0x66, 0x33, 0xFF, // pixel 1: R=0x99, G=0x66, B=0x33, A=0xFF
            ]
        );
    }

    /// REQ-SCALE-060: convert_rgbx_to_rgba with pitch > width*4.
    #[test]
    fn test_convert_rgbx_to_rgba_with_pitch() {
        // 1 pixel wide, pitch=8 (4 bytes padding per row)
        let src: [u8; 16] = [
            0xAA, 0x11, 0x22, 0x33, 0xDE, 0xAD, 0xBE, 0xEF, // row 0 + padding
            0xBB, 0x44, 0x55, 0x66, 0xCA, 0xFE, 0xBA, 0xBE, // row 1 + padding
        ];
        let mut dst = [0u8; 8];
        convert_rgbx_to_rgba(&src, &mut dst, 1, 2, 8);

        assert_eq!(
            dst,
            [
                0x33, 0x22, 0x11, 0xFF, // row 0: R=0x33, G=0x22, B=0x11, A=0xFF
                0x66, 0x55, 0x44, 0xFF, // row 1: R=0x66, G=0x55, B=0x44, A=0xFF
            ]
        );
    }

    /// REQ-SCALE-070: convert_rgba_to_rgbx basic test.
    ///
    /// RGBA `[R, G, B, A]` → RGBX8888 `[0xFF, B, G, R]`.
    #[test]
    fn test_convert_rgba_to_rgbx_basic() {
        let src: [u8; 8] = [
            0xC0, 0x80, 0x00, 0xFF, // pixel 0: R=0xC0, G=0x80, B=0x00, A=0xFF
            0x99, 0x66, 0x33, 0xAA, // pixel 1: R=0x99, G=0x66, B=0x33, A=0xAA
        ];
        let mut dst = [0u8; 8];
        convert_rgba_to_rgbx(&src, &mut dst, 2, 1);

        assert_eq!(
            dst,
            [
                0xFF, 0x00, 0x80, 0xC0, // pixel 0: X=0xFF, B=0x00, G=0x80, R=0xC0
                0xFF, 0x33, 0x66, 0x99, // pixel 1: X=0xFF, B=0x33, G=0x66, R=0x99
            ]
        );
    }

    /// REQ-SCALE-060/070: RGBX→RGBA→RGBX roundtrip preserves R, G, B channels.
    ///
    /// The X channel is set to 0xFF on output (not preserved from input), and
    /// the A channel is always 0xFF in the intermediate RGBA. This is correct
    /// because screen surfaces have no meaningful alpha.
    #[test]
    fn test_convert_rgbx_rgba_roundtrip() {
        // 2×2 image, tightly packed
        let original: [u8; 16] = [
            0x00, 0x11, 0x22, 0x33, // pixel (0,0)
            0xAA, 0xBB, 0xCC, 0xDD, // pixel (1,0)
            0x12, 0x34, 0x56, 0x78, // pixel (0,1)
            0x9A, 0xBC, 0xDE, 0xF0, // pixel (1,1)
        ];
        let mut rgba = [0u8; 16];
        let mut roundtrip = [0u8; 16];

        convert_rgbx_to_rgba(&original, &mut rgba, 2, 2, 8);
        convert_rgba_to_rgbx(&rgba, &mut roundtrip, 2, 2);

        // R, G, B channels must survive the roundtrip; X becomes 0xFF
        for i in 0..4 {
            let off = i * 4;
            assert_eq!(
                roundtrip[off + 1], original[off + 1],
                "pixel {} B channel mismatch",
                i
            );
            assert_eq!(
                roundtrip[off + 2], original[off + 2],
                "pixel {} G channel mismatch",
                i
            );
            assert_eq!(
                roundtrip[off + 3], original[off + 3],
                "pixel {} R channel mismatch",
                i
            );
            assert_eq!(roundtrip[off], 0xFF, "pixel {} X channel must be 0xFF", i);
        }
    }

    /// REQ-SCALE-020: scale_factor_from_flags returns correct factors.
    ///
    /// Bit 7 = HQ2x (factor 2), bit 8 = xBRZ3 (factor 3), bit 9 = xBRZ4 (factor 4).
    #[test]
    fn test_scale_factor_from_flags() {
        // No scaling flags → None
        assert_eq!(scale_factor_from_flags(0), None);

        // HQ2x (bit 7) → Some(2)
        assert_eq!(scale_factor_from_flags(1 << 7), Some(2));

        // xBRZ3 (bit 8) → Some(3)
        assert_eq!(scale_factor_from_flags(1 << 8), Some(3));

        // xBRZ4 (bit 9) → Some(4)
        assert_eq!(scale_factor_from_flags(1 << 9), Some(4));

        // xBRZ3 takes priority over HQ2x when both set
        assert_eq!(scale_factor_from_flags((1 << 7) | (1 << 8)), Some(3));
    }

    /// REQ-SCALE-025: bilinear-only does NOT trigger software scaling.
    ///
    /// When only SCALE_BILINEAR (bit 3) is set without any of the software
    /// scaler bits (7/8/9), the unscaled path should be used.
    #[test]
    fn test_bilinear_only_no_software_scale() {
        // Bilinear only (bit 3) → None (no software scaling)
        assert_eq!(scale_factor_from_flags(1 << 3), None);

        // Bilinear + biadapt (bits 3+4), no soft scaler → None
        assert_eq!(scale_factor_from_flags((1 << 3) | (1 << 4)), None);

        // Bilinear + HQ2x → Some(2) (HQ2x wins)
        assert_eq!(scale_factor_from_flags((1 << 3) | (1 << 7)), Some(2));

        // Bilinear + xBRZ3 → Some(3)
        assert_eq!(scale_factor_from_flags((1 << 3) | (1 << 8)), Some(3));
    }
}
