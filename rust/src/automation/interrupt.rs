//! Inbound interruption handling for a supervising run.
//!
//! A supervisor that dies on its own signal leaves the process it was
//! supervising running, its run lock held, and its owned-process record
//! stale. Recording the signal instead lets the run tear down through the
//! same targeted stop-and-reap path it uses for every other failure.

use std::io;
use std::sync::atomic::{AtomicI32, Ordering};

/// The signal that interrupted this process, or zero.
///
/// Written only by the signal handler, which may use nothing but an atomic
/// store.
static INTERRUPTED: AtomicI32 = AtomicI32::new(0);

/// Why interruption handling could not be established.
#[derive(Debug)]
pub struct InterruptError {
    pub signal: i32,
    pub error: io::Error,
}

impl std::fmt::Display for InterruptError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "cannot install interruption handling for signal {}: {}",
            self.signal, self.error
        )
    }
}

impl std::error::Error for InterruptError {}

/// The signal handler.
///
/// Async-signal-safety is the whole constraint here: this does one relaxed
/// atomic store and nothing else. No allocation, no formatting, no locks.
extern "C" fn record_signal(signal: libc::c_int) {
    INTERRUPTED.store(signal, Ordering::SeqCst);
}

/// Record `SIGINT` and `SIGTERM` rather than dying on them.
///
/// Idempotent, and safe to call before any child exists: the flag is only
/// read by the run loop.
///
/// # Errors
///
/// Returns the first signal whose disposition could not be installed.
pub fn install() -> Result<(), InterruptError> {
    for signal in [libc::SIGINT, libc::SIGTERM] {
        install_one(signal)?;
    }
    Ok(())
}

fn install_one(signal: libc::c_int) -> Result<(), InterruptError> {
    uqm_platform::os::signal::install_handler(signal, record_signal)
        .map_err(|error| InterruptError { signal, error })
}

/// The signal that interrupted this process, if one has arrived.
#[must_use]
pub fn interrupted() -> Option<i32> {
    match INTERRUPTED.load(Ordering::SeqCst) {
        0 => None,
        signal => Some(signal),
    }
}

/// Forget any recorded interruption.
///
/// Exists so a test can assert the transition rather than inheriting state
/// from whatever ran before it.
pub fn clear() {
    INTERRUPTED.store(0, Ordering::SeqCst);
}

/// The error a run reports when it stops because it was interrupted.
#[must_use]
pub fn interruption_error(signal: i32) -> io::Error {
    io::Error::other(format!(
        "run interrupted by signal {signal}; tearing down the supervised process"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_interrupting_signal_is_recorded_instead_of_killing_the_run() {
        clear();
        assert_eq!(interrupted(), None);
        install().expect("install interruption handling");

        // Without a handler this terminates the test process outright.
        let own_pid = libc::pid_t::try_from(std::process::id()).unwrap();
        uqm_platform::os::process::send_signal(own_pid, libc::SIGTERM).unwrap();

        // Delivery to a process-directed signal may land on another thread, so
        // wait for the handler to have recorded it.
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while interrupted().is_none() && std::time::Instant::now() < deadline {
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        assert_eq!(
            interrupted(),
            Some(libc::SIGTERM),
            "the run must survive to tear its child down"
        );
        clear();
    }

    #[test]
    fn the_reported_error_names_the_signal() {
        let error = interruption_error(libc::SIGINT);
        let text = error.to_string();
        assert!(text.contains(&libc::SIGINT.to_string()));
        assert!(text.contains("tearing down"));
    }
}
