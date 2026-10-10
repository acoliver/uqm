//! Expansion of a sealed content package into a scratch content directory.
//!
//! The game reads content through the first matching mount, so a packaged
//! content directory has to be expanded before the game can run on it. The
//! package stays the retained input; the expanded view is derived from it and
//! removed afterwards. The native acceptance route and the gameplay proof
//! runner both derive their view here.

use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fs;
use std::io::{self, Read as _, Seek as _, Write as _};
use std::path::{Path, PathBuf};

pub const MAX_CONTENT_ARCHIVE_ENTRIES: usize = 100_000;
const MAX_CONTENT_ARCHIVE_NAME_BYTES: u64 = 8 * 1024 * 1024;
pub const MAX_CONTENT_ARCHIVE_ENTRY_NAME_BYTES: usize = 4096;

#[derive(Clone, Copy)]
enum MaterializedEntryKind {
    File,
    Directory,
}

struct MaterializedEntry {
    relative_path: PathBuf,
    kind: MaterializedEntryKind,
}

pub struct MaterializedContent {
    root: std::os::fd::OwnedFd,
    entries: Vec<MaterializedEntry>,
    top_level: BTreeSet<std::ffi::OsString>,
}

impl MaterializedContent {
    /// Remove every path this materialization created, newest first.
    pub fn cleanup(&mut self) -> Result<(), String> {
        use std::os::fd::AsRawFd as _;

        while let Some(entry) = self.entries.pop() {
            let removal = (|| -> Result<(), String> {
                let (parent, filename) = open_relative_parent(&self.root, &entry.relative_path)?;
                let flags = match entry.kind {
                    MaterializedEntryKind::File => 0,
                    MaterializedEntryKind::Directory => libc::AT_REMOVEDIR,
                };
                if unsafe { libc::unlinkat(parent.as_raw_fd(), filename.as_ptr(), flags) } != 0 {
                    return Err(format!(
                        "remove materialized content {}: {}",
                        entry.relative_path.display(),
                        io::Error::last_os_error()
                    ));
                }
                Ok(())
            })();
            if let Err(error) = removal {
                self.entries.push(entry);
                return Err(error);
            }
        }
        Ok(())
    }

    pub fn is_removed(&self) -> Result<bool, String> {
        use std::os::fd::AsRawFd as _;
        use std::os::unix::ffi::OsStrExt as _;

        for name in &self.top_level {
            let name = std::ffi::CString::new(name.as_bytes())
                .map_err(|_| "materialized top-level name contains a NUL byte".to_string())?;
            let mut status = std::mem::MaybeUninit::<libc::stat>::uninit();
            if unsafe {
                libc::fstatat(
                    self.root.as_raw_fd(),
                    name.as_ptr(),
                    status.as_mut_ptr(),
                    libc::AT_SYMLINK_NOFOLLOW,
                )
            } == 0
            {
                return Ok(false);
            }
            if io::Error::last_os_error().raw_os_error() != Some(libc::ENOENT) {
                return Err(format!(
                    "inspect materialized content {}: {}",
                    name.to_string_lossy(),
                    io::Error::last_os_error()
                ));
            }
        }
        Ok(true)
    }
}

impl Drop for MaterializedContent {
    fn drop(&mut self) {
        let _ = self.cleanup();
    }
}

/// Identity a package must still have when it is opened for expansion.
pub struct PackageIdentity<'a> {
    pub byte_length: u64,
    pub sha256: &'a str,
}

