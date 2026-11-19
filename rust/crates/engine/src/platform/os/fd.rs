//! Descriptor flags, descriptor duplication and deadline-bounded transfers.

use std::ffi::{c_int, c_short, CStr};
use std::io;
use std::os::fd::{AsRawFd as _, BorrowedFd, FromRawFd as _, OwnedFd, RawFd};

use super::clock::monotonic_millis;

/// Clear close-on-exec on `fd`, so a child inherits it across `exec`.
///
/// Async-signal-safe: one `fcntl`.
///
/// # Errors
///
/// Returns the OS error, for example `EBADF` for a descriptor that is not open.
pub fn set_inheritable(fd: RawFd) -> io::Result<()> {
    set_descriptor_flags(fd, 0)
}

/// Set close-on-exec on `fd`, so `exec` closes it in a child.
///
/// Async-signal-safe: one `fcntl`.
///
/// # Errors
///
/// Returns the OS error, for example `EBADF` for a descriptor that is not open.
pub fn set_close_on_exec(fd: RawFd) -> io::Result<()> {
    set_descriptor_flags(fd, libc::FD_CLOEXEC)
}

fn set_descriptor_flags(fd: RawFd, flags: c_int) -> io::Result<()> {
    // SAFETY: F_SETFD takes an integer and never dereferences memory; a
    // descriptor that is not open is reported as EBADF.
    if unsafe { libc::fcntl(fd, libc::F_SETFD, flags) } == -1 {
        return Err(io::Error::last_os_error());
    }
    Ok(())
}

/// Whether `fd` is closed by `exec`.
///
/// # Errors
///
/// Returns the OS error from `fcntl`.
pub fn is_close_on_exec(fd: BorrowedFd<'_>) -> io::Result<bool> {
    // SAFETY: F_GETFD takes no argument and never dereferences memory.
    let flags = unsafe { libc::fcntl(fd.as_raw_fd(), libc::F_GETFD) };
    if flags < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(flags & libc::FD_CLOEXEC != 0)
}

/// A new close-on-exec descriptor for the same open file description as `fd`.
///
/// # Errors
///
/// Returns the OS error from `fcntl`, for example `EMFILE`.
pub fn duplicate_close_on_exec(fd: BorrowedFd<'_>) -> io::Result<OwnedFd> {
    // SAFETY: F_DUPFD_CLOEXEC takes an integer and never dereferences memory.
    let duplicate = unsafe { libc::fcntl(fd.as_raw_fd(), libc::F_DUPFD_CLOEXEC, 0) };
    if duplicate < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: fcntl returned a new descriptor that nothing else owns.
    Ok(unsafe { OwnedFd::from_raw_fd(duplicate) })
}

/// Open `path`; the flags decide access, `O_DIRECTORY` and symlink handling.
///
/// # Errors
///
/// Returns the OS error from `open`.
pub fn open(path: &CStr, flags: c_int) -> io::Result<OwnedFd> {
    // SAFETY: path is NUL-terminated and outlives the call.
    let fd = unsafe { libc::open(path.as_ptr(), flags) };
    owned_descriptor(fd)
}

/// Open `name` relative to the directory `directory`.
///
/// `mode` applies only when `flags` contains `O_CREAT`.
///
/// # Errors
///
/// Returns the OS error from `openat`, for example `ELOOP` when `O_NOFOLLOW`
/// meets a symlink.
pub fn open_at(
    directory: BorrowedFd<'_>,
    name: &CStr,
    flags: c_int,
    mode: libc::mode_t,
) -> io::Result<OwnedFd> {
    // SAFETY: directory is open for the call and name is NUL-terminated. The
    // variadic mode is promoted to c_uint as the C calling convention requires.
    let fd = unsafe {
        libc::openat(
            directory.as_raw_fd(),
            name.as_ptr(),
            flags,
            libc::c_uint::from(mode),
        )
    };
    owned_descriptor(fd)
}

fn owned_descriptor(fd: c_int) -> io::Result<OwnedFd> {
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: open and openat returned a new descriptor that nothing else owns.
    Ok(unsafe { OwnedFd::from_raw_fd(fd) })
}

/// Read exactly `buffer.len()` bytes from `fd`, waiting no later than the
/// monotonic `deadline_millis` (see [`monotonic_millis`]) for each chunk.
///
/// Async-signal-safe: no allocation and no lock.
///
/// # Errors
///
/// `ETIMEDOUT` when the deadline passes, `EPIPE` when the peer closed or the
/// descriptor reports an error, otherwise the OS error from `poll` or `read`.
pub fn read_exact_until(fd: RawFd, buffer: &mut [u8], deadline_millis: u64) -> io::Result<()> {
    let mut offset = 0;
    while offset < buffer.len() {
        wait_ready(fd, libc::POLLIN, deadline_millis)?;
        let chunk = &mut buffer[offset..];
        // SAFETY: chunk is writable for chunk.len() bytes.
        let result = unsafe { libc::read(fd, chunk.as_mut_ptr().cast(), chunk.len()) };
        offset += transferred(result)?;
    }
    Ok(())
}

