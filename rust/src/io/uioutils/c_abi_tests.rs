//! `fixture-spec.md` sections A, B, D, E, F7 and G against the C entry
//! points: byte contracts, `malloc` ownership through the test ledger,
//! pointers the C caller passes, and the caller-bug aborts.

use std::ffi::{c_char, c_int, c_void, CStr, CString};
use std::mem::{size_of, MaybeUninit};
use std::ptr;

use super::c_abi::*;
use crate::io::paths::lifetime_probe::{caller_free, Probe};
use crate::resource::abort_child::expect_abort;

const CANARY: u8 = 0xA5;
const SLACK: usize = 8;

/// An array of `data` followed by `SLACK` uninitialised bytes and then
/// canaries: reading past `data.len()` either reads uninitialised memory or
/// a canary, and a write would break a canary.
pub(super) struct Padded {
    bytes: Vec<MaybeUninit<u8>>,
    len: usize,
}

impl Padded {
    pub(super) fn new(data: &[u8]) -> Self {
        let mut bytes = vec![MaybeUninit::<u8>::uninit(); data.len() + 2 * SLACK];
        for (slot, &byte) in bytes.iter_mut().zip(data) {
            slot.write(byte);
        }
        for slot in &mut bytes[data.len() + SLACK..] {
            slot.write(CANARY);
        }
        Padded {
            bytes,
            len: data.len(),
        }
    }

    pub(super) fn ptr(&self) -> *const c_void {
        self.bytes.as_ptr().cast()
    }

    pub(super) fn assert_canaries_intact(&self) {
        for slot in &self.bytes[self.len + SLACK..] {
            assert_eq!(unsafe { slot.assume_init() }, CANARY);
        }
    }
}

/// Reads the `len` bytes of a block the entry point returned, checks the
/// allocation was exactly that big, and frees it the way a C caller does.
pub(super) fn take(probe: &Probe, block: *mut c_void, len: usize) -> Vec<u8> {
    assert!(!block.is_null());
    assert_eq!(probe.size_of_live(block), Some(len), "allocation size");
    let bytes = unsafe { std::slice::from_raw_parts(block.cast::<u8>(), len) }.to_vec();
    caller_free(block);
    bytes
}

fn pattern(count: usize, stride: usize) -> Vec<u8> {
    (0..count * stride).map(|i| (i % 200) as u8 + 1).collect()
}

fn insert_model(data: &[u8], stride: usize, pos: usize, element: &[u8]) -> Vec<u8> {
    let at = pos * stride;
    [&data[..at], element, &data[at..]].concat()
}

fn exclude_model(data: &[u8], stride: usize, start: usize, count: usize) -> Vec<u8> {
    [&data[..start * stride], &data[(start + count) * stride..]].concat()
}

#[test]
fn insert_at_start_middle_and_end_for_every_stride() {
    for stride in [1usize, 3, 8, 16] {
        for pos in [0usize, 2, 4] {
            let probe = Probe::start();
            let data = pattern(4, stride);
            let element = vec![0xE0; stride];
            let source = Padded::new(&data);
            let block = unsafe {
                insert_array(
                    source.ptr(),
                    4,
                    pos as c_int,
                    element.as_ptr().cast(),
                    stride,
                )
            };
            let got = take(&probe, block, 5 * stride);
            assert_eq!(
                got,
                insert_model(&data, stride, pos, &element),
                "stride {stride} pos {pos}"
            );
            source.assert_canaries_intact();
            probe.assert_balanced();
        }
    }
}

#[test]
fn insert_into_an_empty_null_array_gives_just_the_element() {
    let probe = Probe::start();
    let element = [7u8, 8, 9];
    let block = unsafe { insert_array(ptr::null(), 0, 0, element.as_ptr().cast(), 3) };
    assert_eq!(take(&probe, block, 3), element);
    probe.assert_balanced();
}

#[test]
fn insert_with_element_size_zero_gives_a_live_block_without_bytes() {
    let probe = Probe::start();
    let block = unsafe { insert_array(ptr::null(), 5, 3, ptr::null(), 0) };
    assert!(!block.is_null());
    assert_eq!(probe.size_of_live(block), Some(0));
    caller_free(block);
    probe.assert_balanced();
}

