//! The mount table behind `host::Files`, and the directory handles over it.
//!
//! Mounts overlay host directories and ZIP archives at virtual paths. Lower
//! `position` wins; a mount placed on top of all others gets the lowest
//! position. Directory handles keep the resolved host path next to the
//! virtual path they were opened with, so a handle stays usable after the
//! mount it came from is removed.

use crate::host::{File, FileStat, FileStream, Files};
use crate::io::zip_reader::{Disk, ZipEntryReader, ZipIndex};
use parking_lot::Mutex;
use std::collections::BTreeSet;
use std::ffi::{CStr, CString};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[cfg(test)]
#[path = "mount_owner_tests.rs"]
mod owner_tests;

/// Mount positions start here so `Top` insertions cannot underflow.
const ORIGIN_POSITION: usize = 1000;

/// Handle for removing a mount or placing another mount next to it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MountId(usize);

/// Where a mount sits in the lookup order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Placement {
    Top,
    Bottom,
    Above(MountId),
    Below(MountId),
}

/// How a directory listing compares names with its pattern.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Match {
    Literal,
    Prefix,
    Suffix,
    Substring,
    Regex,
}

#[derive(Clone)]
enum Backing {
    Directory,
    Archive(Arc<ZipIndex>),
}

#[derive(Clone)]
struct Mount {
    id: MountId,
    point: String,
    root: PathBuf,
    backing: Backing,
    position: usize,
    read_only: bool,
}

#[derive(Clone)]
struct MountTable {
    mounts: Vec<Mount>,
    next_id: usize,
    /// The host file system behind directory mounts and archives.
    disk: Disk,
}

struct Resolution<'a> {
    mount: &'a Mount,
    host_path: PathBuf,
}

/// Mounts plus the retained content, config, save and melee directories.
#[derive(Clone)]
pub struct Vfs {
    table: Arc<Mutex<Arc<MountTable>>>,
    dirs: Arc<Mutex<Retained>>,
}

#[derive(Default)]
struct Retained {
    content: Option<Dir>,
    config: Option<Dir>,
    save: Option<Dir>,
    melee: Option<Dir>,
    content_mount: Option<MountId>,
}

impl Default for Vfs {
    fn default() -> Self {
        Self::new()
    }
}

impl Vfs {
    /// A file system with no mounts over the platform's host files.
    pub fn new() -> Self {
        Self::with_disk(crate::host::platform().disk())
    }

    /// A run-local mount builder over an explicitly supplied host file system.
    pub fn with_disk(disk: Disk) -> Self {
        Self {
            table: Arc::new(Mutex::new(Arc::new(MountTable {
                mounts: Vec::new(),
                next_id: 0,
                disk,
            }))),
            dirs: Arc::default(),
        }
    }

    /// Mount host directory `host` at `point`.
    pub fn mount_dir(
        &self,
        point: &str,
        host: &Path,
        placement: Placement,
        read_only: bool,
    ) -> MountId {
        Arc::make_mut(&mut self.table.lock()).add(
            point,
            host.to_path_buf(),
            Backing::Directory,
            placement,
            read_only,
        )
    }

    /// Mount the archive `name` found in `source` at `point`.
    pub fn mount_zip(
        &self,
        point: &str,
        source: &Dir,
        name: &Path,
        placement: Placement,
        read_only: bool,
    ) -> io::Result<MountId> {
        let archive = normalize(&source.path, name);
        let disk = self.table.lock().disk.clone();
        let index = Arc::new(ZipIndex::new(disk, &archive)?);
        Ok(Arc::make_mut(&mut self.table.lock()).add(
            point,
            archive,
            Backing::Archive(index),
            placement,
            read_only,
        ))
    }

    /// Mount the host directory behind `source` at `point` as read-only content.
    pub fn transplant(&self, point: &str, source: &Dir, placement: Placement) -> MountId {
        Arc::make_mut(&mut self.table.lock()).add(
            point,
            source.path.clone(),
            Backing::Directory,
            placement,
            true,
        )
    }

    /// Remove one mount. Returns false when it was already gone.
    pub fn unmount(&self, id: MountId) -> bool {
        let mut table = self.table.lock();
        let table = Arc::make_mut(&mut table);
        let before = table.mounts.len();
        table.mounts.retain(|mount| mount.id != id);
        table.mounts.len() != before
    }

    /// Remove every mount. Removed IDs are never issued again.
    pub fn unmount_all(&self) {
        let mut table = self.table.lock();
        Arc::make_mut(&mut table).mounts.clear();
    }

