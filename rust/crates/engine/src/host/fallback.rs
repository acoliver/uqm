//! The platform of a test process that installs none: the real clock and
//! file system, silent audio, an in-memory network and a display that shows
//! nothing.
//!
//! The game binary always installs a platform (`uqm-platform`); this stand-in
//! exists only in test builds, so engine, content and mode unit tests run
//! without the OS backends.

use super::clock::{Clock, RealClock, ReplayClock};
use super::files::{File, FileStat, FileStream, Files};
use super::system::{System, Worker};
use super::{AudioPort, Display, Net, Platform, PresentFrame};
use crate::callback::Dispatch;
use crate::graphics::presentation_diagnostics::PresentationSample;
use crate::graphics::presented_frame::PresentedFrame;
use crate::sound::rodio_audio::ReplayAudioPort;
use std::io;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

pub(super) struct TestPlatform {
    clock: RealClock,
    audio: ReplayAudioPort,
    dispatch: Dispatch,
    system: TestSystem,
}

impl TestPlatform {
    pub(super) fn new() -> Self {
        Self {
            clock: RealClock,
            audio: ReplayAudioPort::new(Arc::new(ReplayClock::default())),
            dispatch: Dispatch::new(),
            system: TestSystem(Instant::now()),
        }
    }
}

impl Platform for TestPlatform {
    fn clock(&self) -> &dyn Clock {
        &self.clock
    }
    fn audio(&self) -> &dyn AudioPort {
        &self.audio
    }
    fn dispatch(&self) -> &Dispatch {
        &self.dispatch
    }
    fn poll_input(&self) -> Vec<sdl2::event::Event> {
        Vec::new()
    }
    fn system(&self) -> &dyn System {
        &self.system
    }
    fn net(&self) -> Box<dyn Net> {
        Box::new(crate::network::loopback::LoopbackNet::default())
    }
    fn disk(&self) -> Arc<dyn Files + Send + Sync> {
        Arc::new(TestDisk)
    }
    fn open_display(
        &self,
        _flags: i32,
        _position: Option<(i32, i32)>,
        width: i32,
        height: i32,
    ) -> Result<Box<dyn Display>, String> {
        Ok(Box::new(TestDisplay {
            size: (width as u32, height as u32),
            presented: 0,
            samples: Vec::new(),
        }))
    }
}

/// Counts presentations and reports diagnostics but keeps no pixels: tests
/// that read the window back install the OS platform.
struct TestDisplay {
    size: (u32, u32),
    presented: u64,
    samples: Vec<PresentationSample>,
}

impl Display for TestDisplay {
    fn present(
        &mut self,
        _frame: &PresentFrame<'_>,
        capture: &mut dyn FnMut(u64, &[PresentationSample]) -> Option<u64>,
    ) -> Option<PresentedFrame> {
        self.presented += 1;
        capture(self.presented, &self.samples);
        self.samples.clear();
        None
    }
    fn record(&mut self, sample: PresentationSample) {
        self.samples.push(sample);
    }
    fn present_video(&mut self, _pixels: &[u32], _width: u32, _height: u32) -> Result<(), String> {
        Ok(())
    }
    #[cfg(feature = "debug-process")]
    fn native_window(&mut self, _show: bool) -> (u32, (i32, i32, u32, u32)) {
        (0, (0, 0, self.size.0, self.size.1))
    }
    fn reconfigure(&mut self, _flags: i32, width: i32, height: i32) -> Result<(), String> {
        self.size = (width as u32, height as u32);
        Ok(())
    }
}

struct TestSystem(Instant);

struct TestWorker(std::thread::JoinHandle<()>);

impl Worker for TestWorker {
    fn join(self: Box<Self>) -> bool {
        self.0.join().is_ok()
    }
}

impl System for TestSystem {
    fn elapsed(&self) -> Duration {
        self.0.elapsed()
    }
    fn sleep(&self, duration: Duration) {
        std::thread::sleep(duration);
    }
    fn yield_now(&self) {
        std::thread::yield_now();
    }
    fn unix_millis(&self) -> Option<u64> {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .ok()
            .map(|elapsed| elapsed.as_millis() as u64)
    }
    fn spawn(&self, name: &str, body: Box<dyn FnOnce() + Send>) -> Box<dyn Worker> {
        Box::new(TestWorker(
            std::thread::Builder::new()
                .name(name.into())
                .spawn(body)
                .unwrap_or_else(|error| panic!("spawn {name}: {error}")),
        ))
    }
    fn at_exit(&self, _handler: fn()) {}
    fn exit(&self, code: i32) -> ! {
        panic!("a test process exited with {code}")
    }
}

/// Host paths through `std::fs`.
struct TestDisk;

struct TestFile(std::fs::File);

impl File for TestFile {
    fn len(&self) -> io::Result<u64> {
        self.0.metadata().map(|meta| meta.len())
    }
}
impl io::Read for TestFile {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        self.0.read(bytes)
    }
}
impl io::Write for TestFile {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.write(bytes)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.0.flush()
    }
}
impl io::Seek for TestFile {
    fn seek(&mut self, position: io::SeekFrom) -> io::Result<u64> {
        self.0.seek(position)
    }
}

impl Files for TestDisk {
    fn open_read(&self, path: &Path) -> io::Result<FileStream> {
        Ok(FileStream::new(TestFile(std::fs::File::open(path)?)))
    }
    fn open_write(&self, path: &Path) -> io::Result<FileStream> {
        Ok(FileStream::new(TestFile(std::fs::File::create(path)?)))
    }
    fn remove(&self, path: &Path) -> io::Result<()> {
        std::fs::remove_file(path)
    }
    fn list(&self, path: &Path) -> io::Result<Vec<String>> {
        Ok(std::fs::read_dir(path)?
            .filter_map(|entry| {
                entry
                    .ok()
                    .map(|entry| entry.file_name().to_string_lossy().into_owned())
            })
            .collect())
    }
    fn stat(&self, path: &Path) -> io::Result<FileStat> {
        let meta = std::fs::metadata(path)?;
        Ok(FileStat {
            len: meta.len(),
            is_file: meta.is_file(),
            is_dir: meta.is_dir(),
        })
    }
}
