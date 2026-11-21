//! Advisory whole-file locks.

use std::io;
use std::os::fd::{AsRawFd as _, BorrowedFd};

/// Take an exclusive lock on the open file `fd` without waiting.
///
/// Returns `Ok(false)` when another open file description holds the lock. The
/// lock lasts until [`unlock`] or until the last descriptor for the open file
/// description closes.
///
/// # Errors
///
/// Returns the OS error from `flock` for any failure other than contention.
pub fn try_lock_exclusive(fd: BorrowedFd<'_>) -> io::Result<bool> {
    // SAFETY: flock takes an integer descriptor and never dereferences memory.
    if unsafe { libc::flock(fd.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } == 0 {
        return Ok(true);
    }
    let error = io::Error::last_os_error();
    if error.kind() == io::ErrorKind::WouldBlock {
        Ok(false)
    } else {
        Err(error)
    }
}

/// Release a lock taken with [`try_lock_exclusive`].
///
/// # Errors
///
/// Returns the OS error from `flock`.
pub fn unlock(fd: BorrowedFd<'_>) -> io::Result<()> {
    // SAFETY: flock takes an integer descriptor and never dereferences memory.
    if unsafe { libc::flock(fd.as_raw_fd(), libc::LOCK_UN) } == -1 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::fd::AsFd as _;

    #[test]
    fn a_second_open_file_description_cannot_take_a_held_lock() {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("guard");
        let first = std::fs::File::create(&path).unwrap();
        let second = std::fs::File::open(&path).unwrap();
        assert!(try_lock_exclusive(first.as_fd()).unwrap());
        assert!(!try_lock_exclusive(second.as_fd()).unwrap());
        unlock(first.as_fd()).unwrap();
        assert!(try_lock_exclusive(second.as_fd()).unwrap());
    }
}
