//! The OS sound device for whole PCM clips, through rodio. rodio's output
//! stream is not `Send`, so it lives on its own thread fed by a channel.
use std::collections::HashMap;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Mutex;
use std::thread::JoinHandle;

use rodio::{OutputStream, Sink};

use crate::bridge_log::rust_bridge_log_msg;
use crate::host::AudioPort;
use crate::sound::audio_device::{AudioCommand, PcmClip};

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

fn device_thread(commands: Receiver<AudioCommand>) {
    rust_bridge_log_msg("RUST_AUDIO_THREAD: starting");
    let (_stream, output) = match OutputStream::try_default() {
        Ok(stream) => stream,
        Err(error) => {
            rust_bridge_log_msg(&format!(
                "RUST_AUDIO_THREAD: failed to open output - {error}"
            ));
            // Answer plays with "no handle" until shutdown.
            answer_without_device(commands);
            return;
        }
    };
    rust_bridge_log_msg("RUST_AUDIO_THREAD: output stream opened");
    let mut sinks = HashMap::<u32, Sink>::new();
    let mut next_handle = 0u32;
    while let Ok(command) = commands.recv() {
        match command {
            AudioCommand::Play(clip, reply) => {
                let handle = match play(&output, clip) {
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

fn answer_without_device(commands: Receiver<AudioCommand>) {
    while let Ok(command) = commands.recv() {
        match command {
            AudioCommand::Play(_, reply) => {
                let _ = reply.send(0);
            }
            AudioCommand::IsPlaying(_, reply) => {
                let _ = reply.send(false);
            }
            AudioCommand::Stop(_) => {}
            AudioCommand::Shutdown => break,
        }
    }
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
