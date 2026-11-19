//! Seatbelt sandbox installation for a child process.

use std::ffi::CString;
use std::io;
use std::os::unix::process::CommandExt as _;
use std::process::Command;

#[link(name = "sandbox")]
unsafe extern "C" {
    fn sandbox_init(
        profile: *const libc::c_char,
        flags: u64,
        error: *mut *mut libc::c_char,
    ) -> libc::c_int;
    fn sandbox_free_error(error: *mut libc::c_char);
}

/// Confine the child to the sandbox `profile`, installed between `fork` and
/// `exec`. The child fails to start if the OS refuses the profile.
pub fn restrict_candidate(command: &mut Command, profile: CString) {
    // SAFETY: the profile CString is allocated before fork and outlives
    // sandbox_init; sandbox_init and sandbox_free_error are the only calls the
    // hook makes, and neither allocates through Rust.
    unsafe {
        command.pre_exec(move || {
            let mut error = std::ptr::null_mut();
            if sandbox_init(profile.as_ptr(), 0, &raw mut error) != 0 {
                if !error.is_null() {
                    sandbox_free_error(error);
                }
                return Err(io::Error::new(
                    io::ErrorKind::PermissionDenied,
                    "native candidate sandbox installation failed",
                ));
            }
            Ok(())
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_accepted_sandbox_profile_lets_the_child_run() {
        let mut command = Command::new("/usr/bin/true");
        restrict_candidate(
            &mut command,
            CString::new("(version 1) (allow default)").unwrap(),
        );
        assert!(command.status().unwrap().success());
    }

    #[test]
    fn a_refused_sandbox_profile_fails_the_spawn() {
        let mut command = Command::new("/usr/bin/true");
        restrict_candidate(&mut command, CString::new("not a profile").unwrap());
        // std reports a pre-exec failure without an OS code as EINVAL, so only
        // the refusal itself is observable.
        assert!(command.status().is_err());
    }
}
