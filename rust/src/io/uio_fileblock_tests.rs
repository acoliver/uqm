//! File blocks (windowed reads over an open handle) and the character and
//! block writers of the stream API, checked against real files on a mount.

use std::ffi::CString;

use serial_test::serial;
use tempfile::TempDir;

use super::layered_open_tests::Stack;
use super::*;

fn errno_value() -> c_int {
    std::io::Error::last_os_error().raw_os_error().unwrap_or(0)
}

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

fn digits_stack() -> Stack {
    let mut stack = Stack::new();
    stack.mount_dir(&[("digits.txt", DIGITS)], UIO_MOUNT_TOP);
    stack
}

fn handle(stack: &Stack, name: &str) -> *mut uio_Handle {
    let handle = unsafe { uio_open(stack.root, c(name).as_ptr(), 0, 0) };
    assert!(!handle.is_null(), "open {name}");
    handle
}

unsafe fn access(block: *mut uio_FileBlock, offset: off_t, length: size_t) -> (isize, Vec<u8>) {
    let mut buffer: *mut c_char = ptr::null_mut();
    let got = uio_access_file_block(block, offset, length, &mut buffer);
    let bytes = if got > 0 && !buffer.is_null() {
        std::slice::from_raw_parts(buffer as *const u8, got as usize).to_vec()
    } else {
        Vec::new()
    };
    (got, bytes)
}

unsafe fn copy(block: *mut uio_FileBlock, offset: off_t, length: size_t) -> (c_int, Vec<u8>) {
    let mut out = vec![b'.'; length];
    let result = uio_copy_file_block(block, offset, out.as_mut_ptr() as *mut c_char, length);
    (result, out)
}

#[test]
#[serial]
fn a_block_over_the_whole_file_serves_any_window_of_it() {
    let stack = digits_stack();
    let file = handle(&stack, "digits.txt");
    unsafe {
        let block = uio_open_file_block(file);
        assert!(!block.is_null());
        assert_eq!(access(block, 0, 10), (10, DIGITS.to_vec()));
        assert_eq!(access(block, 3, 4), (4, b"3456".to_vec()));
        assert_eq!(
            access(block, 8, 10),
            (2, b"89".to_vec()),
            "clamped to the end"
        );
        assert_eq!(access(block, 10, 4).0, 0, "at the end");
        assert_eq!(access(block, 50, 4).0, 0, "past the end");
        assert_eq!(uio_close_file_block(block), 0);
        uio_close(file);
    }
}

#[test]
#[serial]
fn an_access_past_the_end_leaves_the_caller_with_no_buffer() {
    let stack = digits_stack();
    let file = handle(&stack, "digits.txt");
    unsafe {
        let block = uio_open_file_block(file);
        let mut buffer = ptr::dangling_mut::<c_char>();
        assert_eq!(uio_access_file_block(block, 10, 4, &mut buffer), 0);
        assert!(buffer.is_null());
        uio_close_file_block(block);
        uio_close(file);
    }
}

#[test]
#[serial]
fn copying_from_a_block_fills_the_callers_buffer() {
    let stack = digits_stack();
    let file = handle(&stack, "digits.txt");
    unsafe {
        let block = uio_open_file_block(file);
        assert_eq!(copy(block, 2, 5), (0, b"23456".to_vec()));
        let (result, out) = copy(block, 8, 6);
        assert_eq!(result, 0);
        assert_eq!(&out[..2], b"89", "only what exists is copied");
        assert_eq!(&out[2..], b"....", "the rest of the buffer is untouched");
        assert_eq!(
            copy(block, 12, 3),
            (0, b"...".to_vec()),
            "past the end copies nothing"
        );
        uio_close_file_block(block);
        uio_close(file);
    }
}

#[test]
#[serial]
fn a_sub_block_addresses_offsets_relative_to_its_start() {
    let stack = digits_stack();
    let file = handle(&stack, "digits.txt");
    unsafe {
        let block = uio_open_file_block2(file, 3, 5);
        assert!(!block.is_null());
        assert_eq!(access(block, 0, 5), (5, b"34567".to_vec()));
        assert_eq!(
            access(block, 2, 10),
            (3, b"567".to_vec()),
            "clamped to the block"
        );
        assert_eq!(access(block, 5, 1).0, 0, "the block ends at offset 5");
        assert_eq!(copy(block, 1, 2), (0, b"45".to_vec()));
        uio_close_file_block(block);
        uio_close(file);
    }
}

#[test]
#[serial]
fn a_sub_block_must_lie_inside_the_file() {
    let stack = digits_stack();
    let file = handle(&stack, "digits.txt");
    unsafe {
        assert!(!uio_open_file_block2(file, 0, 10).is_null());
        assert!(
            uio_open_file_block2(file, 5, 6).is_null(),
            "ends past the file"
        );
        assert_eq!(errno_value(), libc::EINVAL);
        assert!(uio_open_file_block2(file, -1, 2).is_null());
        assert_eq!(errno_value(), libc::EINVAL);
        assert!(
            uio_open_file_block2(file, 1, usize::MAX).is_null(),
            "offset plus size overflows"
        );
        assert_eq!(errno_value(), libc::EINVAL);
        assert!(uio_open_file_block2(ptr::null_mut(), 0, 1).is_null());
        assert_eq!(errno_value(), libc::EINVAL);
        assert!(uio_open_file_block(ptr::null_mut()).is_null());
        assert_eq!(errno_value(), libc::EINVAL);
        uio_close(file);
    }
}

#[test]
#[serial]
fn block_calls_refuse_missing_arguments_and_negative_offsets() {
    let stack = digits_stack();
    let file = handle(&stack, "digits.txt");
    unsafe {
        let block = uio_open_file_block(file);
        let mut buffer: *mut c_char = ptr::null_mut();
        let mut out = [0u8; 4];
        assert_eq!(
            uio_access_file_block(ptr::null_mut(), 0, 1, &mut buffer),
            -1
        );
        assert_eq!(errno_value(), libc::EINVAL);
        assert_eq!(uio_access_file_block(block, 0, 1, ptr::null_mut()), -1);
        assert_eq!(uio_access_file_block(block, -1, 1, &mut buffer), -1);
        assert_eq!(errno_value(), libc::EINVAL);
        assert_eq!(
            uio_copy_file_block(ptr::null_mut(), 0, out.as_mut_ptr() as *mut c_char, 4),
            -1
        );
        assert_eq!(uio_copy_file_block(block, 0, ptr::null_mut(), 4), -1);
        assert_eq!(
            uio_copy_file_block(block, -1, out.as_mut_ptr() as *mut c_char, 4),
            -1
        );
        assert_eq!(errno_value(), libc::EINVAL);
        uio_clear_file_block_buffers(ptr::null_mut());
        uio_set_file_block_usage_hint(block, 0, 0);
        assert_eq!(uio_close_file_block(ptr::null_mut()), 0);
        uio_close_file_block(block);
        uio_close(file);
    }
}

#[test]
#[serial]
fn clearing_block_buffers_does_not_change_what_a_later_access_returns() {
    let stack = digits_stack();
    let file = handle(&stack, "digits.txt");
    unsafe {
        let block = uio_open_file_block(file);
        assert_eq!(access(block, 1, 3), (3, b"123".to_vec()));
        uio_clear_file_block_buffers(block);
        assert_eq!(access(block, 1, 3), (3, b"123".to_vec()));
        uio_close_file_block(block);
        uio_close(file);
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
