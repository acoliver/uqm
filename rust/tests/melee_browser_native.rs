//! Saved-fleet integration tests over a real mounted directory.
#![forbid(unsafe_code)]
#[cfg(test)]
mod tests {
    use std::io;
    use uqm_engine::io::{Dir, Placement, Vfs};
    use uqm_rust::supermelee::setup::browser::*;
    use uqm_rust::supermelee::setup::team::MeleeTeam;
    use uqm_rust::supermelee::types::MeleeShip;

    fn directory(scratch: &tempfile::TempDir) -> Dir {
        let vfs = Vfs::new();
        vfs.mount_dir("/", scratch.path(), Placement::Top, false);
        vfs.open_dir(std::path::Path::new("/"))
    }

    fn scratch() -> tempfile::TempDir {
        let root =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target/team-browser-tests");
        std::fs::create_dir_all(&root).unwrap();
        tempfile::TempDir::new_in(root).unwrap()
    }

    #[test]
    #[serial_test::serial]
    fn real_team_roundtrip_and_short_reads_release_the_stream() {
        let scratch = scratch();
        let directory = directory(&scratch);
        let mut team = MeleeTeam::new();
        team.ships[0] = MeleeShip::Chmmr;
        team.set_name("Test fleet");
        write_team(&directory, c"fleet.mle", &team).unwrap();
        let mut expected = [0; 69];
        expected[..14].fill(255);
        expected[0] = 3;
        expected[14..24].copy_from_slice(b"Test fleet");
        assert_eq!(
            std::fs::read(scratch.path().join("fleet.mle")).unwrap(),
            expected
        );
        assert_eq!(read_team(&directory, c"fleet.mle").unwrap(), team);
        std::fs::write(scratch.path().join("fleet.mle"), &expected[..68]).unwrap();
        assert_eq!(
            read_team(&directory, c"fleet.mle").unwrap_err().kind(),
            io::ErrorKind::UnexpectedEof
        );
        assert_eq!(team.ships[0], MeleeShip::Chmmr);
    }

    #[test]
    #[serial_test::serial]
    fn browser_owns_independent_view_values_and_matches_filenames_without_case() {
        let scratch = scratch();
        let directory = directory(&scratch);
        let mut browser = TeamBrowser::new(uqm_rust::supermelee::setup::builtin::builtin_teams());
        write_team(&directory, c"Fleet.mle", &browser.builtins[1]).unwrap();
        browser.refresh(&directory);
        assert_eq!(browser.count(), 16);
        assert_eq!(browser.file_index(c"fLEET.mle"), Some(15));
        assert!(browser.fill_view(&directory));
        assert_eq!(browser.bottom, 5);
        browser.view[0].ships[0] = MeleeShip::MeleeNone;
        assert_ne!(browser.view[0], browser.builtins[0]);
        browser.top = 15;
        assert!(browser.fill_view(&directory));
        assert_eq!(browser.view[0], browser.builtins[1]);
        assert_eq!(browser.bottom, 16);
        directory.remove_named(c"Fleet.mle").unwrap();
        assert!(!browser.fill_view(&directory));
        assert!(browser.entries.is_empty());
        assert_eq!(browser.file_index(c"Fleet.mle"), None);
    }
}
