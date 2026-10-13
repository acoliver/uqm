//! The `FILE *` boundary: `uio_debugInteractive` and the two print functions
//! the C commands call.
//!
//! [`uio_debug_interactive`] has the signature of `debug.h` plus the
//! repository, which the C took from the `repository` global of `setup.c`.
//! It carries no export attribute while the C `debug.c.o` provider is the
//! selected one: both define `uio_debugInteractive`, and a second definition
//! is a duplicate-provider link. Nothing calls it, so retirement needs no
//! replacement symbol (see the module documentation).
//!
//! # Contracts
//!
//! * The `FILE`s are read and written through stdio, byte for byte as the C
//!   did, so nothing past the newline that ends a command is taken from
//!   `in`, and `out` and `err` keep their own buffering.
//! * A null `FILE`, and a repository the working directory cannot be opened
//!   in, are caller bugs. The first is logged and aborts the process; the
//!   second prints `Fatal: Could not open working dir.` and aborts, as the C.
//!   Nothing unwinds into C.
//! * A failed read or write is reported in the log and ends the session.

use std::io::{self, BufRead, Read, Write};
use std::ptr::NonNull;

use libc::FILE;

use super::listing::{format_dir_handle, format_mounts};
use super::{DebugSession, Streams};
use crate::io::paths::c_abi::required;
use crate::io::uio_bridge::{mount_listing, uio_DirHandle, uio_Repository};
use crate::logging::{log_add, LogLevel};

/// A `FILE` read one byte at a time, so that a command reads exactly one
/// line from it, as `fgets` did.
pub struct FileReader {
    file: NonNull<FILE>,
    byte: u8,
    filled: bool,
}

impl FileReader {
    /// # Safety
    ///
    /// `file` is an open `FILE *` readable for the reader's lifetime.
    pub unsafe fn new(file: NonNull<FILE>) -> Self {
        Self {
            file,
            byte: 0,
            filled: false,
        }
    }
}

impl Read for FileReader {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let Some(slot) = buffer.first_mut() else {
            return Ok(0);
        };
        let available = self.fill_buf()?;
        let Some(&byte) = available.first() else {
            return Ok(0);
        };
        *slot = byte;
        self.consume(1);
        Ok(1)
    }
}

impl BufRead for FileReader {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        if !self.filled {
            // SAFETY: the file is open per `new`.
            let next = unsafe { libc::fgetc(self.file.as_ptr()) };
            if next == libc::EOF {
                // SAFETY: the file is open per `new`.
                return if unsafe { libc::ferror(self.file.as_ptr()) } != 0 {
                    Err(io::Error::last_os_error())
                } else {
                    Ok(&[])
                };
            }
            self.byte = next as u8;
            self.filled = true;
        }
        Ok(std::slice::from_ref(&self.byte))
    }

    fn consume(&mut self, amount: usize) {
        if amount > 0 {
            self.filled = false;
        }
    }
}

/// A `FILE` written through `fwrite`.
pub struct FileWriter {
    file: NonNull<FILE>,
}

impl FileWriter {
    /// # Safety
    ///
    /// `file` is an open `FILE *` writable for the writer's lifetime.
    pub unsafe fn new(file: NonNull<FILE>) -> Self {
        Self { file }
    }
}

impl Write for FileWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        // SAFETY: the file is open per `new` and the buffer readable.
        let written =
            unsafe { libc::fwrite(bytes.as_ptr().cast(), 1, bytes.len(), self.file.as_ptr()) };
        if written < bytes.len() {
            return Err(io::Error::last_os_error());
        }
        Ok(written)
    }

    fn flush(&mut self) -> io::Result<()> {
        // SAFETY: the file is open per `new`.
        if unsafe { libc::fflush(self.file.as_ptr()) } == 0 {
            Ok(())
        } else {
            Err(io::Error::last_os_error())
        }
    }
}

fn file(function: &str, argument: &str, file: *mut FILE) -> NonNull<FILE> {
    required(function, argument, file);
    NonNull::new(file).expect("required() aborted on null")
}

