//! Wall clock, monotonic deadlines, thread yielding and whole-file IO for the
//! crates the layer rules keep away from `std::{time, thread, fs}`.

use std::{
    io,
    path::Path,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

/// Milliseconds since the Unix epoch, or `None` when the clock reads earlier.
pub fn unix_millis() -> Option<u64> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|elapsed| elapsed.as_millis() as u64)
}

/// A monotonic point in time that a polling loop waits for.
#[derive(Clone, Copy, Debug)]
pub struct Deadline(Instant);

impl Deadline {
    pub fn after_millis(millis: u32) -> Self {
        Self(Instant::now() + Duration::from_millis(u64::from(millis)))
    }

    pub fn has_passed(self) -> bool {
        Instant::now() >= self.0
    }

    /// Sleep until the deadline, but for no longer than one millisecond.
    pub fn nap(self) {
        std::thread::sleep(
            self.0
                .saturating_duration_since(Instant::now())
                .min(Duration::from_millis(1)),
        );
    }
}

pub fn yield_now() {
    std::thread::yield_now();
}

pub fn read_file(path: &Path) -> io::Result<Vec<u8>> {
    std::fs::read(path)
}

/// Write `contents` to `path`. A failed write removes the partial file.
pub fn write_file_or_remove(path: &Path, contents: &[u8]) -> io::Result<()> {
    std::fs::write(path, contents).inspect_err(|_| {
        let _ = std::fs::remove_file(path);
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_zero_millisecond_deadline_has_passed_and_a_long_one_has_not() {
        assert!(Deadline::after_millis(0).has_passed());
        assert!(!Deadline::after_millis(60_000).has_passed());
    }

    #[test]
    fn nap_returns_within_a_millisecond_of_a_distant_deadline() {
        let start = Instant::now();
        Deadline::after_millis(60_000).nap();
        assert!(start.elapsed() < Duration::from_millis(500));
    }

    #[test]
    fn files_round_trip_and_a_failed_write_leaves_no_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("file.bin");
        write_file_or_remove(&path, b"abc").unwrap();
        assert_eq!(read_file(&path).unwrap(), b"abc");
        let missing = dir.path().join("no-such-dir/file.bin");
        assert!(write_file_or_remove(&missing, b"abc").is_err());
        assert!(read_file(&missing).is_err());
        assert!(unix_millis().is_some());
    }
}
