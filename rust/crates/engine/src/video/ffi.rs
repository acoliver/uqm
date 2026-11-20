//! The pure Rust video player and the clip entry points the game calls.

use std::ffi::{c_int, c_void, CString};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Mutex;

use crate::bridge_log::rust_bridge_log_msg;
use crate::io::Dir;
use crate::sound::decoder::Decoder;
use crate::sound::dukaud::DukAudDecoder;
use crate::sound::rodio_audio::{self, SoundCategory};

use super::player::VideoPlayer;
use super::scaler::VideoScaler;

/// Rodio handle for the currently-playing video audio track.
/// 0 means no audio is playing.
static VIDEO_AUDIO_HANDLE: AtomicU32 = AtomicU32::new(0);

/// Rodio handle for video speech track. 0 = none.
static VIDEO_SPEECH_HANDLE: AtomicU32 = AtomicU32::new(0);

// ============================================================================

fn read_video_file(dir: &Dir, filename: &str) -> Option<Vec<u8>> {
    super::files::read(dir, filename)
}

fn calculate_video_scale(_src_w: u32, _src_h: u32) -> u32 {
    1
}

// ============================================================================
// Clip source
// ============================================================================

/// What a clip knows about its video file: where to read it and its geometry.
pub struct ClipSource {
    pub w: u32,
    pub h: u32,
    pub length: f32,
    pub audio_synced: bool,
    pub dir: Dir,
    pub filename: CString,
}

// ============================================================================
// Pure Rust player: simple C API
// ============================================================================

static PLAYER: Mutex<Option<VideoPlayer>> = Mutex::new(None);

pub fn rust_play_video(dir: &Dir, filename: &str, x: i32, y: i32, looping: bool) -> bool {
    let decoder = match super::files::open(dir, filename) {
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
        filename, src_w, src_h, scale, x, y, looping
    ));

    player.play();

    if let Ok(mut guard) = PLAYER.lock() {
        *guard = Some(player);
    }

    true
}

pub fn rust_stop_video() {
    if let Ok(mut guard) = PLAYER.lock() {
        *guard = None;
    }
}

pub fn rust_video_playing() -> bool {
    if let Ok(guard) = PLAYER.lock() {
        if let Some(ref p) = *guard {
            return p.playing();
        }
    }
    false
}

