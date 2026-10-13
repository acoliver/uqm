//! The input loop, and the `FILE *` boundary over real `FILE`s.

use std::ffi::CString;
use std::io::{self, BufRead, Cursor, Read, Write};
use std::ptr::NonNull;

use serial_test::serial;

use super::c_abi::{run_interactive, FileReader, FileWriter};
use super::world::World;
use super::{DebugSession, Streams};
use crate::io::uio_bridge::{uio_DirHandle_print, uio_openDir, uio_printMounts};
use crate::resource::abort_child::expect_abort;

struct Run {
    out: String,
    err: String,
    result: io::Result<()>,
}

fn interactive(world: &World, input: &mut dyn BufRead, prompt: bool) -> Run {
    let mut session = world.session();
    let (mut out, mut err) = (Vec::new(), Vec::new());
    let result = session.interactive(
        input,
        &mut Streams {
            out: &mut out,
            err: &mut err,
        },
        prompt,
    );
    Run {
        out: String::from_utf8_lossy(&out).into_owned(),
        err: String::from_utf8_lossy(&err).into_owned(),
        result,
    }
}

fn lines(text: &[u8], prompt: bool, world: &World) -> Run {
    interactive(world, &mut Cursor::new(text.to_vec()), prompt)
}

#[test]
#[serial]
fn prompts_come_before_each_read_and_a_newline_ends_the_session() {
    let world = World::new();
    let run = lines(b"pwd\ncd sub\npwd\n", true, &world);
    run.result.unwrap();
    assert_eq!(run.out, "> []> > []sub> \n");
    assert!(run.err.is_empty());

    let run = lines(b"pwd\nexit\n", true, &world);
    assert_eq!(run.out, "> []> \n");

    let run = lines(b"pwd\ncd sub\npwd\n", false, &world);
    assert_eq!(run.out, "[][]sub");
}

#[test]
#[serial]
fn blank_lines_are_skipped_and_failures_do_not_end_the_session() {
    let world = World::new();
    let run = lines(b"\n   \t\nbogus\n\ncat nope\npwd\n", false, &world);
    run.result.unwrap();
    assert_eq!(run.out, "[]");
    assert_eq!(
        run.err,
        "Invalid command.\nCould not open file: No such file or directory\n"
    );
}

#[test]
#[serial]
fn exit_stops_reading_at_the_end_of_its_line() {
    let world = World::new();
    let mut input = Cursor::new(b"pwd\nexit\npwd\nmore\n".to_vec());
    let run = interactive(&world, &mut input, false);
    run.result.unwrap();
    assert_eq!(run.out, "[]");
    assert_eq!(input.position(), 9, "reading stopped after `exit\\n`");
    let mut rest = String::new();
    input.read_to_string(&mut rest).unwrap();
    assert_eq!(rest, "pwd\nmore\n");
}

#[test]
#[serial]
fn the_end_of_input_ends_the_session_and_a_last_line_needs_no_newline() {
    let world = World::new();
    let run = lines(b"", false, &world);
    run.result.unwrap();
    assert!(run.out.is_empty() && run.err.is_empty());
    let run = lines(b"pwd", false, &world);
    assert_eq!(run.out, "[]");
    let run = lines(b"pwd\ncd sub\nexit", false, &world);
    assert_eq!(run.out, "[]");
    let run = lines(b"pwd\r\npwd\r\n", false, &world);
    assert_eq!(run.out, "[][]");
}

#[test]
#[serial]
fn a_long_line_is_reported_and_dropped_through_its_end() {
    let world = World::new();
    let at_limit = format!("{}\n", "x".repeat(1022));
    let run = lines(at_limit.as_bytes(), false, &world);
    assert_eq!(run.err, "Invalid command.\n", "1022 bytes still fit");

    let too_long = format!("{}\n", "x".repeat(1023));
    let run = lines(too_long.as_bytes(), false, &world);
    assert_eq!(run.err, "Too long command line.\n");

    // The tail of the long line is not a command.
    let tail = format!("{} pwd\npwd\n", "y".repeat(1100));
    let run = lines(tail.as_bytes(), false, &world);
    assert_eq!(run.err, "Too long command line.\n");
    assert_eq!(run.out, "[]", "only the next line ran");

    // A long last line without a newline.
    let run = lines("z".repeat(5000).as_bytes(), false, &world);
    assert_eq!(run.err, "Too long command line.\n");
}

