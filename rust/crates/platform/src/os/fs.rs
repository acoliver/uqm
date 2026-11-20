//! Directory-relative file-system calls and directory listing.
//!
//! These take a directory descriptor and a name so a caller that holds a
//! verified directory is not exposed to path replacement between calls.

use std::ffi::{c_int, CStr, OsString};
use std::io;
use std::os::fd::{AsRawFd as _, BorrowedFd};
use std::os::unix::ffi::OsStringExt as _;

use super::errno::{errno, set_errno};
use super::fd::duplicate_close_on_exec;

fn check(result: c_int) -> io::Result<()> {
    if result == 0 {
        Ok(())
    } else {
        Err(io::Error::last_os_error())
    }
}

/// Remove `name` below `directory`; `flags` is `0` for a file or
/// `libc::AT_REMOVEDIR` for an empty directory.
///
/// # Errors
///
/// Returns the OS error from `unlinkat`, for example `ENOENT`.
pub fn unlink_at(directory: BorrowedFd<'_>, name: &CStr, flags: c_int) -> io::Result<()> {
    // SAFETY: directory is open for the call and name is NUL-terminated.
    check(unsafe { libc::unlinkat(directory.as_raw_fd(), name.as_ptr(), flags) })
}

/// Create the directory `name` below `directory`.
///
/// # Errors
///
/// Returns the OS error from `mkdirat`, for example `EEXIST`.
pub fn make_directory_at(
    directory: BorrowedFd<'_>,
    name: &CStr,
    mode: libc::mode_t,
) -> io::Result<()> {
    // SAFETY: directory is open for the call and name is NUL-terminated.
    check(unsafe { libc::mkdirat(directory.as_raw_fd(), name.as_ptr(), mode) })
}

/// Give `source` below `source_directory` a second name `target` below
/// `target_directory`, without following a symlink at `source`.
///
/// # Errors
///
/// Returns the OS error from `linkat`, for example `EEXIST` when `target`
/// already names something.
pub fn link_at(
    source_directory: BorrowedFd<'_>,
    source: &CStr,
    target_directory: BorrowedFd<'_>,
    target: &CStr,
) -> io::Result<()> {
    // SAFETY: both directories are open for the call and both names are
    // NUL-terminated; flags 0 do not follow a symlink.
    check(unsafe {
        libc::linkat(
            source_directory.as_raw_fd(),
            source.as_ptr(),
            target_directory.as_raw_fd(),
            target.as_ptr(),
            0,
        )
    })
}

/// Rename `from` below `from_directory` to `to` below `to_directory`,
/// replacing an existing `to` atomically.
///
/// # Errors
///
/// Returns the OS error from `renameat`.
pub fn rename_at(
    from_directory: BorrowedFd<'_>,
    from: &CStr,
    to_directory: BorrowedFd<'_>,
    to: &CStr,
) -> io::Result<()> {
    // SAFETY: both directories are open for the call and both names are
    // NUL-terminated.
    check(unsafe {
        libc::renameat(
            from_directory.as_raw_fd(),
            from.as_ptr(),
            to_directory.as_raw_fd(),
            to.as_ptr(),
        )
    })
}

/// Set the permission bits of the open file or directory `fd`.
///
/// # Errors
///
/// Returns the OS error from `fchmod`.
pub fn change_mode(fd: BorrowedFd<'_>, mode: libc::mode_t) -> io::Result<()> {
    // SAFETY: fchmod takes an integer descriptor and never dereferences memory.
    check(unsafe { libc::fchmod(fd.as_raw_fd(), mode) })
}

/// Flush the open file or directory `fd` to stable storage.
///
/// # Errors
///
/// Returns the OS error from `fsync`.
pub fn sync(fd: BorrowedFd<'_>) -> io::Result<()> {
    // SAFETY: fsync takes an integer descriptor and never dereferences memory.
    check(unsafe { libc::fsync(fd.as_raw_fd()) })
}

/// Make the open directory `fd` the process working directory.
///
/// # Errors
///
/// Returns the OS error from `fchdir`, for example `ENOTDIR`.
pub fn change_directory(fd: BorrowedFd<'_>) -> io::Result<()> {
    // SAFETY: fchdir takes an integer descriptor and never dereferences memory.
    check(unsafe { libc::fchdir(fd.as_raw_fd()) })
}