    /// Open a virtual directory in this run's overlay. It need not exist yet.
    pub fn open_dir(&self, path: &Path) -> Dir {
        Dir::at(MountView::Building(self.table.clone()), path)
    }

    /// Admit an explicit host directory, independent of the virtual namespace.
    pub fn open_host_dir(&self, path: &Path) -> Dir {
        Dir {
            table: MountView::Ready(self.table.lock().clone()),
            path: path.to_path_buf(),
            virtual_path: path.to_path_buf(),
            write_read_only: Some(false),
            host: true,
        }
    }

    pub fn content_dir(&self) -> Option<Dir> {
        self.dirs.lock().content.clone()
    }
    pub fn config_dir(&self) -> Option<Dir> {
        self.dirs.lock().config.clone()
    }
    pub fn save_dir(&self) -> Option<Dir> {
        self.dirs.lock().save.clone()
    }
    pub fn melee_dir(&self) -> Option<Dir> {
        self.dirs.lock().melee.clone()
    }
    pub fn content_mount(&self) -> Option<MountId> {
        self.dirs.lock().content_mount
    }
    pub fn set_content_dir(&self, dir: Dir) {
        self.dirs.lock().content = Some(dir);
    }
    pub fn set_config_dir(&self, dir: Dir) {
        self.dirs.lock().config = Some(dir);
    }
    pub fn set_save_dir(&self, dir: Dir) {
        self.dirs.lock().save = Some(dir);
    }
    pub fn set_melee_dir(&self, dir: Dir) {
        self.dirs.lock().melee = Some(dir);
    }
    pub fn set_content_mount(&self, mount: MountId) {
        self.dirs.lock().content_mount = Some(mount);
    }

    /// Release the save, melee, content and config directories.
    pub fn release_dirs(&self) {
        let mut dirs = self.dirs.lock();
        dirs.save = None;
        dirs.melee = None;
        dirs.content = None;
        dirs.config = None;
    }
}

impl Files for Vfs {
    fn open_read(&self, path: &Path) -> io::Result<FileStream> {
        let root = Path::new("/");
        let table = self.table.lock().clone();
        table.open_at(root, root, path, false, false)
    }
    fn open_write(&self, path: &Path) -> io::Result<FileStream> {
        let root = Path::new("/");
        let table = self.table.lock().clone();
        table.open_at(root, root, path, true, false)
    }
    fn remove(&self, path: &Path) -> io::Result<()> {
        let root = Path::new("/");
        let table = self.table.lock().clone();
        table.remove_at(root, path)
    }
    fn list(&self, path: &Path) -> io::Result<Vec<String>> {
        let root = Path::new("/");
        let table = self.table.lock().clone();
        table.list_at(root, root, path, false)
    }
    fn stat(&self, path: &Path) -> io::Result<FileStat> {
        let root = Path::new("/");
        let table = self.table.lock().clone();
        table.stat_at(root, root, path, false)
    }
}

#[derive(Clone)]
enum MountView {
    Building(Arc<Mutex<Arc<MountTable>>>),
    Ready(Arc<MountTable>),
}

impl MountView {
    fn snapshot(&self) -> Arc<MountTable> {
        match self {
            Self::Building(table) => table.lock().clone(),
            Self::Ready(table) => table.clone(),
        }
    }
}

/// A retained directory: cheap to clone, valid after its mount is removed.
#[derive(Clone)]
pub struct Dir {
    table: MountView,
    path: PathBuf,
    virtual_path: PathBuf,
    write_read_only: Option<bool>,
    host: bool,
}

impl std::fmt::Debug for Dir {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Dir")
            .field("path", &self.path)
            .field("virtual_path", &self.virtual_path)
            .finish()
    }
}

impl Dir {
    /// Retain the current overlay revision, independent of later mount edits.
    pub fn snapshot(&self) -> Self {
        Self {
            table: MountView::Ready(self.table.snapshot()),
            ..self.clone()
        }
    }

    /// A package overlay private to this acquisition and its child directories.
    pub fn with_package(&self, point: &str, name: &Path) -> io::Result<Self> {
        assert!(!self.host, "package overlays require a virtual directory");
        let mut table = self.table.snapshot();
        let archive = name.to_path_buf();
        let index = Arc::new(ZipIndex::new(Arc::new(self.snapshot()), &archive)?);
        Arc::make_mut(&mut table).add(
            point,
            archive,
            Backing::Archive(index),
            Placement::Top,
            true,
        );
        Ok(Self {
            table: MountView::Ready(table),
            ..self.clone()
        })
    }

