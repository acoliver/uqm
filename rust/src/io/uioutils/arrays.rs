//! Copies of a generic array with an element inserted or a range removed.
//!
//! The arrays are raw `elementSize`-strided bytes the caller owns; the result
//! is always a new `malloc` block (from `rust_hmalloc`, so the C `uio_free`
//! releases it) that shares nothing with the input. Sizes are computed first,
//! with checked arithmetic, by [`plan_insert`] and [`plan_exclude`]; the copy
//! only runs on a plan.

use std::ffi::{c_int, c_void};
use std::fmt;
use std::ptr;

use crate::io::paths::comp_list::alloc_block;

/// Why an insert or exclude cannot be done: an argument the C would turn
/// into out-of-bounds copies or a wrapped allocation size.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArrayError {
    Negative(&'static str),
    OutOfRange(&'static str),
    SizeOverflow,
}

impl fmt::Display for ArrayError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ArrayError::Negative(name) => write!(f, "{name} is negative"),
            ArrayError::OutOfRange(what) => write!(f, "{what}"),
            ArrayError::SizeOverflow => write!(f, "the array size overflows size_t"),
        }
    }
}

impl std::error::Error for ArrayError {}

fn index(name: &'static str, value: c_int) -> Result<usize, ArrayError> {
    usize::try_from(value).map_err(|_| ArrayError::Negative(name))
}

/// Byte counts of an insert: the head copied from before the position, the
/// new element, and the tail copied from after it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InsertPlan {
    pub head: usize,
    pub element: usize,
    pub tail: usize,
}

impl InsertPlan {
    pub fn total(&self) -> usize {
        self.head + self.element + self.tail
    }
}

/// Byte counts of an exclude: the head before the removed range, the bytes
/// removed, and the tail after it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExcludePlan {
    pub head: usize,
    pub removed: usize,
    pub tail: usize,
}

impl ExcludePlan {
    pub fn total(&self) -> usize {
        self.head + self.tail
    }
}

/// Inserting at `insert_pos` (0 to `old_len`) into `old_len` elements of
/// `stride` bytes.
pub fn plan_insert(
    old_len: usize,
    insert_pos: c_int,
    stride: usize,
) -> Result<InsertPlan, ArrayError> {
    let pos = index("insertPos", insert_pos)?;
    if pos > old_len {
        return Err(ArrayError::OutOfRange(
            "insertPos is past the end of the array",
        ));
    }
    // Everything below is at most the new array's size in bytes.
    old_len
        .checked_mul(stride)
        .and_then(|bytes| bytes.checked_add(stride))
        .ok_or(ArrayError::SizeOverflow)?;
    Ok(InsertPlan {
        head: pos * stride,
        element: stride,
        tail: (old_len - pos) * stride,
    })
}

/// Removing `num_exclude` elements from `start_pos` of `old_len` elements of
/// `stride` bytes.
pub fn plan_exclude(
    old_len: usize,
    start_pos: c_int,
    num_exclude: c_int,
    stride: usize,
) -> Result<ExcludePlan, ArrayError> {
    let start = index("startPos", start_pos)?;
    let count = index("numExclude", num_exclude)?;
    let end = start
        .checked_add(count)
        .filter(|&end| end <= old_len)
        .ok_or(ArrayError::OutOfRange(
            "startPos + numExclude is past the end of the array",
        ))?;
    // Everything below is at most old_len * stride.
    old_len
        .checked_mul(stride)
        .ok_or(ArrayError::SizeOverflow)?;
    Ok(ExcludePlan {
        head: start * stride,
        removed: count * stride,
        tail: (old_len - end) * stride,
    })
}

/// `len` bytes from `src` to `dst`. A zero-length copy touches neither
/// pointer, so a null array of no elements is fine.
///
/// # Safety
/// For `len > 0`, `src` is readable and `dst` writable for `len` bytes and
/// the two ranges do not overlap.
unsafe fn copy_bytes(src: *const u8, dst: *mut u8, len: usize) {
    if len > 0 {
        // SAFETY: per the contract.
        unsafe { ptr::copy_nonoverlapping(src, dst, len) };
    }
}

/// A new block holding `array` with `element` spliced in as `plan` says.
///
/// # Safety
/// `array` is readable for `plan.head + plan.tail` bytes (null when that is
/// 0); `element` is readable for `plan.element` bytes (null when that is 0).
/// They may overlap each other; the result is a fresh block.
pub unsafe fn insert(array: *const u8, element: *const u8, plan: InsertPlan) -> *mut c_void {
    let block = alloc_block(plan.total());
    let out = block.cast::<u8>();
    // SAFETY: `out` has `plan.total()` bytes and the ranges below tile it;
    // the source ranges are readable per the contract.
    unsafe {
        copy_bytes(array, out, plan.head);
        copy_bytes(element, out.add(plan.head), plan.element);
        copy_bytes(
            array.wrapping_add(plan.head),
            out.add(plan.head + plan.element),
            plan.tail,
        );
    }
    block
}

/// A new block holding `array` without the range `plan` removes.
///
/// # Safety
/// `array` is readable for `plan.head + plan.removed + plan.tail` bytes (null
/// when that is 0).
pub unsafe fn exclude(array: *const u8, plan: ExcludePlan) -> *mut c_void {
    let block = alloc_block(plan.total());
    let out = block.cast::<u8>();
    // SAFETY: `out` has `plan.total()` bytes and the ranges below tile it;
    // the source ranges are readable per the contract.
    unsafe {
        copy_bytes(array, out, plan.head);
        copy_bytes(
            array.wrapping_add(plan.head + plan.removed),
            out.add(plan.head),
            plan.tail,
        );
    }
    block
}
