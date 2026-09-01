//! Bounded, no-follow file input and staging for CI control-plane data.

use std::ffi::CString;
use std::fs::File;
use std::io::{self, Read as _, Seek as _, SeekFrom, Write as _};
use std::os::fd::{AsRawFd as _, FromRawFd as _, OwnedFd};
use std::os::unix::ffi::OsStrExt as _;
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Component, Path};

pub const AUTHORITY_BOOTSTRAP_LIMIT_BYTES: u64 = 1_048_576;

#[cfg(target_os = "macos")]
fn platform_path(path: &Path) -> std::path::PathBuf {
    for (alias, physical) in [("/var", "/private/var"), ("/tmp", "/private/tmp")] {
        if let Ok(suffix) = path.strip_prefix(alias) {
            return Path::new(physical).join(suffix);
        }
    }
    path.to_path_buf()
}

#[cfg(not(target_os = "macos"))]
fn platform_path(path: &Path) -> std::path::PathBuf {
    path.to_path_buf()
}

fn open_parent_nofollow(path: &Path) -> Result<(OwnedFd, CString), String> {
    let platform_path = platform_path(path);
    let path = platform_path.as_path();
    let start = if path.is_absolute() { b"/" } else { b"." };
    let start = CString::new(start.as_slice()).expect("static path has no NUL");
    let descriptor = unsafe {
        libc::open(
            start.as_ptr(),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC | libc::O_NOFOLLOW,
        )
    };
    if descriptor < 0 {
        return Err(format!(
            "cannot open starting directory for {}: {}",
            path.display(),
            io::Error::last_os_error()
        ));
    }
    let mut directory = unsafe { OwnedFd::from_raw_fd(descriptor) };
    let components = path
        .components()
        .filter_map(|component| match component {
            Component::Normal(name) => Some(Ok(name)),
            Component::RootDir | Component::CurDir => None,
            Component::ParentDir | Component::Prefix(_) => Some(Err(format!(
                "{} contains a forbidden path component",
                path.display()
            ))),
        })
        .collect::<Result<Vec<_>, _>>()?;
    let (file_name, parents) = components
        .split_last()
        .ok_or_else(|| format!("{} does not name a file", path.display()))?;
    for parent in parents {
        let parent = CString::new(parent.as_bytes())
            .map_err(|_| format!("{} contains a NUL byte", path.display()))?;
        let next = unsafe {
            libc::openat(
                directory.as_raw_fd(),
                parent.as_ptr(),
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC | libc::O_NOFOLLOW,
            )
        };
        if next < 0 {
            return Err(format!(
                "cannot open {}: {}",
                path.display(),
                io::Error::last_os_error()
            ));
        }
        directory = unsafe { OwnedFd::from_raw_fd(next) };
    }
    let file_name = CString::new(file_name.as_bytes())
        .map_err(|_| format!("{} contains a NUL byte", path.display()))?;
    Ok((directory, file_name))
}

fn open_components_nofollow(path: &Path) -> Result<File, String> {
    let (directory, file_name) = open_parent_nofollow(path)?;
    let descriptor = unsafe {
        libc::openat(
            directory.as_raw_fd(),
            file_name.as_ptr(),
            libc::O_RDONLY | libc::O_CLOEXEC | libc::O_NOFOLLOW | libc::O_NONBLOCK,
        )
    };
    if descriptor < 0 {
        return Err(format!(
            "cannot open {}: {}",
            path.display(),
            io::Error::last_os_error()
        ));
    }
    Ok(unsafe { File::from_raw_fd(descriptor) })
}

pub fn validate_directory_nofollow(path: &Path) -> Result<(), String> {
    let (directory, file_name) = open_parent_nofollow(path)?;
    let descriptor = unsafe {
        libc::openat(
            directory.as_raw_fd(),
            file_name.as_ptr(),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_CLOEXEC | libc::O_NOFOLLOW,
        )
    };
    if descriptor < 0 {
        return Err(format!(
            "cannot open directory {} without following links: {}",
            path.display(),
            io::Error::last_os_error()
        ));
    }
    drop(unsafe { OwnedFd::from_raw_fd(descriptor) });
    Ok(())
}

