//! The commands that read or write a file: `cat`, `stat`, `writetest`,
//! `fwritetest`.

use std::ffi::{c_char, CStr, CString};
use std::io::{self, Write};

use super::{invalid_arguments, report};
use crate::io::debug::repo::Stream;
use crate::io::debug::{CommandStatus, DebugSession, Errno, Streams};
use crate::io::uio_bridge::stat;

/// The size of the C's read buffer.
const READ_BUFFER_SIZE: usize = 0x10000;

pub(super) fn cat(
    session: &mut DebugSession,
    streams: &mut Streams<'_>,
    argv: &[CString],
) -> io::Result<CommandStatus> {
    if argv.len() != 2 {
        return invalid_arguments(streams);
    }
    let file = match session.cwd.open_file(&argv[1], libc::O_RDONLY, 0) {
        Ok(file) => file,
        Err(errno) => return report(streams.err, b"Could not open file: ", errno, b"\n"),
    };
    let mut buffer = vec![0u8; READ_BUFFER_SIZE];
    let mut total: u64 = 0;
    loop {
        match file.read(&mut buffer) {
            Err(Errno(libc::EINTR)) => continue,
            Err(errno) => return report(streams.err, b"Could not read from file: ", errno, b"\n"),
            Ok(0) => break,
            Ok(count) => {
                streams.out.write_all(&buffer[..count])?;
                total += count as u64;
            }
        }
    }
    streams
        .out
        .write_all(format!("[{total} bytes]\n").as_bytes())?;
    Ok(CommandStatus::Success)
}

pub(super) fn stat(
    session: &mut DebugSession,
    streams: &mut Streams<'_>,
    argv: &[CString],
) -> io::Result<CommandStatus> {
    if argv.len() != 2 {
        return invalid_arguments(streams);
    }
    let formatted = session
        .cwd
        .stat(&argv[1])
        .and_then(|info| format_stat(&info));
    match formatted {
        Ok(text) => {
            streams.out.write_all(&text)?;
            Ok(CommandStatus::Success)
        }
        Err(errno) => report(streams.err, b"Could not stat file: ", errno, b"\n"),
    }
}

/// The `stat` report: size, owner, permission bits, and the three times in
/// `ctime` form in the local time zone. A time `ctime` cannot show is
/// `EOVERFLOW`.
pub(crate) fn format_stat(info: &stat) -> Result<Vec<u8>, Errno> {
    let permission_bits =
        libc::S_IRWXU | libc::S_IRWXG | libc::S_IRWXO | libc::S_ISUID | libc::S_ISGID;
    let mut text = format!(
        "size {} bytes\nuid {}  gid {}  mode 0{:o}\n",
        info.st_size as u64,
        info.st_uid as i32,
        info.st_gid as i32,
        u32::from(info.st_mode) & u32::from(permission_bits),
    )
    .into_bytes();
    for (label, time) in [
        (&b"last access:        "[..], info.st_atime),
        (b"last modification:  ", info.st_mtime),
        (b"last status change: ", info.st_ctime),
    ] {
        text.extend_from_slice(label);
        text.extend_from_slice(&ctime(time)?);
    }
    Ok(text)
}

/// `ctime`: `Thu Jan  1 00:00:00 1970\n`.
fn ctime(time: libc::time_t) -> Result<Vec<u8>, Errno> {
    // `ctime_r` needs 26 bytes.
    let mut buffer = [0 as c_char; 64];
    // SAFETY: the time is readable and the buffer writable for 26+ bytes.
    let text = unsafe { libc::ctime_r(&time, buffer.as_mut_ptr()) };
    if text.is_null() {
        return Err(Errno(libc::EOVERFLOW));
    }
    // SAFETY: a non-null result is a NUL-terminated string in `buffer`.
    Ok(unsafe { CStr::from_ptr(text) }.to_bytes().to_vec())
}

/// `sizeof testString` of `writetest`: the text and its NUL.
const WRITE_TEST_TEXT: &[u8] = b"Hello world!\n\0";

pub(super) fn writetest(
    session: &mut DebugSession,
    streams: &mut Streams<'_>,
    argv: &[CString],
) -> io::Result<CommandStatus> {
    if argv.len() != 2 {
        return invalid_arguments(streams);
    }
    let flags = libc::O_WRONLY | libc::O_CREAT | libc::O_EXCL;
    let file = match session.cwd.open_file(&argv[1], flags, 0o644) {
        Ok(file) => file,
        Err(errno) => return report(streams.err, b"Could not open file: ", errno, b"\n"),
    };
    match file.write(WRITE_TEST_TEXT) {
        Ok(()) => Ok(CommandStatus::Success),
        Err(errno) => report(streams.err, b"Write failed: ", errno, b"\n"),
    }
}

/// Which stream call failed, for the `uio_fwrite() failed: ...` line.
struct StepFailure {
    call: &'static str,
    errno: Errno,
}

enum FwriteError {
    Step(StepFailure),
    Console(io::Error),
}

impl From<io::Error> for FwriteError {
    fn from(error: io::Error) -> Self {
        FwriteError::Console(error)
    }
}

fn step(call: &'static str, result: Result<(), Errno>) -> Result<(), FwriteError> {
    result.map_err(|errno| FwriteError::Step(StepFailure { call, errno }))
}

pub(super) fn fwritetest(
    session: &mut DebugSession,
    streams: &mut Streams<'_>,
    argv: &[CString],
) -> io::Result<CommandStatus> {
    if argv.len() != 2 {
        return invalid_arguments(streams);
    }
    let stream = match session.cwd.open_stream(&argv[1], c"w+b") {
        Ok(stream) => stream,
        Err(errno) => return report(streams.err, b"Could not open file: ", errno, b"\n"),
    };
    match fwrite_sequence(&stream, streams.out) {
        Ok(()) => Ok(CommandStatus::Success),
        Err(FwriteError::Console(error)) => Err(error),
        Err(FwriteError::Step(failure)) => {
            let prefix = format!("{}() failed: ", failure.call);
            report(streams.err, prefix.as_bytes(), failure.errno, b"\n")
        }
    }
}

/// The fixed sequence of stream calls of `debugCmdFwriteTest`: writes, seeks,
/// a line-by-line read of what is there (printed), a write past the end and a
/// last overwrite.
fn fwrite_sequence(stream: &Stream, out: &mut dyn Write) -> Result<(), FwriteError> {
    const TEXT: &CStr = c"0123456789\n";
    step(
        "uio_fwrite",
        stream.write_items(TEXT.to_bytes(), TEXT.to_bytes().len()),
    )?;
    step("uio_fputs", stream.put_str(TEXT))?;
    step("uio_fseek", stream.seek(15, libc::SEEK_SET))?;
    step("uio_fputc", stream.put_char(b'A'))?;
    step("uio_fseek", stream.seek(0, libc::SEEK_SET))?;

    let mut buffer = [0u8; 6];
    let mut number = 1;
    while let Some(length) = stream.get_line(&mut buffer) {
        let mut line = format!("{number}: [").into_bytes();
        line.extend_from_slice(&buffer[..length]);
        line.extend_from_slice(b"]\n");
        out.write_all(&line)?;
        number += 1;
    }
    if stream.has_error() {
        return step("uio_fgets", Err(Errno::last()));
    }
    stream.clear_error();

    step("uio_fseek", stream.seek(4, libc::SEEK_END))?;
    step("uio_fwrite", stream.write_items(&[b'Q'; 2000], 100))?;
    step("uio_fseek", stream.seek(5, libc::SEEK_SET))?;
    step("uio_fputc", stream.put_char(b'B'))
}
