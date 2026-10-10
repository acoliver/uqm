//! Chunked string arena with the exact contract of the historical C
//! `stringbank` (`sc2/src/libs/resource/stringbank.c`).
//!
//! A bank hands out NUL-terminated byte strings that stay at a stable address
//! until the bank itself is dropped. Strings are byte strings, not Unicode:
//! whatever bytes the caller supplies (valid UTF-8 or not) are stored and
//! returned unchanged.
//!
//! Storage is a singly linked chain of fixed-size chunks. A chunk keeps the C
//! layout (`char data[STRBANK_CHUNK_SIZE]; int len; chunk *next;`, 1024 bytes),
//! and the handle given to C is the pointer to the first chunk, exactly like
//! `stringbank *`. The behaviours that callers can observe, and that are kept
//! on purpose:
//!
//! * a string of `N` bytes occupies `N + 1` bytes and is refused (`None`, the
//!   C `NULL`) when `N + 1 > STRBANK_CHUNK_SIZE`;
//! * an existing chunk accepts a string only while `N + 1 < remaining`
//!   (strictly less), otherwise the next chunk is tried and a new chunk is
//!   appended at the tail when none fits, so a string of exactly
//!   `STRBANK_CHUNK_SIZE` bytes always lands alone in a freshly appended chunk;
//! * `add_or_find_*` scans the chunks in chain order and returns the first
//!   identical entry;
//! * `add_*substring` takes at most the first `n` bytes of the string.
//!
//! Every operation takes `&self`: the bank is append-only, so a string that
//! has been handed out is never rewritten, and interior mutability keeps the
//! returned references valid while more strings are added.

use std::cell::{Cell, OnceCell};
use std::ffi::{c_int, CStr};
use std::iter::successors;
use std::mem::size_of;
use std::num::{NonZeroU8, NonZeroUsize};
use std::slice;

pub mod c_abi;

#[cfg(test)]
mod model_tests;
#[cfg(test)]
mod tests;

/// Usable bytes per chunk; the C `STRBANK_CHUNK_SIZE` macro.
pub const STRBANK_CHUNK_SIZE: usize = 1024 - size_of::<*const ()>() - size_of::<c_int>();

/// A bank, which is also its first chunk (the C `stringbank` typedef).
#[repr(C)]
pub struct StringBank {
    data: [Cell<u8>; STRBANK_CHUNK_SIZE],
    len: Cell<c_int>,
    next: OnceCell<Box<StringBank>>,
}

const _: () = assert!(size_of::<StringBank>() == 1024);

impl Drop for StringBank {
    /// Unlinks the chain iteratively so a very long chain cannot exhaust the
    /// stack the way recursive `Box` drops (and the C `StringBank_Free`) do.
    fn drop(&mut self) {
        let mut next = self.next.take();
        while let Some(mut chunk) = next {
            next = chunk.next.take();
        }
    }
}

impl StringBank {
    /// `StringBank_Create`.
    pub fn create() -> Box<Self> {
        Box::new(Self::empty_chunk())
    }

    fn empty_chunk() -> Self {
        Self {
            data: [const { Cell::new(0) }; STRBANK_CHUNK_SIZE],
            len: Cell::new(0),
            next: OnceCell::new(),
        }
    }

    /// `StringBank_AddString`: copies `s` into the bank without looking for
    /// duplicates. `None` when the string cannot fit in a chunk.
    pub fn add_string(&self, s: &CStr) -> Option<&CStr> {
        self.add_bytes(s.to_bytes())
    }

    /// `StringBank_AddSubstring`: copies the first `n` bytes of `s` (all of
    /// it when `n` exceeds its length).
    pub fn add_substring(&self, s: &CStr, n: usize) -> Option<&CStr> {
        self.add_bytes(prefix(s, n))
    }

    /// `StringBank_AddOrFindString`: returns the first identical entry in the
    /// bank, adding `s` only when there is none.
    pub fn add_or_find_string(&self, s: &CStr) -> Option<&CStr> {
        self.add_or_find_bytes(s.to_bytes())
    }

    /// `StringBank_AddOrFindSubstring`.
    pub fn add_or_find_substring(&self, s: &CStr, n: usize) -> Option<&CStr> {
        self.add_or_find_bytes(prefix(s, n))
    }

