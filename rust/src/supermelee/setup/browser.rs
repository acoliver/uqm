//! Owned saved-fleet browser state over the existing UIO directories.
use super::team::MeleeTeam;
use std::{
    ffi::{CStr, CString},
    io,
};
use uqm_engine::io::uio_owned::Directory;
use uqm_save::mle::Team as TeamRecord;

pub fn read_team(directory: &Directory, name: &CStr) -> io::Result<MeleeTeam> {
    TeamRecord::read_from(directory.open(name)?).map(MeleeTeam::from)
}

pub fn write_team(directory: &Directory, name: &CStr, team: &MeleeTeam) -> io::Result<()> {
    let mut stream = directory.create(name)?;
    let result = TeamRecord::from(team).write_to(&mut stream);
    let closed = stream.finish();
    match result.and(closed) {
        Ok(()) => Ok(()),
        Err(error) => {
            // The native save UI removes a partial file only after opening succeeded.
            let _ = directory.remove(name);
            Err(error)
        }
    }
}

#[derive(Debug)]
pub struct TeamBrowser {
    pub builtins: Vec<MeleeTeam>,
    pub entries: Vec<CString>,
    pub view: [MeleeTeam; 5],
    pub top: u16,
    pub bottom: u16,
    pub current: u16,
    pub next_flash: u32,
    pub highlighted: bool,
}

impl TeamBrowser {
    pub fn new(builtins: Vec<MeleeTeam>) -> Self {
        Self {
            builtins,
            entries: Vec::new(),
            view: std::array::from_fn(|_| MeleeTeam::new()),
            top: 0,
            bottom: 0,
            current: 0,
            next_flash: 0,
            highlighted: false,
        }
    }

    pub fn refresh(&mut self, directory: &Directory) {
        self.entries = directory.files_with_suffix(c".mle");
    }

    pub fn count(&self) -> u16 {
        (self.builtins.len() + self.entries.len()) as u16
    }

    pub fn file_index(&self, name: &CStr) -> Option<u16> {
        self.entries
            .iter()
            .position(|entry| entry.to_bytes().eq_ignore_ascii_case(name.to_bytes()))
            .map(|index| (self.builtins.len() + index) as u16)
    }

    pub fn team(&mut self, directory: &Directory, index: u16) -> Option<MeleeTeam> {
        if let Some(team) = self.builtins.get(usize::from(index)) {
            return Some(team.clone());
        }
        let first = usize::from(index) - self.builtins.len();
        let mut index = first;
        let mut loaded = None;
        while let Some(name) = self.entries.get(index) {
            match read_team(directory, name) {
                Ok(team) => {
                    loaded = Some(team);
                    break;
                }
                Err(_) => {
                    log::warn!(
                        "File '{}' is not a valid SuperMelee team",
                        name.to_string_lossy()
                    );
                    index += 1;
                }
            }
        }
        if index != first {
            self.entries.drain(first..index);
        }
        // Keep the native browser's post-compaction end test at this boundary.
        if index < self.entries.len() {
            loaded
        } else {
            None
        }
    }

    pub fn fill_view(&mut self, directory: &Directory) -> bool {
        let mut count = 0;
        while count < self.view.len() {
            let Some(team) = self.team(directory, self.top + count as u16) else {
                break;
            };
            self.view[count] = team;
            count += 1;
        }
        if count == 0 {
            return false;
        }
        self.bottom = self.top + count as u16;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::supermelee::types::MeleeShip;

    struct MountedDirectory {
        directory: Option<Directory>,
        repository: *mut crate::io::uio_bridge::uio_Repository,
        mount: *mut crate::io::uio_bridge::uio_MountHandle,
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
                crate::io::uio_bridge::uio_unmountDir(self.mount);
                crate::io::uio_bridge::uio_closeRepository(self.repository);
            }
        }
    }
    fn directory(scratch: &tempfile::TempDir) -> MountedDirectory {
        use crate::io::uio_bridge::*;
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
        let mut browser = TeamBrowser::new(super::super::persistence::builtin_teams());
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
