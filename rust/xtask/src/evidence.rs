//! The directory a native run writes its results into, and reading it back.

use std::collections::BTreeMap;
use std::io;
use std::path::{Component, Path, PathBuf};

fn invalid(detail: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, detail.to_string())
}

/// A bundle path must stay inside the bundle: relative, normalized, no `..`.
fn bundle_path(root: &Path, relative: &str) -> io::Result<PathBuf> {
    let escapes = relative.is_empty()
        || Path::new(relative)
            .components()
            .any(|part| !matches!(part, Component::Normal(_)));
    if escapes {
        return Err(invalid("bundle path is not a normalized relative path"));
    }
    Ok(root.join(relative))
}

pub(crate) fn read_regular_relative(root: &Path, relative: &str) -> io::Result<Vec<u8>> {
    std::fs::read(bundle_path(root, relative)?)
}

#[derive(Debug, Clone)]
pub(crate) struct RegularFileSnapshot {
    pub relative_path: String,
    pub bytes: Vec<u8>,
}

/// Every regular file under `root`. Anything that is neither a file nor a
/// directory (a symlink, a socket) is an error.
pub(crate) fn native_regular_file_inventory(root: &Path) -> io::Result<Vec<RegularFileSnapshot>> {
    let mut files = BTreeMap::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        for entry in std::fs::read_dir(&directory)? {
            let path = entry?.path();
            let kind = std::fs::symlink_metadata(&path)?.file_type();
            if kind.is_dir() {
                pending.push(path);
            } else if kind.is_file() {
                let relative = path
                    .strip_prefix(root)
                    .ok()
                    .and_then(Path::to_str)
                    .ok_or_else(|| invalid("bundle path is not UTF-8"))?
                    .to_string();
                files.insert(relative, std::fs::read(&path)?);
            } else {
                return Err(invalid(
                    "bundle contains an entry that is not a regular file",
                ));
            }
        }
    }
    Ok(files
        .into_iter()
        .map(|(relative_path, bytes)| RegularFileSnapshot {
            relative_path,
            bytes,
        })
        .collect())
}

/// The bundle's contents as they were when it was opened.
pub(crate) struct EvidenceSnapshot {
    files: BTreeMap<String, Vec<u8>>,
}

impl EvidenceSnapshot {
    pub(crate) fn open_native(root: &Path) -> io::Result<Self> {
        let files = native_regular_file_inventory(root)?
            .into_iter()
            .map(|file| (file.relative_path, file.bytes))
            .collect();
        Ok(Self { files })
    }

    pub(crate) fn read(&self, relative: &str) -> io::Result<&[u8]> {
        self.files
            .get(relative)
            .map(Vec::as_slice)
            .ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))
    }

    pub(crate) fn files(&self) -> Vec<RegularFileSnapshot> {
        self.files
            .iter()
            .map(|(relative_path, bytes)| RegularFileSnapshot {
                relative_path: relative_path.clone(),
                bytes: bytes.clone(),
            })
            .collect()
    }
}

pub(crate) struct EvidencePublisher {
    root: PathBuf,
}

impl EvidencePublisher {
    pub(crate) fn open(root: &Path) -> io::Result<Self> {
        if !root.is_dir() {
            return Err(invalid("bundle root is not a directory"));
        }
        Ok(Self {
            root: root.to_path_buf(),
        })
    }

    /// Write a new file; an existing one is an error.
    pub(crate) fn create(&self, relative: &str, bytes: &[u8]) -> io::Result<()> {
        use std::io::Write;
        let path = bundle_path(&self.root, relative)?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)?
            .write_all(bytes)
    }

    /// Replace a file so a reader sees either the old or the new contents.
    pub(crate) fn replace(&self, relative: &str, bytes: &[u8]) -> io::Result<()> {
        let path = bundle_path(&self.root, relative)?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let staging = path.with_extension("tmp");
        std::fs::write(&staging, bytes)?;
        std::fs::rename(staging, path)
    }

    pub(crate) fn remove(&self, relative: &str) -> io::Result<()> {
        std::fs::remove_file(bundle_path(&self.root, relative)?)
    }
}
