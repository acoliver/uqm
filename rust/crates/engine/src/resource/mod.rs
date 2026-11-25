//! The resource index and the typed loaders that read through it.

mod color;
mod index;
mod load_error;
pub mod propfile;
pub mod settings;

#[cfg(test)]
mod acquisition_tests;
#[cfg(test)]
mod qualification_tests;

pub use index::{
    AssetKind, CatalogBuilder, Content, IndexSource, ResolveError, Resolved, Revision,
};

pub use load_error::{AssetContext, AssetLoadError, AssetSource, Cause, FileError};

/// A resource name as game records hold it (C: `RESOURCE`); `None` is `NULL_RESOURCE`.
pub type ResourceKey = Option<&'static std::ffi::CStr>;