    fn at(view: MountView, path: &Path) -> Self {
        let table = view.snapshot();
        let resolved = table.resolve_dir(path);
        let virtual_path = normalize(Path::new("/"), path);
        let write_read_only = table.mounts.iter().find_map(|mount| {
            let suffix = virtual_path.strip_prefix(&mount.point).ok()?;
            (confine(&mount.root, suffix) == resolved)
                .then_some(mount.read_only || matches!(mount.backing, Backing::Archive(_)))
        });
        Self {
            table: view,
            path: resolved,
            virtual_path,
            write_read_only,
            host: false,
        }
    }

    /// A child directory in the same namespace; absolute virtual names stay virtual.
    pub fn open_relative(&self, path: &Path) -> Dir {
        if path.is_absolute() && !self.host {
            return Self::at(self.table.clone(), path);
        }
        Dir {
            table: self.table.clone(),
            path: normalize(&self.path, path),
            virtual_path: normalize(&self.virtual_path, path),
            write_read_only: self.write_read_only,
            host: self.host,
        }
    }

    /// Names in `path` matching `pattern`, across every mount covering it.
    /// An unreadable directory lists as empty.
    pub fn list_matching(&self, path: &Path, pattern: &str, kind: Match) -> Vec<String> {
        self.list(path)
            .unwrap_or_default()
            .into_iter()
            .filter(|name| matches_pattern(name, pattern, kind))
            .collect()
    }

    /// Regular, non-hidden files in `path` matching `pattern`, in listing order.
    pub fn files_matching(&self, path: &Path, pattern: &str, kind: Match) -> Vec<String> {
        self.list(path)
            .unwrap_or_default()
            .into_iter()
            .filter(|name| {
                !name.starts_with('.')
                    && matches_pattern(name, pattern, kind)
                    && self.stat(&path.join(name)).is_ok_and(|stat| stat.is_file)
            })
            .collect()
    }

    pub fn open(&self, name: &CStr) -> io::Result<FileStream> {
        self.open_read(c_path(name)?)
    }

    pub fn create(&self, name: &CStr) -> io::Result<FileStream> {
        self.open_write(c_path(name)?)
    }

    pub fn remove_named(&self, name: &CStr) -> io::Result<()> {
        self.remove(c_path(name)?)
    }

    /// Regular, non-hidden entries whose name contains `suffix`, in the same
    /// enumeration order as the native browser.
    pub fn files_with_suffix(&self, suffix: &CStr) -> Vec<CString> {
        self.files_matching(Path::new(""), &suffix.to_string_lossy(), Match::Substring)
            .into_iter()
            .map(|name| CString::new(name).unwrap_or_else(|error| panic!("listed name: {error}")))
            .collect()
    }
}

fn c_path(name: &CStr) -> io::Result<&Path> {
    name.to_str()
        .map(Path::new)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidInput, error))
}

impl Files for Dir {
    fn open_read(&self, path: &Path) -> io::Result<FileStream> {
        let table = self.table.snapshot();
        if self.host {
            return table.disk.open_read(&normalize(&self.path, path));
        }
        table.open_at(
            &self.path,
            &self.virtual_path,
            path,
            false,
            self.write_read_only.is_some(),
        )
    }
    fn open_write(&self, path: &Path) -> io::Result<FileStream> {
        let table = self.table.snapshot();
        match self.write_read_only {
            Some(true) => Err(io::Error::from(io::ErrorKind::PermissionDenied)),
            Some(false) => table.disk.open_write(&normalize(&self.path, path)),
            None => table.open_at(&self.path, &self.virtual_path, path, true, false),
        }
    }
    fn remove(&self, path: &Path) -> io::Result<()> {
        let table = self.table.snapshot();
        match self.write_read_only {
            Some(true) => Err(io::Error::from(io::ErrorKind::PermissionDenied)),
            Some(false) => table.disk.remove(&normalize(&self.path, path)),
            None => table.remove_at(&self.virtual_path, path),
        }
    }
    fn list(&self, path: &Path) -> io::Result<Vec<String>> {
        let table = self.table.snapshot();
        if self.host {
            return table.disk.list(&normalize(&self.path, path));
        }
        table.list_at(
            &self.path,
            &self.virtual_path,
            path,
            self.write_read_only.is_some(),
        )
    }
    fn stat(&self, path: &Path) -> io::Result<FileStat> {
        let table = self.table.snapshot();
        if self.host {
            return table.disk.stat(&normalize(&self.path, path));
        }
        table.stat_at(
            &self.path,
            &self.virtual_path,
            path,
            self.write_read_only.is_some(),
        )
    }
}

