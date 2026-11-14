//! Legacy video resource descriptors and playback, ported from the video library.
#![allow(non_snake_case)]

use super::ffi::TFB_VideoClip;
use crate::resource::ffi_bridge::InstallResTypeVectors;
use crate::resource::ffi_types::ResourceData;
use std::ffi::{c_char, c_int, c_void, CStr, CString};
use std::ptr;

struct Descriptor {
    video: CString,
    audio: Option<CString>,
    speech: Option<CString>,
    loop_frame: u32,
}

use crate::sound::resource_types::LoadedMusic;
use std::ptr::NonNull;

pub struct LoadedVideo(Box<Descriptor>);

impl LoadedVideo {
    pub fn load(resource: &CStr) -> Option<Self> {
        crate::resource::ffi_bridge::detach_resource_of_type(resource, "3DOVID").map(|video| {
            // SAFETY: the registered descriptor loader transfers one unique Box.
            Self(unsafe { Box::from_raw(video.as_ptr().cast()) })
        })
    }

    pub fn play(&self) -> Option<VideoPlayback> {
        crate::graphics::ffi::assert_owner_thread();
        // SAFETY: the descriptor retains its pathname on the admitted graphics thread.
        let video =
            NonNull::new(unsafe { super::control::_init_video_file(self.0.video.as_ptr()) })?;
        let audio = self
            .0
            .audio
            .as_ref()
            .and_then(|path| LoadedMusic::load_file(path));
        let speech = self
            .0
            .speech
            .as_ref()
            .and_then(|path| LoadedMusic::load_file(path));
        let playback = VideoPlayback {
            video,
            speech,
            audio,
        };
        // SAFETY: playback owns the clip and both optional music handles through stop.
        let result = unsafe {
            super::control::VidPlayEx(
                video.as_ptr(),
                playback
                    .audio
                    .as_ref()
                    .map_or(ptr::null_mut(), LoadedMusic::as_raw),
                playback
                    .speech
                    .as_ref()
                    .map_or(ptr::null_mut(), LoadedMusic::as_raw),
                self.0.loop_frame,
            )
        };
        (result != 0).then_some(playback)
    }
}

pub struct VideoPlayback {
    video: NonNull<TFB_VideoClip>,
    speech: Option<LoadedMusic>,
    audio: Option<LoadedMusic>,
}

impl VideoPlayback {
    pub fn is_playing(&self) -> bool {
        // SAFETY: this playback owns the initialized video clip.
        unsafe { super::ffi::tfb_video_playing(self.video.as_ptr()) }
    }

    pub fn position(&self) -> u32 {
        // SAFETY: this playback retains the active game-thread video.
        unsafe { super::control::VidGetPosition() }
    }

    pub fn seek(&self, position: u32) {
        // SAFETY: this playback retains the active game-thread video.
        unsafe { super::control::VidSeek(position) };
    }

    pub fn process_frame(&self) -> bool {
        // SAFETY: this playback retains the active game-thread video.
        unsafe { super::control::VidProcessFrame() != 0 }
    }
}

impl Drop for VideoPlayback {
    fn drop(&mut self) {
        // SAFETY: unique playback ownership keeps the clip and its audio alive through stop.
        unsafe {
            super::control::VidStop();
            super::control::DestroyVideo(self.video.as_ptr());
        }
        drop(self.speech.take());
        drop(self.audio.take());
    }
}

unsafe extern "C" fn free_legacy_data(data: *mut c_void) -> c_int {
    if data.is_null() {
        return 0;
    }
    // SAFETY: the resource registry returns one unreleased descriptor allocation.
    drop(unsafe { Box::from_raw(data.cast::<Descriptor>()) });
    1
}

fn terminated(bytes: &[u8]) -> CString {
    CString::new(bytes)
        .unwrap_or_else(|error| panic!("terminated video descriptor contains NUL: {error}"))
}

fn components(path: &[u8]) -> Vec<&[u8]> {
    path[..path.len().min(1023)]
        .splitn(4, |&c| c == b':')
        .collect()
}

unsafe extern "C" fn get_legacy_data(path: *const c_char, data: *mut ResourceData) {
    // SAFETY: the resource registry supplies a terminated descriptor and writable output.
    let parts = components(unsafe { CStr::from_ptr(path) }.to_bytes());
    let mut descriptor = Descriptor {
        video: terminated(parts[0]),
        audio: parts.get(1).map(|bytes| terminated(bytes)),
        speech: parts.get(2).map(|bytes| terminated(bytes)),
        loop_frame: u32::MAX,
    };
    if let Some(loop_string) = parts.get(3) {
        let value = terminated(loop_string);
        let mut end = ptr::null_mut();
        // SAFETY: the owned string and end pointer remain live throughout parsing.
        let parsed = unsafe { libc::strtol(value.as_ptr(), &mut end, 10) } as u32;
        // The original parser checks only the first unconsumed byte and permits whitespace.
        if unsafe { *end <= 32 } {
            descriptor.loop_frame = parsed;
        }
    }
    // SAFETY: the registry receives the unique descriptor allocation for loading or detachment.
    unsafe {
        (*data).ptr = Box::into_raw(Box::new(descriptor)).cast();
    };
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
                let descriptor = data.ptr.cast::<Descriptor>();
                assert_eq!((*descriptor).loop_frame, expected);
                assert_eq!((*descriptor).video.as_c_str(), c"v");
                assert_eq!(free_legacy_data(descriptor.cast()), 1);
            }
        }
    }
}