pub fn open_regular_nofollow(path: &Path, limit: u64) -> Result<(File, u64), String> {
    let file = open_components_nofollow(path)?;
    let metadata = file
        .metadata()
        .map_err(|error| format!("cannot inspect {}: {error}", path.display()))?;
    if !metadata.is_file() {
        return Err(format!("{} is not a regular file", path.display()));
    }
    if metadata.len() > limit {
        return Err(format!(
            "{} exceeds its byte limit ({} > {limit})",
            path.display(),
            metadata.len()
        ));
    }
    Ok((file, metadata.len()))
}

pub fn read_regular_nofollow(path: &Path, limit: u64) -> Result<Vec<u8>, String> {
    let (mut file, expected) = open_regular_nofollow(path, limit)?;
    read_open_regular(&mut file, path, expected, limit)
}

pub fn read_open_regular(
    file: &mut File,
    path: &Path,
    expected: u64,
    limit: u64,
) -> Result<Vec<u8>, String> {
    if expected > limit {
        return Err(format!("{} exceeds its byte limit", path.display()));
    }
    file.seek(SeekFrom::Start(0))
        .map_err(|error| format!("cannot seek {}: {error}", path.display()))?;
    let capacity = usize::try_from(expected)
        .map_err(|_| format!("{} is too large for this host", path.display()))?;
    let read_limit = limit
        .checked_add(1)
        .ok_or_else(|| "file byte limit overflow".to_string())?;
    let mut bytes = Vec::with_capacity(capacity);
    file.take(read_limit)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    let final_length = file
        .metadata()
        .map_err(|error| format!("cannot re-inspect {}: {error}", path.display()))?
        .len();
    if bytes.len() as u64 != expected || final_length != expected {
        return Err(format!(
            "{} changed length while being read",
            path.display()
        ));
    }
    Ok(bytes)
}

pub fn copy_regular_nofollow(source: &Path, destination: &Path, limit: u64) -> Result<u64, String> {
    copy_regular_nofollow_mode(source, destination, limit, 0o666, None)
}

pub fn copy_group_readable_executable_nofollow(
    source: &Path,
    destination: &Path,
    limit: u64,
) -> Result<u64, String> {
    copy_regular_nofollow_mode(source, destination, limit, 0o550, Some(0o550))
}

fn copy_regular_nofollow_mode(
    source: &Path,
    destination: &Path,
    limit: u64,
    creation_mode: libc::mode_t,
    exact_mode: Option<u32>,
) -> Result<u64, String> {
    let (mut input, expected) = open_regular_nofollow(source, limit)?;
    let (destination_parent, destination_name) = open_parent_nofollow(destination)?;
    let descriptor = unsafe {
        libc::openat(
            destination_parent.as_raw_fd(),
            destination_name.as_ptr(),
            libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL | libc::O_CLOEXEC | libc::O_NOFOLLOW,
            creation_mode as libc::c_uint,
        )
    };
    if descriptor < 0 {
        return Err(format!(
            "cannot create {}: {}",
            destination.display(),
            io::Error::last_os_error()
        ));
    }
    let mut output = unsafe { File::from_raw_fd(descriptor) };
    let result = (|| {
        if let Some(mode) = exact_mode {
            if unsafe { libc::fchmod(output.as_raw_fd(), mode as libc::mode_t) } != 0 {
                return Err(format!(
                    "cannot set mode on {}: {}",
                    destination.display(),
                    io::Error::last_os_error()
                ));
            }
        }
        let copied = copy_open_regular(
            &mut input,
            &mut output,
            source,
            destination,
            expected,
            limit,
        )?;
        if let Some(mode) = exact_mode {
            let observed = output
                .metadata()
                .map_err(|error| format!("cannot inspect {}: {error}", destination.display()))?
                .permissions()
                .mode()
                & 0o7777;
            if observed != mode {
                return Err(format!(
                    "{} has mode {observed:#06o}, expected {mode:#06o}",
                    destination.display()
                ));
            }
        }
        Ok(copied)
    })();
    if result.is_err() {
        drop(output);
        unsafe {
            libc::unlinkat(destination_parent.as_raw_fd(), destination_name.as_ptr(), 0);
        }
    }
    result
}

pub fn write_group_readable_regular_nofollow(
    destination: &Path,
    bytes: &[u8],
    limit: u64,
) -> Result<(), String> {
    write_regular_nofollow_with_mode(destination, bytes, limit, 0o440)
}