#[test]
fn insert_may_copy_an_element_that_lives_inside_the_array() {
    let probe = Probe::start();
    let data = pattern(4, 3);
    let source = Padded::new(&data);
    let inside = unsafe { source.ptr().cast::<u8>().add(6) };
    let block = unsafe { insert_array(source.ptr(), 4, 1, inside.cast(), 3) };
    assert_eq!(
        take(&probe, block, 15),
        insert_model(&data, 3, 1, &data[6..9])
    );
    probe.assert_balanced();
}

#[test]
fn insert_pointer_stores_the_pointer_value_itself() {
    let probe = Probe::start();
    let (a, b, c) = (0x1000usize, 0x2000usize, 0x3000usize);
    let array = [a as *const c_void, b as *const c_void];
    for (pos, element, want) in [
        (0, c, vec![c, a, b]),
        (1, c, vec![a, c, b]),
        (2, c, vec![a, b, c]),
        (1, 0, vec![a, 0, b]),
    ] {
        let block =
            unsafe { insert_array_pointer(array.as_ptr(), 2, pos, element as *const c_void) };
        let bytes = take(&probe, block.cast(), 3 * size_of::<usize>());
        let got: Vec<usize> = bytes
            .chunks(size_of::<usize>())
            .map(|chunk| usize::from_ne_bytes(chunk.try_into().unwrap()))
            .collect();
        assert_eq!(got, want, "pos {pos}");
    }
    probe.assert_balanced();
}

#[test]
fn exclude_ranges_for_every_stride() {
    // The C steps over the tail by sizeof (elementSize): strides 3 and 16 tell.
    for stride in [1usize, 3, 8, 16] {
        for (start, count) in [(0usize, 1usize), (1, 2), (3, 1), (0, 4), (2, 0)] {
            let probe = Probe::start();
            let data = pattern(4, stride);
            let source = Padded::new(&data);
            let block =
                unsafe { exclude_array(source.ptr(), 4, start as c_int, count as c_int, stride) };
            let got = take(&probe, block, (4 - count) * stride);
            assert_eq!(
                got,
                exclude_model(&data, stride, start, count),
                "stride {stride} {start}+{count}"
            );
            source.assert_canaries_intact();
            probe.assert_balanced();
        }
    }
}

#[test]
fn exclude_pointer_keeps_the_other_pointer_values() {
    let probe = Probe::start();
    let values = [0x10usize, 0x20, 0x30, 0x40];
    let array: Vec<*const c_void> = values.iter().map(|&v| v as *const c_void).collect();
    let block = unsafe { exclude_array_pointer(array.as_ptr(), 4, 1, 2) };
    let bytes = take(&probe, block.cast(), 2 * size_of::<usize>());
    let got: Vec<usize> = bytes
        .chunks(size_of::<usize>())
        .map(|chunk| usize::from_ne_bytes(chunk.try_into().unwrap()))
        .collect();
    assert_eq!(got, [0x10, 0x40]);
    probe.assert_balanced();
}

#[test]
fn excluding_everything_gives_a_live_empty_block() {
    let probe = Probe::start();
    let data = pattern(3, 5);
    let block = unsafe { exclude_array(data.as_ptr().cast(), 3, 0, 3, 5) };
    assert!(take(&probe, block, 0).is_empty());
    let block = unsafe { exclude_array(ptr::null(), 0, 0, 0, 5) };
    assert!(take(&probe, block, 0).is_empty());
}

fn join(first: &[u8], second: &[u8]) -> Vec<u8> {
    let (first, second) = (CString::new(first).unwrap(), CString::new(second).unwrap());
    let probe = Probe::start();
    let block = unsafe { strcata(first.as_ptr(), second.as_ptr()) };
    let len = first.as_bytes().len() + second.as_bytes().len();
    let mut got = take(&probe, block.cast(), len + 1);
    assert_eq!(got.pop(), Some(0), "NUL terminated");
    probe.assert_balanced();
    got
}