/// Write all of `bytes` to `fd`, waiting no later than the monotonic
/// `deadline_millis` for each chunk.
///
/// Async-signal-safe: no allocation and no lock.
///
/// # Errors
///
/// As for [`read_exact_until`].
pub fn write_all_until(fd: RawFd, bytes: &[u8], deadline_millis: u64) -> io::Result<()> {
    let mut offset = 0;
    while offset < bytes.len() {
        wait_ready(fd, libc::POLLOUT, deadline_millis)?;
        let chunk = &bytes[offset..];
        // SAFETY: chunk is readable for chunk.len() bytes.
        let result = unsafe { libc::write(fd, chunk.as_ptr().cast(), chunk.len()) };
        offset += transferred(result)?;
    }
    Ok(())
}

/// Bytes moved by one `read` or `write`; zero when it was interrupted and must
/// be retried. Must run before anything else can change `errno`.
fn transferred(result: isize) -> io::Result<usize> {
    match usize::try_from(result) {
        Ok(0) => Err(io::Error::from_raw_os_error(libc::EPIPE)),
        Ok(count) => Ok(count),
        Err(_) => {
            let error = io::Error::last_os_error();
            if error.kind() == io::ErrorKind::Interrupted {
                Ok(0)
            } else {
                Err(error)
            }
        }
    }
}