/// Takes over `uio_debugInteractive(FILE *in, FILE *out, FILE *err)` for the
/// `repository` the C kept in a global. Returns when `exit` runs or `input`
/// ends; the prompt is shown when `input` is a terminal.
///
/// A failed read or write ends the session with that error.
///
/// # Safety
///
/// `repository` is null or a live repository, and the three `FILE`s are open
/// (or null, which aborts) for the duration of the call.
pub unsafe fn run_interactive(
    repository: *mut uio_Repository,
    input: *mut FILE,
    out: *mut FILE,
    err: *mut FILE,
) -> io::Result<()> {
    const FUNCTION: &str = "uio_debugInteractive";
    let input = file(FUNCTION, "in", input);
    let out = file(FUNCTION, "out", out);
    let err = file(FUNCTION, "err", err);
    // SAFETY: open files per the contract.
    let (mut reader, mut out_writer, mut err_writer) = unsafe {
        (
            FileReader::new(input),
            FileWriter::new(out),
            FileWriter::new(err),
        )
    };

    // SAFETY: a live repository per the contract.
    let opened = NonNull::new(repository)
        .ok_or(())
        .and_then(|repository| unsafe { DebugSession::open(repository) }.map_err(drop));
    let Ok(mut session) = opened else {
        // The C wrote this unbuffered stream and aborted; abort does not flush.
        let _ = err_writer.write_all(b"Fatal: Could not open working dir.\n");
        let _ = err_writer.flush();
        std::process::abort();
    };

    // SAFETY: `input` is an open file.
    let prompt = unsafe { libc::isatty(libc::fileno(input.as_ptr())) } != 0;
    let mut streams = Streams {
        out: &mut out_writer,
        err: &mut err_writer,
    };
    session.interactive(&mut reader, &mut streams, prompt)
}

/// [`run_interactive`] with the C's `void` result: a failed read or write is
/// written to the log.
///
/// # Safety
///
/// As for [`run_interactive`].
pub unsafe fn uio_debug_interactive(
    repository: *mut uio_Repository,
    input: *mut FILE,
    out: *mut FILE,
    err: *mut FILE,
) {
    // SAFETY: per the contract.
    if let Err(error) = unsafe { run_interactive(repository, input, out, err) } {
        // SAFETY: `log_add` has no preconditions beyond a valid message.
        unsafe {
            log_add(
                LogLevel::Error,
                &format!("uio_debugInteractive() ended on an input/output error: {error}"),
            );
        }
    }
}

/// Writes `text` to `out`; a failed write is ignored, as `fprintf` was.
fn write_text(out: NonNull<FILE>, text: &[u8]) {
    // SAFETY: the file is open per the callers' contracts.
    unsafe { libc::fwrite(text.as_ptr().cast(), 1, text.len(), out.as_ptr()) };
}

/// The body of the exported `uio_printMounts`.
///
/// # Safety
///
/// `out` is an open `FILE *` and `repository` a live repository; null aborts.
pub(crate) unsafe fn print_mounts(out: *mut FILE, repository: *const uio_Repository) {
    const FUNCTION: &str = "uio_printMounts";
    let out = file(FUNCTION, "outStream", out);
    required(FUNCTION, "repository", repository);
    write_text(out, &format_mounts(&mount_listing(repository)));
}

/// The body of the exported `uio_DirHandle_print`.
///
/// # Safety
///
/// `out` is an open `FILE *` and `dir_handle` a live handle; null aborts.
pub(crate) unsafe fn print_dir_handle(dir_handle: *const uio_DirHandle, out: *mut FILE) {
    const FUNCTION: &str = "uio_DirHandle_print";
    let out = file(FUNCTION, "out", out);
    required(FUNCTION, "dirHandle", dir_handle);
    // SAFETY: a live handle per the contract.
    let path = unsafe { &*dir_handle }.virtual_path_bytes();
    write_text(out, &format_dir_handle(path));
}
