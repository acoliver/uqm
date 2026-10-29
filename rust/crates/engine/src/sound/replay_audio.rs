//! Audio completion derived from decoded sample counts and the replay clock.

use std::collections::HashMap;
use std::io::Cursor;

use rodio::Source;

use super::mixer::{source as mixer_source, SourceProp, SourceState};
use super::rodio_audio::AudioCommand;
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
            .expect("audio handles exhausted");
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

    fn decoded(&mut self, data: Vec<u8>, looping: bool, now: u32) -> u32 {
        let Ok(source) = rodio::Decoder::new(Cursor::new(data)) else {
            return 0;
        };
        let rate = source.sample_rate();
        let channels = source.channels();
        let frames = source.count() as u64 / u64::from(channels);
        self.play(frames, rate, looping, now)
    }

    pub(super) fn command(&mut self, command: AudioCommand, now: u32) {
        match command {
            AudioCommand::PlayWav(data, _, looping, reply)
            | AudioCommand::PlayOgg(data, _, looping, reply) => {
                let handle = self.decoded(data, looping, now);
                reply.send(handle).expect("audio play receiver dropped");
            }
            AudioCommand::PlayRaw(data, rate, channels, bits, _, looping, reply) => {
                let handle = if !matches!(bits, 8 | 16) || rate == 0 || channels == 0 {
                    0
                } else {
                    self.play(
                        data.len() as u64 / u64::from(channels) / u64::from(bits / 8),
                        rate,
                        looping,
                        now,
                    )
                };
                reply.send(handle).expect("audio play receiver dropped");
            }
            AudioCommand::IsPlaying(handle, reply) => {
                reply
                    .send(
                        self.sounds
                            .get(&handle)
                            .is_some_and(|sound| sound.playing(now)),
                    )
                    .expect("audio query receiver dropped");
            }
            AudioCommand::Pause(handle) => {
                if let Some(sound) = self.sounds.get_mut(&handle) {
                    sound.elapsed = sound.elapsed_ticks(now);
                    sound.paused = true;
                }
            }
            AudioCommand::Resume(handle) => {
                if let Some(sound) = self.sounds.get_mut(&handle) {
                    if sound.paused {
                        sound.started = now;
                        sound.paused = false;
                    }
                }
            }
            AudioCommand::Stop(handle) => {
                self.sounds.remove(&handle);
            }
            AudioCommand::StopAll | AudioCommand::Shutdown => self.sounds.clear(),
            AudioCommand::Cleanup => self.sounds.retain(|_, sound| sound.playing(now)),
            AudioCommand::SetVolume(_, _)
            | AudioCommand::SetMasterVolume(_)
            | AudioCommand::SetMusicVolume(_)
            | AudioCommand::SetSfxVolume(_)
            | AudioCommand::SetSpeechVolume(_) => {}
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
    use super::super::rodio_audio::SoundCategory;
    use super::*;
    use std::sync::mpsc;

    fn playing(audio: &mut ReplayAudio, handle: u32, now: u32) -> bool {
        let (tx, rx) = mpsc::channel();
        audio.command(AudioCommand::IsPlaying(handle, tx), now);
        rx.recv().unwrap()
    }

    #[test]
    fn decoded_wav_completion_uses_sample_count_and_rate() {
        let mut wav = Vec::new();
        wav.extend_from_slice(b"RIFF");
        wav.extend_from_slice(&2036u32.to_le_bytes());
        wav.extend_from_slice(b"WAVEfmt ");
        wav.extend_from_slice(&16u32.to_le_bytes());
        for value in [1u16, 1] {
            wav.extend_from_slice(&value.to_le_bytes());
        }
        for value in [1000u32, 2000] {
            wav.extend_from_slice(&value.to_le_bytes());
        }
        for value in [2u16, 16] {
            wav.extend_from_slice(&value.to_le_bytes());
        }
        wav.extend_from_slice(b"data");
        wav.extend_from_slice(&2000u32.to_le_bytes());
        wav.resize(2044, 0);
        let mut audio = ReplayAudio::default();
        let (tx, rx) = mpsc::channel();
        audio.command(
            AudioCommand::PlayWav(wav, SoundCategory::Speech, false, tx),
            0,
        );
        let handle = rx.recv().unwrap();
        assert_ne!(handle, 0);
        assert!(playing(&mut audio, handle, 839));
        assert!(!playing(&mut audio, handle, 840));
        assert_eq!(audio.decoded(vec![0; 20], false, 0), 0);
    }

    #[test]
    fn raw_stereo_completion_and_pause_use_replay_time() {
        let mut audio = ReplayAudio::default();
        let (tx, rx) = mpsc::channel();
        audio.command(
            AudioCommand::PlayRaw(vec![0; 4000], 1000, 2, 16, SoundCategory::Speech, false, tx),
            0,
        );
        let handle = rx.recv().unwrap();
        assert!(playing(&mut audio, handle, 419));
        audio.command(AudioCommand::Pause(handle), 420);
        assert!(playing(&mut audio, handle, 8400));
        audio.command(AudioCommand::Resume(handle), 8400);
        assert!(playing(&mut audio, handle, 8819));
        assert!(!playing(&mut audio, handle, 8820));
    }

    #[test]
    fn loop_stop_and_cleanup_keep_handle_identity() {
        let mut audio = ReplayAudio::default();
        let handle = audio.play(100, 1000, true, 0);
        assert!(playing(&mut audio, handle, 100_000));
        audio.command(AudioCommand::Cleanup, 100_000);
        assert!(playing(&mut audio, handle, 100_000));
        audio.command(AudioCommand::Stop(handle), 100_000);
        assert!(!playing(&mut audio, handle, 100_000));
        assert!(audio.play(100, 1000, false, 100_000) > handle);
    }

    #[test]
    fn fractional_frames_are_independent_of_sleep_partitioning() {
        let mut mixer = ReplayMixer::default();
        let total: u64 = (1..=840).map(|now| mixer.frames_until(now, 44100)).sum();
        assert_eq!(total, 44100);
        assert_eq!(ReplayMixer::default().frames_until(840, 44100), total);
    }
}
