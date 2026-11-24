//! Safe kernel calls needed by process supervision, without game or multimedia dependencies.

pub use libc::{
    pid_t as Pid, EACCES, ECHILD, EIO, EOVERFLOW, EPROTO, ESRCH, ETIMEDOUT, SIGKILL, SIGTERM,
};

pub mod clock;
pub mod descriptor;
pub mod fd;
pub mod lock;
pub mod process;
