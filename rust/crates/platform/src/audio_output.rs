//! The default output device through cpal, pulling 44.1 kHz stereo from the
//! engine's mixer.
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use uqm_engine::host::{AudioOutput, FillAudio};

/// A cpal stream kept alive on its own thread (cpal streams are not `Send`).
struct CpalOutput {
    shutdown: Arc<AtomicBool>,
    thread: std::thread::JoinHandle<()>,
}

impl AudioOutput for CpalOutput {
    fn stop(self: Box<Self>) {
        self.shutdown.store(true, Ordering::Release);
        let _ = self.thread.join();
    }
}

/// Start the default output device; `None` when the thread cannot start.
/// A device that fails to open is logged and plays nothing.
pub fn start(fill: FillAudio) -> Option<Box<dyn AudioOutput>> {
    let shutdown = Arc::new(AtomicBool::new(false));
    let stop = shutdown.clone();
    let thread = std::thread::Builder::new()
        .name("mixer pump".into())
        .spawn(move || run(fill, &stop))
        .ok()?;
    Some(Box::new(CpalOutput { shutdown, thread }))
}

/// Keep the stream alive until stopped. Without a device the pump idles the
/// same way, so stopping it is the same either way.
fn run(fill: FillAudio, shutdown: &AtomicBool) {
    let stream = open(fill)
        .inspect_err(|error| eprintln!("[mixer_pump] {error}"))
        .ok();
    while !shutdown.load(Ordering::Acquire) {
        std::thread::sleep(Duration::from_millis(50));
    }
    drop(stream);
}

/// The mixer's output format: 44.1 kHz stereo.
const MIXER_FORMAT: cpal::StreamConfig = cpal::StreamConfig {
    channels: 2,
    sample_rate: cpal::SampleRate(44100),
    buffer_size: cpal::BufferSize::Default,
};

/// Open and start the default device in the mixer's format.
fn open(mut fill: FillAudio) -> Result<cpal::Stream, String> {
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

    let device = cpal::default_host()
        .default_output_device()
        .ok_or("no output audio device found")?;
    let stream = device
        .build_output_stream(
            &MIXER_FORMAT,
            move |output: &mut [f32], _: &cpal::OutputCallbackInfo| fill_output(&mut fill, output),
            |err| eprintln!("[mixer_pump] audio stream error: {err}"),
            None,
        )
        .map_err(|e| format!("failed to build output stream: {e}"))?;
    stream
        .play()
        .map_err(|e| format!("failed to start playback: {e}"))?;
    Ok(stream)
}

/// One device callback: the mixer fills `output`, or silence when it panics.
fn fill_output(fill: &mut FillAudio, output: &mut [f32]) {
    use std::panic::{catch_unwind, AssertUnwindSafe};
    use std::sync::atomic::AtomicU64;
    static PANICS: AtomicU64 = AtomicU64::new(0);

    if catch_unwind(AssertUnwindSafe(|| fill(output))).is_err() {
        output.fill(0.0);
        let panics = PANICS.fetch_add(1, Ordering::Relaxed);
        if panics < 5 || panics.is_multiple_of(1000) {
            eprintln!("[mixer_pump_cb] recovered from panic in output callback #{panics}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_panicking_mixer_leaves_silence_and_a_working_one_fills_the_buffer() {
        let mut working: FillAudio = Box::new(|output: &mut [f32]| output.fill(0.5));
        let mut output = [0.0; 4];
        fill_output(&mut working, &mut output);
        assert_eq!(output, [0.5; 4]);
        let mut broken: FillAudio = Box::new(|_: &mut [f32]| panic!("mixer"));
        fill_output(&mut broken, &mut output);
        assert_eq!(output, [0.0; 4]);
    }

    #[test]
    fn the_pump_starts_and_stops_with_or_without_a_device() {
        let output = start(Box::new(|output: &mut [f32]| output.fill(0.0)))
            .unwrap_or_else(|| panic!("pump thread"));
        output.stop();
    }
}