/// Expand a content package into `content_root`.
///
/// Paths in `overrides` are retained loose files that already exist under the
/// root and win over the package, as a top mount does. They are neither
/// created nor counted as written, and `cleanup` never removes them.
pub fn materialize_content_package(
    package: &Path,
    content_root: &Path,
    budget_bytes: u64,
    expected: Option<PackageIdentity<'_>>,
    overrides: &BTreeSet<PathBuf>,
) -> Result<MaterializedContent, String> {
    #[cfg(unix)]
    use std::os::unix::fs::OpenOptionsExt as _;

    let mut options = fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    options.custom_flags(libc::O_CLOEXEC | libc::O_NOFOLLOW);
    let mut package_file = options
        .open(package)
        .map_err(|error| format!("open content package {}: {error}", package.display()))?;
    let metadata = package_file
        .metadata()
        .map_err(|error| format!("inspect content package {}: {error}", package.display()))?;
    if !metadata.is_file() {
        return Err(format!(
            "content package is not a regular file: {}",
            package.display()
        ));
    }
    if let Some(expected) = expected {
        let mut digest = Sha256::new();
        let mut buffer = [0_u8; 64 * 1024];
        loop {
            let read = package_file
                .read(&mut buffer)
                .map_err(|error| format!("hash content package {}: {error}", package.display()))?;
            if read == 0 {
                break;
            }
            digest.update(&buffer[..read]);
        }
        if metadata.len() != expected.byte_length
            || format!("{:x}", digest.finalize()) != expected.sha256
        {
            return Err("retained content package changed before archive parsing".to_string());
        }
        package_file
            .rewind()
            .map_err(|error| format!("rewind content package {}: {error}", package.display()))?;
    }
    let mut archive = zip::ZipArchive::new(package_file)
        .map_err(|error| format!("parse content package {}: {error}", package.display()))?;
    let root = open_directory_path_nofollow(content_root)?;
    let mut materialized = MaterializedContent {
        root,
        entries: Vec::new(),
        top_level: BTreeSet::new(),
    };
    let extraction = extract_archive(&mut archive, &mut materialized, budget_bytes, overrides);
    if let Err(error) = extraction {
        return match materialized.cleanup() {
            Ok(()) => Err(error),
            Err(cleanup) => Err(format!(
                "{error}; partial content cleanup failed: {cleanup}"
            )),
        };
    }
    Ok(materialized)
}

fn extract_archive(
    archive: &mut zip::ZipArchive<fs::File>,
    materialized: &mut MaterializedContent,
    budget_bytes: u64,
    overrides: &BTreeSet<PathBuf>,
) -> Result<(), String> {
    if archive.len() > MAX_CONTENT_ARCHIVE_ENTRIES {
        return Err(format!(
            "content package contains more than {MAX_CONTENT_ARCHIVE_ENTRIES} entries"
        ));
    }
    let mut declared_expanded = 0_u64;
    let mut written_expanded = 0_u64;
    let mut name_bytes = 0_u64;
    let mut seen = BTreeSet::new();
    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .map_err(|error| format!("read content package entry {index}: {error}"))?;
        let entry_name_bytes = entry.name_raw().len();
        if entry_name_bytes > MAX_CONTENT_ARCHIVE_ENTRY_NAME_BYTES {
            return Err(format!(
                "content package entry {index} has an overlong name"
            ));
        }
        name_bytes = name_bytes
            .checked_add(entry_name_bytes as u64)
            .ok_or_else(|| "content package entry-name length overflowed".to_string())?;
        if name_bytes > MAX_CONTENT_ARCHIVE_NAME_BYTES {
            return Err(format!(
                "content package entry names exceed {MAX_CONTENT_ARCHIVE_NAME_BYTES} bytes"
            ));
        }
        let relative = validate_archive_entry(index, &entry)?;
        if !seen.insert(relative.clone()) {
            return Err(format!(
                "content package entry {index} duplicates an earlier path"
            ));
        }
        declared_expanded = declared_expanded
            .checked_add(entry.size())
            .ok_or_else(|| "content package expanded length overflowed".to_string())?;
        if declared_expanded > budget_bytes {
            return Err(format!(
                "content package exceeds expansion budget {budget_bytes}"
            ));
        }
        let first = relative
            .components()
            .next()
            .ok_or_else(|| format!("content package entry {index} has an empty path"))?;
        materialized
            .top_level
            .insert(first.as_os_str().to_os_string());
        if entry.is_dir() {
            ensure_directories(materialized, &relative)?;
        } else if overrides.contains(&relative) {
            continue;
        } else {
            let expected = entry.size();
            extract_archive_file(
                materialized,
                &relative,
                &mut entry,
                expected,
                &mut written_expanded,
                budget_bytes,
            )?;
        }
    }
    Ok(())
}

fn validate_archive_entry(index: usize, entry: &zip::read::ZipFile<'_>) -> Result<PathBuf, String> {
    let relative = entry
        .enclosed_name()
        .ok_or_else(|| format!("content package entry {index} has an unsafe path"))?
        .to_path_buf();
    let Some(std::path::Component::Normal(first)) = relative.components().next() else {
        return Err(format!(
            "content package entry {index} has an empty or unsafe path"
        ));
    };
    if first == "version" || first == "packages" {
        return Err(format!(
            "content package entry {index} conflicts with retained input layout"
        ));
    }
    if let Some(mode) = entry.unix_mode() {
        let kind = u64::from(mode) & u64::from(libc::S_IFMT);
        let expected = if entry.is_dir() {
            u64::from(libc::S_IFDIR)
        } else {
            u64::from(libc::S_IFREG)
        };
        if kind != 0 && kind != expected {
            return Err(format!(
                "content package entry {index} has an unsupported or contradictory Unix file type"
            ));
        }
    }
    Ok(relative)
}

