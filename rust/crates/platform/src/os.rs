//! Reviewed operating-system calls behind safe functions.
//!
//! Every `unsafe` block that talks to the kernel on behalf of the automation
//! supervisor and the C runtime error state lives in this module. Each function
//! validates nothing it cannot validate and documents the one reason its
//! `unsafe` block is sound; callers stay free of `unsafe`.
//!
//! Descriptors cross this boundary as [`std::os::fd::BorrowedFd`] and
//! [`std::os::fd::OwnedFd`]. The nested-group protocol inherits descriptors by
//! number through the environment, so its functions take `RawFd`.

#[cfg(unix)]
pub use libc::{
    pid_t as Pid, AT_REMOVEDIR, ECHILD, EEXIST, EOVERFLOW, EPROTO, ESRCH, ETIMEDOUT, O_CLOEXEC,
    O_CREAT, O_DIRECTORY, O_EXCL, O_NOFOLLOW, O_NONBLOCK, O_RDONLY, O_WRONLY, SIGINT, SIGKILL,
    SIGSEGV, SIGTERM, S_IFDIR, S_IFMT, S_IFREG,
};
/// The OS vocabulary that callers outside the platform layer name: process
/// IDs, signal and error numbers, `open` flags and file-type bits.
pub use libc::{EACCES, EINVAL, EIO, ENOSPC};

pub mod errno;
pub mod sdl;
pub mod stderr;

#[cfg(unix)]
pub use uqm_process_support::os::clock;
#[cfg(unix)]
pub use uqm_process_support::os::descriptor;
#[cfg(unix)]
pub mod fd;
#[cfg(unix)]
pub mod fs;
#[cfg(unix)]
pub use uqm_process_support::os::lock;
#[cfg(unix)]
pub mod process;
#[cfg(unix)]
pub mod signal;
