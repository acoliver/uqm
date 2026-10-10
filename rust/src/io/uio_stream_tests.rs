//! Stream (FILE-like) calls against real files in a temporary directory:
//! line and character reads, writes, seeking and the status flags that the
//! C callers poll after each call.

use std::ffi::{CStr, CString};
use std::path::Path;

use serial_test::serial;
use tempfile::TempDir;

use super::*;

struct Dir {
    temp: TempDir,
    repo: *mut uio_Repository,
    handle: *mut uio_DirHandle,
}

impl Dir {
    fn new(files: &[(&str, &[u8])]) -> Self {
        if let Ok(mut registry) = get_mount_registry().lock() {
            registry.clear();
        }
        let temp = TempDir::new().unwrap();
        for (name, data) in files {
            std::fs::write(temp.path().join(name), data).unwrap();
        }
        let repo = Box::into_raw(Box::new(uio_Repository { flags: 0 }));
        let handle = Box::into_raw(Box::new(uio_DirHandle {
            path: temp.path().to_path_buf(),
            virtual_path: temp.path().to_path_buf(),
            refcount: std::sync::atomic::AtomicI32::new(1),
            repository: repo,
            root_end: temp.path().to_path_buf(),
        }));
        Dir { temp, repo, handle }
    }

    fn open(&self, name: &str, mode: &str) -> *mut uio_Stream {
        let name = CString::new(name).unwrap();
        let mode = CString::new(mode).unwrap();
        unsafe { uio_fopen(self.handle, name.as_ptr(), mode.as_ptr()) }
    }

    fn must_open(&self, name: &str, mode: &str) -> *mut uio_Stream {
        let stream = self.open(name, mode);
        assert!(!stream.is_null(), "fopen({name}, {mode}) failed");
        stream
    }

    fn host(&self, name: &str) -> std::path::PathBuf {
        self.temp.path().join(name)
    }
}

impl Drop for Dir {
    fn drop(&mut self) {
        unsafe {
            let _ = Box::from_raw(self.handle);
            let _ = Box::from_raw(self.repo);
        }
    }
}

fn gets(stream: *mut uio_Stream, size: c_int) -> Option<String> {
    let mut buf = vec![0x55 as c_char; size.max(1) as usize + 4];
    let got = unsafe { uio_fgets(buf.as_mut_ptr(), size, stream) };
    if got.is_null() {
        return None;
    }
    assert_eq!(got, buf.as_mut_ptr());
    Some(
        unsafe { CStr::from_ptr(buf.as_ptr()) }
            .to_string_lossy()
            .into_owned(),
    )
}

fn status(stream: *mut uio_Stream) -> c_int {
    unsafe { (*stream).status }
}

#[test]
#[serial]
fn fgets_returns_one_line_at_a_time_including_the_newline() {
    let dir = Dir::new(&[("t.txt", b"alpha\nbeta\r\nlast")]);
    let stream = dir.must_open("t.txt", "r");
    assert_eq!(gets(stream, 64).as_deref(), Some("alpha\n"));
    assert_eq!(gets(stream, 64).as_deref(), Some("beta\r\n"));
    assert_eq!(
        status(stream),
        UIO_STREAM_STATUS_OK,
        "a line ended by its newline has not reached the end of the file"
    );
    assert_eq!(gets(stream, 64).as_deref(), Some("last"));
    assert_eq!(
        status(stream),
        UIO_STREAM_STATUS_EOF,
        "the unterminated last line ran into the end of the file"
    );
    unsafe { uio_fclose(stream) };
}

#[test]
#[serial]
fn fgets_at_the_end_returns_null_and_flags_eof() {
    let dir = Dir::new(&[("t.txt", b"x\n")]);
    let stream = dir.must_open("t.txt", "r");
    assert_eq!(gets(stream, 8).as_deref(), Some("x\n"));
    assert_eq!(
        unsafe { uio_feof(stream) },
        0,
        "EOF is only flagged by a read that finds nothing"
    );
    assert_eq!(gets(stream, 8), None);
    assert_eq!(unsafe { uio_feof(stream) }, 1);
    unsafe { uio_fclose(stream) };
}

