//! The process platforms: the OS one and the replay one, which shares the OS
//! display, files and threads but runs on the replay clock and silent audio.

use std::sync::Arc;

use super::system::{OsDisk, OsSystem};
use crate::callback::Dispatch;
use crate::graphics::sdl::{input, SdlDisplay};
use crate::host::clock::{Clock, RealClock, ReplayClock};
use crate::host::{
    AudioOutput, AudioPort, Display, FillAudio, Files, HostEvent, Net, Platform, System,
};
use super::rodio_device::RODIO;
use crate::sound::audio_device::ReplayAudioPort;

fn open_display(
    flags: i32,
    position: Option<(i32, i32)>,
    width: i32,
    height: i32,
) -> Result<Box<dyn Display>, String> {
    Ok(Box::new(SdlDisplay::open(flags, position, width, height)?))
}

struct OsPlatform {
    clock: RealClock,
    dispatch: Dispatch,
    system: OsSystem,
}

impl Platform for OsPlatform {
    fn clock(&self) -> &dyn Clock {
        &self.clock
    }
    fn audio(&self) -> &dyn AudioPort {
        &RODIO
    }
    fn dispatch(&self) -> &Dispatch {
        &self.dispatch
    }
    fn poll_input(&self) -> Vec<HostEvent> {
        input::poll()
    }
    fn set_text_input(&self, on: bool) {
        input::set_text_input(on);
    }
    fn audio_output(&self, fill: FillAudio) -> Option<Box<dyn AudioOutput>> {
        super::audio_output::start(fill)
    }
    fn init_joysticks(&self) -> Result<Vec<String>, String> {
        input::init_joysticks()
    }
    fn system(&self) -> &dyn System {
        &self.system
    }
    fn net(&self) -> Box<dyn Net> {
        Box::new(crate::network::NativeNet::default())
    }
    fn disk(&self) -> Arc<dyn Files + Send + Sync> {
        Arc::new(OsDisk)
    }
    fn open_display(
        &self,
        flags: i32,
        position: Option<(i32, i32)>,
        width: i32,
        height: i32,
    ) -> Result<Box<dyn Display>, String> {
        open_display(flags, position, width, height)
    }
}

struct ReplayPlatform {
    clock: Arc<ReplayClock>,
    audio: ReplayAudioPort,
    dispatch: Dispatch,
    system: OsSystem,
}

impl Platform for ReplayPlatform {
    fn clock(&self) -> &dyn Clock {
        &*self.clock
    }
    fn audio(&self) -> &dyn AudioPort {
        &self.audio
    }
    fn dispatch(&self) -> &Dispatch {
        &self.dispatch
    }
    /// A replay has no window system: its input is the script's.
    fn poll_input(&self) -> Vec<HostEvent> {
        Vec::new()
    }
    fn set_text_input(&self, _on: bool) {}
    /// The replay mixes for its trace; nothing plays.
    fn audio_output(&self, _fill: FillAudio) -> Option<Box<dyn AudioOutput>> {
        None
    }
    fn init_joysticks(&self) -> Result<Vec<String>, String> {
        input::init_joysticks()
    }
    fn system(&self) -> &dyn System {
        &self.system
    }
    fn net(&self) -> Box<dyn Net> {
        Box::new(crate::network::NativeNet::default())
    }
    fn disk(&self) -> Arc<dyn Files + Send + Sync> {
        Arc::new(OsDisk)
    }
    fn open_display(
        &self,
        flags: i32,
        position: Option<(i32, i32)>,
        width: i32,
        height: i32,
    ) -> Result<Box<dyn Display>, String> {
        open_display(flags, position, width, height)
    }
}

/// Build the process platform: the replay clock and audio for a replay, the
/// OS clock and rodio otherwise.
pub fn new(replay: bool) -> Box<dyn Platform> {
    if replay {
        let clock = Arc::new(ReplayClock::default());
        Box::new(ReplayPlatform {
            audio: ReplayAudioPort::new(clock.clone()),
            clock,
            dispatch: Dispatch::new(),
            system: OsSystem::new(),
        })
    } else {
        Box::new(OsPlatform {
            clock: RealClock,
            dispatch: Dispatch::new(),
            system: OsSystem::new(),
        })
    }
}