#[test]
#[serial]
fn the_tail_of_a_long_line_is_dropped_however_the_input_is_chunked() {
    let world = World::new();
    // The blanks make the part after the limit a runnable `pwd`.
    let text = format!("{}pwd\npwd\nexit\n", " ".repeat(1100));
    for capacity in [1, 7, 16, 4096] {
        let mut input = io::BufReader::with_capacity(capacity, Cursor::new(text.clone()));
        let run = interactive(&world, &mut input, false);
        run.result.unwrap();
        assert_eq!(run.err, "Too long command line.\n", "capacity {capacity}");
        assert_eq!(run.out, "[]", "capacity {capacity}: only the next line ran");
    }
}

/// The shipped reader hands over one byte at a time, so the limit is crossed
/// in the middle of the line. A tail that ran would create `tail.txt` in the
/// test's own temporary directory.
#[test]
#[serial]
fn a_command_in_the_tail_of_a_long_file_line_does_not_run() {
    let world = World::new();
    let script = format!(
        "{}writetest tail.txt\nwritetest next.txt\nexit\nwritetest after.txt\n",
        " ".repeat(1100)
    );
    let input = tmpfile_with(script.as_bytes());
    let (out, err) = (tmpfile_with(b""), tmpfile_with(b""));
    // SAFETY: live repository and open files.
    let result = unsafe {
        run_interactive(
            world.raw_repository(),
            input.as_ptr(),
            out.as_ptr(),
            err.as_ptr(),
        )
    };
    result.unwrap();
    assert_eq!(read_back(err), b"Too long command line.\n");
    assert!(read_back(out).is_empty());
    assert!(
        !world.host.path().join("tail.txt").exists(),
        "the tail of the long line ran"
    );
    assert_eq!(
        std::fs::read(world.host.path().join("next.txt")).unwrap(),
        b"Hello world!\n\0",
        "the line after the long one ran"
    );
    assert!(!world.host.path().join("after.txt").exists());
}

#[test]
#[serial]
fn a_nul_byte_rejects_the_line_and_the_next_runs() {
    let world = World::new();
    let run = lines(b"cat a\0b\npwd\n", false, &world);
    run.result.unwrap();
    assert_eq!(run.err, "Command line contains a NUL byte.\n");
    assert_eq!(run.out, "[]");
}

#[test]
#[serial]
fn non_utf8_command_lines_are_ordinary() {
    let world = World::new();
    let run = lines(
        b"c\xe0t hello.txt\ncat rawmnt\xfe\xff/inner.txt\n",
        false,
        &world,
    );
    run.result.unwrap();
    assert_eq!(run.err, "Invalid command.\n");
    assert_eq!(run.out, "raw mount[9 bytes]\n");
}

/// Input that delivers `first`, then fails (or is interrupted once).
struct Flaky {
    first: Cursor<Vec<u8>>,
    interrupted: bool,
    then: Option<io::ErrorKind>,
}

impl Read for Flaky {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        let count = self.first.read(buffer)?;
        if count > 0 {
            return Ok(count);
        }
        match self.then {
            Some(kind) => Err(io::Error::new(kind, "injected")),
            None => Ok(0),
        }
    }
}

impl BufRead for Flaky {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        if self.interrupted {
            self.interrupted = false;
            return Err(io::ErrorKind::Interrupted.into());
        }
        if self.first.position() as usize >= self.first.get_ref().len() {
            if let Some(kind) = self.then {
                return Err(io::Error::new(kind, "injected"));
            }
        }
        self.first.fill_buf()
    }

    fn consume(&mut self, amount: usize) {
        self.first.consume(amount);
    }
}

#[test]
#[serial]
fn a_read_error_ends_the_session_after_the_lines_before_it() {
    let world = World::new();
    let mut input = Flaky {
        first: Cursor::new(b"pwd\n".to_vec()),
        interrupted: false,
        then: Some(io::ErrorKind::Other),
    };
    let run = interactive(&world, &mut input, false);
    assert_eq!(run.result.unwrap_err().kind(), io::ErrorKind::Other);
    assert_eq!(run.out, "[]");
}

