//! Shared fixtures for the sound engine's behavioural tests.
//!
//! Streams, the track player and the mixer share process-wide state, so tests
//! that drive them take `ENGINE_TEST_LOCK`.

use super::decoder::{DecodeError, SoundDecoder};
use super::formats::{AudioFormat, DecoderFormats};
use parking_lot::Mutex;
use std::path::Path;
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

/// Serializes tests that touch the process-wide sources, mixer pool, track
/// player or the decoder thread.
pub(crate) static ENGINE_TEST_LOCK: Mutex<()> = Mutex::new(());

/// How long a [`Probe`] is given to prove it is not waiting on a lock.
const PROBE_GRACE: Duration = Duration::from_millis(150);

/// A test body or fixture step running on its own thread, so a regression test
/// can hold a lock and observe whether the step respects it. A step that takes
/// the lock stays blocked until the holder lets go; one that skips it finishes
/// within the grace period.
pub(crate) struct Probe {
    handle: JoinHandle<()>,
}

impl Probe {
    pub(crate) fn spawn(step: impl FnOnce() + Send + 'static) -> Self {
        Probe {
            handle: thread::spawn(step),
        }
    }

    /// Whether the step finished, by returning or panicking, within the grace
    /// period.
    pub(crate) fn finishes_within_grace(&self) -> bool {
        let start = Instant::now();
        while !self.handle.is_finished() {
            if start.elapsed() >= PROBE_GRACE {
                return false;
            }
            thread::sleep(Duration::from_millis(1));
        }
        true
    }

    /// Whether every step is still unfinished once the grace period, shared by
    /// all of them, has passed.
    pub(crate) fn all_blocked_for_grace(probes: &[Probe]) -> bool {
        let start = Instant::now();
        while start.elapsed() < PROBE_GRACE {
            if probes.iter().any(|p| p.handle.is_finished()) {
                return false;
            }
            thread::sleep(Duration::from_millis(1));
        }
        true
    }

    /// The step's outcome: `Err` carries its panic.
    pub(crate) fn join(self) -> thread::Result<()> {
        self.handle.join()
    }
}

/// A decoder that hands out a fixed list of PCM chunks, then reports EOF.
pub(crate) struct ScriptedDecoder {
    chunks: Vec<Vec<u8>>,
    pub(crate) next: usize,
    frequency: u32,
    format: AudioFormat,
    pub(crate) seeks: Arc<Mutex<Vec<u32>>>,
    pub(crate) fail_after: Option<usize>,
    length_secs: f32,
}

impl ScriptedDecoder {
    pub(crate) fn new(chunks: Vec<Vec<u8>>) -> Self {
        ScriptedDecoder {
            chunks,
            next: 0,
            frequency: 44100,
            format: AudioFormat::Mono16,
            seeks: Arc::new(Mutex::new(Vec::new())),
            fail_after: None,
            length_secs: 1.0,
        }
    }

    pub(crate) fn with_format(mut self, frequency: u32, format: AudioFormat) -> Self {
        self.frequency = frequency;
        self.format = format;
        self
    }

    pub(crate) fn with_length(mut self, secs: f32) -> Self {
        self.length_secs = secs;
        self
    }
}

impl SoundDecoder for ScriptedDecoder {
    fn name(&self) -> &'static str {
        "scripted"
    }
    fn init_module(&mut self, _flags: i32, _formats: &DecoderFormats) -> bool {
        true
    }
    fn term_module(&mut self) {}
    fn get_error(&mut self) -> i32 {
        0
    }
    fn init(&mut self) -> bool {
        true
    }
    fn term(&mut self) {}
    fn open(&mut self, _path: &Path) -> Result<(), DecodeError> {
        Ok(())
    }
    fn open_from_bytes(&mut self, _data: &[u8], _name: &str) -> Result<(), DecodeError> {
        Ok(())
    }
    fn close(&mut self) {}
    fn decode(&mut self, buf: &mut [u8]) -> Result<usize, DecodeError> {
        if self.fail_after == Some(self.next) {
            return Err(DecodeError::DecoderError("scripted failure".into()));
        }
        let chunk = self.chunks.get(self.next).ok_or(DecodeError::EndOfFile)?;
        buf[..chunk.len()].copy_from_slice(chunk);
        self.next += 1;
        Ok(chunk.len())
    }
    fn seek(&mut self, pcm_pos: u32) -> Result<u32, DecodeError> {
        self.seeks.lock().push(pcm_pos);
        self.next = 0;
        Ok(pcm_pos)
    }
    /// Each scripted chunk stands for half a second of 44.1 kHz audio.
    fn get_frame(&self) -> u32 {
        self.next as u32 * 22050
    }
    fn frequency(&self) -> u32 {
        self.frequency
    }
    fn format(&self) -> AudioFormat {
        self.format
    }
    fn length(&self) -> f32 {
        self.length_secs
    }
    fn is_null(&self) -> bool {
        false
    }
    fn needs_swap(&self) -> bool {
        false
    }
}