/// Whether `name` exists below `directory`, not following a symlink.
///
/// # Errors
///
/// Returns the OS error from `fstatat` for any failure other than `ENOENT`.
pub fn exists_at_no_follow(directory: BorrowedFd<'_>, name: &CStr) -> io::Result<bool> {
    let mut status = std::mem::MaybeUninit::<libc::stat>::uninit();
    // SAFETY: directory is open and name is NUL-terminated; status is writable
    // stat storage the kernel fills on success and which is never read here.
    let result = unsafe {
        libc::fstatat(
            directory.as_raw_fd(),
            name.as_ptr(),
            status.as_mut_ptr(),
            libc::AT_SYMLINK_NOFOLLOW,
        )
    };
    if result == 0 {
        return Ok(true);
    }
    let error = io::Error::last_os_error();
    if error.raw_os_error() == Some(libc::ENOENT) {
        Ok(false)
    } else {
        Err(error)
    }
}

/// The entry names of a directory, `.` and `..` excluded, in directory order.
///
/// The stream owns a duplicate of the descriptor it was opened from, so the
/// caller's descriptor stays open when the stream closes. Both share one file
/// offset: the listing starts wherever the caller's descriptor is positioned.
pub struct DirectoryNames {
    stream: std::ptr::NonNull<libc::DIR>,
}

impl DirectoryNames {
    /// List the open directory `directory`.
    ///
    /// # Errors
    ///
    /// Returns the OS error from `fcntl` or `fdopendir`, for example `ENOTDIR`.
    pub fn open(directory: BorrowedFd<'_>) -> io::Result<Self> {
        use std::os::fd::IntoRawFd as _;

        // fdopendir takes ownership of its descriptor, so give it a duplicate.
        let duplicate = duplicate_close_on_exec(directory)?.into_raw_fd();
        // SAFETY: duplicate is an open descriptor that nothing else owns.
        let stream = unsafe { libc::fdopendir(duplicate) };
        match std::ptr::NonNull::new(stream) {
            Some(stream) => Ok(Self { stream }),
            None => {
                let error = io::Error::last_os_error();
                // SAFETY: fdopendir did not take ownership of the descriptor on failure.
                unsafe { libc::close(duplicate) };
                Err(error)
            }
        }
    }
}

impl Iterator for DirectoryNames {
    type Item = io::Result<OsString>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            // POSIX requires clearing errno before readdir to tell the end of
            // the directory from a failure.
            set_errno(0);
            // SAFETY: the stream is open and exclusively borrowed by this call.
            let entry = unsafe { libc::readdir(self.stream.as_ptr()) };
            if entry.is_null() {
                return match errno() {
                    0 => None,
                    code => Some(Err(io::Error::from_raw_os_error(code))),
                };
            }
            // SAFETY: d_name is NUL-terminated and valid until the next readdir.
            let name = unsafe { CStr::from_ptr((*entry).d_name.as_ptr()) }.to_bytes();
            if name != b"." && name != b".." {
                return Some(Ok(OsString::from_vec(name.to_vec())));
            }
        }
    }
}

impl Drop for DirectoryNames {
    fn drop(&mut self) {
        // SAFETY: the stream came from fdopendir and is closed exactly once.
        unsafe { libc::closedir(self.stream.as_ptr()) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;
    use std::os::fd::AsFd as _;

    fn name(text: &str) -> CString {
        CString::new(text).unwrap()
    }

    fn open_dir(path: &std::path::Path) -> std::fs::File {
        std::fs::File::open(path).unwrap()
    }

    #[test]
    fn unlink_removes_files_and_directories_and_reports_absent_names() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("file"), b"x").unwrap();
        std::fs::create_dir(root.path().join("dir")).unwrap();
        let directory = open_dir(root.path());
        unlink_at(directory.as_fd(), &name("file"), 0).unwrap();
        unlink_at(directory.as_fd(), &name("dir"), libc::AT_REMOVEDIR).unwrap();
        assert!(!root.path().join("file").exists());
        assert!(!root.path().join("dir").exists());
        let error = unlink_at(directory.as_fd(), &name("file"), 0).unwrap_err();
        assert_eq!(error.raw_os_error(), Some(libc::ENOENT));
    }

