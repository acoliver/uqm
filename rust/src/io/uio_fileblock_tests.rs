//! File blocks (windowed reads over an open handle) and the character and
//! block writers of the stream API, checked against real files on a mount.

use std::ffi::CString;
use std::mem::ManuallyDrop;
use std::sync::{Arc, Weak};

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
        let (repo, handle) = super::layered_open_tests::mount_host_root(temp.path());
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
        unsafe { super::layered_open_tests::release_host_root(self.repo, self.handle) };
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
    let got = uio_accessFileBlock(block, offset, length, &mut buffer);
    let bytes = if got > 0 && !buffer.is_null() {
        std::slice::from_raw_parts(buffer as *const u8, got as usize).to_vec()
    } else {
        Vec::new()
    };
    (got, bytes)
}

unsafe fn copy(block: *mut uio_FileBlock, offset: off_t, length: size_t) -> (c_int, Vec<u8>) {
    let mut out = vec![b'.'; length];
    let result = uio_copyFileBlock(block, offset, out.as_mut_ptr() as *mut c_char, length);
    (result, out)
}

#[test]
#[serial]
fn a_block_over_the_whole_file_serves_any_window_of_it() {
    let stack = digits_stack();
    let file = handle(&stack, "digits.txt");
    unsafe {
        let block = uio_openFileBlock(file);
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
        assert_eq!(uio_closeFileBlock(block), 0);
        uio_close(file);
    }
}

#[test]
#[serial]
fn an_access_past_the_end_leaves_the_caller_with_no_buffer() {
    let stack = digits_stack();
    let file = handle(&stack, "digits.txt");
    unsafe {
        let block = uio_openFileBlock(file);
        let mut buffer = ptr::dangling_mut::<c_char>();
        assert_eq!(uio_accessFileBlock(block, 10, 4, &mut buffer), 0);
        assert!(buffer.is_null());
        uio_closeFileBlock(block);
        uio_close(file);
    }
}

#[test]
#[serial]
fn copying_from_a_block_fills_the_callers_buffer() {
    let stack = digits_stack();
    let file = handle(&stack, "digits.txt");
    unsafe {
        let block = uio_openFileBlock(file);
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
        uio_closeFileBlock(block);
        uio_close(file);
    }
}

#[test]
#[serial]
fn a_sub_block_addresses_offsets_relative_to_its_start() {
    let stack = digits_stack();
    let file = handle(&stack, "digits.txt");
    unsafe {
        let block = uio_openFileBlock2(file, 3, 5);
        assert!(!block.is_null());
        assert_eq!(access(block, 0, 5), (5, b"34567".to_vec()));
        assert_eq!(
            access(block, 2, 10),
            (3, b"567".to_vec()),
            "clamped to the block"
        );
        assert_eq!(access(block, 5, 1).0, 0, "the block ends at offset 5");
        assert_eq!(copy(block, 1, 2), (0, b"45".to_vec()));
        uio_closeFileBlock(block);
        uio_close(file);
    }
}

#[test]
#[serial]
fn a_sub_block_must_lie_inside_the_file() {
    let stack = digits_stack();
    let file = handle(&stack, "digits.txt");
    unsafe {
        assert!(!uio_openFileBlock2(file, 0, 10).is_null());
        assert!(
            uio_openFileBlock2(file, 5, 6).is_null(),
            "ends past the file"
        );
        assert_eq!(errno_value(), libc::EINVAL);
        assert!(uio_openFileBlock2(file, -1, 2).is_null());
        assert_eq!(errno_value(), libc::EINVAL);
        assert!(
            uio_openFileBlock2(file, 1, usize::MAX).is_null(),
            "offset plus size overflows"
        );
        assert_eq!(errno_value(), libc::EINVAL);
        assert!(uio_openFileBlock2(ptr::null_mut(), 0, 1).is_null());
        assert_eq!(errno_value(), libc::EINVAL);
        assert!(uio_openFileBlock(ptr::null_mut()).is_null());
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
        let block = uio_openFileBlock(file);
        let mut buffer: *mut c_char = ptr::null_mut();
        let mut out = [0u8; 4];
        assert_eq!(uio_accessFileBlock(ptr::null_mut(), 0, 1, &mut buffer), -1);
        assert_eq!(errno_value(), libc::EINVAL);
        assert_eq!(uio_accessFileBlock(block, 0, 1, ptr::null_mut()), -1);
        assert_eq!(uio_accessFileBlock(block, -1, 1, &mut buffer), -1);
        assert_eq!(errno_value(), libc::EINVAL);
        assert_eq!(
            uio_copyFileBlock(ptr::null_mut(), 0, out.as_mut_ptr() as *mut c_char, 4),
            -1
        );
        assert_eq!(uio_copyFileBlock(block, 0, ptr::null_mut(), 4), -1);
        assert_eq!(
            uio_copyFileBlock(block, -1, out.as_mut_ptr() as *mut c_char, 4),
            -1
        );
        assert_eq!(errno_value(), libc::EINVAL);
        uio_clear_file_block_buffers(ptr::null_mut());
        uio_setFileBlockUsageHint(block, 0, 0);
        assert_eq!(uio_closeFileBlock(ptr::null_mut()), 0);
        uio_closeFileBlock(block);
        uio_close(file);
    }
}