fn extract_archive_file<R: io::Read>(
    materialized: &mut MaterializedContent,
    relative: &Path,
    input: &mut R,
    expected: u64,
    written_expanded: &mut u64,
    budget_bytes: u64,
) -> Result<(), String> {
    let (parent, filename) = ensure_parent_directories(materialized, relative)?;
    let mut output = create_file_at(&parent, &filename, relative)?;
    materialized.entries.push(MaterializedEntry {
        relative_path: relative.to_path_buf(),
        kind: MaterializedEntryKind::File,
    });
    let mut buffer = [0_u8; 64 * 1024];
    let mut written = 0_u64;
    loop {
        let read = input
            .read(&mut buffer)
            .map_err(|error| format!("extract content entry {}: {error}", relative.display()))?;
        if read == 0 {
            break;
        }
        written = written
            .checked_add(read as u64)
            .ok_or_else(|| "content entry written length overflowed".to_string())?;
        *written_expanded = written_expanded
            .checked_add(read as u64)
            .ok_or_else(|| "content package written length overflowed".to_string())?;
        if written > expected || *written_expanded > budget_bytes {
            return Err(format!(
                "content entry {} exceeds the authority expansion budget",
                relative.display()
            ));
        }
        output
            .write_all(&buffer[..read])
            .map_err(|error| format!("extract content entry {}: {error}", relative.display()))?;
    }
    if written != expected {
        return Err(format!(
            "content entry {} length changed during extraction",
            relative.display()
        ));
    }
    use std::os::unix::fs::PermissionsExt as _;
    output
        .set_permissions(fs::Permissions::from_mode(0o640))
        .and_then(|()| output.sync_all())
        .map_err(|error| format!("publish content entry {}: {error}", relative.display()))
}

fn ensure_parent_directories(
    materialized: &mut MaterializedContent,
    relative: &Path,
) -> Result<(std::os::fd::OwnedFd, std::ffi::CString), String> {
    let parent = relative
        .parent()
        .ok_or_else(|| format!("content entry has no parent: {}", relative.display()))?;
    let directory = ensure_directories(materialized, parent)?;
    let filename = relative
        .file_name()
        .ok_or_else(|| format!("content entry has no filename: {}", relative.display()))?;
    Ok((directory, component_cstring(filename)?))
}

fn ensure_directories(
    materialized: &mut MaterializedContent,
    relative: &Path,
) -> Result<std::os::fd::OwnedFd, String> {
    use std::os::fd::AsRawFd as _;

    let mut current = duplicate_fd(&materialized.root)?;
    let mut traversed = PathBuf::new();
    for component in relative.components() {
        let std::path::Component::Normal(name) = component else {
            return Err(format!("unsafe content directory {}", relative.display()));
        };
        let name_c = component_cstring(name)?;
        traversed.push(name);
        let created = if unsafe { libc::mkdirat(current.as_raw_fd(), name_c.as_ptr(), 0o700) } == 0
        {
            materialized.entries.push(MaterializedEntry {
                relative_path: traversed.clone(),
                kind: MaterializedEntryKind::Directory,
            });
            true
        } else if io::Error::last_os_error().raw_os_error() == Some(libc::EEXIST) {
            false
        } else {
            return Err(format!(
                "create content directory {}: {}",
                traversed.display(),
                io::Error::last_os_error()
            ));
        };
        current = open_directory_at(&current, &name_c, &traversed)?;
        if created && unsafe { libc::fchmod(current.as_raw_fd(), 0o750) } != 0 {
            return Err(format!(
                "publish content directory {}: {}",
                traversed.display(),
                io::Error::last_os_error()
            ));
        }
    }
    Ok(current)
}