#[test]
fn strcata_joins_raw_bytes() {
    assert_eq!(join(b"", b""), b"");
    assert_eq!(join(b"a", b""), b"a");
    assert_eq!(join(b"", b"b"), b"b");
    assert_eq!(join(b"foo", b"bar"), b"foobar");
    assert_eq!(join(&[0xFF, 0xFE], &[0xC3]), [0xFF, 0xFE, 0xC3]);
    let big = vec![b'x'; 4096];
    assert_eq!(join(&big, b"y"), [big.as_slice(), b"y"].concat());
}

#[test]
fn strcata_stops_each_input_at_its_nul() {
    let probe = Probe::start();
    let second: [u8; 6] = *b"cd\0ef\0";
    let block = unsafe { strcata(c"ab".as_ptr(), second.as_ptr().cast()) };
    assert_eq!(take(&probe, block.cast(), 5), b"abcd\0");
}

#[test]
fn strcata_of_a_string_with_itself() {
    let probe = Probe::start();
    let s = c"ab";
    let block = unsafe { strcata(s.as_ptr(), s.as_ptr()) };
    assert_eq!(take(&probe, block.cast(), 5), b"abab\0");
    assert_eq!(s.to_bytes(), b"ab");
}

fn convert(path: &[u8]) -> Vec<u8> {
    let source = CString::new(path).unwrap();
    let probe = Probe::start();
    let block = unsafe { dos_to_unix_path(source.as_ptr()) };
    let mut got = take(&probe, block.cast(), path.len() + 1);
    assert_eq!(got.pop(), Some(0), "NUL terminated");
    assert_eq!(source.as_bytes(), path, "the source is never written");
    probe.assert_balanced();
    got
}

#[test]
fn dos_paths_become_unix_paths() {
    assert_eq!(convert(b""), b"");
    assert_eq!(convert(b"a\\b\\c"), b"a/b/c");
    assert_eq!(convert(b"a/b"), b"a/b");
    assert_eq!(convert(b"a\\b/c\\d"), b"a/b/c/d");
    assert_eq!(convert(b"\\a\\"), b"/a/");
    assert_eq!(convert(b"C:\\x"), b"C:/x");
    assert_eq!(
        convert(&[0xC3, b'\\', 0xFF, b'\\']),
        [0xC3, b'/', 0xFF, b'/']
    );
}

#[test]
fn a_unc_prefix_keeps_its_backslashes() {
    assert_eq!(
        convert(b"\\\\server\\share\\dir\\f"),
        b"\\\\server\\share/dir/f"
    );
    assert_eq!(
        convert(b"\\\\server/share/dir\\f"),
        b"\\\\server\\share/dir/f"
    );
    assert_eq!(convert(b"\\\\server"), b"//server");
    assert_eq!(convert(b"\\\\"), b"//");
}

#[test]
fn dos_to_unix_time_gives_time_t_seconds_or_zero() {
    assert_eq!(dos_to_unix_time(0x21, 0), 315_532_800);
    assert_eq!(dos_to_unix_time(0x52cf, 0x43c6), 1_623_745_812);
    assert_eq!(dos_to_unix_time(0xff9f, 0xbf7d), 4_354_819_198);
    assert_eq!(dos_to_unix_time(0, 0), 0);
    assert_eq!(dos_to_unix_time(0xffff, 0xffff), 0);
}

#[test]
fn entry_points_have_the_header_signatures() {
    let _: unsafe extern "C" fn(*const c_char, *const c_char) -> *mut c_char = strcata;
    let _: unsafe extern "C" fn(*const c_void, usize, c_int, *const c_void, usize) -> *mut c_void =
        insert_array;
    let _: unsafe extern "C" fn(
        *const *const c_void,
        usize,
        c_int,
        *const c_void,
    ) -> *mut *mut c_void = insert_array_pointer;
    let _: unsafe extern "C" fn(*const c_void, usize, c_int, c_int, usize) -> *mut c_void =
        exclude_array;
    let _: unsafe extern "C" fn(*const *const c_void, usize, c_int, c_int) -> *mut *mut c_void =
        exclude_array_pointer;
    let _: extern "C" fn(u16, u16) -> libc::time_t = dos_to_unix_time;
    let _: unsafe extern "C" fn(*const c_char) -> *mut c_char = dos_to_unix_path;
}