    #[test]
    fn unlinking_a_directory_as_a_file_or_a_full_directory_as_empty_fails() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir(root.path().join("dir")).unwrap();
        std::fs::write(root.path().join("dir/inner"), b"x").unwrap();
        let directory = open_dir(root.path());
        assert!(unlink_at(directory.as_fd(), &name("dir"), 0).is_err());
        let error = unlink_at(directory.as_fd(), &name("dir"), libc::AT_REMOVEDIR).unwrap_err();
        assert_eq!(error.raw_os_error(), Some(libc::ENOTEMPTY));
    }

    #[test]
    fn make_directory_creates_once() {
        let root = tempfile::tempdir().unwrap();
        let directory = open_dir(root.path());
        make_directory_at(directory.as_fd(), &name("made"), 0o700).unwrap();
        assert!(root.path().join("made").is_dir());
        let error = make_directory_at(directory.as_fd(), &name("made"), 0o700).unwrap_err();
        assert_eq!(error.raw_os_error(), Some(libc::EEXIST));
    }

    #[test]
    fn link_gives_a_second_name_and_refuses_to_clobber() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("source"), b"payload").unwrap();
        let directory = open_dir(root.path());
        link_at(
            directory.as_fd(),
            &name("source"),
            directory.as_fd(),
            &name("alias"),
        )
        .unwrap();
        assert_eq!(
            std::fs::read(root.path().join("alias")).unwrap(),
            b"payload"
        );
        let error = link_at(
            directory.as_fd(),
            &name("source"),
            directory.as_fd(),
            &name("alias"),
        )
        .unwrap_err();
        assert_eq!(error.raw_os_error(), Some(libc::EEXIST));
        let error = link_at(
            directory.as_fd(),
            &name("absent"),
            directory.as_fd(),
            &name("other"),
        )
        .unwrap_err();
        assert_eq!(error.raw_os_error(), Some(libc::ENOENT));
    }

    #[test]
    fn rename_replaces_the_destination_and_reports_a_missing_source() {
        let root = tempfile::tempdir().unwrap();
        std::fs::write(root.path().join("from"), b"new").unwrap();
        std::fs::write(root.path().join("to"), b"old").unwrap();
        let directory = open_dir(root.path());
        rename_at(
            directory.as_fd(),
            &name("from"),
            directory.as_fd(),
            &name("to"),
        )
        .unwrap();
        assert_eq!(std::fs::read(root.path().join("to")).unwrap(), b"new");
        let error = rename_at(
            directory.as_fd(),
            &name("from"),
            directory.as_fd(),
            &name("to"),
        )
        .unwrap_err();
        assert_eq!(error.raw_os_error(), Some(libc::ENOENT));
    }

    #[test]
    fn change_mode_and_sync_act_on_the_open_file() {
        use std::os::unix::fs::PermissionsExt as _;
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("file");
        let file = std::fs::File::create(&path).unwrap();
        change_mode(file.as_fd(), 0o640).unwrap();
        sync(file.as_fd()).unwrap();
        assert_eq!(
            std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o640
        );
    }

    #[test]
    fn change_mode_and_sync_reject_descriptors_that_cannot_take_them() {
        // A read-only descriptor on a pipe has no mode to change on every
        // platform and no stable storage to sync.
        let (reader, _writer) = std::io::pipe().unwrap();
        assert!(sync(reader.as_fd()).is_err());
    }

    #[test]
    fn change_directory_requires_a_directory() {
        let root = tempfile::tempdir().unwrap();
        let file = std::fs::File::create(root.path().join("plain")).unwrap();
        let error = change_directory(file.as_fd()).unwrap_err();
        assert_eq!(error.raw_os_error(), Some(libc::ENOTDIR));
    }

    #[test]
    fn exists_at_no_follow_sees_a_dangling_symlink_and_a_missing_name() {
        let root = tempfile::tempdir().unwrap();
        std::os::unix::fs::symlink("nowhere", root.path().join("dangling")).unwrap();
        let directory = open_dir(root.path());
        assert!(exists_at_no_follow(directory.as_fd(), &name("dangling")).unwrap());
        assert!(!exists_at_no_follow(directory.as_fd(), &name("missing")).unwrap());
    }

    #[test]
    fn exists_at_no_follow_reports_failures_other_than_absence() {
        let root = tempfile::tempdir().unwrap();
        let file = std::fs::File::create(root.path().join("plain")).unwrap();
        let error = exists_at_no_follow(file.as_fd(), &name("child")).unwrap_err();
        assert_eq!(error.raw_os_error(), Some(libc::ENOTDIR));
    }

    #[test]
    fn directory_names_lists_entries_without_dots_and_keeps_the_descriptor_usable() {
        let root = tempfile::tempdir().unwrap();
        for entry in ["b", "a", "c"] {
            std::fs::write(root.path().join(entry), b"").unwrap();
        }
        let directory = open_dir(root.path());
        let mut names = DirectoryNames::open(directory.as_fd())
            .unwrap()
            .collect::<io::Result<Vec<_>>>()
            .unwrap();
        names.sort();
        assert_eq!(names, ["a", "b", "c"]);
        // Closing the stream's duplicate leaves the caller's descriptor open.
        assert!(directory.metadata().unwrap().is_dir());
    }

    #[test]
    fn directory_names_requires_a_directory() {
        let root = tempfile::tempdir().unwrap();
        let file = std::fs::File::create(root.path().join("plain")).unwrap();
        let error = DirectoryNames::open(file.as_fd()).err().unwrap();
        assert_eq!(error.raw_os_error(), Some(libc::ENOTDIR));
    }
}
