//! User identity and child resource limits.

use std::io;
use std::process::Command;

pub use uqm_process_support::os::process::*;

/// The real user ID of this process.
#[must_use]
pub fn real_user_id() -> u32 {
    // SAFETY: getuid has no preconditions and cannot fail.
    unsafe { libc::getuid() }
}

/// The effective user ID of this process.
#[must_use]
pub fn effective_user_id() -> u32 {
    // SAFETY: geteuid has no preconditions and cannot fail.
    unsafe { libc::geteuid() }
}

/// Cap the size of any file the child writes to `bytes`, before it executes.
pub fn limit_file_size_before_exec(command: &mut Command, bytes: u64) {
    let limit = libc::rlimit {
        rlim_cur: bytes as libc::rlim_t,
        rlim_max: bytes as libc::rlim_t,
    };
    before_exec(command, move || {
        // SAFETY: limit is a fully initialized rlimit that outlives the call.
        if unsafe { libc::setrlimit(libc::RLIMIT_FSIZE, &raw const limit) } != 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(())
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_ids_are_the_owner_of_files_this_process_creates() {
        use std::os::unix::fs::MetadataExt as _;
        // The effective ID can differ from the real one only under setuid,
        // which a test binary is not.
        assert_eq!(real_user_id(), effective_user_id());
        let root = tempfile::tempdir().unwrap();
        assert_eq!(
            std::fs::metadata(root.path()).unwrap().uid(),
            effective_user_id()
        );
    }

    #[test]
    fn a_child_cannot_write_past_its_file_size_limit() {
        let root = tempfile::tempdir().unwrap();
        let target = root.path().join("out");
        let mut command = Command::new("/bin/sh");
        command
            .arg("-c")
            .arg("trap '' XFSZ; head -c 4096 /dev/zero > \"$1\"")
            .arg("sh")
            .arg(&target);
        limit_file_size_before_exec(&mut command, 1_024);
        let status = command.status().unwrap();
        assert!(!status.success());
        assert_eq!(std::fs::metadata(&target).unwrap().len(), 1_024);
    }
}
