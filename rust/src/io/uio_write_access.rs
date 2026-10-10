//! Where a change to a virtual path lands in one repository's mount stack.
//!
//! This is the write half of `uio_getPhysicalAccess` and of the removal loops
//! of `uio_unlink` and `uio_rmdir`. Open, create, truncate, append, mkdir,
//! rename and the file location for writing all ask [`plan_write`], and
//! unlink and rmdir ask [`plan_removal`], so no operation picks a mount on its
//! own.
//!
//! The write location is the first writable mount the walk reached, not the
//! topmost mount. The walk goes through the covering mounts in priority order
//! and stops at the first mount that holds the entry, so a read-only mount
//! that holds it above every writable one means EPERM, and an entry in a
//! mount below the write location is copied up first.

use std::ffi::{CString, OsStr};
use std::fs::{self, DirBuilder, File, OpenOptions};
use std::io;
use std::os::raw::c_int;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::resolve::{covering, probe, Covering, Kind, Place};
use super::{MountInfo, O_TRUNC};
use crate::io::zip_reader::ZipIndex;

/// Permission bits of a copy of an archive entry; archives carry none.
const ARCHIVE_COPY_MODE: u32 = 0o644;

/// The mode `uio_makePath` gives the directories a copy-up makes; the umask
/// takes its share.
const PARENT_DIRECTORY_MODE: u32 = 0o777;

