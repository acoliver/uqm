//! Video player with playback control, scaling, and direct SDL surface rendering.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use super::{
    decoder::DukVideoDecoder,
    scaler::{LanczosVideoScaler, VideoScaler},
    VideoFrame, DUCK_FPS,
};
use crate::bridge_log::rust_bridge_log_msg;
use crate::video::ffi::SDL_Surface;

/// Current state of video playback
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaybackState {
    Stopped,
    Playing,
    Paused,
    Finished,
}

impl Default for PlaybackState {
    fn default() -> Self {
        Self::Stopped
    }
}

#[derive(Debug)]
pub struct VideoPlayer {
    decoder: DukVideoDecoder,
    scaler: Option<VideoScaler>,
    lanczos_scaler: Option<LanczosVideoScaler>,

    state: PlaybackState,

    /// Where to blit onto the destination surface (upper-left corner).
    dst_x: i32,
    dst_y: i32,

    /// When playback started (or resumed).
    start_time: Option<Instant>,
    /// Time when playback was paused.
    pause_time: Option<Instant>,
    /// Accumulated pause duration to subtract from elapsed time.
    pause_duration: f32,

    current_frame: u32,
    last_rendered_frame: Option<u32>,

    loop_playback: bool,

    /// Next time (monotonic) when a new frame should be rendered.
    next_frame_due: Option<Instant>,

    /// Last known playback position in milliseconds.
    pos_ms: u32,

    /// Whether to use direct window presentation (bypasses 320x240 surface)
    direct_window_mode: bool,
}

static FRAME_LOG_COUNTER: AtomicUsize = AtomicUsize::new(0);

impl VideoPlayer {
    pub fn new(decoder: DukVideoDecoder) -> Self {
        Self {
            decoder,
            scaler: None,
            lanczos_scaler: None,
            state: PlaybackState::Stopped,
            dst_x: 0,
            dst_y: 0,
            start_time: None,
            pause_time: None,
            pause_duration: 0.0,
            current_frame: 0,
            last_rendered_frame: None,
            loop_playback: false,
            next_frame_due: None,
            pos_ms: 0,
            direct_window_mode: false,
        }
    }

    pub fn set_loop(&mut self, loop_enabled: bool) {
        self.loop_playback = loop_enabled;
    }

    pub fn set_position(&mut self, x: i32, y: i32) {
        self.dst_x = x;
        self.dst_y = y;
    }

    pub fn set_scaler(&mut self, scaler: Option<VideoScaler>) {
        self.scaler = scaler;
    }

    pub fn set_lanczos_scaler(&mut self, scaler: Option<LanczosVideoScaler>) {
        self.lanczos_scaler = scaler;
    }

    pub fn set_direct_window_mode(&mut self, enabled: bool) {
        self.direct_window_mode = enabled;
    }

    pub fn direct_window_mode(&self) -> bool {
        self.direct_window_mode
    }

    pub fn play(&mut self) {
        match self.state {
            PlaybackState::Stopped | PlaybackState::Finished => {
                self.current_frame = 0;
                self.last_rendered_frame = None;
                self.pause_duration = 0.0;
                self.start_time = Some(Instant::now());
                self.pause_time = None;
                self.state = PlaybackState::Playing;
                self.next_frame_due = Some(Instant::now());
                self.pos_ms = 0;
            }
            PlaybackState::Paused => {
                if let Some(pause_start) = self.pause_time {
                    self.pause_duration += pause_start.elapsed().as_secs_f32();
                }
                self.pause_time = None;
                self.state = PlaybackState::Playing;
            }
            PlaybackState::Playing => {}
        }
    }

    pub fn stop(&mut self) {
        self.state = PlaybackState::Stopped;
        self.start_time = None;
        self.pause_time = None;
        self.pause_duration = 0.0;
        self.current_frame = 0;
        self.last_rendered_frame = None;
        self.next_frame_due = None;
        self.pos_ms = 0;
    }

    pub fn pause(&mut self) {
        if self.state == PlaybackState::Playing {
            self.pause_time = Some(Instant::now());
            self.state = PlaybackState::Paused;
        }
    }

    #[inline]
    pub fn state(&self) -> PlaybackState {
        self.state
    }

    #[inline]
    pub fn playing(&self) -> bool {
        self.state == PlaybackState::Playing
    }

    #[inline]
    pub fn position_ms(&self) -> u32 {
        self.pos_ms
    }

