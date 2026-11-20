//! The platform port: what the engine takes from the host process.
//!
//! The composition root builds one `Platform` (OS or replay) and installs it
//! before the game starts. The engine reads the clock and sends audio commands
//! through it.

use std::panic::Location;
use std::sync::{Arc, OnceLock};

use super::clock::{Clock, OsClock, ReplayClock};
use crate::sound::rodio_audio::{AudioCommand, ReplayAudioPort, RodioAudio, RODIO};

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
}

struct OsPlatform {
    clock: OsClock,
    audio: &'static RodioAudio,
}

impl Platform for OsPlatform {
    fn clock(&self) -> &dyn Clock {
        &self.clock
    }
    fn audio(&self) -> &dyn AudioPort {
        self.audio
    }
}

struct ReplayPlatform {
    clock: Arc<ReplayClock>,
    audio: ReplayAudioPort,
}

impl Platform for ReplayPlatform {
    fn clock(&self) -> &dyn Clock {
        &*self.clock
    }
    fn audio(&self) -> &dyn AudioPort {
        &self.audio
    }
}

static PLATFORM: OnceLock<Box<dyn Platform>> = OnceLock::new();

/// Build and install the process platform: replay clock and audio for a
/// replay, the OS clock and rodio otherwise. Installing twice is a bug.
pub fn install(replay: bool) {
    let platform: Box<dyn Platform> = if replay {
        let clock = Arc::new(ReplayClock::default());
        Box::new(ReplayPlatform {
            audio: ReplayAudioPort::new(clock.clone()),
            clock,
        })
    } else {
        Box::new(OsPlatform {
            clock: OsClock::new(),
            audio: &RODIO,
        })
    };
    assert!(PLATFORM.set(platform).is_ok(), "platform installed twice");
}

/// The installed platform. Before installation the engine runs on the OS
/// clock with no audio device.
pub fn platform() -> &'static dyn Platform {
    static UNINSTALLED: OnceLock<OsPlatform> = OnceLock::new();
    match PLATFORM.get() {
        Some(platform) => platform.as_ref(),
        None => UNINSTALLED.get_or_init(|| OsPlatform {
            clock: OsClock::new(),
            audio: &RODIO,
        }),
    }
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
