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

fn run(mut fill: FillAudio, shutdown: &AtomicBool) {
    use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

    eprintln!("[mixer_pump] thread started, opening cpal output stream...");
    let Some(device) = cpal::default_host().default_output_device() else {
        eprintln!("[mixer_pump] no output audio device found");
        return;
    };
    eprintln!(
        "[mixer_pump] using device: {:?}",
        device.name().unwrap_or_default()
    );
    match device.default_output_config() {
        Ok(config) => eprintln!(
            "[mixer_pump] device default config: channels={} sample_rate={} sample_format={:?}",
            config.channels(),
            config.sample_rate().0,
            config.sample_format()
        ),
        Err(e) => {
            eprintln!("[mixer_pump] failed to get default output config: {}", e);
            return;
        }
    }
    let config = cpal::StreamConfig {
        channels: 2,
        sample_rate: cpal::SampleRate(44100),
        buffer_size: cpal::BufferSize::Default,
    };
    let stream = match device.build_output_stream(
        &config,
        move |output: &mut [f32], _: &cpal::OutputCallbackInfo| {
            use std::panic::{catch_unwind, AssertUnwindSafe};
            use std::sync::atomic::AtomicU64;
            static CB_COUNT: AtomicU64 = AtomicU64::new(0);
            static CB_PANIC_COUNT: AtomicU64 = AtomicU64::new(0);

            let n = CB_COUNT.fetch_add(1, Ordering::Relaxed);
            if n < 3 {
                eprintln!("[mixer_pump_cb#{}] output.len()={}", n, output.len());
            }
            // A panic must not unwind into the audio driver's thread.
            if catch_unwind(AssertUnwindSafe(|| fill(output))).is_err() {
                output.fill(0.0);
                let panic_n = CB_PANIC_COUNT.fetch_add(1, Ordering::Relaxed);
                if panic_n < 5 || panic_n.is_multiple_of(1000) {
                    eprintln!(
                        "[mixer_pump_cb] recovered from panic in output callback #{}",
                        panic_n
                    );
                }
            }
        },
        |err| eprintln!("[mixer_pump] audio stream error: {}", err),
        None,
    ) {
        Ok(stream) => stream,
        Err(e) => {
            eprintln!("[mixer_pump] failed to build output stream: {}", e);
            return;
        }
    };
    if let Err(e) = stream.play() {
        eprintln!("[mixer_pump] failed to start playback: {}", e);
        return;
    }
    eprintln!("[mixer_pump] started — feeding mixer output to cpal");
    while !shutdown.load(Ordering::Acquire) {
        std::thread::sleep(Duration::from_millis(50));
    }
    drop(stream);
    eprintln!("[mixer_pump] stopped");
}
