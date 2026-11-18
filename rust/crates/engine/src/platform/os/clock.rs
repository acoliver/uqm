//! The monotonic clock.

use std::io;

/// Milliseconds on the monotonic clock, from an arbitrary fixed origin.
///
/// Async-signal-safe: it allocates nothing and takes no lock, so it may run
/// between `fork` and `exec`.
///
/// # Errors
///
/// Returns the OS error if the clock cannot be read, or `EOVERFLOW` if the
/// reading does not fit in `u64` milliseconds.
pub fn monotonic_millis() -> io::Result<u64> {
    let mut value = libc::timespec {
        tv_sec: 0,
        tv_nsec: 0,
    };
    // SAFETY: value is writable timespec storage.
    if unsafe { libc::clock_gettime(libc::CLOCK_MONOTONIC, &raw mut value) } == -1 {
        return Err(io::Error::last_os_error());
    }
    let seconds =
        u64::try_from(value.tv_sec).map_err(|_| io::Error::from_raw_os_error(libc::EOVERFLOW))?;
    let nanos =
        u64::try_from(value.tv_nsec).map_err(|_| io::Error::from_raw_os_error(libc::EOVERFLOW))?;
    seconds
        .checked_mul(1_000)
        .and_then(|millis| millis.checked_add(nanos / 1_000_000))
        .ok_or_else(|| io::Error::from_raw_os_error(libc::EOVERFLOW))
}

#[cfg(test)]
mod tests {
    use super::monotonic_millis;

    #[test]
    fn the_clock_never_runs_backwards_and_advances_with_sleep() {
        let before = monotonic_millis().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(20));
        let after = monotonic_millis().unwrap();
        assert!(after >= before + 20, "{before} -> {after}");
    }
}
