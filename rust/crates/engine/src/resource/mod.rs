//! The resource index and the typed loaders that read through it.

pub mod cache;
mod color;
mod index;
pub mod propfile;
pub mod settings;

pub use index::{descriptor, has_key, init, kind, load_index, path_of, remove, string, uninit};

/// A resource name as game records hold it (C: `RESOURCE`).
pub type ResourceKey = *const std::ffi::c_char;
