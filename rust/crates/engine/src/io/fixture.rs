//! A temporary host directory mounted at `/` of a fresh process file system.

use super::{install, Dir, Placement, Vfs};
use std::path::{Path, PathBuf};
use tempfile::TempDir;

/// Installs a new [`Vfs`] with one temporary directory mounted at `/`, and
/// puts the previously installed file system back on drop.
pub struct TempMount {
    files: TempDir,
    vfs: Vfs,
    root: Dir,
    previous: Option<Vfs>,
}

impl TempMount {
    pub fn new() -> Self {
        Self::build(false)
    }

    /// As [`TempMount::new`], with the mount read-only like packaged content.
    pub fn read_only() -> Self {
        Self::build(true)
    }

    fn build(read_only: bool) -> Self {
        let files = TempDir::new().unwrap_or_else(|error| panic!("fixture directory: {error}"));
        let vfs = Vfs::new();
        vfs.mount_dir("/", files.path(), Placement::Top, read_only);
        let root = vfs.open_dir(Path::new("/"));
        let previous = install(vfs.clone());
        Self {
            files,
            vfs,
            root,
            previous,
        }
    }

    pub fn vfs(&self) -> &Vfs {
        &self.vfs
    }

    /// The directory mounted at `/`.
    pub fn dir(&self) -> Dir {
        self.root.clone()
    }

    /// Make the mounted directory the content directory.
    pub fn as_content(self) -> Self {
        self.vfs.set_content_dir(self.root.clone());
        self
    }

    pub fn write(&self, name: &str, bytes: &[u8]) {
        let path = self.files.path().join(name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .unwrap_or_else(|error| panic!("fixture parent {name}: {error}"));
        }
        std::fs::write(path, bytes).unwrap_or_else(|error| panic!("fixture file {name}: {error}"));
    }

    pub fn path_of(&self, name: &str) -> PathBuf {
        self.files.path().join(name)
    }
}

impl Default for TempMount {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for TempMount {
    fn drop(&mut self) {
        match self.previous.take() {
            Some(previous) => {
                install(previous);
            }
            None => {
                super::uninstall();
            }
        }
    }
}
