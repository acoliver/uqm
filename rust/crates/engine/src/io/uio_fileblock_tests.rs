//! File blocks (windowed reads over an open handle) and the character and
//! block writers of the stream API, checked against real files on a mount.

use std::ffi::CString;

use serial_test::serial;
use tempfile::TempDir;

use super::*;

fn c(path: &str) -> CString {
    CString::new(path).unwrap()
}

const DIGITS: &[u8] = b"0123456789";

/// A directory handle rooted at a temporary host directory, the way the
/// stream tests open files.
struct HostDir {
    temp: TempDir,
    repo: *mut uio_Repository,
    handle: *mut uio_DirHandle,
}

impl HostDir {
    fn new(files: &[(&str, &[u8])]) -> Self {
        if let Ok(mut topology) = get_mount_registry().lock() {
            let registry = &mut topology.mounts;
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
        HostDir { temp, repo, handle }
    }

    fn open(&self, name: &str, mode: &str) -> *mut uio_Stream {
        let stream = unsafe { uio_fopen(self.handle, c(name).as_ptr(), c(mode).as_ptr()) };
        assert!(!stream.is_null(), "fopen({name}, {mode})");
        stream
    }
}

impl Drop for HostDir {
    fn drop(&mut self) {
        unsafe {
            drop(Box::from_raw(self.handle));
            drop(Box::from_raw(self.repo));
        }
    }
}

#[test]
#[serial]
fn writing_items_to_a_stream_reports_the_item_count_and_lands_on_disk() {
    let dir = HostDir::new(&[]);
    unsafe {
        let stream = dir.open("out.bin", "w");
        assert_eq!(uio_fwrite(b"abcdef".as_ptr() as *const _, 2, 3, stream), 3);
        assert_eq!(
            uio_fwrite(b"x".as_ptr() as *const _, 0, 5, stream),
            5,
            "nothing to write is a success"
        );
        assert_eq!(uio_fwrite(ptr::null(), 1, 1, stream), 0);
        assert_eq!(
            uio_fwrite(b"x".as_ptr() as *const _, 1, 1, ptr::null_mut()),
            0
        );
        assert_eq!(uio_ferror(stream), 0);
        assert_eq!(uio_fclose(stream), 0);
    }
    assert_eq!(
        std::fs::read(dir.temp.path().join("out.bin")).unwrap(),
        b"abcdef"
    );
}

#[test]
#[serial]
fn writing_to_a_stream_opened_for_reading_sets_the_error_flag() {
    let dir = HostDir::new(&[("digits.txt", DIGITS)]);
    unsafe {
        let stream = dir.open("digits.txt", "r");
        assert_eq!(uio_fwrite(b"zz".as_ptr() as *const _, 1, 2, stream), 0);
        assert_ne!(uio_ferror(stream), 0);
        uio_clearerr(stream);
        assert_eq!(uio_ferror(stream), 0);
        uio_fclose(stream);
    }
}

#[test]
#[serial]
fn single_characters_are_read_to_the_end_and_the_end_is_flagged() {
    let dir = HostDir::new(&[("digits.txt", DIGITS)]);
    unsafe {
        let stream = dir.open("digits.txt", "r");
        for expected in DIGITS {
            assert_eq!(uio_fgetc(stream), *expected as c_int);
        }
        assert_eq!(uio_feof(stream), 0, "not flagged until a read hits the end");
        assert_eq!(uio_fgetc(stream), -1);
        assert_ne!(uio_feof(stream), 0);
        uio_clearerr(stream);
        assert_eq!(uio_feof(stream), 0);
        assert_eq!(uio_fgetc(ptr::null_mut()), -1);
        uio_fclose(stream);
    }
}

#[test]
#[serial]
fn pushing_back_the_character_just_read_makes_it_readable_again() {
    let dir = HostDir::new(&[("digits.txt", DIGITS)]);
    unsafe {
        let stream = dir.open("digits.txt", "r");
        assert_eq!(
            uio_ungetc(b'0' as c_int, stream),
            -1,
            "nothing has been read yet"
        );
        let first = uio_fgetc(stream);
        assert_eq!(first, b'0' as c_int);
        assert_eq!(uio_ungetc(first, stream), first);
        assert_eq!(uio_fgetc(stream), b'0' as c_int);
        assert_eq!(uio_ungetc(b'x' as c_int, ptr::null_mut()), -1);
        uio_fclose(stream);
    }
}

#[test]
#[serial]
fn characters_are_written_one_at_a_time() {
    let dir = HostDir::new(&[]);
    unsafe {
        let stream = dir.open("chars.txt", "w");
        for byte in b"hey" {
            assert_eq!(uio_fputc(*byte as c_int, stream), *byte as c_int);
        }
        assert_eq!(uio_fputc(b'x' as c_int, ptr::null_mut()), -1);
        assert_eq!(uio_fclose(stream), 0);
    }
    assert_eq!(
        std::fs::read(dir.temp.path().join("chars.txt")).unwrap(),
        b"hey"
    );
}
