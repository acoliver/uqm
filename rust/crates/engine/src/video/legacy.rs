//! Movie resources and their playback: one movie plays at a time.

use super::descriptor::Descriptor;
use super::player::VideoPlayer;
use crate::bridge_log::rust_bridge_log_msg;
use crate::io::Dir;
use crate::sound::audio_device::{self, PcmClip};
use crate::sound::decoder::Decoder;
use crate::sound::dukaud::DukAudDecoder;
use crate::sound::resource_types::LoadedMusic;
use std::ffi::CStr;
use std::sync::{Mutex, MutexGuard};

/// `VID_NO_LOOP`: the descriptor names no loop frame.
const NO_LOOP: u32 = u32::MAX;

/// The movie that is playing, if any.
#[derive(Default)]
struct Current {
    /// The decoded picture stream of the started movie.
    player: Option<VideoPlayer>,
    /// A playback was started and has not been stopped.
    started: bool,
    /// The started playback carries a speech track.
    speech: bool,
    /// Device handle of the movie's own soundtrack; 0 when none plays.
    audio: u32,
}

static CURRENT: Mutex<Current> = Mutex::new(Current {
    player: None,
    started: false,
    speech: false,
    audio: 0,
});

fn current() -> MutexGuard<'static, Current> {
    CURRENT
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

impl Current {
    /// Silence the soundtrack and drop the picture stream.
    fn stop_clip(&mut self) {
        let audio = std::mem::take(&mut self.audio);
        if audio != 0 {
            audio_device::stop(audio);
            rust_bridge_log_msg(&format!("RUST_VIDEO: stopped audio handle={audio}"));
        }
        self.player = None;
    }

    /// Stop the started playback: its speech, then its clip.
    fn stop(&mut self) {
        if self.speech {
            crate::sound::heart_ffi::snd_stop_speech();
        }
        if self.started {
            self.stop_clip();
        }
        self.started = false;
        self.speech = false;
    }

    fn playing(&self) -> bool {
        self.player.as_ref().is_some_and(VideoPlayer::playing)
    }

    fn position(&self) -> u32 {
        if !self.started || !self.playing() {
            return 0;
        }
        self.player.as_ref().map_or(0, VideoPlayer::position_ms)
    }
}

/// Stop whatever movie is playing; the video subsystem is being torn down or
/// reconfigured.
pub fn stop_player() {
    current().player = None;
}

/// Where a movie's files are and its picture size.
struct Clip {
    dir: Dir,
    filename: String,
    w: u32,
    h: u32,
}

impl Clip {
    /// Probe the movie named by `path` in the content directory; only `.duk`
    /// movies have a decoder.
    fn probe(path: &CStr) -> Option<Self> {
        let dir = crate::io::content_dir()?;
        let filename = path.to_str().ok()?;
        let (_, extension) = filename.rsplit_once('.')?;
        if extension != "duk" {
            return None;
        }
        let duk = super::files::open(&dir, filename).ok()?;
        Some(Self {
            w: duk.width(),
            h: duk.height(),
            dir,
            filename: filename.to_owned(),
        })
    }

    /// Start the picture stream at `(x, y)` on the main screen.
    fn start_player(&self, current: &mut Current, x: i32, y: i32, looping: bool) -> bool {
        let decoder = match super::files::open(&self.dir, &self.filename) {
            Ok(decoder) => decoder,
            Err(error) => {
                rust_bridge_log_msg(&format!("RUST_VIDEO: decoder init failed: {error:?}"));
                return false;
            }
        };
        let (width, height) = (decoder.width(), decoder.height());
        let mut player = VideoPlayer::new(decoder);
        player.set_position(x, y);
        player.set_loop(looping);
        rust_bridge_log_msg(&format!(
            "RUST_VIDEO: play {} ({width}x{height}), scale=1, dst=({x}, {y}), looping={looping}",
            self.filename
        ));
        player.play();
        current.player = Some(player);
        true
    }