impl MountTable {
    fn add(
        &mut self,
        point: &str,
        root: PathBuf,
        backing: Backing,
        placement: Placement,
        read_only: bool,
    ) -> MountId {
        self.next_id += 1;
        let id = MountId(self.next_id);
        let position = match placement {
            Placement::Top => self
                .mounts
                .iter()
                .map(|mount| mount.position)
                .min()
                .unwrap_or(ORIGIN_POSITION)
                .saturating_sub(1),
            Placement::Bottom => self
                .mounts
                .iter()
                .map(|mount| mount.position)
                .max()
                .unwrap_or(ORIGIN_POSITION)
                .saturating_add(1),
            Placement::Above(relative) => self.position_of(relative).saturating_sub(1),
            Placement::Below(relative) => self.position_of(relative).saturating_add(1),
        };
        self.mounts.push(Mount {
            id,
            point: normalize(Path::new("/"), Path::new(point))
                .to_string_lossy()
                .into_owned(),
            root,
            backing,
            position,
            read_only,
        });
        // Priority order: position, then the longer (more specific) mount
        // point, then creation order.
        self.mounts.sort_by(|a, b| {
            a.position
                .cmp(&b.position)
                .then_with(|| b.point.len().cmp(&a.point.len()))
                .then_with(|| a.id.0.cmp(&b.id.0))
        });
        id
    }

    fn position_of(&self, id: MountId) -> usize {
        self.mounts
            .iter()
            .find(|mount| mount.id == id)
            .map_or(ORIGIN_POSITION, |mount| mount.position)
    }

    /// The topmost mount covering virtual path `path`, with its host path.
    fn resolve(&self, path: &Path) -> Option<Resolution<'_>> {
        let normalized = normalize(Path::new("/"), path);
        self.mounts.iter().find_map(|mount| {
            let suffix = normalized.strip_prefix(&mount.point).ok()?;
            Some(Resolution {
                mount,
                host_path: confine(&mount.root, suffix),
            })
        })
    }

    /// Capture the host backing of the top covering virtual mount.
    fn resolve_dir(&self, path: &Path) -> PathBuf {
        let normalized = normalize(Path::new("/"), path);
        self.mounts
            .iter()
            .find_map(|mount| {
                let suffix = normalized.strip_prefix(&mount.point).ok()?;
                Some(if suffix.as_os_str().is_empty() {
                    mount.root.clone()
                } else {
                    mount.root.join(suffix)
                })
            })
            .unwrap_or_else(|| path.to_path_buf())
    }

    fn open_at(
        &self,
        host: &Path,
        base: &Path,
        path: &Path,
        write: bool,
        retained: bool,
    ) -> io::Result<FileStream> {
        let virtual_path = normalize(base, path);
        if write {
            let target = self.write_path(&virtual_path)?;
            return self.disk.open_write(&target);
        }
        let mut covered = false;
        for mount in &self.mounts {
            let Ok(suffix) = virtual_path.strip_prefix(&mount.point) else {
                continue;
            };
            covered = true;
            let opened = match &mount.backing {
                Backing::Archive(index) => index
                    .open_entry(&suffix.to_string_lossy())
                    .map(FileStream::new),
                Backing::Directory => self.disk.open_read(&confine(&mount.root, suffix)),
            };
            match opened {
                Ok(file) => return Ok(file),
                Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
                Err(error) => return Err(error),
            }
        }
        if covered || !retained {
            return Err(io::Error::from(io::ErrorKind::NotFound));
        }
        self.disk.open_read(&normalize(host, path))
    }

    fn write_path(&self, virtual_path: &Path) -> io::Result<PathBuf> {
        let resolved = self.resolve(virtual_path);
        match resolved {
            Some(resolved)
                if resolved.mount.read_only
                    || matches!(resolved.mount.backing, Backing::Archive(_)) =>
            {
                Err(io::Error::from(io::ErrorKind::PermissionDenied))
            }
            Some(resolved) => Ok(resolved.host_path),
            None => Err(io::Error::from(io::ErrorKind::NotFound)),
        }
    }

    fn remove_at(&self, base: &Path, path: &Path) -> io::Result<()> {
        let virtual_path = normalize(base, path);
        self.disk.remove(&self.write_path(&virtual_path)?)
    }

    fn stat_at(
        &self,
        host: &Path,
        base: &Path,
        path: &Path,
        retained: bool,
    ) -> io::Result<FileStat> {
        let virtual_path = normalize(base, path);
        let mut covered = false;
        for mount in &self.mounts {
            let Ok(suffix) = virtual_path.strip_prefix(&mount.point) else {
                continue;
            };
            covered = true;
            match &mount.backing {
                Backing::Archive(zip) => {
                    let name = suffix.to_string_lossy();
                    if zip.is_directory(&name) {
                        return Ok(FileStat {
                            len: 0,
                            is_dir: true,
                            is_file: false,
                        });
                    }
                    if let Some(entry) = zip.get_entry(&name) {
                        return Ok(FileStat {
                            len: entry.uncompressed_size,
                            is_dir: false,
                            is_file: true,
                        });
                    }
                }
                Backing::Directory => {
                    if let Ok(stat) = self.disk.stat(&confine(&mount.root, suffix)) {
                        return Ok(stat);
                    }
                }
            }
        }
        if covered || !retained {
            return Err(io::Error::from(io::ErrorKind::NotFound));
        }
        self.disk.stat(&normalize(host, path))
    }

    fn list_at(
        &self,
        host: &Path,
        base: &Path,
        path: &Path,
        retained: bool,
    ) -> io::Result<Vec<String>> {
        let virtual_path = normalize(base, path);
        let mut covered = false;
        let mut seen = BTreeSet::new();
        let mut names = Vec::new();
        for mount in &self.mounts {
            let Ok(suffix) = virtual_path.strip_prefix(&mount.point) else {
                continue;
            };
            covered = true;
            let entries = match &mount.backing {
                Backing::Archive(zip) => zip.list_directory(&suffix.to_string_lossy()),
                Backing::Directory => match self.disk.list(&confine(&mount.root, suffix)) {
                    Ok(names) => names,
                    Err(_) => continue,
                },
            };
            for name in entries {
                if seen.insert(name.clone()) {
                    names.push(name);
                }
            }
        }
        if covered {
            Ok(names)
        } else if retained {
            self.disk.list(&normalize(host, path))
        } else {
            Err(io::Error::from(io::ErrorKind::NotFound))
        }
    }
}

