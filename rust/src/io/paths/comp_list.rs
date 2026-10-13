//! The `uio_PathComp` list: the `#[repr(C)]` node, its allocation rules, and
//! an owning handle for Rust callers.
//!
//! # Ownership
//!
//! `uio_PathComp_new` takes over `name`: the node does not copy it, and
//! `uio_PathComp_delete` frees it. The node and every name are separate
//! blocks from `rust_hmalloc`, which is libc `malloc`, so the C
//! `uio_free` (`#define uio_free free`) releases either. A list is owned by
//! its first node: `next` links own the rest, `up` links are borrowed back
//! references and are never followed when deleting. A list built with an
//! `up` argument (`uio_makePathComps`) therefore does not own that node.

use std::ffi::{c_char, c_void};
use std::marker::PhantomData;
use std::mem::{align_of, offset_of, size_of};
use std::ptr;

use crate::memory::{rust_hfree, rust_hmalloc};

/// `struct uio_PathComp` of `paths.h`.
#[repr(C)]
#[derive(Debug)]
pub struct PathComp {
    /// NUL-terminated name.
    pub name: *mut c_char,
    /// `strlen(name)`, kept for fast lookups.
    pub name_len: usize,
    /// Next component of the path.
    pub next: *mut PathComp,
    /// Previous component of the path (borrowed).
    pub up: *mut PathComp,
}

// The C layout on the supported 64-bit and 32-bit targets: four
// pointer-sized fields in declaration order, no padding.
const _: () = {
    assert!(size_of::<PathComp>() == 4 * size_of::<usize>());
    assert!(align_of::<PathComp>() == align_of::<usize>());
    assert!(offset_of!(PathComp, name) == 0);
    assert!(offset_of!(PathComp, name_len) == size_of::<usize>());
    assert!(offset_of!(PathComp, next) == 2 * size_of::<usize>());
    assert!(offset_of!(PathComp, up) == 3 * size_of::<usize>());
};

/// A new block of `size` bytes; every allocation of this module goes through
/// here so tests can account for its owners.
pub(crate) fn alloc_block(size: usize) -> *mut c_void {
    // SAFETY: aborts on out-of-memory; the block is owned by the caller.
    let block = unsafe { rust_hmalloc(size) };
    #[cfg(test)]
    super::lifetime_probe::allocated(block, size);
    block
}

/// Releases a block from [`alloc_block`] (or a `malloc` block that a node
/// adopted).
///
/// # Safety
/// `block` is a live block that nothing else frees or uses afterwards.
unsafe fn free_block(block: *mut c_void) {
    #[cfg(test)]
    super::lifetime_probe::released(block);
    // SAFETY: per the contract.
    unsafe { rust_hfree(block) }
}

/// A new heap copy of `bytes` with a NUL after it, for a node to own.
pub fn alloc_name(bytes: &[u8]) -> *mut c_char {
    // SAFETY: the block has room for `bytes` and the NUL; it aborts on OOM.
    unsafe {
        let name = alloc_block(bytes.len() + 1).cast::<u8>();
        ptr::copy_nonoverlapping(bytes.as_ptr(), name, bytes.len());
        name.add(bytes.len()).write(0);
        name.cast()
    }
}

/// `uio_PathComp_new`. `next` starts null (the C leaves it unset).
///
/// # Safety
/// `name` is a `rust_hmalloc`/`malloc` block holding a NUL-terminated string
/// of `name_len` bytes that nothing else frees. `up` is null or a live node.
pub unsafe fn new(name: *mut c_char, name_len: usize, up: *mut PathComp) -> *mut PathComp {
    // SAFETY: the block is large enough for one node; it aborts on OOM.
    unsafe {
        let node = alloc_block(size_of::<PathComp>()).cast::<PathComp>();
        node.write(PathComp {
            name,
            name_len,
            next: ptr::null_mut(),
            up,
        });
        node
    }
}

/// `uio_PathComp_delete`: frees `head` and every node reachable through
/// `next`, each with its name.
///
/// # Safety
/// `head` is null or the first node of a list made by this module, and no
/// node of it is used afterwards.
pub unsafe fn delete(mut head: *mut PathComp) {
    while !head.is_null() {
        // SAFETY: a live node per the contract; the next link is read first.
        unsafe {
            let next = (*head).next;
            free_block((*head).name.cast());
            free_block(head.cast());
            head = next;
        }
    }
}

