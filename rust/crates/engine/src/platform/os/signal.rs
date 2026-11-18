//! Signal dispositions.

use std::ffi::c_int;
use std::io;

/// Run `handler` when the process receives `signal`.
///
/// The handler runs on whichever thread the kernel picks and may interrupt any
/// code, so it must be async-signal-safe: an atomic store and nothing else.
/// Installing the same signal again replaces the previous handler.
///
/// # Errors
///
/// Returns the OS error from `sigaction`; `EINVAL` for a signal that cannot be
/// caught (`SIGKILL`, `SIGSTOP`) or does not exist.
pub fn install_handler(signal: c_int, handler: extern "C" fn(c_int)) -> io::Result<()> {
    // SAFETY: sigaction is plain data for which all-zero is a valid empty value.
    let mut action: libc::sigaction = unsafe { std::mem::zeroed() };
    action.sa_sigaction = handler as *const () as usize;
    // SAFETY: the mask is initialized by sigemptyset before sigaction reads it.
    unsafe { libc::sigemptyset(&raw mut action.sa_mask) };
    // SAFETY: action is fully initialized and outlives the call; the old action is not requested.
    if unsafe { libc::sigaction(signal, &raw const action, std::ptr::null_mut()) } == -1 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    extern "C" fn ignore_signal(_signal: c_int) {}

    #[test]
    fn a_catchable_signal_accepts_a_handler() {
        install_handler(libc::SIGUSR2, ignore_signal).unwrap();
        install_handler(libc::SIGUSR2, ignore_signal).unwrap();
    }

    #[test]
    fn a_signal_that_cannot_be_caught_is_rejected() {
        let error = install_handler(libc::SIGKILL, ignore_signal).unwrap_err();
        assert_eq!(error.raw_os_error(), Some(libc::EINVAL));
    }

    #[test]
    fn a_signal_that_does_not_exist_is_rejected() {
        let error = install_handler(-1, ignore_signal).unwrap_err();
        assert_eq!(error.raw_os_error(), Some(libc::EINVAL));
    }
}
