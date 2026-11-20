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

pub use live::checkpoint;
pub(super) use live::fighters;

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
    #[serial_test::serial]
    fn checkpoint_records_the_supplied_context_once() {
        let directory = super::test_directory();
        let mut journal = crate::automation::parity::Journal::create(directory.path()).unwrap();
        let mut game = uqm_engine::Game::new();
        game.world.set_activity(0x0203);
        game.with_context(|cx| super::checkpoint(cx, &mut journal, "supplied"))
            .unwrap();
        journal.flush().unwrap();
        let trace = std::fs::read_to_string(directory.path().join("parity.trace")).unwrap();
        assert_eq!(
            trace
                .lines()
                .filter(|line| line.starts_with("S 1 515 "))
                .count(),
            1
        );
        let fields = std::fs::read_to_string(directory.path().join("parity.fields")).unwrap();
        assert!(fields.contains(r#""mode.boundary":"supplied""#));
    }
}