/// `uio_countPathComps`.
///
/// # Safety
/// `comp` is null or the head of a well-formed list.
pub unsafe fn count(mut comp: *const PathComp) -> usize {
    let mut count = 0;
    while !comp.is_null() {
        count += 1;
        // SAFETY: live node per the contract.
        comp = unsafe { (*comp).next };
    }
    count
}

/// `uio_lastPathComp`: the last node reachable through `next`, null for null.
///
/// # Safety
/// `comp` is null or the head of a well-formed list.
pub unsafe fn last(mut comp: *mut PathComp) -> *mut PathComp {
    if comp.is_null() {
        return comp;
    }
    // SAFETY: live nodes per the contract.
    unsafe {
        while !(*comp).next.is_null() {
            comp = (*comp).next;
        }
    }
    comp
}

/// A list of nodes, one per name, whose first node has the given `up`.
/// Each name is copied. Empty `names` gives null.
///
/// # Safety
/// `up` is null or a live node, and outlives the list's use of it.
pub unsafe fn from_names<'a>(
    names: impl IntoIterator<Item = &'a [u8]>,
    mut up: *mut PathComp,
) -> *mut PathComp {
    let mut head: *mut PathComp = ptr::null_mut();
    let mut link: *mut *mut PathComp = &raw mut head;
    for name in names {
        // SAFETY: a fresh heap name; `link` points at `head` or the `next`
        // field of the node made on the previous turn.
        unsafe {
            let node = new(alloc_name(name), name.len(), up);
            *link = node;
            up = node;
            link = &raw mut (*node).next;
        }
    }
    head
}

/// Owner of a list from this module for Rust callers; deleting it with
/// [`delete`] when dropped.
#[derive(Debug)]
pub struct OwnedPathComps {
    head: *mut PathComp,
}

impl OwnedPathComps {
    /// A list of copies of `names`, none above it.
    pub fn new<'a>(names: impl IntoIterator<Item = &'a [u8]>) -> Self {
        // SAFETY: a null `up` has no lifetime to respect.
        Self {
            head: unsafe { from_names(names, ptr::null_mut()) },
        }
    }

    /// Takes over the list starting at `head`.
    ///
    /// # Safety
    /// `head` is null or the first node of a well-formed list from this
    /// module that nothing else owns or frees.
    pub unsafe fn from_raw(head: *mut PathComp) -> Self {
        Self { head }
    }

    /// The first node, still owned by `self`.
    pub fn as_ptr(&self) -> *const PathComp {
        self.head
    }

    /// Gives the list to the caller, who must [`delete`] it.
    pub fn into_raw(self) -> *mut PathComp {
        let head = self.head;
        std::mem::forget(self);
        head
    }

    pub fn len(&self) -> usize {
        // SAFETY: the list is owned and well formed.
        unsafe { count(self.head) }
    }

    pub fn is_empty(&self) -> bool {
        self.head.is_null()
    }

    /// The names in order.
    pub fn names(&self) -> Names<'_> {
        Names {
            next: self.head,
            owner: PhantomData,
        }
    }
}

impl Drop for OwnedPathComps {
    fn drop(&mut self) {
        // SAFETY: the list is owned and not used again.
        unsafe { delete(self.head) }
    }
}

/// Iterator over the names of an [`OwnedPathComps`]; each is the first
/// `name_len` bytes of the node's name.
pub struct Names<'a> {
    next: *const PathComp,
    owner: PhantomData<&'a OwnedPathComps>,
}

impl<'a> Iterator for Names<'a> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<&'a [u8]> {
        if self.next.is_null() {
            return None;
        }
        // SAFETY: the owner outlives `'a` and keeps every node and name alive;
        // names hold `name_len` bytes.
        unsafe {
            let node = &*self.next;
            self.next = node.next;
            Some(std::slice::from_raw_parts(
                node.name.cast::<u8>(),
                node.name_len,
            ))
        }
    }
}
