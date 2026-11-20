//! The platform port: what the engine takes from the host process.
//!
//! The composition root builds one `Platform` (OS or replay) and installs it
//! before the game starts. The engine reads the clock and sends audio commands
//! through it.

use std::panic::Location;
use std::sync::{Arc, OnceLock};

use super::clock::Clock;
use super::{Display, Files, Net, System};
use crate::callback::Dispatch;
use crate::sound::rodio_audio::AudioCommand;

/// A running output stream; `stop` ends it.
pub trait AudioOutput: Send {
    fn stop(self: Box<Self>);
}

/// What an output device calls to fill its next buffer of interleaved
/// 44.1 kHz stereo samples.
pub type FillAudio = Box<dyn FnMut(&mut [f32]) + Send>;

/// A running audio device, or its replay stand-in.
pub trait AudioPort: Send + Sync {
    /// Bring the device up; `true` when it is running afterwards.
    fn start(&self) -> bool;
    /// Deliver a command; `false` when the device is not running.
    fn send(&self, command: AudioCommand) -> bool;
    fn shutdown(&self);
}

pub trait Platform: Send + Sync {
    fn clock(&self) -> &dyn Clock;
    fn audio(&self) -> &dyn AudioPort;
    /// The game's callback queue and alarms.
    fn dispatch(&self) -> &Dispatch;
    /// Everything the window system has queued since the last poll.
    fn poll_input(&self) -> Vec<super::HostEvent>;
    /// Turn the window system's text input (and its on-screen keyboard) on or off.
    fn set_text_input(&self, on: bool);
    /// Start the joystick subsystem and name the joysticks found.
    fn init_joysticks(&self) -> Result<Vec<String>, String>;
    /// Real time, sleeping, threads and process exit.
    fn system(&self) -> &dyn System;
    /// A new network endpoint.
    fn net(&self) -> Box<dyn Net>;
    /// Host paths, which the virtual file system mounts.
    fn disk(&self) -> Arc<dyn Files + Send + Sync>;
    /// Start the default output device pulling samples from `fill`; `None`
    /// when no device can play.
    fn audio_output(&self, fill: FillAudio) -> Option<Box<dyn AudioOutput>>;
    /// Open the game window at `width` x `height`; `position` places a
    /// borderless window on the desktop.
    fn open_display(
        &self,
        flags: i32,
        position: Option<(i32, i32)>,
        width: i32,
        height: i32,
    ) -> Result<Box<dyn Display>, String>;
}

static PLATFORM: OnceLock<Box<dyn Platform>> = OnceLock::new();

/// Make `platform` the process platform. Installing twice is a bug.
pub fn install_platform(platform: Box<dyn Platform>) {
    assert!(PLATFORM.set(platform).is_ok(), "platform installed twice");
}

/// Build and install the process platform: replay clock and audio for a
/// replay, the OS clock and rodio otherwise.
pub fn install(replay: bool) {
    install_platform(crate::platform::host::new(replay));
}

/// The installed platform.
pub fn platform() -> &'static dyn Platform {
    match PLATFORM.get() {
        Some(platform) => platform.as_ref(),
        None => uninstalled(),
    }
}

/// Tests that install no platform run on a stand-in with the real clock and
/// file system, silent audio and no display.
#[cfg(any(test, feature = "test-support"))]
fn uninstalled() -> &'static dyn Platform {
    static FALLBACK: OnceLock<super::fallback::TestPlatform> = OnceLock::new();
    FALLBACK.get_or_init(|| super::fallback::TestPlatform::new(false))
}

/// Install the test platform; `replay` runs it on the replay clock with
/// silent audio.
#[cfg(any(test, feature = "test-support"))]
pub fn install_test(replay: bool) {
    install_platform(Box::new(super::fallback::TestPlatform::new(replay)));
}

#[cfg(not(any(test, feature = "test-support")))]
fn uninstalled() -> &'static dyn Platform {
    panic!("the engine ran before the application installed its platform")
}

#[track_caller]
pub fn now_ticks() -> u32 {
    platform().clock().now_ticks(Location::caller())
}

#[track_caller]
pub fn now_ms() -> u32 {
    platform().clock().now_ms(Location::caller())
}

pub fn is_replay() -> bool {
    platform().clock().is_replay()
}

pub fn advance_to(wake: u32) {
    platform().clock().advance_to(wake);
}

pub fn steps() -> u64 {
    platform().clock().steps()
}

/// The game's callback queue and alarms.
pub fn dispatch() -> &'static Dispatch {
    platform().dispatch()
}

/// Run the pending callbacks and every due alarm.
#[track_caller]
pub fn process_async() {
    dispatch().process(now_ms);
}

/// Milliseconds the game thread may wait before callbacks or alarms are due.
#[track_caller]
pub fn async_wait_ms() -> u32 {
    dispatch().time_before_next_ms(now_ms)
}