#[test]
#[serial]
fn fgets_stops_one_byte_short_of_the_buffer_for_the_terminator() {
    let dir = Dir::new(&[("t.txt", b"abcdefghij\n")]);
    let stream = dir.must_open("t.txt", "r");
    assert_eq!(gets(stream, 4).as_deref(), Some("abc"));
    assert_eq!(gets(stream, 4).as_deref(), Some("def"));
    assert_eq!(gets(stream, 100).as_deref(), Some("ghij\n"));
    unsafe { uio_fclose(stream) };
}

#[test]
#[serial]
fn fgets_with_a_one_byte_buffer_reads_nothing() {
    let dir = Dir::new(&[("t.txt", b"abc")]);
    let stream = dir.must_open("t.txt", "r");
    let mut buf = [0x55 as c_char; 1];
    assert!(unsafe { uio_fgets(buf.as_mut_ptr(), 1, stream) }.is_null());
    assert_eq!(
        gets(stream, 8).as_deref(),
        Some("abc"),
        "nothing was consumed"
    );
    unsafe { uio_fclose(stream) };
}

#[test]
#[serial]
fn fgets_refuses_bad_arguments() {
    let dir = Dir::new(&[("t.txt", b"abc")]);
    let stream = dir.must_open("t.txt", "r");
    let mut buf = [0 as c_char; 8];
    unsafe {
        assert!(uio_fgets(buf.as_mut_ptr(), 8, ptr::null_mut()).is_null());
        assert!(uio_fgets(ptr::null_mut(), 8, stream).is_null());
        assert!(uio_fgets(buf.as_mut_ptr(), 0, stream).is_null());
        assert!(uio_fgets(buf.as_mut_ptr(), -3, stream).is_null());
        uio_fclose(stream);
    }
}

#[test]
#[serial]
fn fgetc_walks_the_bytes_then_reports_eof() {
    let dir = Dir::new(&[("t.txt", &[0x41, 0xFF, 0x00, 0x7F])]);
    let stream = dir.must_open("t.txt", "r");
    unsafe {
        assert_eq!(uio_fgetc(stream), 0x41);
        assert_eq!(
            uio_fgetc(stream),
            0xFF,
            "bytes above 0x7f are not sign extended"
        );
        assert_eq!(uio_fgetc(stream), 0);
        assert_eq!(uio_fgetc(stream), 0x7F);
        assert_eq!(uio_fgetc(stream), -1);
        assert_eq!(uio_feof(stream), 1);
        assert_eq!(uio_fgetc(ptr::null_mut()), -1);
        uio_fclose(stream);
    }
}

#[test]
#[serial]
fn ungetc_steps_back_so_the_byte_is_read_again() {
    let dir = Dir::new(&[("t.txt", b"QRS")]);
    let stream = dir.must_open("t.txt", "r");
    unsafe {
        assert_eq!(uio_fgetc(stream), b'Q' as c_int);
        assert_eq!(uio_ungetc(b'Q' as c_int, stream), b'Q' as c_int);
        assert_eq!(uio_fgetc(stream), b'Q' as c_int);
        assert_eq!(uio_fgetc(stream), b'R' as c_int);
        assert_eq!(uio_ftell(stream), 2);
        uio_fclose(stream);
    }
}

#[test]
#[serial]
fn ungetc_pushes_back_the_byte_it_is_given_even_when_the_file_held_another() {
    let dir = Dir::new(&[("t.txt", b"QRS")]);
    let stream = dir.must_open("t.txt", "r");
    unsafe {
        assert_eq!(uio_fgetc(stream), b'Q' as c_int);
        assert_eq!(uio_ungetc(b'Z' as c_int, stream), b'Z' as c_int);
        assert_eq!(uio_ftell(stream), 0, "the parked byte is back in front");
        assert_eq!(uio_fgetc(stream), b'Z' as c_int);
        assert_eq!(uio_ftell(stream), 1);
        assert_eq!(uio_fgetc(stream), b'R' as c_int, "the file is unchanged");
        uio_fclose(stream);
    }
    assert_eq!(std::fs::read(dir.host("t.txt")).unwrap(), b"QRS");
}

