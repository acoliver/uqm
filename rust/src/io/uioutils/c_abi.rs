//! C-callable entry points, one per function `uioutils.c` defines for
//! `sc2/src/libs/uio/uioutils.h`, with the header's exact signatures.
//!
//! None carries an export attribute while the C `uioutils.c.o` provider is
//! still the selected one: exporting the same symbols from both would be a
//! duplicate-provider link. Activation gives each function its C symbol with
//! `#[export_name = "..."]` in the same change that removes `uioutils.c` from
//! the build; each function names the symbol it takes over.
//!
//! # Contracts
//!
//! * Strings are raw bytes up to the NUL; no UTF-8 is assumed.
//! * Everything returned (`char *` and the array copies) is a block from
//!   `malloc` (through `rust_hmalloc`), released by the caller with the C
//!   `uio_free` (`free`). Inputs are only read; the result never aliases them.
//! * Arrays are `oldNumElements` elements of `elementSize` bytes. A null
//!   `array` is valid when it holds no bytes (`oldNumElements` of 0 or an
//!   `elementSize` of 0), as is a null `element` of size 0. Only
//!   `oldNumElements * elementSize` bytes of `array` are read.
//! * A null pointer where the C would fault, and a position, count or size
//!   the C would turn into out-of-bounds copies or a wrapped allocation, are
//!   caller bugs: logged, then the process aborts. Nothing unwinds into C.
//! * `dosToUnixTime` is total: a date or time that is not a calendar value
//!   (the C leaves the result unspecified) gives 0.

use std::ffi::{c_char, c_int, c_void};
use std::mem::size_of;

use super::arrays::{self, ArrayError};
use super::{dos_path, dos_time};
use crate::io::dirs::c_abi::contract_violation;
use crate::io::paths::c_abi::{required, string};
use crate::io::paths::comp_list::alloc_block;

fn or_abort<T>(function: &str, planned: Result<T, ArrayError>) -> T {
    planned.unwrap_or_else(|error| contract_violation(function, &error.to_string()))
}

/// A new NUL-terminated string holding `parts` one after another.
fn alloc_joined(function: &str, parts: &[&[u8]]) -> *mut c_char {
    let len = parts
        .iter()
        .try_fold(1usize, |total, part| total.checked_add(part.len()))
        .unwrap_or_else(|| contract_violation(function, "the string size overflows size_t"));
    let block = alloc_block(len).cast::<u8>();
    let mut at = 0;
    for part in parts {
        // SAFETY: the parts and the NUL together fill the `len` bytes.
        unsafe { std::ptr::copy_nonoverlapping(part.as_ptr(), block.add(at), part.len()) };
        at += part.len();
    }
    // SAFETY: `at == len - 1`.
    unsafe { block.add(at).write(0) };
    block.cast()
}

/// Takes over `strcata`: the concatenation of two strings, newly allocated.
///
/// # Safety
/// `first` and `second` are NUL-terminated strings (they may be the same).
pub unsafe extern "C" fn strcata(first: *const c_char, second: *const c_char) -> *mut c_char {
    const F: &str = "strcata";
    // SAFETY: per the contract.
    let (first, second) = unsafe { (string(F, "first", first), string(F, "second", second)) };
    alloc_joined(F, &[first, second])
}

/// The bytes of `count` elements of `stride` bytes, or null for none.
fn array_start(function: &str, argument: &str, array: *const c_void, bytes: usize) -> *const u8 {
    if bytes > 0 {
        required(function, argument, array);
    }
    array.cast()
}

