//! The console state (`DebugContext`) and the input loop
//! (`uio_debugInteractive`).

use std::ffi::CString;
use std::io::{self, BufRead, Write};
use std::ptr::NonNull;

use super::commands;
use super::repo::Dir;
use super::tokens::make_args;
use super::Errno;
use crate::io::uio_bridge::uio_Repository;

/// The longest line `fgets` took into the C's `LINEBUFLEN` buffer, newline
/// included.
const LINE_BUFFER_LEN: usize = 1024;

/// The streams a command writes to.
pub struct Streams<'a> {
    pub out: &'a mut dyn Write,
    pub err: &'a mut dyn Write,
}

/// A command's return value in the C: 0 or 1.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommandStatus {
    Success,
    Failure,
}

impl CommandStatus {
    /// The `int` the C command returned.
    pub fn code(self) -> i32 {
        match self {
            CommandStatus::Success => 0,
            CommandStatus::Failure => 1,
        }
    }
}

/// `/` of the repository could not be opened. The C printed `Fatal: Could
/// not open working dir.` and aborted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OpenError(pub Errno);

/// A console over one repository: its working directory and whether `exit`
/// was asked for.
pub struct DebugSession {
    pub(super) repository: NonNull<uio_Repository>,
    pub(super) cwd: Dir,
    pub(super) exit: bool,
}

impl DebugSession {
    /// Starts a session at `/` of `repository`.
    ///
    /// # Safety
    ///
    /// `repository` is live until the session is dropped.
    pub unsafe fn open(repository: NonNull<uio_Repository>) -> Result<Self, OpenError> {
        // SAFETY: per the contract.
        let cwd = unsafe { Dir::open_root(repository) }.map_err(OpenError)?;
        Ok(Self {
            repository,
            cwd,
            exit: false,
        })
    }

    /// Whether `exit` has run.
    pub fn exit_requested(&self) -> bool {
        self.exit
    }

    /// The working directory as a virtual path.
    pub fn working_dir(&self) -> &[u8] {
        self.cwd.virtual_path()
    }

    /// `debugCallCommand`: runs the command `argv[0]` with the arguments
    /// `argv`. `Err` is a failed write to `streams`.
    ///
    /// # Panics
    ///
    /// `argv` is empty; the caller skips empty lines.
    pub fn execute(
        &mut self,
        argv: &[CString],
        streams: &mut Streams<'_>,
    ) -> io::Result<CommandStatus> {
        let name = argv.first().expect("a command line has a command name");
        match commands::find_command(name.to_bytes()) {
            Some(command) => (command.run)(self, streams, argv),
            None => {
                streams.err.write_all(b"Invalid command.\n")?;
                Ok(CommandStatus::Failure)
            }
        }
    }

    /// `uio_debugInteractive` after the working directory is open: reads
    /// lines from `input` and runs them until `exit` or the end of the input.
    /// `prompt` is what `isatty(fileno(in))` decided: `> ` before each read
    /// and a newline at the end.
    ///
    /// `Err` is a failed read of `input` or write to `streams`.
    pub fn interactive(
        &mut self,
        input: &mut dyn BufRead,
        streams: &mut Streams<'_>,
        prompt: bool,
    ) -> io::Result<()> {
        loop {
            if prompt {
                streams.out.write_all(b"> ")?;
            }
            let Some(line) = read_line(input)? else {
                break;
            };
            match line {
                Line::TooLong => streams.err.write_all(b"Too long command line.\n")?,
                Line::Text(text) => self.run_line(&text, streams)?,
            }
            if self.exit {
                break;
            }
        }
        if prompt {
            streams.out.write_all(b"\n")?;
        }
        Ok(())
    }

    fn run_line(&mut self, text: &[u8], streams: &mut Streams<'_>) -> io::Result<()> {
        match make_args(text) {
            Err(_) => streams
                .err
                .write_all(b"Command line contains a NUL byte.\n"),
            Ok(argv) if argv.is_empty() => Ok(()),
            Ok(argv) => self.execute(&argv, streams).map(drop),
        }
    }
}

enum Line {
    Text(Vec<u8>),
    TooLong,
}

/// The next line, without its newline; `None` at the end of the input. The
/// input is consumed through the newline and no further. Only as much as
/// `fgets` could have held in the C's buffer is kept, so an endless line
/// costs no memory: the rest of it is dropped as it is read.
fn read_line(input: &mut dyn BufRead) -> io::Result<Option<Line>> {
    let mut line = Vec::new();
    let mut too_long = false;
    let mut seen_input = false;
    loop {
        let available = match input.fill_buf() {
            Ok(available) => available,
            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
            Err(error) => return Err(error),
        };
        if available.is_empty() {
            break;
        }
        seen_input = true;
        let newline = available.iter().position(|&byte| byte == b'\n');
        let content = &available[..newline.unwrap_or(available.len())];
        let room = LINE_BUFFER_LEN - 1 - line.len();
        if content.len() >= room {
            too_long = true;
        } else {
            line.extend_from_slice(content);
        }
        let used = newline.map_or(available.len(), |at| at + 1);
        input.consume(used);
        if newline.is_some() {
            break;
        }
    }
    Ok(match (seen_input, too_long) {
        (false, _) => None,
        (true, true) => Some(Line::TooLong),
        (true, false) => Some(Line::Text(line)),
    })
}
