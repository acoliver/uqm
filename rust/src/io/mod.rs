//! File and directory access: the UIO layer the game mounts its content
//! through, the zip reader behind it, and the two `USE_RUST_FILE` helpers
//! (`fileExists`, `copyFile`) exported from [`ffi`].

pub mod dirs;
pub mod ffi;
pub mod files;
mod name_filter;
pub mod uio_bridge;
pub mod zip_reader; // @plan PLAN-20260314-FILE-IO.P09 @requirement REQ-FIO-ARCHIVE-MOUNT
pub mod zip_stat;

pub use files::{copy_file, file_exists, FileError};
