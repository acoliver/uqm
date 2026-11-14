//! Legacy video resource descriptors and playback, ported from the video library.
#![allow(non_snake_case)]

use super::descriptor::Descriptor;
use super::ffi::TFB_VideoClip;
use crate::resource::ffi_bridge::InstallResTypeVectors;
use std::ffi::{c_int, CStr};
use std::ptr;

use crate::sound::resource_types::LoadedMusic;
use std::ptr::NonNull;

pub struct LoadedVideo(Descriptor);

impl LoadedVideo {
    pub fn load(resource: &CStr) -> Option<Self> {
        let descriptor = crate::resource::settings::Settings.descriptor(resource, "3DOVID")?;
        Some(Self(Descriptor::parse(descriptor.as_bytes())))
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
        (playback.operation(Operation::Start(self.0.loop_frame)) != 0).then_some(playback)
    }
}

pub struct VideoPlayback {
    video: NonNull<TFB_VideoClip>,
    speech: Option<LoadedMusic>,
    audio: Option<LoadedMusic>,
}

enum Operation {
    Playing,
    Position,
    Seek(u32),
    Frame,
    Start(u32),
    Stop,
}

impl VideoPlayback {
    fn operation(&self, operation: Operation) -> u32 {
        crate::graphics::ffi::assert_owner_thread();
        let video = self.video.as_ptr();
        let audio = self
            .audio
            .as_ref()
            .map_or(ptr::null_mut(), LoadedMusic::as_raw);
        let speech = self
            .speech
            .as_ref()
            .map_or(ptr::null_mut(), LoadedMusic::as_raw);
        // SAFETY: this non-Send owner retains the clip and audio through each game-thread operation.
        unsafe {
            match operation {
                Operation::Playing => u32::from(super::ffi::tfb_video_playing(video)),
                Operation::Position => super::control::VidGetPosition(),
                Operation::Seek(position) => super::control::VidSeek(position) as u32,
                Operation::Frame => super::control::VidProcessFrame() as u32,
                Operation::Start(loop_frame) => {
                    super::control::VidPlayEx(video, audio, speech, loop_frame) as u32
                }
                Operation::Stop => {
                    super::control::VidStop();
                    super::control::DestroyVideo(video) as u32
                }
            }
        }
    }

    pub fn is_playing(&self) -> bool {
        self.operation(Operation::Playing) != 0
    }
    pub fn position(&self) -> u32 {
        self.operation(Operation::Position)
    }
    pub fn seek(&self, position: u32) {
        self.operation(Operation::Seek(position));
    }
    pub fn process_frame(&self) -> bool {
        self.operation(Operation::Frame) != 0
    }
}

impl Drop for VideoPlayback {
    fn drop(&mut self) {
        self.operation(Operation::Stop);
        drop(self.speech.take());
        drop(self.audio.take());
    }
}

/// # Safety
/// Called during resource registration.
pub unsafe fn InstallVideoResType() -> c_int {
    InstallResTypeVectors(
        c"3DOVID".as_ptr(),
        Some(crate::resource::type_registry::use_descriptor_as_res),
        None,
        None,
    );
    1
}
