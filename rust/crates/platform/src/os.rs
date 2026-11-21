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

pub mod errno;
pub mod sdl;
pub mod stderr;

#[cfg(unix)]
pub mod clock;
#[cfg(unix)]
pub mod descriptor;
#[cfg(unix)]
pub mod fd;
#[cfg(unix)]
pub mod fs;
#[cfg(unix)]
pub mod lock;
#[cfg(unix)]
pub mod process;
#[cfg(unix)]
pub mod signal;
