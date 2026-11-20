//! Video player with playback control, scaling, and direct SDL surface rendering.

use super::{
    decoder::DukVideoDecoder,
    scaler::{LanczosVideoScaler, VideoScaler},
    VideoFrame, DUCK_FPS,
};
use crate::bridge_log::rust_bridge_log_msg;
use crate::graphics::canvas::Canvas;
use std::sync::atomic::{AtomicUsize, Ordering};
use xbrz::scale_rgba;

/// Seconds of host real time; movies play at wall-clock speed, replays too.
type Moment = f64;

fn now() -> Moment {
    crate::host::elapsed().as_secs_f64()
}

/// Current state of video playback
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PlaybackState {
    #[default]
    Stopped,
    Playing,
    Paused,
    Finished,
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
    start_time: Option<Moment>,
    /// Time when playback was paused.
    pause_time: Option<Moment>,
    /// Accumulated pause duration to subtract from elapsed time.
    pause_duration: f32,

    current_frame: u32,
    last_rendered_frame: Option<u32>,

    loop_playback: bool,

    /// Next time (monotonic) when a new frame should be rendered.
    next_frame_due: Option<Moment>,

    /// Last known playback position in milliseconds.
    pos_ms: u32,

    /// Whether to use direct window presentation (bypasses 320x240 surface)
    direct_window_mode: bool,

    /// xBRZ scale factor for direct window mode (2-6, or 0 to disable)
    xbrz_factor: usize,
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
            xbrz_factor: 0,
        }
    }

    pub fn set_loop(&mut self, loop_enabled: bool) {
        self.loop_playback = loop_enabled;
    }

    pub fn set_position(&mut self, x: i32, y: i32) {
        self.dst_x = x;
        self.dst_y = y;
    }

    pub fn set_xbrz_factor(&mut self, factor: usize) {
        self.xbrz_factor = factor;
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
                self.start_time = Some(now());
                self.pause_time = None;
                self.state = PlaybackState::Playing;
                self.next_frame_due = Some(now());
                self.pos_ms = 0;
            }
            PlaybackState::Paused => {
                if let Some(pause_start) = self.pause_time {
                    self.pause_duration += (now() - pause_start) as f32;
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
            self.pause_time = Some(now());
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

    /// Advances playback and (if due) renders the next frame through `blit`.
    /// Returns `true` while still playing, `false` when finished/stopped.
    pub fn process_frame(&mut self, blit: &mut dyn FnMut(&VideoFrame, i32, i32)) -> bool {
        if self.state != PlaybackState::Playing {
            return self.state == PlaybackState::Paused;
        }

        let frame_time = now();
        if let Some(due) = self.next_frame_due {
            if frame_time < due {
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
                self.start_time = Some(now());
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
            self.schedule_next_frame(frame_time);
            return true;
        }

        let mut frame = match self.decoder.decode_frame(self.current_frame) {
            Ok(f) => f,
            Err(_) => {
                self.state = PlaybackState::Finished;
                return false;
            }
        };

        // Direct window mode: xBRZ scale then present directly
        if self.direct_window_mode {
            // Apply xBRZ scaling if configured
            if self.xbrz_factor >= 2 {
                let src_w = frame.width as usize;
                let src_h = frame.height as usize;
                let src_bytes: Vec<u8> = frame.data.iter().flat_map(|p| p.to_ne_bytes()).collect();
                let scaled_bytes = scale_rgba(&src_bytes, src_w, src_h, self.xbrz_factor);
                let scaled_data: Vec<u32> = scaled_bytes
                    .as_chunks::<4>()
                    .0
                    .iter()
                    .map(|c| u32::from_ne_bytes([c[0], c[1], c[2], c[3]]))
                    .collect();
                frame = VideoFrame {
                    width: frame.width * self.xbrz_factor as u32,
                    height: frame.height * self.xbrz_factor as u32,
                    data: scaled_data,
                    timestamp: frame.timestamp,
                };
            }

            if !crate::graphics::present::present_video(&frame.data, frame.width, frame.height) {
                rust_bridge_log_msg(
                    "RUST_VIDEO: Direct window presentation failed; stopping playback",
                );
                self.state = PlaybackState::Finished;
                return false;
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

            blit(&frame, self.dst_x, self.dst_y);
        }

        let log_index = FRAME_LOG_COUNTER.fetch_add(1, Ordering::Relaxed);
        if log_index < 5 {
            let sample = frame.data.first().copied().unwrap_or(0);
            rust_bridge_log_msg(&format!(
                "RUST_VIDEO: frame {} size={}x{} dst=({}, {}) sample=0x{:08x}",
                self.current_frame, frame.width, frame.height, self.dst_x, self.dst_y, sample
            ));
        }

        if !self.direct_window_mode && log_index < 5 {
            let post_sample = frame.data.first().copied().unwrap_or(0);
            rust_bridge_log_msg(&format!(
                "RUST_VIDEO: blit done frame {} sample=0x{:08x}",
                self.current_frame, post_sample
            ));
        }

        self.last_rendered_frame = Some(self.current_frame);
        self.schedule_next_frame(frame_time);
        true
    }

    fn schedule_next_frame(&mut self, now: Moment) {
        self.next_frame_due = Some(now + 1.0 / f64::from(DUCK_FPS));
    }

    fn current_time_seconds(&self) -> f32 {
        match self.state {
            PlaybackState::Stopped => 0.0,
            PlaybackState::Playing => {
                if let Some(start) = self.start_time {
                    (now() - start) as f32 - self.pause_duration
                } else {
                    0.0
                }
            }
            PlaybackState::Paused => {
                if let (Some(start), Some(pause)) = (self.start_time, self.pause_time) {
                    let elapsed_before_pause = (pause - start) as f32;
                    elapsed_before_pause - self.pause_duration
                } else {
                    self.current_frame as f32 / DUCK_FPS
                }
            }
            PlaybackState::Finished => self.decoder.duration(),
        }
    }
}

/// Write a movie frame (RGBA words, red in the low byte) into a screen
/// canvas at `(x0, y0)`, clipped to the canvas.
pub(crate) fn blit_frame(canvas: &mut Canvas, frame: &VideoFrame, x0: i32, y0: i32) {
    let log_index = FRAME_LOG_COUNTER.load(Ordering::Relaxed);
    if log_index < 5 {
        rust_bridge_log_msg(&format!(
            "RUST_VIDEO: surface w={} h={} dst=({}, {})",
            canvas.width(),
            canvas.height(),
            x0,
            y0
        ));
    }
    for y in 0..frame.height as i32 {
        let dst_y = y0 + y;
        if dst_y < 0 || dst_y >= canvas.height() {
            continue;
        }
        for x in 0..frame.width as i32 {
            let dst_x = x0 + x;
            if dst_x < 0 || dst_x >= canvas.width() {
                continue;
            }
            let p = frame.data[y as usize * frame.width as usize + x as usize];
            let (r, g, b) = (p & 0xFF, (p >> 8) & 0xFF, (p >> 16) & 0xFF);
            canvas.set_pixel(dst_x, dst_y, (r << 24) | (g << 16) | (b << 8));
        }
    }
}

#[cfg(test)]
#[path = "playback_tests.rs"]
mod playback_tests;
