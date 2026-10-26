//! Public video lifecycle and playback control, ported from video.c.
#![allow(non_snake_case)]

use super::ffi::*;
use super::registry::*;

use crate::memory::{rust_hcalloc, rust_hfree};
use std::ffi::{c_char, c_int, c_void};
use std::ptr;

static mut CURRENT_VIDEO: *mut TFB_VideoClip = ptr::null_mut();
static mut CURRENT_SPEECH: *mut c_void = ptr::null_mut();

use crate::sound::heart_ffi::snd_stop_speech as snd_StopSpeech;

use crate::threading::legacy_abi::CreateMutex_Core;

use crate::threading::legacy_abi::DestroyMutex;

#[cfg(test)]
use crate::mainloop::dir_prep::c_dir_globals::contentDir;
#[cfg(not(test))]
use crate::mainloop::options_abi::contentDir;

extern "C" {

    static mut ScreenWidth: i32;
    static mut ScreenHeight: i32;
}
use crate::sound::heart_ffi::DestroyMusic;

/// # Safety
/// Must run on the graphics thread with graphics initialized.
pub unsafe extern "C" fn InitVideoPlayer(_use_cdrom: c_int) -> c_int {
    let mut format = std::mem::zeroed::<crate::graphics::native_abi::TFB_PixelFormat>();
    crate::graphics::native_abi::TFB_DrawCanvas_GetScreenFormat(&mut format);
    if !VideoDecoder_Init(
        0,
        format.BitsPerPixel as c_int,
        format.Rmask,
        format.Gmask,
        format.Bmask,
        0,
    ) {
        return 0;
    }
    tfb_init_video_player() as c_int
}

/// # Safety
/// Must run on the graphics thread, serialized with video operations.
pub unsafe extern "C" fn UninitVideoPlayer() {
    tfb_uninit_video_player();
    VideoDecoder_Uninit();
}

/// # Safety
/// Must run on the graphics thread, serialized with video operations.
pub unsafe fn VidStop() {
    if !CURRENT_SPEECH.is_null() {
        snd_StopSpeech();
    }
    if !CURRENT_VIDEO.is_null() {
        tfb_stop_video(CURRENT_VIDEO);
    }
    CURRENT_SPEECH = ptr::null_mut();
    CURRENT_VIDEO = ptr::null_mut();
}

/// # Safety
/// Must run on the graphics thread, serialized with video operations.
pub unsafe extern "C" fn VidPlaying() -> *mut TFB_VideoClip {
    if CURRENT_VIDEO.is_null() || !tfb_video_playing(CURRENT_VIDEO) {
        ptr::null_mut()
    } else {
        CURRENT_VIDEO
    }
}

/// # Safety
/// Must run on the graphics thread, serialized with video operations.
pub unsafe extern "C" fn VidProcessFrame() -> c_int {
    if CURRENT_VIDEO.is_null() {
        0
    } else {
        tfb_process_video_frame(CURRENT_VIDEO) as c_int
    }
}

/// # Safety
/// Must run on the graphics thread, serialized with video operations.
pub unsafe extern "C" fn VidGetPosition() -> u32 {
    if VidPlaying().is_null() {
        0
    } else {
        tfb_get_video_position(CURRENT_VIDEO)
    }
}

/// # Safety
/// Must run on the graphics thread, serialized with video operations.
pub unsafe extern "C" fn VidSeek(position: u32) -> c_int {
    if VidPlaying().is_null() {
        0
    } else {
        tfb_seek_video(CURRENT_VIDEO, position) as c_int
    }
}

/// # Safety
/// Video and audio handles must be live; calls belong to the graphics thread.
#[no_mangle]
pub unsafe extern "C" fn VidPlayEx(
    video: *mut TFB_VideoClip,
    audio: *mut c_void,
    speech: *mut c_void,
    loop_frame: u32,
) -> c_int {
    if video.is_null() {
        return 0;
    }
    if !audio.is_null() {
        if !(*video).h_audio.is_null() {
            DestroyMusic((*video).h_audio);
        }
        (*video).h_audio = audio;
        (*(*video).decoder).audio_synced = false;
    }
    (*video).loop_frame = loop_frame;
    (*video).loop_to = 0;
    (*video).data = speech;
    VidStop();
    // C promotes the unsigned clip dimensions before converting the centered coordinates.
    let x = (ScreenWidth as u32).wrapping_sub((*video).w) / 2;
    let y = (ScreenHeight as u32).wrapping_sub((*video).h) / 2;
    if tfb_play_video(video, x, y) {
        CURRENT_VIDEO = video;
        if !speech.is_null() {
            CURRENT_SPEECH = speech;
        }
        2
    } else {
        0
    }
}

/// # Safety
/// Video must be a live handle; calls belong to the graphics thread.
pub unsafe extern "C" fn VidPlay(video: *mut TFB_VideoClip) -> c_int {
    VidPlayEx(video, ptr::null_mut(), ptr::null_mut(), u32::MAX)
}

/// # Safety
/// `path` is a terminated content pathname; graphics and video are initialized.
#[no_mangle]
pub unsafe extern "C" fn _init_video_file(path: *const c_char) -> *mut TFB_VideoClip {
    let decoder = VideoDecoder_Load(contentDir.cast(), path);
    if decoder.is_null() {
        return ptr::null_mut();
    }
    let video = rust_hcalloc(size_of::<TFB_VideoClip>()).cast::<TFB_VideoClip>();
    (*video).decoder = decoder;
    (*video).length = (*decoder).length;
    (*video).w = (*decoder).w;
    (*video).h = (*decoder).h;
    (*video).guard = (CreateMutex_Core(c"video guard".as_ptr(), 1 << 2)).cast();
    video
}

/// # Safety
/// `video` is null or an owned video handle and must not be used after destruction.
#[no_mangle]
pub unsafe extern "C" fn DestroyVideo(video: *mut TFB_VideoClip) -> c_int {
    if video.is_null() {
        return 0;
    }
    tfb_stop_video(video);
    VideoDecoder_Free((*video).decoder);
    DestroyMutex(((*video).guard).cast());
    rust_hfree(video.cast());
    1
}
