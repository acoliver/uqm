//! The `debugCommands` table and the commands of `debug.c`.
//!
//! A command gets the whole argument vector, the command name first, and
//! returns the C's 0 or 1 as a [`CommandStatus`]. Usage and operating system
//! failures are written to the error stream and are a `Failure`; `Err` is a
//! failed write to the console's own streams.

mod exec;
mod files;
mod tree;

#[cfg(test)]
pub(crate) use files::format_stat;
#[cfg(test)]
pub(crate) use tree::list_query;

use std::ffi::CString;
use std::io::{self, Write};

use super::listing::format_mounts;
use super::{CommandStatus, DebugSession, Errno, Streams};
use crate::io::uio_bridge::mount_listing;

/// A command function.
pub type CommandFn =
    fn(&mut DebugSession, &mut Streams<'_>, &[CString]) -> io::Result<CommandStatus>;

/// One row of `debugCommands`.
pub struct DebugCommand {
    pub name: &'static str,
    pub run: CommandFn,
}

/// The commands, in alphabetical order as in the C.
pub static DEBUG_COMMANDS: [DebugCommand; 15] = [
    DebugCommand {
        name: "cat",
        run: files::cat,
    },
    DebugCommand {
        name: "cd",
        run: tree::cd,
    },
    DebugCommand {
        name: "exec",
        run: exec::exec,
    },
    DebugCommand {
        name: "exit",
        run: exit,
    },
    DebugCommand {
        name: "fwritetest",
        run: files::fwritetest,
    },
    DebugCommand {
        name: "ls",
        run: tree::ls,
    },
    DebugCommand {
        name: "mem",
        run: mem,
    },
    DebugCommand {
        name: "mkdir",
        run: tree::mkdir,
    },
    DebugCommand {
        name: "mount",
        run: mount,
    },
    DebugCommand {
        name: "mv",
        run: tree::mv,
    },
    DebugCommand {
        name: "pwd",
        run: tree::pwd,
    },
    DebugCommand {
        name: "rm",
        run: tree::rm,
    },
    DebugCommand {
        name: "rmdir",
        run: tree::rmdir,
    },
    DebugCommand {
        name: "stat",
        run: files::stat,
    },
    DebugCommand {
        name: "writetest",
        run: files::writetest,
    },
];

/// The command called `name`; the match is exact and case sensitive.
pub fn find_command(name: &[u8]) -> Option<&'static DebugCommand> {
    DEBUG_COMMANDS
        .iter()
        .find(|command| command.name.as_bytes() == name)
}

/// `Invalid number of arguments.` on the error stream.
fn invalid_arguments(streams: &mut Streams<'_>) -> io::Result<CommandStatus> {
    streams.err.write_all(b"Invalid number of arguments.\n")?;
    Ok(CommandStatus::Failure)
}

/// `prefix`, the text of `errno`, `suffix` in one write.
fn report(
    stream: &mut dyn Write,
    prefix: &[u8],
    errno: Errno,
    suffix: &[u8],
) -> io::Result<CommandStatus> {
    let mut text = prefix.to_vec();
    text.extend_from_slice(&errno.message());
    text.extend_from_slice(suffix);
    stream.write_all(&text)?;
    Ok(CommandStatus::Failure)
}

fn exit(
    session: &mut DebugSession,
    _streams: &mut Streams<'_>,
    _argv: &[CString],
) -> io::Result<CommandStatus> {
    session.exit = true;
    Ok(CommandStatus::Success)
}

fn mem(
    _session: &mut DebugSession,
    streams: &mut Streams<'_>,
    _argv: &[CString],
) -> io::Result<CommandStatus> {
    streams
        .out
        .write_all(b"Memory debugging not compiled in.\n")?;
    Ok(CommandStatus::Success)
}

fn mount(
    session: &mut DebugSession,
    streams: &mut Streams<'_>,
    argv: &[CString],
) -> io::Result<CommandStatus> {
    if argv.len() == 1 {
        let mounts = mount_listing(session.repository.as_ptr());
        streams.out.write_all(&format_mounts(&mounts))?;
    }
    Ok(CommandStatus::Success)
}