    /// `SplitString`: splits `s` at `delimiter` into at most `capacity`
    /// pieces and files every piece with `add_or_find_*`. The last piece
    /// yielded keeps the unsplit remainder of `s`. The iterator yields
    /// `min(capacity, delimiter count + 1)` items, which is the count the C
    /// function returns; an item is `None` where the C result slot would hold
    /// `NULL` because the piece does not fit in a chunk.
    pub fn split_string<'bank, 'text>(
        &'bank self,
        s: &'text CStr,
        delimiter: NonZeroU8,
        capacity: NonZeroUsize,
    ) -> Pieces<'bank, 'text> {
        Pieces {
            bank: self,
            rest: s.to_bytes(),
            delimiter: delimiter.get(),
            slots_left: capacity.get(),
            finished: false,
        }
    }

    /// `bytes` must not contain a NUL.
    fn add_bytes(&self, bytes: &[u8]) -> Option<&CStr> {
        let needed = bytes.len() + 1;
        if needed > STRBANK_CHUNK_SIZE {
            return None;
        }
        let chunk = match self.chunks().find(|chunk| needed < chunk.remaining()) {
            Some(chunk) => chunk,
            None => self.append_chunk(),
        };
        Some(chunk.append(bytes))
    }

    /// `bytes` must not contain a NUL.
    fn add_or_find_bytes(&self, bytes: &[u8]) -> Option<&CStr> {
        if bytes.len() + 1 > STRBANK_CHUNK_SIZE {
            return None;
        }
        match self.chunks().find_map(|chunk| chunk.find(bytes)) {
            Some(found) => Some(found),
            None => self.add_bytes(bytes),
        }
    }

    fn chunks(&self) -> impl Iterator<Item = &StringBank> {
        successors(Some(self), |chunk| chunk.next.get().map(|next| &**next))
    }

    fn append_chunk(&self) -> &StringBank {
        let tail = self
            .chunks()
            .last()
            .expect("a chain always contains its first chunk");
        tail.next.get_or_init(|| Box::new(Self::empty_chunk()))
    }

    fn used_len(&self) -> usize {
        // `len` only ever holds values written by `append`, within 0..=SIZE.
        self.len.get() as usize
    }

    fn remaining(&self) -> usize {
        STRBANK_CHUNK_SIZE - self.used_len()
    }

    /// Writes `bytes` and a NUL at the end of the used region. The caller has
    /// checked that `bytes.len() + 1 <= self.remaining()`.
    fn append(&self, bytes: &[u8]) -> &CStr {
        let start = self.used_len();
        let end = start + bytes.len();
        for (cell, byte) in self.data[start..end].iter().zip(bytes) {
            cell.set(*byte);
        }
        self.data[end].set(0);
        self.len.set((end + 1) as c_int);
        entry_cstr(&self.used()[start..])
    }

    /// The committed bytes of this chunk: a sequence of NUL-terminated
    /// entries.
    fn used(&self) -> &[u8] {
        // SAFETY: the first `used_len()` cells are only ever written by
        // `append` before `len` is advanced past them, and they are never
        // written again, so no cell in this slice changes while the borrow is
        // alive. `Cell<u8>` has the layout of `u8`.
        unsafe { slice::from_raw_parts(self.data.as_ptr().cast::<u8>(), self.used_len()) }
    }

    fn find(&self, bytes: &[u8]) -> Option<&CStr> {
        self.used()
            .split_inclusive(|byte| *byte == 0)
            .find(|entry| entry.len() == bytes.len() + 1 && entry.starts_with(bytes))
            .map(entry_cstr)
    }
}

/// `entry` is a committed entry: every entry ends in exactly one NUL.
fn entry_cstr(entry: &[u8]) -> &CStr {
    CStr::from_bytes_with_nul(entry).expect("committed entries are NUL-terminated")
}

fn prefix(s: &CStr, n: usize) -> &[u8] {
    let bytes = s.to_bytes();
    &bytes[..n.min(bytes.len())]
}

/// Pieces produced by [`StringBank::split_string`].
pub struct Pieces<'bank, 'text> {
    bank: &'bank StringBank,
    rest: &'text [u8],
    delimiter: u8,
    slots_left: usize,
    finished: bool,
}

impl<'bank> Iterator for Pieces<'bank, '_> {
    type Item = Option<&'bank CStr>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.finished {
            return None;
        }
        self.slots_left -= 1;
        if self.slots_left > 0 {
            if let Some(at) = self.rest.iter().position(|byte| *byte == self.delimiter) {
                let piece = &self.rest[..at];
                self.rest = &self.rest[at + 1..];
                return Some(self.bank.add_or_find_bytes(piece));
            }
        }
        self.finished = true;
        Some(self.bank.add_or_find_bytes(self.rest))
    }
}
