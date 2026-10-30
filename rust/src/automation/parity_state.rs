//! Field-by-field migration state at the synchronous game-thread boundary.

use super::parity::Fields;

mod custom;
#[cfg(not(test))]
pub(in crate::automation) use custom::phoenix_address;
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
pub(super) fn test_directory() -> tempfile::TempDir {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../tmp/verify");
    std::fs::create_dir_all(root)
        .unwrap_or_else(|error| panic!("create state unit-test root: {error}"));
    tempfile::tempdir_in(root)
        .unwrap_or_else(|error| panic!("create state unit-test directory: {error}"))
}

#[cfg(test)]
mod tests {
    #[test]
    fn unlinked_checkpoint_is_inert_and_does_not_write_a_fake_state() {
        let directory = super::test_directory();
        let mut journal = crate::automation::parity::Journal::create(directory.path()).unwrap();
        super::checkpoint(&mut journal, "unlinked").unwrap();
        journal.flush().unwrap();
        assert!(std::fs::read(directory.path().join("parity.trace"))
            .unwrap()
            .is_empty());
        assert!(std::fs::read(directory.path().join("parity.fields"))
            .unwrap()
            .is_empty());
    }
}