#[test]
#[serial]
fn ungetc_of_end_of_file_is_refused() {
    let dir = Dir::new(&[("t.txt", b"QRS")]);
    let stream = dir.must_open("t.txt", "r");
    unsafe {
        assert_eq!(uio_fgetc(stream), b'Q' as c_int);
        assert_eq!(uio_ungetc(-1, stream), -1);
        assert_eq!(uio_ftell(stream), 1);
        assert_eq!(
            uio_fgetc(stream),
            b'R' as c_int,
            "a refused pushback parks nothing"
        );
        uio_fclose(stream);
    }
}

#[test]
#[serial]
fn a_successful_ungetc_clears_the_end_of_file_flag() {
    let dir = Dir::new(&[("t.txt", b"Q")]);
    let stream = dir.must_open("t.txt", "r");
    unsafe {
        assert_eq!(uio_fgetc(stream), b'Q' as c_int);
        assert_eq!(uio_fgetc(stream), -1);
        assert_ne!(uio_feof(stream), 0);

        assert_eq!(uio_ungetc(b'Q' as c_int, stream), b'Q' as c_int);

        assert_eq!(
            uio_feof(stream),
            0,
            "a pushed-back byte means not at the end"
        );
        assert_eq!(uio_fgetc(stream), b'Q' as c_int);
        uio_fclose(stream);
    }
}

#[test]
#[serial]
fn ungetc_before_any_read_parks_the_byte_in_front_of_the_file() {
    let dir = Dir::new(&[("t.txt", b"QRS")]);
    let stream = dir.must_open("t.txt", "r");
    unsafe {
        assert_eq!(uio_ungetc(b'X' as c_int, stream), b'X' as c_int);
        assert_eq!(uio_fgetc(stream), b'X' as c_int);
        assert_eq!(uio_fgetc(stream), b'Q' as c_int);
        assert_eq!(uio_ungetc(b'Q' as c_int, ptr::null_mut()), -1);
        uio_fclose(stream);
    }
}

#[test]
#[serial]
fn fputc_fputs_and_fwrite_append_to_the_file_in_order() {
    let dir = Dir::new(&[]);
    let stream = dir.must_open("out.txt", "w");
    let tail = b"tail";
    unsafe {
        assert_eq!(uio_fputc(b'H' as c_int, stream), b'H' as c_int);
        let text = CString::new("ello, ").unwrap();
        assert_eq!(uio_fputs(text.as_ptr(), stream), 0);
        assert_eq!(
            uio_fwrite(tail.as_ptr() as *const libc::c_void, 2, 2, stream),
            2,
            "two items of two bytes"
        );
        assert_eq!(uio_fflush(stream), 0);
        assert_eq!(status(stream), UIO_STREAM_STATUS_OK);
        uio_fclose(stream);
    }
    assert_eq!(std::fs::read(dir.host("out.txt")).unwrap(), b"Hello, tail");
}

#[test]
#[serial]
fn fwrite_of_nothing_succeeds_without_touching_the_file() {
    let dir = Dir::new(&[]);
    let stream = dir.must_open("out.txt", "w");
    let data = [1u8; 4];
    unsafe {
        assert_eq!(
            uio_fwrite(data.as_ptr() as *const libc::c_void, 4, 0, stream),
            0
        );
        assert_eq!(
            uio_fwrite(data.as_ptr() as *const libc::c_void, 0, 4, stream),
            4
        );
        uio_fclose(stream);
    }
    assert_eq!(std::fs::read(dir.host("out.txt")).unwrap(), b"");
}