fn wait_ready(fd: RawFd, events: c_short, deadline_millis: u64) -> io::Result<()> {
    loop {
        let now = monotonic_millis()?;
        if now >= deadline_millis {
            return Err(io::Error::from_raw_os_error(libc::ETIMEDOUT));
        }
        let remaining = i32::try_from(deadline_millis - now).unwrap_or(i32::MAX);
        let mut descriptor = libc::pollfd {
            fd,
            events,
            revents: 0,
        };
        // SAFETY: descriptor points to one initialized pollfd.
        let ready = unsafe { libc::poll(&raw mut descriptor, 1, remaining) };
        if ready == 0 {
            return Err(io::Error::from_raw_os_error(libc::ETIMEDOUT));
        }
        if ready == -1 {
            let error = io::Error::last_os_error();
            if error.kind() == io::ErrorKind::Interrupted {
                continue;
            }
            return Err(error);
        }
        if descriptor.revents & (libc::POLLERR | libc::POLLNVAL) != 0
            || descriptor.revents & events == 0
        {
            return Err(io::Error::from_raw_os_error(libc::EPIPE));
        }
        return Ok(());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;
    use std::io::{Read as _, Write as _};
    use std::os::fd::AsFd as _;

    fn deadline_in(millis: u64) -> u64 {
        monotonic_millis().unwrap() + millis
    }

    #[test]
    fn bytes_round_trip_through_a_pipe() {
        let (reader, mut writer) = std::io::pipe().unwrap();
        write_all_until(
            std::os::fd::AsRawFd::as_raw_fd(&writer),
            b"abc",
            deadline_in(1_000),
        )
        .unwrap();
        writer.flush().unwrap();
        let mut bytes = [0_u8; 3];
        read_exact_until(
            std::os::fd::AsRawFd::as_raw_fd(&reader),
            &mut bytes,
            deadline_in(1_000),
        )
        .unwrap();
        assert_eq!(&bytes, b"abc");
    }

    #[test]
    fn reading_an_idle_pipe_times_out() {
        let (reader, _writer) = std::io::pipe().unwrap();
        let mut byte = [0_u8; 1];
        let error = read_exact_until(
            std::os::fd::AsRawFd::as_raw_fd(&reader),
            &mut byte,
            deadline_in(20),
        )
        .unwrap_err();
        assert_eq!(error.raw_os_error(), Some(libc::ETIMEDOUT));
    }

    #[test]
    fn a_deadline_already_in_the_past_times_out_without_polling() {
        let (reader, _writer) = std::io::pipe().unwrap();
        let mut byte = [0_u8; 1];
        let error =
            read_exact_until(std::os::fd::AsRawFd::as_raw_fd(&reader), &mut byte, 0).unwrap_err();
        assert_eq!(error.raw_os_error(), Some(libc::ETIMEDOUT));
    }

    #[test]
    fn an_empty_transfer_needs_no_descriptor_and_no_time() {
        read_exact_until(-1, &mut [], 0).unwrap();
        write_all_until(-1, &[], 0).unwrap();
    }

    #[test]
    fn reading_a_closed_peer_is_a_broken_pipe() {
        let (reader, writer) = std::io::pipe().unwrap();
        drop(writer);
        let mut byte = [0_u8; 1];
        let error = read_exact_until(
            std::os::fd::AsRawFd::as_raw_fd(&reader),
            &mut byte,
            deadline_in(1_000),
        )
        .unwrap_err();
        assert_eq!(error.raw_os_error(), Some(libc::EPIPE));
    }

    #[test]
    fn writing_to_a_closed_peer_is_a_broken_pipe() {
        let (reader, writer) = std::io::pipe().unwrap();
        drop(reader);
        let error = write_all_until(
            std::os::fd::AsRawFd::as_raw_fd(&writer),
            b"x",
            deadline_in(1_000),
        )
        .unwrap_err();
        assert_eq!(error.raw_os_error(), Some(libc::EPIPE));
    }

    #[test]
    fn a_full_pipe_stops_a_write_at_the_deadline() {
        let (_reader, writer) = std::io::pipe().unwrap();
        let raw = std::os::fd::AsRawFd::as_raw_fd(&writer);
        // One byte at a time never blocks inside write: poll reports the pipe
        // writable only while it has room, and the first refusal is the timeout.
        let error = loop {
            if let Err(error) = write_all_until(raw, &[0], deadline_in(30)) {
                break error;
            }
        };
        assert_eq!(error.raw_os_error(), Some(libc::ETIMEDOUT));
    }

    #[test]
    fn close_on_exec_can_be_cleared_and_set() {
        let (reader, _writer) = std::io::pipe().unwrap();
        let raw = std::os::fd::AsRawFd::as_raw_fd(&reader);
        set_inheritable(raw).unwrap();
        assert!(!is_close_on_exec(reader.as_fd()).unwrap());
        set_close_on_exec(raw).unwrap();
        assert!(is_close_on_exec(reader.as_fd()).unwrap());
    }

    #[test]
    fn descriptor_flags_reject_a_descriptor_that_is_not_open() {
        assert_eq!(
            set_inheritable(-1).unwrap_err().raw_os_error(),
            Some(libc::EBADF)
        );
        assert_eq!(
            set_close_on_exec(-1).unwrap_err().raw_os_error(),
            Some(libc::EBADF)
        );
    }

    #[test]
    fn a_duplicate_is_close_on_exec_and_shares_the_file() {
        let (mut reader, mut writer) = std::io::pipe().unwrap();
        let duplicate = duplicate_close_on_exec(reader.as_fd()).unwrap();
        assert!(is_close_on_exec(duplicate.as_fd()).unwrap());
        writer.write_all(b"z").unwrap();
        let mut file = std::fs::File::from(duplicate);
        let mut byte = [0_u8; 1];
        file.read_exact(&mut byte).unwrap();
        assert_eq!(&byte, b"z");
        let _ = &mut reader;
    }

    #[test]
    fn open_reports_missing_files_and_refuses_symlinks_when_asked() {
        let root = tempfile::tempdir().unwrap();
        let missing = CString::new(root.path().join("absent").to_str().unwrap()).unwrap();
        let error = open(&missing, libc::O_RDONLY).unwrap_err();
        assert_eq!(error.raw_os_error(), Some(libc::ENOENT));

        let target = root.path().join("target");
        std::fs::write(&target, b"x").unwrap();
        let link = root.path().join("link");
        std::os::unix::fs::symlink(&target, &link).unwrap();
        let link = CString::new(link.to_str().unwrap()).unwrap();
        assert!(open(&link, libc::O_RDONLY).is_ok());
        let error = open(&link, libc::O_RDONLY | libc::O_NOFOLLOW).unwrap_err();
        assert!(matches!(
            error.raw_os_error(),
            Some(libc::ELOOP) | Some(libc::EMLINK)
        ));
    }

    #[test]
    fn open_at_creates_with_the_requested_mode_and_rejects_duplicates() {
        use std::os::unix::fs::PermissionsExt as _;
        let root = tempfile::tempdir().unwrap();
        let directory = std::fs::File::open(root.path()).unwrap();
        let name = CString::new("created").unwrap();
        let flags = libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL | libc::O_CLOEXEC;
        let created = open_at(directory.as_fd(), &name, flags, 0o600).unwrap();
        let mode = std::fs::File::from(created)
            .metadata()
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o600);
        let error = open_at(directory.as_fd(), &name, flags, 0o600).unwrap_err();
        assert_eq!(error.raw_os_error(), Some(libc::EEXIST));
    }

    #[test]
    fn open_at_below_a_regular_file_is_not_a_directory() {
        let root = tempfile::tempdir().unwrap();
        let file_path = root.path().join("plain");
        std::fs::write(&file_path, b"x").unwrap();
        let file = std::fs::File::open(&file_path).unwrap();
        let name = CString::new("child").unwrap();
        let error = open_at(file.as_fd(), &name, libc::O_RDONLY, 0).unwrap_err();
        assert_eq!(error.raw_os_error(), Some(libc::ENOTDIR));
    }
}