#[test]
#[serial]
fn an_interrupted_read_is_retried() {
    let world = World::new();
    let mut input = Flaky {
        first: Cursor::new(b"pwd\n".to_vec()),
        interrupted: true,
        then: None,
    };
    let run = interactive(&world, &mut input, false);
    run.result.unwrap();
    assert_eq!(run.out, "[]");
}

struct Broken;

impl Write for Broken {
    fn write(&mut self, _: &[u8]) -> io::Result<usize> {
        Err(io::Error::new(io::ErrorKind::BrokenPipe, "closed"))
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[test]
#[serial]
fn a_failed_write_to_the_console_ends_the_session() {
    let world = World::new();
    let mut session = world.session();
    let mut err = Vec::new();
    let result = session.interactive(
        &mut Cursor::new(b"pwd\npwd\n".to_vec()),
        &mut Streams {
            out: &mut Broken,
            err: &mut err,
        },
        false,
    );
    assert_eq!(result.unwrap_err().kind(), io::ErrorKind::BrokenPipe);

    let argv = [CString::new("bogus").unwrap()];
    let result = session.execute(
        &argv,
        &mut Streams {
            out: &mut Vec::new(),
            err: &mut Broken,
        },
    );
    assert_eq!(result.unwrap_err().kind(), io::ErrorKind::BrokenPipe);
}

#[test]
#[serial]
fn a_session_can_follow_another_on_one_repository() {
    let world = World::new();
    let first = lines(b"cd sub\nmkdir fresh\npwd\n", false, &world);
    first.result.unwrap();
    assert_eq!(first.out, "[]sub");
    let second = lines(b"pwd\nls fresh/\n", false, &world);
    second.result.unwrap();
    assert_eq!(second.out, "[]");
    assert!(second.err.is_empty());
}

#[test]
#[serial]
fn a_session_outlives_the_unmounting_of_its_directory() {
    let mut world = World::new();
    let mut session = world.session();
    world.run(&mut session, "cd /ro");
    world.unmount_ro();
    assert_eq!(world.run(&mut session, "pwd").out, b"[]ro");
    // The directory is gone from the tree, so nothing in it can be read.
    let outcome = world.run(&mut session, "cat fixed.txt");
    assert_eq!(outcome.status, super::CommandStatus::Failure);
    assert_eq!(
        outcome.err_text(),
        "Could not open file: No such file or directory\n"
    );
    assert_eq!(
        world.run(&mut session, "cd /").status,
        super::CommandStatus::Success
    );
}

// --- the FILE boundary -----------------------------------------------------

fn tmpfile_with(content: &[u8]) -> NonNull<libc::FILE> {
    // SAFETY: tmpfile has no preconditions.
    let file = NonNull::new(unsafe { libc::tmpfile() }).expect("tmpfile");
    // SAFETY: the file is open for writing and the buffer readable.
    unsafe {
        assert_eq!(
            libc::fwrite(content.as_ptr().cast(), 1, content.len(), file.as_ptr()),
            content.len()
        );
        libc::rewind(file.as_ptr());
    }
    file
}

fn read_back(file: NonNull<libc::FILE>) -> Vec<u8> {
    let mut bytes = Vec::new();
    // SAFETY: the file is open; it is rewound and read to its end.
    unsafe {
        libc::fflush(file.as_ptr());
        libc::rewind(file.as_ptr());
        let mut chunk = [0u8; 256];
        loop {
            let count = libc::fread(chunk.as_mut_ptr().cast(), 1, chunk.len(), file.as_ptr());
            if count == 0 {
                break;
            }
            bytes.extend_from_slice(&chunk[..count]);
        }
    }
    bytes
}

#[test]
#[serial]
fn interactive_over_real_files_reads_one_line_at_a_time() {
    let world = World::new();
    let input = tmpfile_with(b"pwd\ncd sub\npwd\nexit\ncat hello.txt\n");
    let (out, err) = (tmpfile_with(b""), tmpfile_with(b""));
    // SAFETY: live repository and open files.
    let result = unsafe {
        run_interactive(
            world.raw_repository(),
            input.as_ptr(),
            out.as_ptr(),
            err.as_ptr(),
        )
    };
    result.unwrap();
    assert_eq!(read_back(out), b"[][]sub");
    assert!(read_back(err).is_empty());
    // Nothing past `exit\n` was taken from the file.
    // SAFETY: the file is open.
    assert_eq!(unsafe { libc::ftell(input.as_ptr()) }, 20);
    // A plain file is not a terminal: no prompt was shown.
}

#[test]
#[serial]
fn the_file_reader_and_writer_follow_stdio() {
    let file = tmpfile_with(b"ab\ncd");
    // SAFETY: an open file.
    let mut reader = unsafe { FileReader::new(file) };
    let mut line = Vec::new();
    reader.read_until(b'\n', &mut line).unwrap();
    assert_eq!(line, b"ab\n");
    let mut rest = String::new();
    reader.read_to_string(&mut rest).unwrap();
    assert_eq!(rest, "cd");
    assert_eq!(reader.fill_buf().unwrap(), b"");

    let target = tmpfile_with(b"");
    // SAFETY: an open file.
    let mut writer = unsafe { FileWriter::new(target) };
    writer.write_all(b"xyz").unwrap();
    writer.flush().unwrap();
    assert_eq!(read_back(target), b"xyz");
}

#[test]
#[serial]
fn a_read_only_file_cannot_be_written() {
    // SAFETY: opening /dev/null read-only.
    let file = NonNull::new(unsafe { libc::fopen(c"/dev/null".as_ptr(), c"r".as_ptr()) }).unwrap();
    // SAFETY: an open file.
    let mut writer = unsafe { FileWriter::new(file) };
    assert!(writer
        .write_all(b"x")
        .and_then(|()| writer.flush())
        .is_err());
    // SAFETY: closing the file opened above.
    unsafe { libc::fclose(file.as_ptr()) };
}

#[test]
#[serial]
fn the_exported_print_functions_write_real_text() {
    let world = World::new();
    let out = tmpfile_with(b"");
    // SAFETY: live repository, open file.
    unsafe { uio_printMounts(out.as_ptr(), world.raw_repository()) };
    let mut session = world.session();
    assert_eq!(read_back(out), world.run(&mut session, "mount").out);

    world.run(&mut session, "cd /pack/dir");
    let out = tmpfile_with(b"");
    // SAFETY: a live handle opened through the exported call, an open file.
    unsafe {
        let dir = uio_openDir(world.raw_repository(), c"/pack/dir".as_ptr(), 0);
        assert!(!dir.is_null());
        uio_DirHandle_print(dir, out.as_ptr());
        crate::io::uio_bridge::uio_closeDir(dir);
    }
    assert_eq!(read_back(out), b"[]pack/dir");
}

#[test]
fn a_null_file_is_a_caller_bug_that_aborts() {
    expect_abort(
        module_path!(),
        "a_null_file_is_a_caller_bug_that_aborts",
        "uio_debugInteractive() FATAL: caller bug: in is null",
        || {
            let (out, err) = (tmpfile_with(b""), tmpfile_with(b""));
            // SAFETY: the null input is the violation being tested.
            let _ = unsafe {
                run_interactive(
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    out.as_ptr(),
                    err.as_ptr(),
                )
            };
        },
    );
}

#[test]
fn a_repository_without_a_root_aborts_with_the_fatal_line() {
    expect_abort(
        module_path!(),
        "a_repository_without_a_root_aborts_with_the_fatal_line",
        "Fatal: Could not open working dir.",
        || {
            let input = tmpfile_with(b"pwd\n");
            let out = tmpfile_with(b"");
            // SAFETY: fd 2 is open; the null repository is the violation.
            let err = NonNull::new(unsafe { libc::fdopen(2, c"w".as_ptr()) }).unwrap();
            let _ = unsafe {
                run_interactive(
                    std::ptr::null_mut(),
                    input.as_ptr(),
                    out.as_ptr(),
                    err.as_ptr(),
                )
            };
        },
    );
}

#[test]
fn a_null_stream_for_the_print_functions_aborts() {
    expect_abort(
        module_path!(),
        "a_null_stream_for_the_print_functions_aborts",
        "uio_printMounts() FATAL: caller bug: outStream is null",
        || {
            // SAFETY: the null stream is the violation being tested.
            unsafe {
                uio_printMounts(std::ptr::null_mut(), std::ptr::NonNull::dangling().as_ptr())
            };
        },
    );
}

#[allow(dead_code)]
fn _type_check(session: &DebugSession) -> &[u8] {
    session.working_dir()
}