#[test]
#[serial]
fn clearing_block_buffers_does_not_change_what_a_later_access_returns() {
    let stack = digits_stack();
    let file = handle(&stack, "digits.txt");
    unsafe {
        let block = uio_openFileBlock(file);
        assert_eq!(access(block, 1, 3), (3, b"123".to_vec()));
        uio_clear_file_block_buffers(block);
        assert_eq!(access(block, 1, 3), (3, b"123".to_vec()));
        uio_closeFileBlock(block);
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

/// Follows a handle's owners without taking part: the weak count's strong
/// side is how many of the caller, streams and blocks still hold it, and it
/// reaches zero exactly when the inner file is dropped (closed).
///
/// The handle must be live, and is left untouched.
unsafe fn watch(handle: *mut uio_Handle) -> Weak<uio_Handle> {
    let owner = ManuallyDrop::new(Arc::from_raw(handle.cast_const()));
    Arc::downgrade(&owner)
}

#[test]
#[serial]
fn a_block_still_reads_after_the_public_handle_is_closed() {
    let stack = digits_stack();
    let file = handle(&stack, "digits.txt");
    unsafe {
        let owners = watch(file);
        assert_eq!(owners.strong_count(), 1);
        let block = uio_openFileBlock(file);
        assert_eq!(owners.strong_count(), 2, "the block shares the handle");
        assert_eq!(uio_close(file), 0);
        assert_eq!(owners.strong_count(), 1, "the block keeps the file open");
        assert_eq!(access(block, 2, 4), (4, b"2345".to_vec()));
        assert_eq!(copy(block, 6, 3), (0, b"678".to_vec()));
        assert_eq!(uio_closeFileBlock(block), 0);
        assert_eq!(owners.strong_count(), 0, "the last owner closes the file");
    }
}

#[test]
#[serial]
fn a_handle_closes_with_its_last_owner_whatever_the_release_order() {
    const ORDERS: [[usize; 3]; 6] = [
        [0, 1, 2],
        [0, 2, 1],
        [1, 0, 2],
        [1, 2, 0],
        [2, 0, 1],
        [2, 1, 0],
    ];
    for order in ORDERS {
        let stack = digits_stack();
        let file = handle(&stack, "digits.txt");
        unsafe {
            let owners = watch(file);
            let whole = uio_openFileBlock(file);
            let window = uio_openFileBlock2(file, 4, 3);
            assert_eq!(owners.strong_count(), 3, "handle plus two blocks");
            let mut left = 3;
            for owner in order {
                match owner {
                    0 => assert_eq!(uio_close(file), 0),
                    1 => assert_eq!(uio_closeFileBlock(whole), 0),
                    _ => assert_eq!(uio_closeFileBlock(window), 0),
                }
                left -= 1;
                assert_eq!(owners.strong_count(), left, "order {order:?}");
                if owner != 1 && still_open(&order, owner, 1) {
                    assert_eq!(access(whole, 7, 3), (3, b"789".to_vec()), "{order:?}");
                }
                if owner != 2 && still_open(&order, owner, 2) {
                    assert_eq!(access(window, 0, 9), (3, b"456".to_vec()), "{order:?}");
                }
            }
        }
    }
}

/// Whether `block` is still unreleased once `released` has been, in `order`.
fn still_open(order: &[usize; 3], released: usize, block: usize) -> bool {
    let at = |owner: usize| order.iter().position(|o| *o == owner).unwrap();
    at(block) > at(released)
}

#[test]
#[serial]
fn a_block_that_cannot_be_created_takes_no_share_of_the_handle() {
    let stack = digits_stack();
    let file = handle(&stack, "digits.txt");
    unsafe {
        let owners = watch(file);
        assert!(uio_openFileBlock2(file, 5, 6).is_null());
        assert!(uio_openFileBlock2(file, -1, 2).is_null());
        assert!(uio_openFileBlock2(file, 1, usize::MAX).is_null());
        assert!(uio_openFileBlock2(ptr::null_mut(), 0, 1).is_null());
        assert!(uio_openFileBlock(ptr::null_mut()).is_null());
        assert_eq!(owners.strong_count(), 1);
        assert_eq!(uio_close(file), 0);
        assert_eq!(owners.strong_count(), 0, "nothing else kept the file open");
    }
}

#[test]
#[serial]
fn a_block_over_a_stream_handle_outlives_the_stream() {
    let dir = HostDir::new(&[("digits.txt", DIGITS)]);
    unsafe {
        let stream = dir.open("digits.txt", "r");
        let file = uio_streamHandle(stream);
        let owners = watch(file);
        let block = uio_openFileBlock(file);
        assert_eq!(owners.strong_count(), 2);
        assert_eq!(uio_fclose(stream), 0);
        assert_eq!(owners.strong_count(), 1);
        assert_eq!(access(block, 5, 3), (3, b"567".to_vec()));
        assert_eq!(uio_closeFileBlock(block), 0);
        assert_eq!(owners.strong_count(), 0);
    }
}

#[test]
#[serial]
fn a_block_over_an_archive_entry_outlives_the_handle() {
    let mut stack = Stack::new();
    stack.mount_zip(
        &[("digits.txt", DIGITS)],
        UIO_MOUNT_TOP | UIO_MOUNT_RDONLY,
        0,
    );
    let file = handle(&stack, "digits.txt");
    unsafe {
        let owners = watch(file);
        let block = uio_openFileBlock(file);
        assert!(!block.is_null(), "an archive entry has a size too");
        let window = uio_openFileBlock2(file, 2, 5);
        assert!(!window.is_null());
        assert_eq!(uio_close(file), 0);
        assert_eq!(access(block, 6, 9), (4, b"6789".to_vec()));
        assert_eq!(access(window, 1, 9), (4, b"3456".to_vec()));
        uio_closeFileBlock(block);
        assert_eq!(owners.strong_count(), 1);
        uio_closeFileBlock(window);
        assert_eq!(owners.strong_count(), 0);
    }
}

#[test]
#[serial]
fn a_block_survives_its_mount_stack_being_torn_down_and_rebuilt() {
    let old_stack = digits_stack();
    let file = handle(&old_stack, "digits.txt");
    let (block, owners) = unsafe {
        let block = uio_openFileBlock(file);
        let owners = watch(file);
        uio_close(file);
        (block, owners)
    };
    drop(old_stack);

    let mut new_stack = Stack::new();
    new_stack.mount_dir(&[("other.txt", b"abcdef")], UIO_MOUNT_TOP);
    let other = handle(&new_stack, "other.txt");
    unsafe {
        let other_block = uio_openFileBlock(other);
        assert_eq!(access(block, 0, 3), (3, b"012".to_vec()), "old file");
        assert_eq!(access(other_block, 0, 3), (3, b"abc".to_vec()), "new file");
        assert_eq!(owners.strong_count(), 1);
        uio_closeFileBlock(block);
        assert_eq!(owners.strong_count(), 0);
        uio_closeFileBlock(other_block);
        uio_close(other);
    }
}

#[test]
#[serial]
fn a_block_closed_on_another_thread_after_the_handle_is_gone_reads_there() {
    let stack = digits_stack();
    let file = handle(&stack, "digits.txt");
    let (block, owners) = unsafe { (uio_openFileBlock(file) as usize, watch(file)) };
    unsafe { uio_close(file) };
    let worker = std::thread::spawn(move || unsafe {
        let block = block as *mut uio_FileBlock;
        assert_eq!(access(block, 1, 2), (2, b"12".to_vec()));
        uio_closeFileBlock(block)
    });
    assert_eq!(worker.join().unwrap(), 0);
    assert_eq!(owners.strong_count(), 0);
}

#[test]
#[serial]
fn each_access_replaces_the_buffer_and_the_end_of_the_block_restarts_cleanly() {
    let stack = digits_stack();
    let file = handle(&stack, "digits.txt");
    unsafe {
        let block = uio_openFileBlock(file);
        assert_eq!(access(block, 0, 10), (10, DIGITS.to_vec()));
        assert_eq!(access(block, 3, 2), (2, b"34".to_vec()), "a shorter window");
        assert_eq!(access(block, 8, 10), (2, b"89".to_vec()), "clamped");
        let mut buffer = ptr::dangling_mut::<c_char>();
        assert_eq!(uio_accessFileBlock(block, 10, 1, &mut buffer), 0);
        assert!(buffer.is_null(), "no buffer at the end");
        assert_eq!(
            access(block, 0, 3),
            (3, b"012".to_vec()),
            "reads on after the end"
        );
        uio_clear_file_block_buffers(block);
        assert_eq!(access(block, 9, 5), (1, b"9".to_vec()));
        uio_closeFileBlock(block);
        uio_close(file);
    }
}

#[test]
#[serial]
fn copying_leaves_the_buffer_an_earlier_access_returned_alone() {
    let stack = digits_stack();
    let file = handle(&stack, "digits.txt");
    unsafe {
        let block = uio_openFileBlock(file);
        let mut buffer: *mut c_char = ptr::null_mut();
        assert_eq!(uio_accessFileBlock(block, 1, 3, &mut buffer), 3);
        assert_eq!(copy(block, 6, 2), (0, b"67".to_vec()));
        let kept = std::slice::from_raw_parts(buffer as *const u8, 3);
        assert_eq!(kept, b"123");
        uio_closeFileBlock(block);
        uio_close(file);
    }
}

#[test]
#[serial]
fn an_offset_that_overflows_the_file_position_is_refused() {
    let stack = digits_stack();
    let file = handle(&stack, "digits.txt");
    unsafe {
        let window = uio_openFileBlock2(file, 3, 5);
        let mut buffer: *mut c_char = ptr::null_mut();
        let mut out = [0u8; 2];
        assert_eq!(uio_accessFileBlock(window, off_t::MAX, 1, &mut buffer), -1);
        assert_eq!(errno_value(), libc::EINVAL);
        assert_eq!(
            uio_copyFileBlock(window, off_t::MAX, out.as_mut_ptr() as *mut c_char, 2),
            -1
        );
        assert_eq!(errno_value(), libc::EINVAL);
        uio_closeFileBlock(window);

        let whole = uio_openFileBlock(file);
        assert_eq!(access(whole, off_t::MAX, 1).0, 0, "far past the end");
        uio_closeFileBlock(whole);
        uio_close(file);
    }
}

#[test]
#[serial]
fn a_read_the_file_system_refuses_is_reported_and_leaves_the_block_closable() {
    let mut stack = Stack::new();
    stack.mount_dir(&[("digits.txt", DIGITS)], UIO_MOUNT_TOP);
    let file = unsafe { uio_open(stack.root, c("digits.txt").as_ptr(), libc::O_WRONLY, 0) };
    assert!(!file.is_null(), "a write-only open of an existing file");
    unsafe {
        let owners = watch(file);
        let block = uio_openFileBlock(file);
        assert!(!block.is_null(), "the size is still known");
        let mut buffer: *mut c_char = ptr::null_mut();
        let mut out = [0u8; 4];
        assert_eq!(uio_accessFileBlock(block, 0, 4, &mut buffer), -1);
        assert_eq!(errno_value(), libc::EBADF);
        assert_eq!(
            uio_copyFileBlock(block, 0, out.as_mut_ptr() as *mut c_char, 4),
            -1
        );
        assert_eq!(errno_value(), libc::EBADF);
        assert_eq!(uio_closeFileBlock(block), 0);
        assert_eq!(uio_close(file), 0);
        assert_eq!(owners.strong_count(), 0);
    }
}