    /// Advances playback and (if due) renders the next frame into `dst_surface`.
    ///
    /// Returns `true` while still playing, `false` when finished/stopped.
    pub unsafe fn process_frame(&mut self, dst_surface: *mut SDL_Surface) -> bool {
        if self.state != PlaybackState::Playing {
            return self.state == PlaybackState::Paused;
        }

        let now = Instant::now();
        if let Some(due) = self.next_frame_due {
            if now < due {
                return true;
            }
        }

        // Compute which frame we should be based on wall time.
        let elapsed = self.current_time_seconds();
        let mut target_frame = (elapsed * DUCK_FPS) as u32;
        let frame_count = self.decoder.frame_count();

        if target_frame >= frame_count {
            if self.loop_playback {
                self.current_frame = 0;
                self.last_rendered_frame = None;
                self.pause_duration = 0.0;
                self.start_time = Some(Instant::now());
                target_frame = 0;
            } else {
                self.state = PlaybackState::Finished;
                return false;
            }
        }

        self.current_frame = target_frame;
        self.pos_ms = (elapsed * 1000.0) as u32;

        if self.last_rendered_frame == Some(self.current_frame) {
            // Even if called late/early, schedule the next expected frame time.
            self.schedule_next_frame(now);
            return true;
        }

        let mut frame = match self.decoder.decode_frame(self.current_frame) {
            Ok(f) => f,
            Err(_) => {
                self.state = PlaybackState::Finished;
                return false;
            }
        };

        // Use Lanczos window scaler if in direct window mode
        if self.direct_window_mode {
            if let Some(ref mut lanczos_scaler) = self.lanczos_scaler {
                if let Some(scaled) = lanczos_scaler.scale(&frame.data) {
                    let (dst_w, dst_h) = lanczos_scaler.dst_dimensions();
                    frame = VideoFrame {
                        width: dst_w,
                        height: dst_h,
                        data: scaled,
                        timestamp: frame.timestamp,
                    };

                    // Present directly to window instead of SDL surface
                    if super::ffi::rust_present_video_to_window(
                        frame.data.as_ptr() as *const u8,
                        frame.width as i32,
                        frame.height as i32,
                        (frame.width * 4) as i32, // RGBA stride
                    ) {
                        rust_bridge_log_msg(&format!(
                            "RUST_VIDEO: Direct window presentation frame {} ({}x{}), bypassing xBRZ/hq2x",
                            self.current_frame,
                            frame.width,
                            frame.height
                        ));
                    } else {
                        rust_bridge_log_msg(
                            "RUST_VIDEO: Direct window presentation failed; stopping playback",
                        );
                        self.state = PlaybackState::Finished;
                        return false;
                    }
                } else {
                    // If Lanczos scaling fails, fall back to original frame
                    rust_bridge_log_msg("RUST_VIDEO: Lanczos scaling failed, using original frame");

                    if !super::ffi::rust_present_video_to_window(
                        frame.data.as_ptr() as *const u8,
                        frame.width as i32,
                        frame.height as i32,
                        (frame.width * 4) as i32,
                    ) {
                        rust_bridge_log_msg(
                            "RUST_VIDEO: Direct window presentation failed; stopping playback",
                        );
                        self.state = PlaybackState::Finished;
                        return false;
                    }
                }
            } else {
                // If no Lanczos scaler configured, still try direct presentation
                // This bypasses the 320x240 pipeline completely
                if super::ffi::rust_present_video_to_window(
                    frame.data.as_ptr() as *const u8,
                    frame.width as i32,
                    frame.height as i32,
                    (frame.width * 4) as i32,
                ) {
                    rust_bridge_log_msg(&format!(
                        "RUST_VIDEO: Direct window presentation unscaled frame {} ({}x{}), bypassing xBRZ/hq2x",
                        self.current_frame,
                        frame.width,
                        frame.height
                    ));
                } else {
                    rust_bridge_log_msg(
                        "RUST_VIDEO: Direct window presentation failed; stopping playback",
                    );
                    self.state = PlaybackState::Finished;
                    return false;
                }
            }
        } else {
            // Legacy path: use traditional SDL surface rendering
            // Optional Lanczos3 scaling.
            if let Some(ref mut scaler) = self.scaler {
                if let Some(scaled) = scaler.scale(&frame.data) {
                    let (dst_w, dst_h) = scaler.dst_dimensions();
                    frame = VideoFrame {
                        width: dst_w,
                        height: dst_h,
                        data: scaled,
                        timestamp: frame.timestamp,
                    };
                }
            }

            blit_frame_to_sdl_surface(dst_surface, &frame, self.dst_x, self.dst_y);
        }

        let log_index = FRAME_LOG_COUNTER.fetch_add(1, Ordering::Relaxed);
        if log_index < 5 {
            let sample = frame.data.get(0).copied().unwrap_or(0);
            rust_bridge_log_msg(&format!(
                "RUST_VIDEO: frame {} size={}x{} dst=({}, {}) sample=0x{:08x}",
                self.current_frame, frame.width, frame.height, self.dst_x, self.dst_y, sample
            ));
        }

        if !self.direct_window_mode && log_index < 5 {
            let post_sample = frame.data.get(0).copied().unwrap_or(0);
            rust_bridge_log_msg(&format!(
                "RUST_VIDEO: blit done frame {} sample=0x{:08x}",
                self.current_frame, post_sample
            ));
        }

        self.last_rendered_frame = Some(self.current_frame);
        self.schedule_next_frame(now);
        true
    }

