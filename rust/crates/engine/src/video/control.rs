//! Public video lifecycle and playback control, ported from video.c.
#![allow(non_snake_case)]

use super::ffi::*;

use crate::memory::{rust_hcalloc, rust_hfree};
use std::ffi::{c_char, c_int, c_void, CStr, CString};
use std::ptr;

static mut CURRENT_VIDEO: *mut TFB_VideoClip = ptr::null_mut();
static mut CURRENT_SPEECH: *mut c_void = ptr::null_mut();

use crate::sound::heart_ffi::snd_stop_speech as snd_StopSpeech;

extern "C" {

    static mut ScreenWidth: i32;
    static mut ScreenHeight: i32;
}

/// # Safety
/// Must run on the graphics thread with graphics initialized.
pub unsafe extern "C" fn InitVideoPlayer(_use_cdrom: c_int) -> c_int {
    tfb_init_video_player() as c_int
}

/// # Safety
/// Must run on the graphics thread, serialized with video operations.
pub unsafe extern "C" fn UninitVideoPlayer() {
    tfb_uninit_video_player();
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

/// Probe the movie named by `path`; only `.duk` movies have a decoder.
fn load_clip_source(dir: crate::io::Dir, path: &CStr) -> Option<ClipSource> {
    let name = path.to_str().ok()?;
    let (_, extension) = name.rsplit_once('.')?;
    if extension != "duk" {
        return None;
    }
    let duk = super::files::open(&dir, name).ok()?;
    Some(ClipSource {
        w: duk.width(),
        h: duk.height(),
        length: duk.duration(),
        audio_synced: false,
        dir,
        filename: CString::from(path),
    })
}

/// # Safety
/// `path` is a terminated content pathname; graphics and video are initialized.
#[no_mangle]
pub unsafe extern "C" fn _init_video_file(path: *const c_char) -> *mut TFB_VideoClip {
    let Some(content) = crate::io::content_dir() else {
        return ptr::null_mut();
    };
    let Some(decoder) = load_clip_source(content, CStr::from_ptr(path)) else {
        return ptr::null_mut();
    };
    let decoder = Box::into_raw(Box::new(decoder));
    let video = rust_hcalloc(size_of::<TFB_VideoClip>()).cast::<TFB_VideoClip>();
    (*video).decoder = decoder;
    (*video).length = (*decoder).length;
    (*video).w = (*decoder).w;
    (*video).h = (*decoder).h;
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
    drop(Box::from_raw((*video).decoder));
    rust_hfree(video.cast());
    1
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::video::legacy::LoadedVideo;

    #[test]
    fn video_lifecycle_uses_real_content_and_replay_audio_without_a_window() {
        const CHILD: &str = "UQM_ENGINE_VIDEO_CONTROL_CHILD";
        if std::env::var_os(CHILD).is_none() {
            let status = std::process::Command::new(std::env::current_exe().unwrap())
                .args(["--exact", "video::control::tests::video_lifecycle_uses_real_content_and_replay_audio_without_a_window", "--nocapture"])
                .env(CHILD, "1")
                .status().unwrap();
            assert!(status.success());
            return;
        }
        crate::host::install(true);
        let directory = crate::video::ffi_tests::logo_directory().as_content();
        directory.write(
            "video.rmp",
            b"fixture.video = 3DOVID:logo.duk
fixture.absent = 3DOVID:missing.duk
",
        );
        // SAFETY: the child owns the content directory, surfaces and decoder handles for the full lifecycle.
        unsafe {
            let screen =
                sdl2_sys::SDL_CreateRGBSurface(0, 320, 240, 32, 0xff, 0xff00, 0xff0000, 0xff000000);
            assert!(!screen.is_null());
            crate::graphics::sdl::lifecycle_abi::SDL_Screen = screen.cast();
            crate::graphics::sdl::lifecycle_abi::GRAPHICS_BACKEND =
                ptr::addr_of_mut!(crate::graphics::sdl::lifecycle_abi::RUST_BACKEND);
            assert_eq!(InitVideoPlayer(0), 1);
            assert!(VidPlaying().is_null());
            assert_eq!(VidProcessFrame(), 0);
            assert_eq!(VidGetPosition(), 0);
            assert_eq!(VidSeek(100), 0);
            assert_eq!(VidPlay(ptr::null_mut()), 0);
            assert_eq!(DestroyVideo(ptr::null_mut()), 0);
            assert!(_init_video_file(c"missing.duk".as_ptr()).is_null());
            assert!(_init_video_file(c"no-extension".as_ptr()).is_null());
            assert!(_init_video_file(c"movie.unknown".as_ptr()).is_null());
            let clip = _init_video_file(c"logo.duk".as_ptr());
            assert!(!clip.is_null());
            assert_eq!((*clip).w, (*(*clip).decoder).w);
            assert_eq!(VidPlay(clip), 2);
            assert_eq!(VidPlaying(), clip);
            assert_eq!(VidGetPosition(), 0);
            assert_eq!(VidSeek(100), 0);
            assert_eq!(VidProcessFrame(), 1);
            VidStop();
            assert!(VidPlaying().is_null());
            assert_eq!(DestroyVideo(clip), 1);
            crate::video::legacy::InstallVideoResType();
            crate::resource::ffi_bridge::load_resource_index(
                &directory.dir(),
                c"video.rmp".as_ptr(),
                ptr::null(),
            );
            retained_playback_keeps_its_clip_after_descriptor_drop();
            assert!(VidPlaying().is_null());
            UninitVideoPlayer();
            crate::sound::rodio_audio::rust_audio_uninit();
            crate::graphics::sdl::lifecycle_abi::GRAPHICS_BACKEND = ptr::null_mut();
            crate::graphics::sdl::lifecycle_abi::SDL_Screen = ptr::null_mut();
            sdl2_sys::SDL_FreeSurface(screen);
        }
    }
    fn retained_playback_keeps_its_clip_after_descriptor_drop() {
        assert!(LoadedVideo::load(c"fixture.unknown").is_none());
        let absent = LoadedVideo::load(c"fixture.absent")
            .unwrap_or_else(|| panic!("missing-file descriptor"));
        assert!(absent.play().is_none());
        drop(absent);
        let descriptor =
            LoadedVideo::load(c"fixture.video").unwrap_or_else(|| panic!("logo descriptor"));
        let playback = descriptor.play().unwrap_or_else(|| panic!("logo playback"));
        drop(descriptor);
        assert!(playback.is_playing());
        assert_eq!(playback.position(), 0);
        playback.seek(100);
        assert!(playback.process_frame());
        drop(playback);
    }
}
