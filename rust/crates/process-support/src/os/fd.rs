//! Descriptor flags and deadline-bounded transfers.

use std::ffi::{c_int, c_short};
use std::io;
use std::os::fd::{AsRawFd as _, BorrowedFd, RawFd};

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
    use std::io::Write as _;
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
}
