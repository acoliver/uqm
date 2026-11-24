//! The virtual file system: one mount table behind `host::Files`, the retained
//! content, config, save and melee directories, and ZIP package reading.

pub mod vfs;
pub mod zip_reader;

pub use crate::host::FileStream as Stream;
pub use vfs::{matches_pattern, Dir, Match, MountId, Placement, Vfs};

#[cfg(any(test, feature = "test-support"))]
pub mod fixture;
#[cfg(test)]
mod qualification_tests;
#[cfg(test)]
mod vfs_stream_tests;
#[cfg(test)]
mod vfs_tests;

use parking_lot::Mutex;

/// The file system of this process. The application installs it at startup;
/// resource loaders and the save code read their directories from it.
static INSTALLED: Mutex<Option<Vfs>> = Mutex::new(None);

/// Make `vfs` the process file system, returning the one it replaces.
pub fn install(vfs: Vfs) -> Option<Vfs> {
    INSTALLED.lock().replace(vfs)
}

/// Remove and return the process file system.
pub fn uninstall() -> Option<Vfs> {
    INSTALLED.lock().take()
}

/// The process file system, if one is installed.
pub fn installed() -> Option<Vfs> {
    INSTALLED.lock().clone()
}

pub fn content_dir() -> Option<Dir> {
    installed()?.content_dir()
}

pub fn config_dir() -> Option<Dir> {
    installed()?.config_dir()
}

pub fn save_dir() -> Option<Dir> {
    installed()?.save_dir()
}

pub fn melee_dir() -> Option<Dir> {
    installed()?.melee_dir()
}
