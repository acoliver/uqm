//! `exec`: runs a host program with the files it names made available as
//! host paths.

use std::ffi::{CStr, CString, OsStr};
use std::io::{self, Write};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::process::ExitStatusExt;
use std::process::Command;

use super::{invalid_arguments, report};
use crate::io::debug::repo::StdioAccess;
use crate::io::debug::{CommandStatus, DebugSession, Errno, Streams};

pub(super) fn exec(
    session: &mut DebugSession,
    streams: &mut Streams<'_>,
    argv: &[CString],
) -> io::Result<CommandStatus> {
    if argv.len() < 2 {
        return invalid_arguments(streams);
    }
    let temp_dir = match session.cwd.open_relative(c"/tmp") {
        Ok(dir) => dir,
        Err(errno) => return report(streams.err, b"Could not open temp dir: ", errno, b".\n"),
    };

    // Declared after `temp_dir`, so the accesses are released first, and
    // after the program ran: a copy of an archive entry exists only until its
    // access is released.
    let mut accesses: Vec<StdioAccess> = Vec::new();
    let mut arguments: Vec<Vec<u8>> = Vec::new();
    for arg in &argv[2..] {
        match session.cwd.stdio_access(arg, &temp_dir) {
            Ok(access) => {
                arguments.push(access.path().to_bytes().to_vec());
                accesses.push(access);
            }
            // No such file: the word is an option or a plain argument.
            Err(Errno(libc::ENOENT)) => arguments.push(arg.to_bytes().to_vec()),
            Err(errno) => {
                let mut prefix = b"Cannot execute: Cannot get stdio access to ".to_vec();
                prefix.extend_from_slice(arg.to_bytes());
                prefix.extend_from_slice(b": ");
                return report(streams.err, &prefix, errno, b".\n");
            }
        }
    }
    run_program(&argv[1], &arguments, streams.err)?;
    drop(accesses);
    Ok(CommandStatus::Success)
}

/// The `Executing:` line, then the program, then how it ended.
fn run_program(program: &CStr, arguments: &[Vec<u8>], err: &mut dyn Write) -> io::Result<()> {
    let mut announcement = b"Executing: ".to_vec();
    announcement.extend_from_slice(program.to_bytes());
    for argument in arguments {
        announcement.push(b' ');
        announcement.extend_from_slice(argument);
    }
    announcement.push(b'\n');
    err.write_all(&announcement)?;

    let mut command = Command::new(OsStr::from_bytes(program.to_bytes()));
    command.args(arguments.iter().map(|argument| OsStr::from_bytes(argument)));
    let mut child = match command.spawn() {
        Ok(child) => child,
        Err(error) => {
            let errno = Errno(error.raw_os_error().unwrap_or(libc::EINVAL));
            report(err, b"Error: execvp() failed: ", errno, b".\n")?;
            return err.write_all(b"Exit status: 1\n");
        }
    };
    match child.wait() {
        Err(error) => {
            let errno = Errno(error.raw_os_error().unwrap_or(libc::EINVAL));
            report(err, b"Error: waitpid() failed: ", errno, b"\n").map(drop)
        }
        Ok(status) => match (status.code(), status.signal()) {
            (Some(code), _) => err.write_all(format!("Exit status: {code}\n").as_bytes()),
            (None, Some(signal)) => {
                err.write_all(format!("Terminated on signal {signal}.\n").as_bytes())
            }
            (None, None) => err.write_all(b"Error: weird exit status.\n"),
        },
    }
}
