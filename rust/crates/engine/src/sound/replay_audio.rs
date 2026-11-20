//! Audio completion derived from decoded sample counts and the replay clock.

use std::collections::HashMap;


use super::mixer::{source as mixer_source, SourceProp, SourceState};
use super::audio_device::{clip_frames, AudioCommand};
use crate::time::counter_abi::ONE_SECOND;

pub(super) fn source_may_have_drained(handle: usize, replay: bool) -> bool {
    // Replay consumes exactly the frames due at a sleep boundary. The mixer
    // retires the last buffer there, but stops only on the next device sample.
    replay
        || mixer_source::mixer_get_source_i(handle, SourceProp::SourceState).unwrap_or(0)
            != SourceState::Playing as i32
}

struct Playback {
    frames: u64,
    rate: u32,
    started: u32,
    elapsed: u32,
    paused: bool,
    looping: bool,
}

impl Playback {
    fn elapsed_ticks(&self, now: u32) -> u32 {
        self.elapsed + if self.paused { 0 } else { now - self.started }
    }

    fn playing(&self, now: u32) -> bool {
        self.looping
            || u64::from(self.elapsed_ticks(now)) * u64::from(self.rate)
                < self.frames * u64::from(ONE_SECOND)
    }
}

#[derive(Default)]
pub(super) struct ReplayAudio {
    sounds: HashMap<u32, Playback>,
    next_handle: u32,
}

impl ReplayAudio {
    fn play(&mut self, frames: u64, rate: u32, looping: bool, now: u32) -> u32 {
        self.next_handle = self
            .next_handle
            .checked_add(1)
            .unwrap_or_else(|| panic!("audio handles exhausted"));
        self.sounds.insert(
            self.next_handle,
            Playback {
                frames,
                rate,
                started: now,
                elapsed: 0,
                paused: false,
                looping,
            },
        );
        self.next_handle
    }

    pub(super) fn command(&mut self, command: AudioCommand, now: u32) {
        match command {
            AudioCommand::Play(clip, reply) => {
                let handle = self.play(clip_frames(&clip), clip.sample_rate, clip.looping, now);
                // A requester that stopped waiting has no use for the handle.
                let _ = reply.send(handle);
            }
            AudioCommand::IsPlaying(handle, reply) => {
                let playing = self
                    .sounds
                    .get(&handle)
                    .is_some_and(|sound| sound.playing(now));
                let _ = reply.send(playing);
            }
            AudioCommand::Stop(handle) => {
                self.sounds.remove(&handle);
            }
            AudioCommand::Shutdown => self.sounds.clear(),
        }
    }
}

/// Fractional sample frames are carried across sleeps, never rounded per pass.
#[derive(Default)]
pub(super) struct ReplayMixer {
    last_ticks: u32,
    remainder: u64,
}

impl ReplayMixer {
    pub(super) fn new(now: u32) -> Self {
        Self {
            last_ticks: now,
            remainder: 0,
        }
    }

    pub(super) fn frames_until(&mut self, now: u32, rate: u32) -> u64 {
        let scaled = u64::from(now - self.last_ticks) * u64::from(rate) + self.remainder;
        self.last_ticks = now;
        self.remainder = scaled % u64::from(ONE_SECOND);
        scaled / u64::from(ONE_SECOND)
    }
}

#[cfg(test)]
mod replay {
    use super::super::audio_device::PcmClip;
    use super::*;
    use std::sync::mpsc;

    fn playing(audio: &mut ReplayAudio, handle: u32, now: u32) -> bool {
        let (tx, rx) = mpsc::channel();
        audio.command(AudioCommand::IsPlaying(handle, tx), now);
        rx.recv().unwrap()
    }

    fn play(audio: &mut ReplayAudio, bytes: usize, looping: bool, now: u32) -> u32 {
        let clip = PcmClip::from_bytes(&vec![0; bytes], 1000, 2, 16, looping).unwrap();
        let (tx, rx) = mpsc::channel();
        audio.command(AudioCommand::Play(clip, tx), now);
        rx.recv().unwrap()
    }

    #[test]
    fn stereo_completion_uses_frames_and_replay_time() {
        let mut audio = ReplayAudio::default();
        // 4000 bytes of 16-bit stereo at 1000 Hz is one second.
        let handle = play(&mut audio, 4000, false, 0);
        assert!(playing(&mut audio, handle, ONE_SECOND - 1));
        assert!(!playing(&mut audio, handle, ONE_SECOND));
    }

    #[test]
    fn loops_play_until_stopped_and_shutdown_forgets_every_clip() {
        let mut audio = ReplayAudio::default();
        let looped = play(&mut audio, 4, true, 0);
        let once = play(&mut audio, 4, false, 0);
        assert_ne!(looped, once);
        assert!(playing(&mut audio, looped, 100_000));
        audio.command(AudioCommand::Stop(looped), 100_000);
        assert!(!playing(&mut audio, looped, 100_000));
        let again = play(&mut audio, 4, true, 0);
        audio.command(AudioCommand::Shutdown, 0);
        assert!(!playing(&mut audio, again, 0));
    }

    #[test]
    fn fractional_frames_are_independent_of_sleep_partitioning() {
        let mut mixer = ReplayMixer::default();
        let total: u64 = (1..=840).map(|now| mixer.frames_until(now, 44100)).sum();
        assert_eq!(total, 44100);
        assert_eq!(ReplayMixer::default().frames_until(840, 44100), total);
    }
}