///
/// # Safety
///
/// No safety requirements; marked unsafe for C ABI compatibility.
pub unsafe extern "C" fn rust_process_video_frame() -> bool {
    use crate::graphics::{dcq::Screen, present, state};
    if !state::initialized() {
        rust_bridge_log_msg("RUST_VIDEO: the main screen is not initialized");
        return false;
    }

    let mut still_playing = false;

    if let Ok(mut guard) = PLAYER.lock() {
        if let Some(ref mut p) = *guard {
            still_playing = p.process_frame(&mut |frame, x, y| {
                state::draw_on_screen(Screen::Main, |canvas| {
                    super::player::blit_frame(canvas, frame, x, y)
                })
            });
            if still_playing {
                if !p.direct_window_mode() {
                    present::swap_buffers(present::Redraw::Fading);
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

/// Present a video frame directly to the window, bypassing the 320x240 main screen.
/// The frame data should be RGBA format where frame.data[0] is the R component.
/// Returns true on success, false on failure.
///
/// # Safety
///
/// No safety requirements; marked unsafe for C ABI compatibility.
pub unsafe extern "C" fn rust_present_video_to_window(
    frame_data: *const u8,
    width: c_int,
    height: c_int,
    stride: c_int, // row stride in bytes, typically width * 4 for RGBA
) -> bool {
    if frame_data.is_null() || width <= 0 || height <= 0 || stride < width * 4 {
        rust_bridge_log_msg(&format!(
            "RUST_VIDEO: rust_present_video_to_window invalid params data={:p} w={} h={} stride={}",
            frame_data, width, height, stride
        ));
        return false;
    }
    let bytes = std::slice::from_raw_parts(frame_data, (height * stride) as usize);
    let pixels: Vec<u32> = bytes
        .chunks(stride as usize)
        .flat_map(|row| {
            row[..width as usize * 4]
                .as_chunks::<4>()
                .0
                .iter()
                .map(|px| u32::from_le_bytes(*px))
        })
        .collect();
    crate::graphics::present::present_video(&pixels, width as u32, height as u32)
}

pub fn rust_get_video_position() -> u32 {
    if let Ok(guard) = PLAYER.lock() {
        if let Some(ref p) = *guard {
            return p.position_ms();
        }
    }
    0
}

// ============================================================================
// TFB_VideoClip layout (matches C struct tfb_videoclip from video.h)
// ============================================================================

/// C `RECT` type (from gfx types)
#[repr(C)]
#[derive(Clone, Copy)]
pub struct CRect {
    pub corner_x: i16,
    pub corner_y: i16,
    pub extent_w: i16,
    pub extent_h: i16,
}

/// Matches `struct tfb_videoclip` from video.h.
/// `VIDEO_REF` is a pointer to this struct.
#[repr(C)]
pub struct TFB_VideoClip {
    pub decoder: *mut ClipSource,
    pub length: f32,
    pub w: u32,
    pub h: u32,
    pub dst_rect: CRect,
    pub src_rect: CRect,
    pub h_audio: *mut c_void, // MUSIC_REF
    pub frame_time: u32,
    pub frame: *mut c_void, // TFB_Image*
    pub cur_frame: u32,
    pub playing: bool,
    pub own_audio: bool,
    pub loop_frame: u32,
    pub loop_to: u32,
    pub want_frame: u32,
    pub lag_cnt: i32,
    pub data: *mut c_void,
}

/// `VID_NO_LOOP` from vidlib.h: `(0U - 1)` = `0xFFFFFFFF`
const VID_NO_LOOP: u32 = 0xFFFF_FFFF;

// ============================================================================
// TFB_* video player API — replaces rust_video.c
// ============================================================================

///
/// # Safety
///
/// No safety requirements; marked unsafe for C ABI compatibility.
pub unsafe extern "C" fn tfb_init_video_player() -> bool {
    true
}

///
/// # Safety
///
/// No safety requirements; marked unsafe for C ABI compatibility.
pub unsafe extern "C" fn tfb_uninit_video_player() {
    rust_stop_video();
}

///
/// # Safety
///
/// Caller must ensure pointer arguments are valid and properly aligned.
pub unsafe extern "C" fn tfb_play_video(vid: *mut TFB_VideoClip, x: u32, y: u32) -> bool {
    if vid.is_null() {
        return false;
    }
    let clip = &mut *vid;

    if clip.decoder.is_null() {
        return false;
    }
    let dec = &*clip.decoder;

    let Ok(filename_str) = dec.filename.to_str() else {
        return false;
    };

    rust_bridge_log_msg(&format!("RUST_VIDEO: TFB_PlayVideo {}", filename_str));

    // Like vidplayer.c, retain movie pixels on TFB_SCREEN_MAIN. Input, pacing
    // and expose swaps must composite the movie, not the cleared game surface.
    if !rust_play_video(
        &dec.dir,
        filename_str,
        x as i32,
        y as i32,
        clip.loop_frame != VID_NO_LOOP,
    ) {
        return false;
    }

    // Ensure the rodio_audio simple playback system is initialized
    // (it's separate from the rodio_backend OpenAL-compat layer)
    rodio_audio::rust_audio_init();

    // Decode audio from the .duk file directly via Rust DukAudDecoder + rodio
    let basename = filename_str.trim_end_matches(".duk");
    let duk_file = format!("{}.duk", basename);
    let frm_file = format!("{}.frm", basename);

    let duk_data = read_video_file(&dec.dir, &duk_file);
    let frm_data = read_video_file(&dec.dir, &frm_file);

    if let (Some(duk_bytes), Some(frm_bytes)) = (duk_data, frm_data) {
        let mut audio_dec = DukAudDecoder::new();
        if audio_dec.open_from_data(&duk_bytes, &frm_bytes).is_ok() {
            // Decode all audio frames into one PCM buffer
            let mut pcm = Vec::new();
            let mut chunk = vec![0u8; 8192];
            loop {
                match audio_dec.decode(&mut chunk) {
                    Ok(n) if n > 0 => pcm.extend_from_slice(&chunk[..n]),
                    _ => break,
                }
            }

            if !pcm.is_empty() {
                let looping = clip.loop_frame != VID_NO_LOOP;
                let handle = rodio_audio::play_raw_pcm(
                    pcm.clone(),
                    22050, // DukAud sample rate
                    2,     // stereo
                    16,    // 16-bit
                    SoundCategory::Music,
                    looping,
                );
                VIDEO_AUDIO_HANDLE.store(handle, Ordering::Relaxed);
                rust_bridge_log_msg(&format!(
                    "RUST_VIDEO: playing embedded audio via rodio, {} bytes PCM, handle={}",
                    pcm.len(),
                    handle
                ));
            } else {
                rust_bridge_log_msg("RUST_VIDEO: no audio decoded from .duk");
            }
        } else {
            rust_bridge_log_msg("RUST_VIDEO: DukAudDecoder failed to open");
        }
    } else {
        rust_bridge_log_msg(&format!(
            "RUST_VIDEO: could not read .duk/.frm for audio from {}",
            filename_str
        ));
    }

    true
}

///
/// # Safety
///
/// Caller must ensure pointer arguments are valid and properly aligned.
pub unsafe extern "C" fn tfb_stop_video(_vid: *mut TFB_VideoClip) {
    // Stop video audio played via rodio
    let audio_handle = VIDEO_AUDIO_HANDLE.swap(0, Ordering::Relaxed);
    if audio_handle != 0 {
        rodio_audio::stop_sound(audio_handle);
        rust_bridge_log_msg(&format!(
            "RUST_VIDEO: stopped audio handle={}",
            audio_handle
        ));
    }
    let speech_handle = VIDEO_SPEECH_HANDLE.swap(0, Ordering::Relaxed);
    if speech_handle != 0 {
        rodio_audio::stop_sound(speech_handle);
    }
    rust_stop_video();
}

///
/// # Safety
///
/// Caller must ensure pointer arguments are valid and properly aligned.
pub unsafe extern "C" fn tfb_video_playing(_vid: *mut TFB_VideoClip) -> bool {
    rust_video_playing()
}

///
/// # Safety
///
/// Caller must ensure pointer arguments are valid and properly aligned.
pub unsafe extern "C" fn tfb_process_video_frame(_vid: *mut TFB_VideoClip) -> bool {
    let ok = rust_process_video_frame();
    rust_bridge_log_msg(&format!(
        "RUST_VIDEO: rust_process_video_frame -> {}",
        ok as i32
    ));
    ok
}

///
/// # Safety
///
/// Caller must ensure pointer arguments are valid and properly aligned.
pub unsafe extern "C" fn tfb_get_video_position(_vid: *mut TFB_VideoClip) -> u32 {
    rust_get_video_position()
}

///
/// # Safety
///
/// Caller must ensure pointer arguments are valid and properly aligned.
pub unsafe extern "C" fn tfb_seek_video(_vid: *mut TFB_VideoClip, _pos: u32) -> bool {
    false
}
