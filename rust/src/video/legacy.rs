//! Legacy video resource descriptors and playback, ported from the video library.
#![allow(non_snake_case)]

use super::ffi::TFB_VideoClip;
use crate::memory::{rust_hcalloc, rust_hfree, rust_hmalloc};
use crate::resource::ffi_bridge::{res_DetachResource, res_GetResource, InstallResTypeVectors};
use crate::resource::ffi_types::ResourceData;
use std::ffi::{c_char, c_int, c_void, CStr};
use std::ptr;

#[repr(C)]
pub struct LegacyVideo {
    pub video: *mut c_char,
    pub audio: *mut c_char,
    pub speech: *mut c_char,
    pub loop_frame: u32,
}

#[repr(C)]
pub struct LegacyVideoRef {
    pub video: *mut TFB_VideoClip,
    pub audio: *mut c_void,
    pub speech: *mut c_void,
}

#[cfg(not(test))]
use crate::video::control::VidStop;
#[cfg(test)]
extern "C" {
    fn VidStop();
}

use crate::video::control::_init_video_file;

use crate::sound::heart_ffi::LoadMusicFile;

use crate::video::control::DestroyVideo;

use crate::sound::heart_ffi::DestroyMusic;
use crate::video::control::VidPlayEx;

/// # Safety
/// `path` must be a terminated resource pathname.
pub unsafe extern "C" fn LoadVideoFile(path: *const c_char) -> *mut TFB_VideoClip {
    _init_video_file(path)
}

/// # Safety
/// `video` must be null or a live legacy descriptor.
pub unsafe extern "C" fn PlayLegacyVideo(video: *mut LegacyVideo) -> *mut LegacyVideoRef {
    if video.is_null() {
        return ptr::null_mut();
    }
    let reference = rust_hcalloc(size_of::<LegacyVideoRef>()).cast::<LegacyVideoRef>();
    (*reference).video = LoadVideoFile((*video).video);
    if (*reference).video.is_null() {
        return ptr::null_mut();
    }
    if !(*video).audio.is_null() {
        (*reference).audio = LoadMusicFile((*video).audio);
    }
    if !(*video).speech.is_null() {
        (*reference).speech = LoadMusicFile((*video).speech);
    }
    if VidPlayEx(
        (*reference).video,
        (*reference).audio,
        (*reference).speech,
        (*video).loop_frame,
    ) == 0
    {
        StopLegacyVideo(reference);
        return ptr::null_mut();
    }
    reference
}

/// # Safety
/// `reference` must be null or an owned, live playback reference.
pub unsafe extern "C" fn StopLegacyVideo(reference: *mut LegacyVideoRef) {
    if reference.is_null() {
        return;
    }
    VidStop();
    DestroyVideo((*reference).video);
    if !(*reference).speech.is_null() {
        DestroyMusic((*reference).speech);
    }
    if !(*reference).audio.is_null() {
        DestroyMusic((*reference).audio);
    }
    rust_hfree(reference.cast());
}

/// # Safety
/// `reference` must be null or a live playback reference.
pub unsafe extern "C" fn PlayingLegacyVideo(reference: *mut LegacyVideoRef) -> c_int {
    if reference.is_null() {
        return 0;
    }
    super::ffi::tfb_video_playing((*reference).video) as c_int
}

unsafe extern "C" fn free_legacy_data(data: *mut c_void) -> c_int {
    if data.is_null() {
        return 0;
    }
    let video = data.cast::<LegacyVideo>();
    rust_hfree((*video).video.cast());
    rust_hfree((*video).audio.cast());
    rust_hfree((*video).speech.cast());
    rust_hfree(data);
    1
}

unsafe fn duplicate(bytes: &[u8]) -> *mut c_char {
    let result = rust_hmalloc(bytes.len() + 1).cast::<c_char>();
    ptr::copy_nonoverlapping(bytes.as_ptr(), result.cast(), bytes.len());
    *result.add(bytes.len()) = 0;
    result
}

fn components(path: &[u8]) -> Vec<&[u8]> {
    path[..path.len().min(1023)]
        .splitn(4, |&c| c == b':')
        .collect()
}

unsafe extern "C" fn get_legacy_data(path: *const c_char, data: *mut ResourceData) {
    let parts = components(CStr::from_ptr(path).to_bytes());
    let video = rust_hcalloc(size_of::<LegacyVideo>()).cast::<LegacyVideo>();
    (*video).loop_frame = u32::MAX;
    (*video).video = duplicate(parts[0]);
    if let Some(audio) = parts.get(1) {
        (*video).audio = duplicate(audio);
    }
    if let Some(speech) = parts.get(2) {
        (*video).speech = duplicate(speech);
    }
    if let Some(loop_string) = parts.get(3) {
        let value = duplicate(loop_string);
        let mut end = ptr::null_mut();
        let parsed = libc::strtol(value, &mut end, 10) as u32;
        // The original parser checks only the first unconsumed byte and permits whitespace.
        if *end <= 32 {
            (*video).loop_frame = parsed;
        }
        rust_hfree(value.cast());
    }
    (*data).ptr = video.cast();
}

/// # Safety
/// Called during resource registration.
pub unsafe fn InstallVideoResType() -> c_int {
    InstallResTypeVectors(
        c"3DOVID".as_ptr(),
        Some(get_legacy_data),
        Some(free_legacy_data),
        None,
    );
    1
}

/// # Safety
/// `resource` must identify a registered legacy video resource.
pub unsafe extern "C" fn LoadLegacyVideoInstance(resource: *const c_char) -> *mut LegacyVideo {
    let data = res_GetResource(resource);
    if !data.is_null() {
        res_DetachResource(resource);
    }
    data.cast()
}

/// # Safety
/// `video` must be null or an owned descriptor returned by resource loading.
pub unsafe extern "C" fn DestroyLegacyVideo(video: *mut LegacyVideo) -> c_int {
    free_legacy_data(video.cast())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resource_components_preserve_empty_fields_and_limit_splitting() {
        assert_eq!(components(b"video.duk"), vec![b"video.duk".as_slice()]);
        assert_eq!(
            components(b"v::speech:12:extra"),
            vec![b"v".as_slice(), b"", b"speech", b"12:extra"]
        );
        assert_eq!(components(&[b'x'; 1024])[0].len(), 1023);
    }

    #[test]
    fn resource_loop_parsing_matches_strtol_and_legacy_whitespace_rule() {
        for (path, expected) in [
            (c"v", u32::MAX),
            (c"v:::12", 12),
            (c"v:::-2", u32::MAX - 1),
            (c"v:::12x", u32::MAX),
            (c"v:::12 x", 12),
            (c"v:::", 0),
        ] {
            unsafe {
                let mut data = ResourceData {
                    ptr: ptr::null_mut(),
                };
                get_legacy_data(path.as_ptr(), &mut data);
                let descriptor = data.ptr.cast::<LegacyVideo>();
                assert_eq!((*descriptor).loop_frame, expected);
                assert_eq!(CStr::from_ptr((*descriptor).video), c"v");
                assert_eq!(DestroyLegacyVideo(descriptor), 1);
            }
        }
    }
}