fn write_regular_nofollow_with_mode(
    destination: &Path,
    bytes: &[u8],
    limit: u64,
    mode: u32,
) -> Result<(), String> {
    let length = u64::try_from(bytes.len())
        .map_err(|_| format!("{} is too large for this host", destination.display()))?;
    if length > limit {
        return Err(format!(
            "{} exceeds its byte limit ({length} > {limit})",
            destination.display()
        ));
    }
    let (destination_parent, destination_name) = open_parent_nofollow(destination)?;
    let descriptor = unsafe {
        libc::openat(
            destination_parent.as_raw_fd(),
            destination_name.as_ptr(),
            libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL | libc::O_CLOEXEC | libc::O_NOFOLLOW,
            mode,
        )
    };
    if descriptor < 0 {
        return Err(format!(
            "cannot create {}: {}",
            destination.display(),
            io::Error::last_os_error()
        ));
    }
    let mut output = unsafe { File::from_raw_fd(descriptor) };
    let result = (|| {
        if unsafe { libc::fchmod(output.as_raw_fd(), mode as libc::mode_t) } != 0 {
            return Err(format!(
                "cannot set mode on {}: {}",
                destination.display(),
                io::Error::last_os_error()
            ));
        }
        output
            .write_all(bytes)
            .and_then(|()| output.flush())
            .and_then(|()| output.sync_all())
            .map_err(|error| format!("cannot write {}: {error}", destination.display()))?;
        let metadata = output
            .metadata()
            .map_err(|error| format!("cannot inspect {}: {error}", destination.display()))?;
        if metadata.len() != length || metadata.permissions().mode() & 0o7777 != mode {
            return Err(format!(
                "{} did not retain its exact bounded regular-file identity",
                destination.display()
            ));
        }
        Ok(())
    })();
    if result.is_err() {
        drop(output);
        unsafe {
            libc::unlinkat(destination_parent.as_raw_fd(), destination_name.as_ptr(), 0);
        }
    }
    result
}

