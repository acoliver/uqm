//! Field-by-field migration state at the synchronous game-thread boundary.

use super::parity::Fields;

mod custom;
mod elements;
mod live;
mod resources;
mod ships;

pub(super) fn value(fields: &mut Fields, prefix: &str, name: &str, value: impl std::fmt::Display) {
    fields.insert(format!("{prefix}.{name}"), value.to_string());
}

#[cfg(not(test))]
pub use live::checkpoint;
#[cfg(not(test))]
pub(super) use live::fighters;

#[cfg(test)]
pub fn checkpoint(_journal: &mut super::parity::Journal, _boundary: &str) -> std::io::Result<()> {
    Ok(())
}

#[cfg(test)]
fn test_directory() -> tempfile::TempDir {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../tmp/verify");
    std::fs::create_dir_all(root)
        .unwrap_or_else(|error| panic!("create state unit-test root: {error}"));
    tempfile::tempdir_in(root)
        .unwrap_or_else(|error| panic!("create state unit-test directory: {error}"))
}
