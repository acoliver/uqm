//! The sound device that plays whole PCM clips (the movie soundtracks): the
//! commands the engine sends it, and the replay stand-in that answers them
//! from the replay clock. The OS device lives in the platform.

use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, Mutex};

use crate::bridge_log::rust_bridge_log_msg;
use crate::host::clock::{Clock, ReplayClock};
use crate::host::AudioPort;

/// Interleaved 16-bit PCM and how to play it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PcmClip {
    pub samples: Vec<i16>,
    pub sample_rate: u32,
    pub channels: u16,
    pub looping: bool,
}

impl PcmClip {
    /// The clip in `data` with `bits` per sample (8 unsigned or 16 signed
    /// little-endian); `None` for other widths or an empty rate or layout.
    pub fn from_bytes(
        data: &[u8],
        sample_rate: u32,
        channels: u16,
        bits: u16,
        looping: bool,
    ) -> Option<Self> {
        if sample_rate == 0 || channels == 0 {
            return None;
        }
        let samples = match bits {
            16 => data
                .as_chunks::<2>()
                .0
                .iter()
                .map(|chunk| i16::from_le_bytes(*chunk))
                .collect(),
            8 => data.iter().map(|&b| (i16::from(b) - 128) * 256).collect(),
            _ => return None,
        };
        Some(Self {
            samples,
            sample_rate,
            channels,
            looping,
        })
    }

    fn frames(&self) -> u64 {
        self.samples.len() as u64 / u64::from(self.channels)
    }
}

/// What the engine asks of the device.
#[derive(Debug)]
pub enum AudioCommand {
    /// Play the clip; the reply is its handle, 0 when it cannot play.
    Play(PcmClip, Sender<u32>),
    Stop(u32),
    IsPlaying(u32, Sender<bool>),
    Shutdown,
}

#[track_caller]
fn send(command: AudioCommand) -> bool {
    crate::host::platform().audio().send(command)
}

/// Bring the device up; `true` when it is running afterwards.
pub fn start() -> bool {
    crate::host::platform().audio().start()
}

pub fn shutdown() {
    crate::host::platform().audio().shutdown();
}

/// Play `clip`; its handle, or 0 when the device is not running or refuses it.
pub fn play(clip: PcmClip) -> u32 {
    let (tx, rx) = mpsc::channel();
    if send(AudioCommand::Play(clip, tx)) {
        rx.recv().unwrap_or(0)
    } else {
        rust_bridge_log_msg("RUST_AUDIO: play failed (audio thread not running?)");
        0
    }
}

pub fn stop(handle: u32) {
    send(AudioCommand::Stop(handle));
}

/// Whether `handle` is still playing; `false` when the device is not running.
pub fn is_playing(handle: u32) -> bool {
    let (tx, rx) = mpsc::channel();
    send(AudioCommand::IsPlaying(handle, tx)) && rx.recv().unwrap_or(false)
}

/// Replay stand-in: clips are bookkeeping on the replay clock, no device is opened.
pub struct ReplayAudioPort {
    clock: Arc<ReplayClock>,
    sounds: Mutex<super::replay_audio::ReplayAudio>,
}

impl ReplayAudioPort {
    pub fn new(clock: Arc<ReplayClock>) -> Self {
        Self {
            clock,
            sounds: Mutex::default(),
        }
    }
}

impl AudioPort for ReplayAudioPort {
    fn start(&self) -> bool {
        true
    }

    #[track_caller]
    fn send(&self, command: AudioCommand) -> bool {
        let now = self.clock.now_ticks(std::panic::Location::caller());
        self.sounds
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .command(command, now);
        true
    }

    fn shutdown(&self) {
        let now = self.clock.now_ticks(std::panic::Location::caller());
        self.sounds
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .command(AudioCommand::Shutdown, now);
    }
}

/// The frame count `ReplayAudio` uses for a clip's length.
pub(super) fn clip_frames(clip: &PcmClip) -> u64 {
    clip.frames()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pcm_bytes_become_signed_samples() {
        let clip = PcmClip::from_bytes(&[0, 128, 255, 255, 0, 0, 255, 127, 42], 8000, 2, 16, false)
            .unwrap();
        assert_eq!(clip.samples, [-32768, -1, 0, 32767]);
        assert_eq!(clip.frames(), 2);
        let clip = PcmClip::from_bytes(&[0, 127, 128, 255], 8000, 1, 8, true).unwrap();
        assert_eq!(clip.samples, [-32768, -256, 0, 32512]);
        assert!(PcmClip::from_bytes(&[], 8000, 1, 16, false).unwrap().samples.is_empty());
        assert!(PcmClip::from_bytes(&[42], 8000, 1, 16, false).unwrap().samples.is_empty());
        assert!(PcmClip::from_bytes(&[0; 3], 8000, 1, 24, false).is_none());
        assert!(PcmClip::from_bytes(&[0; 4], 0, 1, 16, false).is_none());
        assert!(PcmClip::from_bytes(&[0; 4], 8000, 0, 16, false).is_none());
    }

    #[test]
    fn commands_reach_the_installed_device_and_report_when_it_is_down() {
        // The test platform's device starts down: nothing is sent, nothing plays.
        let clip = PcmClip::from_bytes(&[0; 8], 8000, 2, 16, false).unwrap();
        assert_eq!(play(clip), 0);
        assert!(!is_playing(1));
    }
}
