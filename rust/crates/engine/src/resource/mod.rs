//! The resource index and the typed loaders that read through it.

mod color;
mod index;
pub mod propfile;
pub mod settings;

#[cfg(test)]
mod qualification_tests;

pub use index::{
    AssetKind, CatalogBuilder, Content, IndexSource, ResolveError, Resolved, Revision,
};

/// A resource name as game records hold it (C: `RESOURCE`); `None` is `NULL_RESOURCE`.
pub type ResourceKey = Option<&'static std::ffi::CStr>;