/// Takes over `insertArray`: a copy of `array` with `element` inserted at
/// `insertPos`.
///
/// # Safety
/// `array` is readable for `oldNumElements * elementSize` bytes and
/// `element` for `elementSize` bytes (either may be null when that is 0).
pub unsafe extern "C" fn insert_array(
    array: *const c_void,
    old_num_elements: usize,
    insert_pos: c_int,
    element: *const c_void,
    element_size: usize,
) -> *mut c_void {
    const F: &str = "insertArray";
    let plan = or_abort(
        F,
        arrays::plan_insert(old_num_elements, insert_pos, element_size),
    );
    let array = array_start(F, "array", array, plan.head + plan.tail);
    let element = array_start(F, "element", element, plan.element);
    // SAFETY: per the contract.
    unsafe { arrays::insert(array, element, plan) }
}

/// Takes over `insertArrayPointer`: a copy of a pointer array with `element`
/// (the pointer value itself, which may be null) inserted at `insertPos`.
///
/// # Safety
/// `array` is readable for `oldNumElements` pointers (null when 0).
pub unsafe extern "C" fn insert_array_pointer(
    array: *const *const c_void,
    old_num_elements: usize,
    insert_pos: c_int,
    element: *const c_void,
) -> *mut *mut c_void {
    const F: &str = "insertArrayPointer";
    let plan = or_abort(
        F,
        arrays::plan_insert(old_num_elements, insert_pos, size_of::<*const c_void>()),
    );
    let array = array_start(F, "array", array.cast(), plan.head + plan.tail);
    // SAFETY: the element bytes are the pointer value held in `element`.
    unsafe { arrays::insert(array, (&raw const element).cast(), plan).cast() }
}

/// Takes over `excludeArray`: a copy of `array` without `numExclude`
/// elements from `startPos`.
///
/// # Safety
/// `array` is readable for `oldNumElements * elementSize` bytes (null when 0).
pub unsafe extern "C" fn exclude_array(
    array: *const c_void,
    old_num_elements: usize,
    start_pos: c_int,
    num_exclude: c_int,
    element_size: usize,
) -> *mut c_void {
    const F: &str = "excludeArray";
    let plan = or_abort(
        F,
        arrays::plan_exclude(old_num_elements, start_pos, num_exclude, element_size),
    );
    let array = array_start(F, "array", array, plan.head + plan.removed + plan.tail);
    // SAFETY: per the contract.
    unsafe { arrays::exclude(array, plan) }
}

/// Takes over `excludeArrayPointer`: a copy of a pointer array without
/// `numExclude` pointers from `startPos`.
///
/// # Safety
/// `array` is readable for `oldNumElements` pointers (null when 0).
pub unsafe extern "C" fn exclude_array_pointer(
    array: *const *const c_void,
    old_num_elements: usize,
    start_pos: c_int,
    num_exclude: c_int,
) -> *mut *mut c_void {
    const F: &str = "excludeArrayPointer";
    let plan = or_abort(
        F,
        arrays::plan_exclude(
            old_num_elements,
            start_pos,
            num_exclude,
            size_of::<*const c_void>(),
        ),
    );
    let array = array_start(
        F,
        "array",
        array.cast(),
        plan.head + plan.removed + plan.tail,
    );
    // SAFETY: per the contract.
    unsafe { arrays::exclude(array, plan).cast() }
}

/// Takes over `dosToUnixTime`: a DOS `date` and time `tm` as `time_t`
/// seconds, read as UTC. 0 when they are not a calendar date and time, or
/// when `time_t` cannot hold the result.
pub extern "C" fn dos_to_unix_time(date: u16, tm: u16) -> libc::time_t {
    dos_time::dos_to_unix_seconds(date, tm)
        .ok()
        .and_then(|seconds| libc::time_t::try_from(seconds).ok())
        .unwrap_or(0)
}

/// Takes over `dosToUnixPath`: a newly allocated copy of `path` with the
/// DOS separators converted.
///
/// # Safety
/// `path` is a NUL-terminated string.
pub unsafe extern "C" fn dos_to_unix_path(path: *const c_char) -> *mut c_char {
    const F: &str = "dosToUnixPath";
    // SAFETY: per the contract.
    let path = unsafe { string(F, "path", path) };
    alloc_joined(F, &[&dos_path::dos_to_unix_path(path)])
}
