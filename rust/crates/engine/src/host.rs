//! Host ports used by engine clients without exposing native socket handles.
use std::io;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Connection(pub u64);

/// One game-thread network endpoint. Connect and listen start asynchronous work;
/// poll reports when the connection or the first accepted peer is ready.
pub trait Net {
    fn connect(&mut self, host: &str, port: &str) -> io::Result<Connection>;
    fn listen(&mut self, port: &str) -> io::Result<Connection>;
    /// Bound OS port, including an ephemeral listener selected with port zero.
    fn local_port(&self, connection: Connection) -> io::Result<u16>;
    fn poll(&mut self, connection: Connection) -> io::Result<bool>;
    fn send(&mut self, connection: Connection, bytes: &[u8]) -> io::Result<usize>;
    fn recv(&mut self, connection: Connection, bytes: &mut [u8]) -> io::Result<usize>;
    fn close(&mut self, connection: Connection);
}

/// The platform's network transport; modes only retain the port.
pub fn native_net() -> Box<dyn Net> {
    platform().net()
}

mod files;
pub use files::{File, FileStat, FileStream, Files};

pub mod clock;
mod display;
mod event;
#[cfg(any(test, feature = "test-support"))]
mod fallback;
#[cfg(any(test, feature = "test-support"))]
pub use fallback::queue_input;
mod platform;
mod system;
pub use clock::Clock;
pub use display::{Display, Layer, PresentFrame};
pub use event::HostEvent;
#[cfg(any(test, feature = "test-support"))]
pub use platform::install_test;
pub use platform::{
    advance_to, async_wait_ms, dispatch, install_platform, is_replay, now_ms, now_ticks, platform,
    process_async, steps, AudioOutput, AudioPort, FillAudio, Platform,
};
pub use system::{
    at_exit, elapsed, exit, sleep, sleep_precise, spawn, thread_number, unix_millis, yield_now,
    Deadline, System, Worker,
};
