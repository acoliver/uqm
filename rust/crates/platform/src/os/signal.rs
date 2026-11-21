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
    set_disposition(signal, handler as *const () as usize)
}

/// Ignore `signal`, so delivery does nothing and an `exec`ed child inherits
/// the ignored disposition.
///
/// Async-signal-safe: one `sigaction`.
///
/// # Errors
///
/// As for [`install_handler`].
pub fn ignore(signal: c_int) -> io::Result<()> {
    set_disposition(signal, libc::SIG_IGN)
}

/// Deliver `signal` to the calling thread.
///
/// # Errors
///
/// Returns the OS error from `raise`; `EINVAL` for a signal that does not exist.
pub fn raise(signal: c_int) -> io::Result<()> {
    // SAFETY: raise takes the signal number only.
    if unsafe { libc::raise(signal) } != 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

fn set_disposition(signal: c_int, disposition: usize) -> io::Result<()> {
    // SAFETY: sigaction is plain data for which all-zero is a valid empty value.
    let mut action: libc::sigaction = unsafe { std::mem::zeroed() };
    action.sa_sigaction = disposition;
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
        let error = ignore(libc::SIGKILL).unwrap_err();
        assert_eq!(error.raw_os_error(), Some(libc::EINVAL));
    }

    #[test]
    fn an_ignored_signal_is_inherited_across_exec() {
        use crate::os::process::before_exec;
        let mut command = std::process::Command::new("/bin/sleep");
        command.arg("30");
        before_exec(&mut command, || ignore(libc::SIGTERM));
        let mut child = command.spawn().unwrap();
        let pid = libc::pid_t::try_from(child.id()).unwrap();
        crate::os::process::send_signal(pid, libc::SIGTERM).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(100));
        assert!(
            child.try_wait().unwrap().is_none(),
            "SIGTERM was not ignored"
        );
        child.kill().unwrap();
        child.wait().unwrap();
    }

    #[test]
    fn a_signal_that_does_not_exist_is_rejected() {
        let error = install_handler(-1, ignore_signal).unwrap_err();
        assert_eq!(error.raw_os_error(), Some(libc::EINVAL));
    }
}