    /// Decode the soundtrack embedded in the movie and start it on the
    /// audio device; returns the device handle, 0 when there is none.
    fn start_soundtrack(&self, looping: bool) -> u32 {
        let basename = self.filename.trim_end_matches(".duk");
        let duk = super::files::read(&self.dir, &format!("{basename}.duk"));
        let frm = super::files::read(&self.dir, &format!("{basename}.frm"));
        let (Some(duk), Some(frm)) = (duk, frm) else {
            rust_bridge_log_msg(&format!(
                "RUST_VIDEO: could not read .duk/.frm for audio from {}",
                self.filename
            ));
            return 0;
        };
        let mut decoder = DukAudDecoder::new();
        if decoder.open_from_data(&duk, &frm).is_err() {
            rust_bridge_log_msg("RUST_VIDEO: DukAudDecoder failed to open");
            return 0;
        }
        let mut pcm = Vec::new();
        let mut chunk = vec![0u8; 8192];
        while let Ok(count @ 1..) = decoder.decode(&mut chunk) {
            pcm.extend_from_slice(&chunk[..count]);
        }
        if pcm.is_empty() {
            rust_bridge_log_msg("RUST_VIDEO: no audio decoded from .duk");
            return 0;
        }
        let bytes = pcm.len();
        // DukAud is 22,050 Hz 16-bit stereo.
        let handle = PcmClip::from_bytes(&pcm, 22050, 2, 16, looping).map_or(0, audio_device::play);
        rust_bridge_log_msg(&format!(
            "RUST_VIDEO: playing embedded audio, {bytes} bytes PCM, handle={handle}"
        ));
        handle
    }
}

/// A movie resource: the clip, its optional music and speech tracks and the
/// frame it loops back to.
pub struct LoadedVideo(Descriptor);

impl LoadedVideo {
    pub fn load(resource: &CStr) -> Option<Self> {
        let descriptor = crate::resource::descriptor(resource, "3DOVID")?;
        Some(Self(Descriptor::parse(descriptor.as_bytes())))
    }

    /// Stop the movie that is playing and start this one centred on the
    /// screen.
    pub fn play(&self) -> Option<VideoPlayback> {
        let clip = Clip::probe(&self.0.video)?;
        let playback = VideoPlayback {
            audio: self.0.audio.as_deref().and_then(LoadedMusic::load_file),
            speech: self.0.speech.as_deref().and_then(LoadedMusic::load_file),
        };
        let mut current = current();
        current.stop();
        // Centre as C did: the unsigned clip size is subtracted before halving.
        let (width, height) = crate::graphics::state::screen_dimensions();
        let x = (width as u32).wrapping_sub(clip.w) / 2;
        let y = (height as u32).wrapping_sub(clip.h) / 2;
        rust_bridge_log_msg(&format!("RUST_VIDEO: TFB_PlayVideo {}", clip.filename));
        let looping = self.0.loop_frame != NO_LOOP;
        if !clip.start_player(&mut current, x as i32, y as i32, looping) {
            drop(current);
            drop(playback);
            return None;
        }
        audio_device::start();
        current.audio = clip.start_soundtrack(looping);
        current.started = true;
        current.speech = playback.speech.is_some();
        Some(playback)
    }
}

/// The started movie. Dropping it stops the movie and releases its tracks.
pub struct VideoPlayback {
    speech: Option<LoadedMusic>,
    audio: Option<LoadedMusic>,
}

impl VideoPlayback {
    pub fn is_playing(&self) -> bool {
        current().playing()
    }

    /// Milliseconds into the movie; 0 when it is not playing.
    pub fn position(&self) -> u32 {
        current().position()
    }

    /// DUK movies cannot seek: the request is ignored.
    pub fn seek(&self, _position: u32) {}

    /// Show the next due frame; false when the movie has ended.
    pub fn process_frame(&self) -> bool {
        let mut current = current();
        if !current.started {
            return false;
        }
        let playing = process_frame(&mut current);
        rust_bridge_log_msg(&format!(
            "RUST_VIDEO: rust_process_video_frame -> {}",
            i32::from(playing)
        ));
        playing
    }
}

fn process_frame(current: &mut Current) -> bool {
    use crate::graphics::{dcq::Screen, present, state};
    if !state::initialized() {
        rust_bridge_log_msg("RUST_VIDEO: the main screen is not initialized");
        return false;
    }
    let Some(player) = current.player.as_mut() else {
        rust_bridge_log_msg("RUST_VIDEO: process_frame called with no active player");
        return false;
    };
    let playing = player.process_frame(&mut |frame, x, y| {
        state::draw_on_screen(Screen::Main, |canvas| {
            super::player::blit_frame(canvas, frame, x, y)
        })
    });
    if !playing {
        current.player = None;
    } else if !player.direct_window_mode() {
        present::swap_buffers(present::Redraw::Fading);
    }
    playing
}

impl Drop for VideoPlayback {
    fn drop(&mut self) {
        let mut current = current();
        current.stop();
        current.stop_clip();
        drop(current);
        drop(self.speech.take());
        drop(self.audio.take());
    }
}

#[cfg(test)]
#[path = "legacy_tests.rs"]
mod tests;