/// Resolve `.`, `..` and repeated separators of `input` against `base`.
/// An empty `input` is `base`; `..` cannot rise above the root.
fn normalize(base: &Path, input: &Path) -> PathBuf {
    use std::path::Component;
    if input.as_os_str().is_empty() {
        return base.to_path_buf();
    }
    let combined = if input.is_absolute() {
        input.to_path_buf()
    } else {
        base.join(input)
    };
    let mut parts: Vec<Component> = Vec::new();
    for component in combined.components() {
        match component {
            Component::Prefix(_) | Component::RootDir => {
                parts.clear();
                parts.push(component);
            }
            Component::CurDir => {}
            Component::ParentDir => {
                if parts.len() > 1 {
                    parts.pop();
                }
            }
            Component::Normal(_) => parts.push(component),
        }
    }
    if parts.is_empty() || (parts.len() == 1 && matches!(parts[0], Component::RootDir)) {
        return PathBuf::from("/");
    }
    parts.into_iter().collect()
}

/// `root` joined with `relative`, where `..` cannot leave `root`.
fn confine(root: &Path, relative: &Path) -> PathBuf {
    use std::path::Component;
    let mut kept: Vec<Component> = Vec::new();
    for component in relative.components() {
        match component {
            Component::ParentDir => {
                kept.pop();
            }
            Component::Normal(_) => kept.push(component),
            _ => {}
        }
    }
    let mut result = root.to_path_buf();
    result.extend(kept);
    result
}

/// Compare `name` with `pattern`. An empty pattern matches everything and an
/// invalid regular expression matches nothing.
pub fn matches_pattern(name: &str, pattern: &str, kind: Match) -> bool {
    if pattern.is_empty() {
        return true;
    }
    match kind {
        Match::Literal => name == pattern,
        Match::Prefix => name.starts_with(pattern),
        Match::Suffix => name.ends_with(pattern),
        Match::Substring => name.contains(pattern),
        Match::Regex => match regex::Regex::new(pattern) {
            Ok(re) => re.is_match(name),
            Err(error) => {
                log::warn!("invalid directory pattern '{pattern}': {error}");
                false
            }
        },
    }
}

impl File for ZipEntryReader {
    fn len(&self) -> io::Result<u64> {
        Ok(self.size())
    }
}

impl Write for ZipEntryReader {
    fn write(&mut self, _: &[u8]) -> io::Result<usize> {
        Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "Cannot write to ZIP entry",
        ))
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