    fn schedule_next_frame(&mut self, now: Instant) {
        let frame_time = Duration::from_secs_f64(1.0 / DUCK_FPS as f64);
        self.next_frame_due = Some(now + frame_time);
    }

    fn current_time_seconds(&self) -> f32 {
        match self.state {
            PlaybackState::Stopped => 0.0,
            PlaybackState::Playing => {
                if let Some(start) = self.start_time {
                    start.elapsed().as_secs_f32() - self.pause_duration
                } else {
                    0.0
                }
            }
            PlaybackState::Paused => {
                if let (Some(start), Some(pause)) = (self.start_time, self.pause_time) {
                    let elapsed_before_pause = pause.duration_since(start).as_secs_f32();
                    elapsed_before_pause - self.pause_duration
                } else {
                    self.current_frame as f32 / DUCK_FPS
                }
            }
            PlaybackState::Finished => self.decoder.duration(),
        }
    }
}

/// Minimal SDL_Surface declaration for blitting.
///
/// This matches SDL1/SDL2 surface layout sufficiently for width/height/pitch/pixels.
unsafe fn blit_frame_to_sdl_surface(
    surface: *mut SDL_Surface,
    frame: &VideoFrame,
    x0: i32,
    y0: i32,
) {
    if surface.is_null() {
        return;
    }

    // Use the SDL_LockSurface/UnlockSurface helpers from C.
    super::ffi::tfb_drawcanvas_lock(surface as *mut std::ffi::c_void);

    let surf = &*surface;
    let dst_w = surf.w as i32;
    let dst_h = surf.h as i32;
    let pitch = surf.pitch as i32;

    if surf.pixels.is_null() {
        super::ffi::tfb_drawcanvas_unlock(surface as *mut std::ffi::c_void);
        return;
    }

    // We only support 32bpp surfaces for now (which is what UQM typically uses).
    // If the screen is 16bpp, we'd need to map via pixel format masks.
    if dst_w <= 0 || dst_h <= 0 || pitch < dst_w * 4 {
        super::ffi::tfb_drawcanvas_unlock(surface as *mut std::ffi::c_void);
        return;
    }

    let log_index = FRAME_LOG_COUNTER.load(Ordering::Relaxed);
    if log_index < 5 {
        rust_bridge_log_msg(&format!(
            "RUST_VIDEO: surface w={} h={} pitch={} pixels={:p} dst=({}, {})",
            dst_w, dst_h, pitch, surf.pixels, x0, y0
        ));
    }

    let src_w = frame.width as i32;
    let src_h = frame.height as i32;

    let mut y = 0;
    while y < src_h {
        let dst_y = y0 + y;
        if dst_y < 0 {
            y += 1;
            continue;
        }
        if dst_y >= dst_h {
            break;
        }

        let row_ptr = (surf.pixels as *mut u8).add((dst_y * pitch) as usize) as *mut u32;

        let mut x = 0;
        while x < src_w {
            let dst_x = x0 + x;
            if dst_x < 0 {
                x += 1;
                continue;
            }
            if dst_x >= dst_w {
                break;
            }

            // Frame pixels are RGBA with R in bits 0..7 etc (see decoder/scaler).
            let p = frame.data[(y as usize * frame.width as usize) + x as usize];
            // Frame pixels are RGBA in memory. The Rust graphics screen surface
            // uses RGBX8888 with masks R=0xFF000000, G=0x00FF0000, B=0x0000FF00.
            let r = (p & 0xFF) as u32;
            let g = ((p >> 8) & 0xFF) as u32;
            let b = ((p >> 16) & 0xFF) as u32;

            let out = (r << 24) | (g << 16) | (b << 8);

            if (dst_x as usize) < (pitch as usize / 4) {
                *row_ptr.add(dst_x as usize) = out;
            }

            x += 1;
        }

        y += 1;
    }

    super::ffi::tfb_drawcanvas_unlock(surface as *mut std::ffi::c_void);
}
