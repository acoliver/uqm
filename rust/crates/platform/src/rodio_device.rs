//! The OS sound device for whole PCM clips, through rodio. rodio's output
//! stream is not `Send`, so it lives on its own thread fed by a channel.
use std::collections::HashMap;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Mutex;
use std::thread::JoinHandle;

use rodio::{OutputStream, Sink};

use uqm_engine::bridge_log::rust_bridge_log_msg;
use uqm_engine::host::AudioPort;
use uqm_engine::sound::audio_device::{AudioCommand, PcmClip};

pub struct RodioDevice {
    sender: Mutex<Option<Sender<AudioCommand>>>,
    thread: Mutex<Option<JoinHandle<()>>>,
}

/// The process's OS sound device.
pub static RODIO: RodioDevice = RodioDevice {
    sender: Mutex::new(None),
    thread: Mutex::new(None),
};

fn locked<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

impl AudioPort for RodioDevice {
    fn start(&self) -> bool {
        rust_bridge_log_msg("RUST_AUDIO_INIT: starting rodio audio system");
        let mut sender = locked(&self.sender);
        if sender.is_some() {
            rust_bridge_log_msg("RUST_AUDIO_INIT: already initialized");
            return true;
        }
        let (tx, rx) = mpsc::channel();
        let thread = match std::thread::Builder::new()
            .name("rodio audio".into())
            .spawn(move || device_thread(rx))
        {
            Ok(thread) => thread,
            Err(error) => {
                rust_bridge_log_msg(&format!("RUST_AUDIO_INIT: no audio thread: {error}"));
                return false;
            }
        };
        *sender = Some(tx);
        *locked(&self.thread) = Some(thread);
        rust_bridge_log_msg("RUST_AUDIO_INIT: success");
        true
    }

    fn send(&self, command: AudioCommand) -> bool {
        locked(&self.sender)
            .as_ref()
            .is_some_and(|sender| sender.send(command).is_ok())
    }

    fn shutdown(&self) {
        self.send(AudioCommand::Shutdown);
        // Dropping the sender also ends a thread that is waiting for commands.
        *locked(&self.sender) = None;
        if let Some(handle) = locked(&self.thread).take() {
            let _ = handle.join();
        }
    }
}

/// Serve commands until shutdown. Without an output device every clip gets
/// handle 0 and nothing reports playing.
fn device_thread(commands: Receiver<AudioCommand>) {
    rust_bridge_log_msg("RUST_AUDIO_THREAD: starting");
    let stream = match OutputStream::try_default() {
        Ok(stream) => {
            rust_bridge_log_msg("RUST_AUDIO_THREAD: output stream opened");
            Some(stream)
        }
        Err(error) => {
            rust_bridge_log_msg(&format!(
                "RUST_AUDIO_THREAD: failed to open output - {error}"
            ));
            None
        }
    };
    let output = stream.as_ref().map(|(_, output)| output);
    let mut sinks = HashMap::<u32, Sink>::new();
    let mut next_handle = 0u32;
    while let Ok(command) = commands.recv() {
        match command {
            AudioCommand::Play(clip, reply) => {
                let handle = match output.and_then(|output| play(output, clip)) {
                    Some(sink) => {
                        next_handle = next_handle.wrapping_add(1).max(1);
                        sinks.insert(next_handle, sink);
                        next_handle
                    }
                    None => 0,
                };
                let _ = reply.send(handle);
            }
            AudioCommand::Stop(handle) => {
                if let Some(sink) = sinks.remove(&handle) {
                    sink.stop();
                }
            }
            AudioCommand::IsPlaying(handle, reply) => {
                let _ = reply.send(sinks.get(&handle).is_some_and(|sink| !sink.empty()));
            }
            AudioCommand::Shutdown => break,
        }
    }
    for (_, sink) in sinks {
        sink.stop();
    }
    rust_bridge_log_msg("RUST_AUDIO_THREAD: stopped");
}

fn play(output: &rodio::OutputStreamHandle, clip: PcmClip) -> Option<Sink> {
    let sink = match Sink::try_new(output) {
        Ok(sink) => sink,
        Err(error) => {
            rust_bridge_log_msg(&format!("RUST_AUDIO: failed to create sink: {error}"));
            return None;
        }
    };
    let source = rodio::buffer::SamplesBuffer::new(clip.channels, clip.sample_rate, clip.samples);
    if clip.looping {
        sink.append(rodio::Source::repeat_infinite(source));
    } else {
        sink.append(source);
    }
    sink.play();
    Some(sink)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_device_plays_reports_and_stops_clips_and_shuts_down_once() {
        let device = RodioDevice {
            sender: Mutex::new(None),
            thread: Mutex::new(None),
        };
        assert!(
            !device.send(AudioCommand::Stop(1)),
            "nothing runs before start"
        );
        assert!(device.start());
        assert!(device.start(), "a second start keeps the running thread");
        let (reply, handle) = mpsc::channel();
        let clip = PcmClip::from_bytes(&[0; 400], 8000, 1, 16, false)
            .unwrap_or_else(|| panic!("silent clip"));
        assert!(device.send(AudioCommand::Play(clip, reply)));
        let handle = handle
            .recv()
            .unwrap_or_else(|error| panic!("play reply: {error}"));
        let (reply, playing) = mpsc::channel();
        assert!(device.send(AudioCommand::IsPlaying(handle, reply)));
        let playing = playing
            .recv()
            .unwrap_or_else(|error| panic!("query reply: {error}"));
        assert!(
            handle != 0 || !playing,
            "a clip without a device never plays"
        );
        assert!(device.send(AudioCommand::Stop(handle)));
        device.shutdown();
        assert!(
            !device.send(AudioCommand::Stop(handle)),
            "nothing runs after shutdown"
        );
    }
}