#[test]
fn no_entry_point_is_exported_while_the_c_provider_is_selected() {
    let source = include_str!("c_abi.rs");
    for line in source
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
    {
        assert!(
            !line.contains("no_mangle") && !line.contains("export_name"),
            "exported symbol would duplicate uioutils.c.o: {line}"
        );
    }
}

fn dangling() -> *const c_void {
    ptr::NonNull::<u64>::dangling().as_ptr().cast()
}

macro_rules! aborts {
    ($name:ident, $message:expr, $body:expr) => {
        #[test]
        fn $name() {
            expect_abort(module_path!(), stringify!($name), $message, || {
                #[allow(unused_unsafe)]
                unsafe {
                    let _ = $body;
                }
            });
        }
    };
}

aborts!(
    insert_with_a_negative_position_aborts,
    "insertArray() FATAL: caller bug: insertPos is negative",
    insert_array(dangling(), 2, -1, dangling(), 1)
);
aborts!(
    insert_past_the_end_aborts,
    "insertArray() FATAL: caller bug: insertPos is past the end",
    insert_array(dangling(), 2, 3, dangling(), 1)
);
aborts!(
    insert_with_an_overflowing_size_aborts,
    "insertArray() FATAL: caller bug: the array size overflows size_t",
    insert_array(dangling(), usize::MAX / 2, 0, dangling(), 4)
);
aborts!(
    insert_with_a_null_array_aborts,
    "insertArray() FATAL: caller bug: array is null",
    insert_array(ptr::null(), 2, 0, dangling(), 1)
);
aborts!(
    insert_with_a_null_element_aborts,
    "insertArray() FATAL: caller bug: element is null",
    insert_array(dangling(), 0, 0, ptr::null(), 1)
);
aborts!(
    insert_pointer_with_a_null_array_aborts,
    "insertArrayPointer() FATAL: caller bug: array is null",
    insert_array_pointer(ptr::null(), 1, 0, ptr::null())
);
aborts!(
    exclude_with_a_negative_start_aborts,
    "excludeArray() FATAL: caller bug: startPos is negative",
    exclude_array(dangling(), 2, -1, 1, 1)
);
aborts!(
    exclude_with_a_negative_count_aborts,
    "excludeArray() FATAL: caller bug: numExclude is negative",
    exclude_array(dangling(), 2, 0, -1, 1)
);
aborts!(
    exclude_past_the_end_aborts,
    "excludeArray() FATAL: caller bug: startPos + numExclude is past the end",
    exclude_array(dangling(), 2, 1, 2, 1)
);
aborts!(
    exclude_with_huge_ints_aborts,
    "excludeArray() FATAL: caller bug: startPos + numExclude is past the end",
    exclude_array(dangling(), 2, c_int::MAX, c_int::MAX, 1)
);
aborts!(
    exclude_with_an_overflowing_size_aborts,
    "excludeArray() FATAL: caller bug: the array size overflows size_t",
    exclude_array(dangling(), usize::MAX / 2, 0, 0, 4)
);
aborts!(
    exclude_with_a_null_array_aborts,
    "excludeArray() FATAL: caller bug: array is null",
    exclude_array(ptr::null(), 2, 0, 1, 1)
);
aborts!(
    exclude_pointer_past_the_end_aborts,
    "excludeArrayPointer() FATAL: caller bug: startPos + numExclude is past the end",
    exclude_array_pointer(dangling().cast(), 1, 0, 2)
);
aborts!(
    strcata_without_a_first_aborts,
    "strcata() FATAL: caller bug: first is null",
    strcata(ptr::null(), c"b".as_ptr())
);
aborts!(
    strcata_without_a_second_aborts,
    "strcata() FATAL: caller bug: second is null",
    strcata(c"a".as_ptr(), ptr::null())
);
aborts!(
    dos_to_unix_path_without_a_path_aborts,
    "dosToUnixPath() FATAL: caller bug: path is null",
    dos_to_unix_path(ptr::null())
);

#[test]
fn a_cstr_helper_sanity() {
    // The `take` helper reads exactly what the block holds.
    let probe = Probe::start();
    let block = unsafe { strcata(c"x".as_ptr(), c"".as_ptr()) };
    assert_eq!(unsafe { CStr::from_ptr(block) }.to_bytes(), b"x");
    caller_free(block.cast());
    probe.assert_balanced();
}