fn copy_open_regular(
    input: &mut File,
    output: &mut File,
    source: &Path,
    destination: &Path,
    expected: u64,
    limit: u64,
) -> Result<u64, String> {
    input
        .seek(SeekFrom::Start(0))
        .map_err(|error| format!("cannot seek {}: {error}", source.display()))?;
    let copied = io::copy(&mut input.take(limit.saturating_add(1)), &mut *output)
        .map_err(|error| format!("cannot copy {}: {error}", source.display()))?;
    let final_length = input
        .metadata()
        .map_err(|error| format!("cannot re-inspect {}: {error}", source.display()))?
        .len();
    if copied != expected || final_length != expected || copied > limit {
        return Err(format!(
            "{} changed length while being copied",
            source.display()
        ));
    }
    output
        .flush()
        .and_then(|()| output.sync_all())
        .map_err(|error| format!("cannot sync {}: {error}", destination.display()))?;
    Ok(copied)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;
    use std::fs;
    use std::os::unix::fs::symlink;

    #[test]
    fn bounded_read_rejects_oversize_symlink_and_fifo() {
        let temporary = tempfile::tempdir().unwrap();
        let root = fs::canonicalize(temporary.path()).unwrap();
        let source = root.join("source");
        fs::write(&source, b"abcd").unwrap();
        assert!(read_regular_nofollow(&source, 3).is_err());
        let link = root.join("link");
        symlink(&source, &link).unwrap();
        assert!(read_regular_nofollow(&link, 8).is_err());
        let fifo = root.join("fifo");
        let name = CString::new(fifo.as_os_str().as_encoded_bytes()).unwrap();
        assert_eq!(unsafe { libc::mkfifo(name.as_ptr(), 0o600) }, 0);
        assert!(read_regular_nofollow(&fifo, 8).is_err());
    }

    #[test]
    fn bounded_read_rejects_an_intermediate_symlink() {
        let temporary = tempfile::tempdir().unwrap();
        let root = fs::canonicalize(temporary.path()).unwrap();
        let actual = root.join("actual");
        fs::create_dir(&actual).unwrap();
        fs::write(actual.join("source"), b"abc").unwrap();
        let link = root.join("linked-directory");
        symlink(&actual, &link).unwrap();
        assert!(read_regular_nofollow(&link.join("source"), 8).is_err());
    }

    #[test]
    fn directory_validation_rejects_an_intermediate_symlink() {
        let temporary = tempfile::tempdir().unwrap();
        let root = fs::canonicalize(temporary.path()).unwrap();
        let actual = root.join("actual");
        let child = actual.join("child");
        fs::create_dir_all(&child).unwrap();
        let link = root.join("linked-directory");
        symlink(&actual, &link).unwrap();
        assert!(validate_directory_nofollow(&child).is_ok());
        assert!(validate_directory_nofollow(&link.join("child")).is_err());
    }

    #[test]
    fn bounded_copy_rejects_an_intermediate_destination_symlink() {
        let temporary = tempfile::tempdir().unwrap();
        let root = fs::canonicalize(temporary.path()).unwrap();
        let source = root.join("source");
        fs::write(&source, b"abc").unwrap();
        let actual = root.join("actual");
        fs::create_dir(&actual).unwrap();
        let link = root.join("linked-directory");
        symlink(&actual, &link).unwrap();
        assert!(copy_regular_nofollow(&source, &link.join("copy"), 8).is_err());
        assert!(!actual.join("copy").exists());
    }

    #[test]
    fn group_readable_executable_copy_publishes_exact_mode_and_can_be_spawned() {
        let temporary = tempfile::tempdir().unwrap();
        let root = fs::canonicalize(temporary.path()).unwrap();
        let source = root.join("source");
        let destination = root.join("destination");
        fs::write(&source, b"#!/bin/sh\nexit 0\n").unwrap();

        copy_group_readable_executable_nofollow(&source, &destination, 64).unwrap();

        assert_eq!(
            fs::metadata(&destination).unwrap().permissions().mode() & 0o7777,
            0o550
        );
        assert!(std::process::Command::new(&destination)
            .status()
            .unwrap()
            .success());
    }

    #[test]
    fn bounded_writer_publishes_read_only_exact_bytes() {
        let temporary = tempfile::tempdir().unwrap();
        let root = fs::canonicalize(temporary.path()).unwrap();
        let destination = root.join("destination");

        write_regular_nofollow_with_mode(&destination, b"manifest\n", 64, 0o400).unwrap();

        assert_eq!(fs::read(&destination).unwrap(), b"manifest\n");
        assert_eq!(
            fs::metadata(&destination).unwrap().permissions().mode() & 0o7777,
            0o400
        );
    }

    #[test]
    fn group_readable_writer_publishes_read_only_exact_bytes() {
        let temporary = tempfile::tempdir().unwrap();
        let root = fs::canonicalize(temporary.path()).unwrap();
        let destination = root.join("destination");

        write_group_readable_regular_nofollow(&destination, b"manifest\n", 64).unwrap();

        assert_eq!(fs::read(&destination).unwrap(), b"manifest\n");
        assert_eq!(
            fs::metadata(&destination).unwrap().permissions().mode() & 0o7777,
            0o440
        );
    }

    #[test]
    fn bounded_reader_rejects_growth_and_truncation() {
        let temporary = tempfile::tempdir().unwrap();
        let root = fs::canonicalize(temporary.path()).unwrap();
        let source = root.join("source");
        fs::write(&source, b"abc").unwrap();
        let (mut grown, expected) = open_regular_nofollow(&source, 8).unwrap();
        fs::write(&source, b"abcd").unwrap();
        assert!(read_open_regular(&mut grown, &source, expected, 8).is_err());
        let (mut truncated, expected) = open_regular_nofollow(&source, 8).unwrap();
        fs::write(&source, b"ab").unwrap();
        assert!(read_open_regular(&mut truncated, &source, expected, 8).is_err());
    }

    #[test]
    fn failed_bounded_copy_removes_partial_destination() {
        let temporary = tempfile::tempdir().unwrap();
        let root = fs::canonicalize(temporary.path()).unwrap();
        let source = root.join("source");
        let destination = root.join("destination");
        fs::write(&source, b"abcd").unwrap();
        assert!(copy_regular_nofollow(&source, &destination, 3).is_err());
        assert!(!destination.exists());
    }

    #[test]
    fn bounded_copy_does_not_remove_an_existing_destination() {
        let temporary = tempfile::tempdir().unwrap();
        let root = fs::canonicalize(temporary.path()).unwrap();
        let source = root.join("source");
        let destination = root.join("destination");
        fs::write(&source, b"new").unwrap();
        fs::write(&destination, b"incumbent").unwrap();
        assert!(copy_regular_nofollow(&source, &destination, 16).is_err());
        assert_eq!(fs::read(destination).unwrap(), b"incumbent");
    }
}