#[test]
#[serial]
fn write_calls_refuse_null_arguments() {
    let dir = Dir::new(&[]);
    let stream = dir.must_open("out.txt", "w");
    let text = CString::new("x").unwrap();
    unsafe {
        assert_eq!(uio_fputc(65, ptr::null_mut()), -1);
        assert_eq!(uio_fputs(ptr::null(), stream), -1);
        assert_eq!(uio_fputs(text.as_ptr(), ptr::null_mut()), -1);
        assert_eq!(uio_fwrite(ptr::null(), 1, 1, stream), 0);
        assert_eq!(
            uio_fwrite(text.as_ptr() as *const libc::c_void, 1, 1, ptr::null_mut()),
            0
        );
        assert_eq!(
            uio_fflush(ptr::null_mut()),
            0,
            "flushing no stream succeeds"
        );
        uio_fclose(stream);
    }
}

#[test]
#[serial]
fn writing_to_a_read_only_stream_reports_an_error() {
    let dir = Dir::new(&[("t.txt", b"data")]);
    let stream = dir.must_open("t.txt", "r");
    let byte = *b"x";
    unsafe {
        assert_eq!(uio_fputc(b'x' as c_int, stream), -1);
        let text = CString::new("x").unwrap();
        assert_eq!(uio_fputs(text.as_ptr(), stream), -1);
        assert_eq!(
            uio_fwrite(byte.as_ptr() as *const libc::c_void, 1, 1, stream),
            0
        );
        assert_eq!(uio_ferror(stream), 1);
        uio_clearerr(stream);
        assert_eq!(uio_ferror(stream), 0);
        uio_fclose(stream);
    }
    assert_eq!(std::fs::read(dir.host("t.txt")).unwrap(), b"data");
}

#[test]
#[serial]
fn append_mode_keeps_existing_content_and_write_mode_truncates_it() {
    let dir = Dir::new(&[("log.txt", b"old;")]);
    let stream = dir.must_open("log.txt", "a");
    unsafe {
        let text = CString::new("new").unwrap();
        uio_fputs(text.as_ptr(), stream);
        uio_fclose(stream);
    }
    assert_eq!(std::fs::read(dir.host("log.txt")).unwrap(), b"old;new");

    let stream = dir.must_open("log.txt", "wb");
    unsafe { uio_fclose(stream) };
    assert_eq!(std::fs::read(dir.host("log.txt")).unwrap(), b"");
}

#[test]
#[serial]
fn read_write_mode_can_read_what_it_wrote() {
    let dir = Dir::new(&[("rw.txt", b"0123456789")]);
    let stream = dir.must_open("rw.txt", "r+");
    unsafe {
        assert_eq!(uio_fseek(stream, 4, SEEK_SET), 0);
        assert_eq!(uio_fputc(b'X' as c_int, stream), b'X' as c_int);
        assert_eq!(uio_fseek(stream, 0, SEEK_SET), 0);
        assert_eq!(gets(stream, 32).as_deref(), Some("0123X56789"));
        uio_fclose(stream);
    }
}

#[test]
#[serial]
fn fopen_refuses_a_missing_file_and_a_mode_without_r_w_or_a() {
    let dir = Dir::new(&[("t.txt", b"x")]);
    assert!(dir.open("absent.txt", "r").is_null());
    assert_eq!(errno_value(), libc::ENOENT);
    assert!(dir.open("t.txt", "x").is_null());
    assert_eq!(errno_value(), libc::EINVAL);
    let name = CString::new("t.txt").unwrap();
    let mode = CString::new("r").unwrap();
    unsafe {
        assert!(uio_fopen(ptr::null_mut(), name.as_ptr(), mode.as_ptr()).is_null());
        assert!(uio_fopen(dir.handle, name.as_ptr(), ptr::null()).is_null());
        assert!(uio_fopen(dir.handle, ptr::null(), mode.as_ptr()).is_null());
    }
}

fn errno_value() -> c_int {
    std::io::Error::last_os_error().raw_os_error().unwrap_or(0)
}

