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
