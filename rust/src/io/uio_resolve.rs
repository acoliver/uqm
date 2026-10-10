//! Where a virtual path is, in one repository's mount stack, right now.
//!
//! A directory handle holds only a virtual path and its repository. Every
//! question about what lies at that path (is it a directory, which mount
//! provides the file, which physical directory should a transplant copy) is
//! answered here against the registry the caller has locked, so a handle
//! cannot keep pointing at a host directory or archive that has since been
//! unmounted or replaced.
//!
//! The rules are those of the original `uio_getPhysicalAccess` and
//! `uio_verifyPath`: mounts that cover the path are tried in priority order,
//! a mount that lacks the object lets the lookup go on, and an object of the
//! wrong kind or a failure of the host ends it. Failures of the host or of an
//! archive are returned as the errno the host gave, never as "not found".

use std::fs;
use std::os::raw::c_int;
use std::path::{Path, PathBuf};

use super::{map_virtual_to_host_confined, MountInfo, UIO_FSTYPE_ZIP};

/// A mount of the repository that covers a virtual path, with the part of the
/// path below its mount point.
pub(super) struct Covering<'a> {
    pub(super) mount: &'a MountInfo,
    suffix: PathBuf,
}

/// The mounts of `repository` that cover `virtual_path`, highest priority
/// first. `virtual_path` is absolute and normalized.
pub(super) fn covering<'r, 'p>(
    registry: &'r [MountInfo],
    repository: usize,
    virtual_path: &'p Path,
) -> impl Iterator<Item = Covering<'r>> + 'p
where
    'r: 'p,
{
    debug_assert!(virtual_path.is_absolute(), "{virtual_path:?}");
    registry
        .iter()
        .filter(move |mount| mount.active_in_registry && mount.repository == repository)
        .filter_map(move |mount| {
            virtual_path
                .strip_prefix(&mount.mount_point)
                .ok()
                .map(|suffix| Covering {
                    mount,
                    suffix: suffix.to_path_buf(),
                })
        })
}

impl Covering<'_> {
    /// The path below the mount's directory on the host. For an archive mount
    /// this is only the lexical join; use [`Covering::place`] to look at what
    /// the mount holds.
    pub(super) fn host_path(&self) -> PathBuf {
        if self.suffix.as_os_str().is_empty() {
            self.mount.mounted_root.clone()
        } else {
            map_virtual_to_host_confined(&self.mount.mounted_root, &self.suffix)
        }
    }

    /// The name of the object inside the mount's archive, `None` when the
    /// path's bytes are not UTF-8: archive entry names are Unicode, so such a
    /// path names no entry.
    pub(super) fn archive_name(&self) -> Option<String> {
        self.mount.archive_name(&self.suffix)
    }

    /// Where the object lives: a host path for a directory mount, an entry
    /// name for an archive. `None` is an archive path that names no entry.
    fn place(&self) -> Option<Place> {
        if self.mount.fs_type == UIO_FSTYPE_ZIP {
            self.archive_name().map(Place::Archive)
        } else {
            Some(Place::Host(self.host_path()))
        }
    }
}

/// Where an object lives inside the mount that provides it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Place {
    /// A path on the host file system.
    Host(PathBuf),
    /// An entry name inside the mount's archive, with no leading slash. The
    /// empty name is the archive's own root.
    Archive(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Kind {
    Directory,
    File,
}

/// The mount that provides an object, where it lives there, and what it is.
pub(super) struct Located<'a> {
    pub(super) mount: &'a MountInfo,
    pub(super) place: Place,
    pub(super) kind: Kind,
}

/// What this one mount holds at the covered path. `Ok(None)` means nothing:
/// the lookup may go on to the next mount. `Err` is a failure of the host or
/// a path through something that is not a directory, and ends the lookup.
fn probe(covering: &Covering<'_>) -> Result<Option<(Place, Kind)>, c_int> {
    let Some(place) = covering.place() else {
        return Ok(None);
    };
    let kind = match &place {
        Place::Host(path) => match fs::metadata(path) {
            Ok(meta) if meta.is_dir() => Some(Kind::Directory),
            Ok(_) => Some(Kind::File),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => None,
            Err(err) => return Err(err.raw_os_error().unwrap_or(libc::EIO)),
        },
        Place::Archive(name) => archive_kind(covering.mount, name)?,
    };
    Ok(kind.map(|kind| (place, kind)))
}

fn archive_kind(mount: &MountInfo, name: &str) -> Result<Option<Kind>, c_int> {
    let index = mount
        .zip_index
        .as_ref()
        .expect("an archive mount is registered with its index");
    if name.is_empty() || index.is_directory(name) {
        return Ok(Some(Kind::Directory));
    }
    if index.get_entry(name).is_some() {
        return Ok(Some(Kind::File));
    }
    // "a/b" under an archive entry "a" that is a file is a path through a
    // non-directory, not a missing entry.
    let through_a_file = name
        .match_indices('/')
        .map(|(at, _)| &name[..at])
        .any(|ancestor| !index.is_directory(ancestor) && index.get_entry(ancestor).is_some());
    if through_a_file {
        Err(libc::ENOTDIR)
    } else {
        Ok(None)
    }
}

/// The first mount in priority order that holds an object at `virtual_path`.
/// ENOENT when none does.
pub(super) fn locate<'a>(
    registry: &'a [MountInfo],
    repository: usize,
    virtual_path: &Path,
) -> Result<Located<'a>, c_int> {
    for candidate in covering(registry, repository, virtual_path) {
        if let Some((place, kind)) = probe(&candidate)? {
            return Ok(Located {
                mount: candidate.mount,
                place,
                kind,
            });
        }
    }
    Err(libc::ENOENT)
}

/// `Ok` when `virtual_path` is a directory of the repository: the root, a
/// mount point or a directory above one, or a directory one of the covering
/// mounts holds. ENOENT if nothing is there, ENOTDIR if something that is not
/// a directory is.
pub(super) fn verify_directory(
    registry: &[MountInfo],
    repository: usize,
    virtual_path: &Path,
) -> Result<(), c_int> {
    if is_mount_tree_directory(registry, repository, virtual_path) {
        return Ok(());
    }
    match locate(registry, repository, virtual_path)?.kind {
        Kind::Directory => Ok(()),
        Kind::File => Err(libc::ENOTDIR),
    }
}

/// The root and every directory that is a mount point or lies on the way to
/// one exist whether or not a mount holds anything there.
fn is_mount_tree_directory(registry: &[MountInfo], repository: usize, virtual_path: &Path) -> bool {
    virtual_path == Path::new("/")
        || registry
            .iter()
            .filter(|mount| mount.active_in_registry && mount.repository == repository)
            .any(|mount| mount.mount_point.starts_with(virtual_path))
}

/// The physical directory `uio_transplantDir` copies: the first read-only
/// directory, in priority order, that the covering mounts hold at
/// `virtual_path`. A mount that holds a file there ends the search, like the
/// original. ENOENT if there is none.
pub(super) fn transplant_source<'a>(
    registry: &'a [MountInfo],
    repository: usize,
    virtual_path: &Path,
) -> Result<(&'a MountInfo, Place), c_int> {
    for candidate in covering(registry, repository, virtual_path) {
        match probe(&candidate)? {
            Some((place, Kind::Directory)) if candidate.mount.is_read_only() => {
                return Ok((candidate.mount, place));
            }
            Some((_, Kind::File)) => break,
            Some((_, Kind::Directory)) | None => {}
        }
    }
    Err(libc::ENOENT)
}