#[test]
#[serial]
fn fread_returns_whole_items_and_flags_eof_when_the_file_runs_out() {
    let dir = Dir::new(&[("t.bin", b"0123456789")]);
    let stream = dir.must_open("t.bin", "r");
    let mut buf = [0u8; 16];
    unsafe {
        let items = uio_fread(buf.as_mut_ptr() as *mut libc::c_void, 4, 4, stream);
        assert_eq!(items, 2, "ten bytes hold two four-byte items");
        assert_eq!(&buf[..8], b"01234567");
        assert_eq!(
            status(stream),
            UIO_STREAM_STATUS_EOF,
            "a read that comes up short has run into the end of the file"
        );
        assert_eq!(
            uio_fread(buf.as_mut_ptr() as *mut libc::c_void, 4, 4, stream),
            0
        );
        assert_eq!(uio_feof(stream), 1);
        uio_fclose(stream);
    }
}

#[test]
#[serial]
fn fread_refuses_null_and_empty_requests() {
    let dir = Dir::new(&[("t.bin", b"0123456789")]);
    let stream = dir.must_open("t.bin", "r");
    let mut buf = [0u8; 4];
    let out = buf.as_mut_ptr() as *mut libc::c_void;
    unsafe {
        assert_eq!(uio_fread(out, 1, 4, ptr::null_mut()), 0);
        assert_eq!(uio_fread(ptr::null_mut(), 1, 4, stream), 0);
        assert_eq!(uio_fread(out, 0, 4, stream), 0);
        assert_eq!(uio_fread(out, 4, 0, stream), 0);
        assert_eq!(uio_ftell(stream), 0, "refused reads do not consume input");
        uio_fclose(stream);
    }
}

#[test]
#[serial]
fn fseek_and_ftell_follow_each_whence() {
    let dir = Dir::new(&[("t.bin", b"0123456789")]);
    let stream = dir.must_open("t.bin", "r");
    unsafe {
        assert_eq!(uio_fseek(stream, 3, SEEK_SET), 0);
        assert_eq!(uio_ftell(stream), 3);
        assert_eq!(uio_fseek(stream, 2, SEEK_CUR), 0);
        assert_eq!(uio_ftell(stream), 5);
        assert_eq!(uio_fseek(stream, -1, SEEK_END), 0);
        assert_eq!(uio_ftell(stream), 9);
        assert_eq!(uio_fgetc(stream), b'9' as c_int);
        assert_eq!(uio_fseek(stream, 0, 99), -1, "unknown whence");
        assert_eq!(uio_fseek(ptr::null_mut(), 0, SEEK_SET), -1);
        assert_eq!(uio_ftell(ptr::null_mut()), -1);
        uio_fclose(stream);
    }
}

#[test]
#[serial]
fn fseek_before_the_start_of_the_file_fails_for_every_whence() {
    let dir = Dir::new(&[("t.bin", b"0123456789")]);
    let stream = dir.must_open("t.bin", "r");
    unsafe {
        assert_eq!(uio_fseek(stream, -1, SEEK_SET), -1);
        assert_eq!(
            uio_ftell(stream),
            0,
            "a refused seek leaves the position alone"
        );
        assert_eq!(uio_fseek(stream, -1, SEEK_CUR), -1);
        assert_eq!(uio_fseek(stream, -11, SEEK_END), -1);
        assert_eq!(uio_ftell(stream), 0);
        uio_fclose(stream);
    }
}

#[test]
#[serial]
fn streams_report_their_underlying_handle() {
    let dir = Dir::new(&[("t.bin", b"abc")]);
    let stream = dir.must_open("t.bin", "r");
    unsafe {
        assert_eq!(uio_streamHandle(stream), (*stream).handle);
        assert!(uio_streamHandle(ptr::null_mut()).is_null());
        uio_fclose(stream);
        assert_eq!(uio_fclose(ptr::null_mut()), 0);
    }
}

#[test]
#[serial]
fn host_path_is_unchanged_by_reading() {
    let dir = Dir::new(&[("t.txt", b"keep")]);
    let stream = dir.must_open("t.txt", "r");
    assert_eq!(gets(stream, 16).as_deref(), Some("keep"));
    unsafe { uio_fclose(stream) };
    assert!(Path::new(&dir.host("t.txt")).exists());
    assert_eq!(std::fs::read(dir.host("t.txt")).unwrap(), b"keep");
}

#[cfg(test)]
#[path = "uio_pushback_tests.rs"]
mod pushback_tests;
