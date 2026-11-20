//! The OS behind `host::System` and the host file system behind the virtual
//! file system's directory mounts.

use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use crate::host::{File, FileStat, FileStream, Files, System, Worker};

/// Real time, sleeping and threads through `std`; process exit hooks
/// through the C runtime.
pub struct OsSystem {
    epoch: Instant,
}

impl OsSystem {
    pub fn new() -> Self {
        Self {
            epoch: Instant::now(),
        }
    }
}

impl Default for OsSystem {
    fn default() -> Self {
        Self::new()
    }
}

struct OsWorker(std::thread::JoinHandle<()>);

impl Worker for OsWorker {
    fn join(self: Box<Self>) -> bool {
        self.0.join().is_ok()
    }
}

impl System for OsSystem {
    fn elapsed(&self) -> Duration {
        self.epoch.elapsed()
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
        let handle = std::thread::Builder::new()
            .name(name.into())
            .spawn(body)
            .unwrap_or_else(|error| panic!("could not start thread {name}: {error}"));
        Box::new(OsWorker(handle))
    }
    fn at_exit(&self, handler: fn()) {
        super::os::sdl::at_exit(handler);
    }
    fn exit(&self, code: i32) -> ! {
        std::process::exit(code)
    }
}

/// Host paths opened directly.
pub struct OsDisk;

struct DiskFile(std::fs::File);

impl File for DiskFile {
    fn len(&self) -> io::Result<u64> {
        self.0.metadata().map(|meta| meta.len())
    }
}
impl Read for DiskFile {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        self.0.read(bytes)
    }
}
impl Write for DiskFile {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.0.write(bytes)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.0.flush()
    }
}
impl Seek for DiskFile {
    fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
        self.0.seek(position)
    }
}

impl Files for OsDisk {
    fn open_read(&self, path: &Path) -> io::Result<FileStream> {
        Ok(FileStream::new(DiskFile(std::fs::File::open(path)?)))
    }
    /// Create or truncate.
    fn open_write(&self, path: &Path) -> io::Result<FileStream> {
        let file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(path)?;
        Ok(FileStream::new(DiskFile(file)))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn files_round_trip_through_the_disk() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("file.bin");
        let mut file = OsDisk.open_write(&path).unwrap();
        file.write_all(b"abc").unwrap();
        file.finish().unwrap();
        assert_eq!(OsDisk.read(&path).unwrap(), b"abc");
        let stat = OsDisk.stat(&path).unwrap();
        assert_eq!((stat.len, stat.is_file, stat.is_dir), (3, true, false));
        assert_eq!(OsDisk.list(dir.path()).unwrap(), ["file.bin"]);
        OsDisk.remove(&path).unwrap();
        assert!(OsDisk.open_read(&path).is_err());
    }

    #[test]
    fn real_time_moves_forward_across_a_sleep() {
        let system = OsSystem::new();
        let before = system.elapsed();
        system.sleep(Duration::from_millis(5));
        assert!(system.elapsed() >= before + Duration::from_millis(5));
        assert!(system.unix_millis().is_some());
        assert!(system.spawn("worker", Box::new(|| ())).join());
        assert!(!system.spawn("panics", Box::new(|| panic!("worker"))).join());
    }
}
