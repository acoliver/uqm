//! Wake a waiter when one directory changes.

use std::io;
use std::os::fd::{AsFd as _, AsRawFd as _, BorrowedFd, OwnedFd};
use std::time::Duration;

use super::critical_timer::{create_queue, timer_change, wait_for_event};
use super::register_events;

/// Ident of the timeout timer on the watch's queue. The vnode watch uses the
/// directory's descriptor as its ident, which is never `usize::MAX`.
const TIMER_IDENT: libc::uintptr_t = libc::uintptr_t::MAX;

/// A kqueue that reports writes into one directory, with a critical timer for
/// bounded waits.
///
/// A publish that renames a new file over its target replaces the file inode,
/// so the directory is watched rather than either file. The kernel remembers a
/// change that happened before [`Self::wait`] was called, so a change racing
/// the waiter is not lost.
#[derive(Debug)]
pub struct DirectoryWatch {
    queue: OwnedFd,
    _directory: OwnedFd,
}

impl DirectoryWatch {
    /// Watch the directory open at `directory` through an event-only
    /// descriptor of its own, so replacing the directory's path afterwards
    /// cannot redirect the watch.
    ///
    /// # Errors
    ///
    /// Returns the OS error from opening the directory or registering the
    /// watch.
    pub fn new(directory: BorrowedFd<'_>) -> io::Result<Self> {
        let directory = crate::os::fd::open_at(
            directory,
            c".",
            libc::O_RDONLY
                | libc::O_DIRECTORY
                | libc::O_NOFOLLOW
                | libc::O_CLOEXEC
                | libc::O_EVTONLY,
            0,
        )?;
        let queue = create_queue()?;
        let registration = libc::kevent {
            ident: directory.as_raw_fd() as libc::uintptr_t,
            filter: libc::EVFILT_VNODE,
            flags: libc::EV_ADD | libc::EV_ENABLE | libc::EV_CLEAR,
            fflags: libc::NOTE_WRITE | libc::NOTE_EXTEND,
            data: 0,
            udata: std::ptr::null_mut(),
        };
        register_events(queue.as_fd(), &[registration])?;
        Ok(Self {
            queue,
            _directory: directory,
        })
    }

    /// Block until the directory changes or `timeout` passes, and say whether
    /// it changed. An interrupted wait returns early without a change.
    ///
    /// The timer rides on the directory's queue so one blocking call returns
    /// on whichever comes first, and it is a critical timer because a plain
    /// timeout is coalesced to ~16 ms steps on hosted runners. A one-shot
    /// timer that outlives a wake won by the directory is re-armed by the next
    /// wait; if it fires first it can only end a later wait early, and callers
    /// re-read and re-validate after every wake.
    ///
    /// # Errors
    ///
    /// Returns the OS error from `kevent`.
    pub fn wait(&self, timeout: Duration) -> io::Result<bool> {
        let event = wait_for_event(self.queue.as_fd(), &[timer_change(TIMER_IDENT, timeout)])?;
        Ok(event == Some(libc::EVFILT_VNODE))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_write_into_the_directory_wakes_the_wait_and_a_quiet_directory_times_out() {
        let root = tempfile::tempdir().unwrap();
        let directory = std::fs::File::open(root.path()).unwrap();
        let watch = DirectoryWatch::new(directory.as_fd()).unwrap();
        assert!(!watch.wait(Duration::from_millis(5)).unwrap());
        std::fs::write(root.path().join("published"), b"x").unwrap();
        assert!(watch.wait(Duration::from_secs(5)).unwrap());
    }

    #[test]
    fn descriptors_do_not_survive_an_exec() {
        let root = tempfile::tempdir().unwrap();
        let directory = std::fs::File::open(root.path()).unwrap();
        let watch = DirectoryWatch::new(directory.as_fd()).unwrap();
        for descriptor in [&watch.queue, &watch._directory] {
            assert!(crate::os::fd::is_close_on_exec(descriptor.as_fd()).unwrap());
        }
    }
}
