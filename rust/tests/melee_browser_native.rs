//! Saved-fleet integration tests over a real mounted UIO directory.
#[cfg(test)]
mod tests {
    use std::{ffi::CString, io};
    use uqm_engine::io::uio_owned::Directory;
    use uqm_rust::supermelee::setup::browser::*;
    use uqm_rust::supermelee::setup::team::MeleeTeam;
    use uqm_rust::supermelee::types::MeleeShip;

    struct MountedDirectory {
        directory: Option<Directory>,
        repository: *mut uqm_engine::io::uio_bridge::uio_Repository,
        mount: *mut uqm_engine::io::uio_bridge::uio_MountHandle,
    }
    impl std::ops::Deref for MountedDirectory {
        type Target = Directory;
        fn deref(&self) -> &Directory {
            self.directory.as_ref().unwrap()
        }
    }
    impl Drop for MountedDirectory {
        fn drop(&mut self) {
            self.directory.take();
            // SAFETY: the fixture closes its retained directory before its mount and repository.
            unsafe {
                uqm_engine::io::uio_bridge::uio_unmountDir(self.mount);
                uqm_engine::io::uio_bridge::uio_closeRepository(self.repository);
            }
        }
    }
    fn directory(scratch: &tempfile::TempDir) -> MountedDirectory {
        use uqm_engine::io::uio_bridge::*;
        let path = CString::new(scratch.path().to_str().unwrap()).unwrap();
        // SAFETY: the fixture owns each mount, repository and directory until Drop.
        unsafe {
            let repository = uio_openRepository(0);
            let mount = uio_mountDir(
                repository,
                c"/".as_ptr(),
                1,
                std::ptr::null_mut(),
                std::ptr::null(),
                path.as_ptr(),
                std::ptr::null_mut(),
                4,
                std::ptr::null_mut(),
            );
            assert!(!mount.is_null());
            let raw = uio_openDir(repository, c"/".as_ptr(), 0);
            let directory = Directory::retain(std::ptr::NonNull::new(raw).unwrap());
            uio_closeDir(raw);
            MountedDirectory {
                directory: Some(directory),
                repository,
                mount,
            }
        }
    }

    fn scratch() -> tempfile::TempDir {
        let root =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target/team-browser-tests");
        std::fs::create_dir_all(&root).unwrap();
        tempfile::TempDir::new_in(root).unwrap()
    }

    #[test]
    #[serial_test::serial]
    fn real_uio_team_roundtrip_and_short_reads_release_the_stream() {
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
        let mut browser =
            TeamBrowser::new(uqm_rust::supermelee::setup::persistence::builtin_teams());
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
        directory.remove(c"Fleet.mle").unwrap();
        assert!(!browser.fill_view(&directory));
        assert!(browser.entries.is_empty());
        assert_eq!(browser.file_index(c"Fleet.mle"), None);
    }
}