pub fn open_directory_path_nofollow(path: &Path) -> Result<std::os::fd::OwnedFd, String> {
    let resolved;
    let path = if path.is_absolute() {
        let parent = path
            .parent()
            .ok_or_else(|| format!("content root has no parent directory: {}", path.display()))?;
        let name = path
            .file_name()
            .ok_or_else(|| format!("content root has no directory name: {}", path.display()))?;
        resolved = fs::canonicalize(parent)
            .map_err(|error| format!("resolve content-root parent {}: {error}", parent.display()))?
            .join(name);
        resolved.as_path()
    } else {
        path
    };
    let mut current = if path.is_absolute() {
        open_directory_raw(c"/", Path::new("/"))?
    } else {
        open_directory_raw(c".", Path::new("."))?
    };
    for component in path.components() {
        match component {
            std::path::Component::RootDir | std::path::Component::CurDir => continue,
            std::path::Component::Normal(name) => {
                current = open_directory_at(&current, &component_cstring(name)?, path)?;
            }
            _ => return Err(format!("unsafe content root path: {}", path.display())),
        }
    }
    Ok(current)
}

fn open_relative_parent(
    root: &std::os::fd::OwnedFd,
    relative: &Path,
) -> Result<(std::os::fd::OwnedFd, std::ffi::CString), String> {
    let mut current = duplicate_fd(root)?;
    let parent = relative.parent().unwrap_or_else(|| Path::new(""));
    for component in parent.components() {
        let std::path::Component::Normal(name) = component else {
            return Err(format!("unsafe materialized path: {}", relative.display()));
        };
        current = open_directory_at(&current, &component_cstring(name)?, parent)?;
    }
    let filename = relative
        .file_name()
        .ok_or_else(|| format!("materialized path has no filename: {}", relative.display()))?;
    Ok((current, component_cstring(filename)?))
}

fn open_directory_raw(
    path: &std::ffi::CStr,
    display: &Path,
) -> Result<std::os::fd::OwnedFd, String> {
    use std::os::fd::FromRawFd as _;

    let fd = unsafe {
        libc::open(
            path.as_ptr(),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC | libc::O_NOFOLLOW,
        )
    };
    if fd < 0 {
        return Err(format!(
            "open content directory {}: {}",
            display.display(),
            io::Error::last_os_error()
        ));
    }
    Ok(unsafe { std::os::fd::OwnedFd::from_raw_fd(fd) })
}

fn open_directory_at(
    parent: &std::os::fd::OwnedFd,
    name: &std::ffi::CStr,
    display: &Path,
) -> Result<std::os::fd::OwnedFd, String> {
    use std::os::fd::{AsRawFd as _, FromRawFd as _};

    let fd = unsafe {
        libc::openat(
            parent.as_raw_fd(),
            name.as_ptr(),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC | libc::O_NOFOLLOW,
        )
    };
    if fd < 0 {
        return Err(format!(
            "open content directory {}: {}",
            display.display(),
            io::Error::last_os_error()
        ));
    }
    Ok(unsafe { std::os::fd::OwnedFd::from_raw_fd(fd) })
}

fn create_file_at(
    parent: &std::os::fd::OwnedFd,
    name: &std::ffi::CStr,
    display: &Path,
) -> Result<fs::File, String> {
    use std::os::fd::{AsRawFd as _, FromRawFd as _};

    let fd = unsafe {
        libc::openat(
            parent.as_raw_fd(),
            name.as_ptr(),
            libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL | libc::O_CLOEXEC | libc::O_NOFOLLOW,
            0o600,
        )
    };
    if fd < 0 {
        return Err(format!(
            "create content entry {}: {}",
            display.display(),
            io::Error::last_os_error()
        ));
    }
    Ok(unsafe { fs::File::from_raw_fd(fd) })
}

pub fn duplicate_fd(fd: &std::os::fd::OwnedFd) -> Result<std::os::fd::OwnedFd, String> {
    use std::os::fd::{AsRawFd as _, FromRawFd as _};

    let duplicate = unsafe { libc::fcntl(fd.as_raw_fd(), libc::F_DUPFD_CLOEXEC, 0) };
    if duplicate < 0 {
        return Err(format!(
            "duplicate content directory descriptor: {}",
            io::Error::last_os_error()
        ));
    }
    Ok(unsafe { std::os::fd::OwnedFd::from_raw_fd(duplicate) })
}

fn component_cstring(value: &std::ffi::OsStr) -> Result<std::ffi::CString, String> {
    use std::os::unix::ffi::OsStrExt as _;

    std::ffi::CString::new(value.as_bytes())
        .map_err(|_| "content path component contains a NUL byte".to_string())
}