/// What the caller wants to do with the path.
#[derive(Clone, Copy)]
pub(super) struct WriteIntent {
    /// The entry may be made if it does not exist (`O_CREAT`).
    pub(super) create: bool,
    /// The entry must not exist yet (`O_CREAT | O_EXCL`).
    pub(super) exclusive: bool,
    /// What to do when the write location lacks the parent directories that
    /// another mount has.
    pub(super) parents: Parents,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Parents {
    /// Make them in the write location (`uio_open`, `uio_mkdir`).
    Make,
    /// Fail with ENOENT (`uio_rename`, which passes `uio_GPA_NOWRITE`).
    MustExist,
}

/// Where the bytes of an entry that has to be copied up come from.
pub(super) enum CopySource {
    Host(PathBuf),
    Entry(Arc<ZipIndex>, String),
}

/// The mount that receives the change and what has to happen first.
pub(super) struct WritePlan {
    pub(super) mount_id: usize,
    /// The address `uio_getFileLocation` reports for the mount.
    pub(super) mount_handle: usize,
    /// The host path of the entry in the write location.
    pub(super) target: PathBuf,
    /// A directory chain to make in the write location before `target`.
    missing_parent: Option<PathBuf>,
    /// An existing entry in another mount, copied up unless the open
    /// truncates it.
    copy_from: Option<CopySource>,
}

fn errno_of(err: &io::Error) -> c_int {
    err.raw_os_error().unwrap_or(libc::EIO)
}

/// Whether this mount holds the directory the entry would be made in.
fn holds_parent(item: &Covering<'_>) -> Result<bool, c_int> {
    match item.parent() {
        None => Ok(false),
        Some(parent) => Ok(matches!(probe(&parent)?, Some((_, Kind::Directory)))),
    }
}

/// Chooses the mount for a change at `virtual_path`.
///
/// Errors, in the order the original reaches them: EISDIR for a directory at
/// the path, the host's errno or ENOTDIR for a failure on the way, ENOENT when
/// no covering mount holds the entry or its parent directory, EEXIST for an
/// exclusive create of an existing entry, ENOENT for a missing entry without
/// `create`, EPERM when the walk reached no writable mount.
pub(super) fn plan_write(
    registry: &[MountInfo],
    repository: usize,
    virtual_path: &Path,
    intent: WriteIntent,
) -> Result<WritePlan, c_int> {
    let mut writable: Option<Covering<'_>> = None;
    let mut holder: Option<(Covering<'_>, Place)> = None;
    let mut parent_seen = false;
    for item in covering(registry, repository, virtual_path) {
        let probed = probe(&item)?;
        if matches!(probed, Some((_, Kind::Directory))) {
            return Err(libc::EISDIR);
        }
        if writable.is_none() && !item.mount.is_read_only() {
            writable = Some(item.clone());
        }
        match probed {
            Some((place, _)) => {
                holder = Some((item, place));
                break;
            }
            None => parent_seen |= holds_parent(&item)?,
        }
    }

    match (&holder, parent_seen) {
        (None, false) => return Err(libc::ENOENT),
        (Some(_), _) if intent.create && intent.exclusive => return Err(libc::EEXIST),
        (None, _) if !intent.create => return Err(libc::ENOENT),
        _ => {}
    }
    let writable = writable.ok_or(libc::EPERM)?;

    let in_place = matches!(&holder, Some((item, _)) if item.mount.id == writable.mount.id);
    let copy_from = match holder {
        Some((item, place)) if !in_place => Some(copy_source(&item, place)),
        _ => None,
    };
    // An entry in the write location has its directories there already.
    let missing_parent = if in_place {
        None
    } else {
        missing_parent(&writable, intent.parents)?
    };
    Ok(WritePlan {
        mount_id: writable.mount.id,
        mount_handle: writable.mount.handle_ptr,
        target: writable.host_path(),
        missing_parent,
        copy_from,
    })
}

impl WritePlan {
    /// Whether the host lets this process write at the target: the entry
    /// itself if the write location holds it, otherwise the nearest directory
    /// that exists, where the parents and the copy would be made.
    fn host_allows_write(&self) -> Result<(), c_int> {
        let nearest = self
            .target
            .ancestors()
            .find(|path| path.exists())
            .ok_or(libc::ENOENT)?;
        host_access_write(nearest)
    }
}

/// `access(path, W_OK)` as the host answers it for this process: the owner,
/// group and other classes, read-only file systems and all.
fn host_access_write(path: &Path) -> Result<(), c_int> {
    let path = CString::new(path.as_os_str().as_bytes()).map_err(|_| libc::EINVAL)?;
    // SAFETY: `path` is a valid NUL-terminated string for the whole call.
    if unsafe { libc::access(path.as_ptr(), libc::W_OK) } == 0 {
        Ok(())
    } else {
        Err(errno_of(&io::Error::last_os_error()))
    }
}

/// Whether a write to `virtual_path`, which exists, would succeed: the answer
/// of `uio_access(W_OK)`, which asks the planner `uio_open` asks and does
/// nothing. EACCES when the walk reaches no writable mount (the planner's
/// EPERM), or the host's errno for the write location. A directory is
/// writable when the first writable mount that holds it allows it.
pub(super) fn write_permission(
    registry: &[MountInfo],
    repository: usize,
    virtual_path: &Path,
) -> Result<(), c_int> {
    let intent = WriteIntent {
        create: false,
        exclusive: false,
        parents: Parents::Make,
    };
    match plan_write(registry, repository, virtual_path, intent) {
        Ok(plan) => plan.host_allows_write(),
        Err(libc::EPERM) => Err(libc::EACCES),
        Err(libc::EISDIR) => directory_write_permission(registry, repository, virtual_path),
        Err(errno) => Err(errno),
    }
}

fn directory_write_permission(
    registry: &[MountInfo],
    repository: usize,
    virtual_path: &Path,
) -> Result<(), c_int> {
    for item in covering(registry, repository, virtual_path) {
        match probe(&item)? {
            Some((Place::Host(path), Kind::Directory)) if !item.mount.is_read_only() => {
                return host_access_write(&path);
            }
            _ => {}
        }
    }
    Err(libc::EACCES)
}

fn copy_source(item: &Covering<'_>, place: Place) -> CopySource {
    match place {
        Place::Host(path) => CopySource::Host(path),
        Place::Archive(name) => CopySource::Entry(
            item.mount
                .zip_index
                .clone()
                .expect("an archive mount is registered with its index"),
            name,
        ),
    }
}

/// The parent directory of the entry in the write location when it has to be
/// made. A parent that is the mount's own root is never made.
fn missing_parent(writable: &Covering<'_>, parents: Parents) -> Result<Option<PathBuf>, c_int> {
    if writable.depth() < 2 {
        return Ok(None);
    }
    let parent = writable
        .parent()
        .expect("a path of two components has a parent");
    if probe(&parent)?.is_some() {
        return Ok(None);
    }
    match parents {
        Parents::Make => Ok(Some(parent.host_path())),
        Parents::MustExist => Err(libc::ENOENT),
    }
}

/// Makes the parent directories and copies the entry up, as `uio_open` does
/// before it opens the write location. Returns the flags to open with: a
/// truncating open copies nothing and drops `O_TRUNC`, as the original does.
pub(super) fn prepare_write(plan: &WritePlan, flags: c_int) -> Result<c_int, c_int> {
    if let Some(parent) = &plan.missing_parent {
        DirBuilder::new()
            .recursive(true)
            .mode(PARENT_DIRECTORY_MODE)
            .create(parent)
            .map_err(|err| errno_of(&err))?;
    }
    let Some(source) = &plan.copy_from else {
        return Ok(flags);
    };
    if flags & O_TRUNC != 0 {
        return Ok(flags & !O_TRUNC);
    }
    copy_up(source, &plan.target).map_err(|err| errno_of(&err))?;
    Ok(flags)
}

fn copy_up(source: &CopySource, target: &Path) -> io::Result<()> {
    let (mut reader, mode): (Box<dyn io::Read>, u32) = match source {
        CopySource::Host(path) => {
            let mode = fs::metadata(path)?.permissions().mode() & 0o777;
            (Box::new(File::open(path)?), mode)
        }
        CopySource::Entry(index, name) => (Box::new(index.open_entry(name)?), ARCHIVE_COPY_MODE),
    };
    let mut copy = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(mode)
        .open(target)?;
    io::copy(&mut reader, &mut copy).inspect_err(|_| {
        // A half-written copy would shadow the original; the failure itself
        // is what the caller gets.
        let _ = fs::remove_file(target);
    })?;
    Ok(())
}

/// One mount's entry at the path of an unlink or rmdir.
pub(super) struct Removal {
    pub(super) kind: Kind,
    /// The host path to delete; `None` in a read-only mount.
    target: Option<PathBuf>,
}

/// The entries `uio_unlink` and `uio_rmdir` visit: the entry in every
/// covering mount that holds the parent directory, in priority order. A mount
/// whose path to the parent runs through a file ends the search.
pub(super) fn plan_removal(
    registry: &[MountInfo],
    repository: usize,
    virtual_path: &Path,
) -> Result<Vec<Removal>, c_int> {
    let (Some(parent), Some(name)) = (virtual_path.parent(), virtual_path.file_name()) else {
        return Err(libc::ENOENT);
    };
    let mut removals = Vec::new();
    for item in covering(registry, repository, parent) {
        match probe(&item) {
            Ok(Some((_, Kind::Directory))) => {}
            Ok(None) => continue,
            Ok(Some((_, Kind::File))) | Err(libc::ENOTDIR) => break,
            Err(errno) => return Err(errno),
        }
        if let Some(removal) = removal_in(&item.child(name))? {
            removals.push(removal);
        }
    }
    Ok(removals)
}

fn removal_in(entry: &Covering<'_>) -> Result<Option<Removal>, c_int> {
    Ok(probe(entry)?.map(|(place, kind)| Removal {
        kind,
        target: match place {
            Place::Host(path) if !entry.mount.is_read_only() => Some(path),
            _ => None,
        },
    }))
}

/// Deletes `removals` in order, each of which must be a `wanted` entry. A
/// wrong kind (EISDIR for a file removal, ENOTDIR for a directory removal)
/// or a read-only mount (EROFS) ends it where it stands; no entry at all is
/// ENOENT.
pub(super) fn remove_entries(removals: Vec<Removal>, wanted: Kind) -> Result<(), c_int> {
    if removals.is_empty() {
        return Err(libc::ENOENT);
    }
    for removal in removals {
        if removal.kind != wanted {
            return Err(match wanted {
                Kind::File => libc::EISDIR,
                Kind::Directory => libc::ENOTDIR,
            });
        }
        let Some(target) = removal.target else {
            return Err(libc::EROFS);
        };
        match wanted {
            Kind::File => fs::remove_file(&target),
            Kind::Directory => fs::remove_dir(&target),
        }
        .map_err(|err| errno_of(&err))?;
    }
    Ok(())
}

/// Whether a request for `uio_unlink` or `uio_rmdir` names an entry. A path
/// that ends in `/` has an empty name, which the original refuses.
pub(super) fn names_an_entry(request: &OsStr) -> bool {
    use std::os::unix::ffi::OsStrExt;
    request.as_bytes().last().is_some_and(|&last| last != b'/')
}
