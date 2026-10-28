//! Native socket operations. Windows owns SOCKET handles, not CRT descriptors.

#[cfg(unix)]
pub use libc::*;
#[cfg(unix)]
pub type SocketHandle = libc::c_int;
#[cfg(unix)]
pub const INVALID_SOCKET: SocketHandle = -1;

#[cfg(windows)]
mod windows;
#[cfg(windows)]
pub use windows::*;
