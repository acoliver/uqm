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
