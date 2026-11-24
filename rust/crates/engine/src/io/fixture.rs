//! A temporary directory and its independent mount table.

use super::{Dir, Placement, Vfs};
use std::path::{Path, PathBuf};
use tempfile::TempDir;

/// Owns a new [`Vfs`] with one temporary directory mounted at `/`.
pub struct TempMount {
    files: TempDir,
    vfs: Vfs,
    root: Dir,
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
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../tmp");
        std::fs::create_dir_all(&root).unwrap();
        let files =
            TempDir::new_in(root).unwrap_or_else(|error| panic!("fixture directory: {error}"));
        let vfs = Vfs::new();
        vfs.mount_dir("/", files.path(), Placement::Top, read_only);
        let root = vfs.open_dir(Path::new("/"));
        Self { files, vfs, root }
    }

    pub fn vfs(&self) -> &Vfs {
        &self.vfs
    }

    /// The directory mounted at `/`.
    pub fn dir(&self) -> Dir {
        self.root.clone()
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

impl TempMount {
    pub fn content(&self, index: &std::ffi::CStr) -> crate::resource::Content {
        let mut builder = crate::resource::CatalogBuilder::new(self.dir());
        builder.import_index(
            &self.dir(),
            index,
            None,
            &mut Default::default(),
            &mut Default::default(),
        );
        builder.finish()
    }
}
