//! The commands that move around and change the directory tree: `cd`, `pwd`,
//! `ls`, `mkdir`, `rmdir`, `rm`, `mv`.

use std::ffi::CString;
use std::io;

use super::{invalid_arguments, report};
use crate::io::debug::listing::format_dir_handle;
use crate::io::debug::{CommandStatus, DebugSession, Streams};
use crate::io::name_filter::{MATCH_LITERAL, MATCH_PREFIX};

pub(super) fn cd(
    session: &mut DebugSession,
    streams: &mut Streams<'_>,
    argv: &[CString],
) -> io::Result<CommandStatus> {
    if argv.len() != 2 {
        return invalid_arguments(streams);
    }
    match session.cwd.open_relative(&argv[1]) {
        Ok(new_dir) => {
            session.cwd = new_dir;
            Ok(CommandStatus::Success)
        }
        Err(errno) => report(streams.err, b"Could not access new dir: ", errno, b"\n"),
    }
}

pub(super) fn pwd(
    session: &mut DebugSession,
    streams: &mut Streams<'_>,
    _argv: &[CString],
) -> io::Result<CommandStatus> {
    streams
        .out
        .write_all(&format_dir_handle(session.cwd.virtual_path()))?;
    Ok(CommandStatus::Success)
}

/// What `listOneDir` asks `uio_getDirList` for.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct ListQuery<'a> {
    pub(crate) path: &'a [u8],
    pub(crate) pattern: &'a [u8],
    pub(crate) match_type: i32,
}

/// Splits an `ls` argument at its last `/` into a directory and a name
/// pattern. `*` alone means every name (a prefix match on nothing); any other
/// pattern is a literal name, as the build has no `HAVE_GLOB`.
pub(crate) fn list_query(arg: &[u8]) -> ListQuery<'_> {
    let (path, pattern) = match arg.iter().rposition(|&byte| byte == b'/') {
        _ if arg.is_empty() => (arg, &b"*"[..]),
        None => (&b""[..], arg),
        Some(slash) if slash + 1 == arg.len() => (arg, &b"*"[..]),
        Some(0) => (&b"/"[..], &arg[1..]),
        Some(slash) => (&arg[..slash], &arg[slash + 1..]),
    };
    if pattern == b"*" {
        ListQuery {
            path,
            pattern: b"",
            match_type: MATCH_PREFIX,
        }
    } else {
        ListQuery {
            path,
            pattern,
            match_type: MATCH_LITERAL,
        }
    }
}

fn c_string(bytes: &[u8]) -> CString {
    CString::new(bytes).expect("a part of a C string has no NUL")
}

/// `listOneDir`. Its failure text goes to the output stream, not the error
/// stream, as in the C.
fn list_one_dir(
    session: &DebugSession,
    streams: &mut Streams<'_>,
    arg: &[u8],
) -> io::Result<CommandStatus> {
    let query = list_query(arg);
    let listing = session.cwd.list(
        &c_string(query.path),
        &c_string(query.pattern),
        query.match_type,
    );
    match listing {
        Err(errno) => report(streams.out, b"Error in uio_getDirList(): ", errno, b".\n"),
        Ok(list) => {
            let mut text = Vec::new();
            for name in list.names() {
                text.extend_from_slice(name.to_bytes());
                text.push(b'\n');
            }
            streams.out.write_all(&text)?;
            Ok(CommandStatus::Success)
        }
    }
}

pub(super) fn ls(
    session: &mut DebugSession,
    streams: &mut Streams<'_>,
    argv: &[CString],
) -> io::Result<CommandStatus> {
    if argv.len() == 1 {
        return list_one_dir(session, streams, b"");
    }
    for arg in &argv[1..] {
        if list_one_dir(session, streams, arg.to_bytes())? == CommandStatus::Failure {
            return Ok(CommandStatus::Failure);
        }
    }
    Ok(CommandStatus::Success)
}

pub(super) fn mkdir(
    session: &mut DebugSession,
    streams: &mut Streams<'_>,
    argv: &[CString],
) -> io::Result<CommandStatus> {
    if argv.len() != 2 {
        return invalid_arguments(streams);
    }
    match session.cwd.make_dir(&argv[1], 0o777) {
        Ok(()) => Ok(CommandStatus::Success),
        Err(errno) => report(streams.err, b"Could not create directory: ", errno, b"\n"),
    }
}

pub(super) fn rmdir(
    session: &mut DebugSession,
    streams: &mut Streams<'_>,
    argv: &[CString],
) -> io::Result<CommandStatus> {
    if argv.len() != 2 {
        return invalid_arguments(streams);
    }
    match session.cwd.remove_dir(&argv[1]) {
        Ok(()) => Ok(CommandStatus::Success),
        Err(errno) => report(streams.err, b"Could not remove directory: ", errno, b"\n"),
    }
}

pub(super) fn rm(
    session: &mut DebugSession,
    streams: &mut Streams<'_>,
    argv: &[CString],
) -> io::Result<CommandStatus> {
    if argv.len() != 2 {
        return invalid_arguments(streams);
    }
    match session.cwd.unlink(&argv[1]) {
        Ok(()) => Ok(CommandStatus::Success),
        Err(errno) => report(streams.err, b"Could not remove file: ", errno, b"\n"),
    }
}

pub(super) fn mv(
    session: &mut DebugSession,
    streams: &mut Streams<'_>,
    argv: &[CString],
) -> io::Result<CommandStatus> {
    if argv.len() != 3 {
        return invalid_arguments(streams);
    }
    match session.cwd.rename(&argv[1], &argv[2]) {
        Ok(()) => Ok(CommandStatus::Success),
        Err(errno) => report(streams.err, b"Could not rename: ", errno, b"\n"),
    }
}
