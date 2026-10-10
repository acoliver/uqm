//! Test-only owner accounting for the blocks `comp_list` allocates.
//!
//! `comp_list::alloc_block` / `free_block` report every real `rust_hmalloc` /
//! `rust_hfree` here, so a test sees which blocks are live and how often each
//! address was released. The ledger is per thread (each test has its own),
//! observes only, and lets the real allocation and free run. A release of an
//! address that is not live is a double free or a free of memory the module
//! never allocated; it panics before the real free.

use std::cell::RefCell;
use std::collections::HashMap;
use std::ffi::c_void;

#[derive(Default)]
struct Ledger {
    /// Live block address to its size.
    live: HashMap<usize, usize>,
    /// Releases per address since that address was last allocated.
    releases: HashMap<usize, u32>,
    allocations: usize,
    attempted_releases: usize,
}

thread_local! {
    static LEDGER: RefCell<Ledger> = RefCell::new(Ledger::default());
}

pub(crate) fn allocated(block: *mut c_void, size: usize) {
    LEDGER.with_borrow_mut(|ledger| {
        let address = block as usize;
        let previous = ledger.live.insert(address, size);
        assert!(
            previous.is_none(),
            "malloc returned {address:#x} while it was live"
        );
        ledger.releases.remove(&address);
        ledger.allocations += 1;
    });
}

pub(crate) fn released(block: *mut c_void) {
    let address = block as usize;
    let freed = LEDGER.with_borrow_mut(|ledger| {
        ledger.attempted_releases += 1;
        let freed = ledger.live.remove(&address);
        if freed.is_some() {
            *ledger.releases.entry(address).or_default() += 1;
        }
        freed
    });
    assert!(
        freed.is_some(),
        "release of {address:#x}: not a live block (double free or never allocated here)"
    );
}

/// The C caller's `free` of a block the module returned.
pub(crate) fn caller_free(block: *mut c_void) {
    released(block);
    // SAFETY: a live block from `rust_hmalloc`, which is libc malloc.
    unsafe { libc::free(block) };
}

/// A fresh ledger for the calling test thread.
pub(crate) struct Probe;

impl Probe {
    pub(crate) fn start() -> Self {
        LEDGER.with_borrow_mut(|ledger| *ledger = Ledger::default());
        Probe
    }

    pub(crate) fn live_blocks(&self) -> usize {
        LEDGER.with_borrow(|ledger| ledger.live.len())
    }

    pub(crate) fn live_bytes(&self) -> usize {
        LEDGER.with_borrow(|ledger| ledger.live.values().sum())
    }

    pub(crate) fn is_live<T>(&self, block: *const T) -> bool {
        LEDGER.with_borrow(|ledger| ledger.live.contains_key(&(block as usize)))
    }

    pub(crate) fn size_of_live<T>(&self, block: *const T) -> Option<usize> {
        LEDGER.with_borrow(|ledger| ledger.live.get(&(block as usize)).copied())
    }

    /// Releases of `block` since it was allocated.
    pub(crate) fn releases_of<T>(&self, block: *const T) -> u32 {
        LEDGER.with_borrow(|ledger| ledger.releases.get(&(block as usize)).copied().unwrap_or(0))
    }

    pub(crate) fn allocations(&self) -> usize {
        LEDGER.with_borrow(|ledger| ledger.allocations)
    }

    pub(crate) fn attempted_releases(&self) -> usize {
        LEDGER.with_borrow(|ledger| ledger.attempted_releases)
    }

    /// Every block ever allocated has been released.
    pub(crate) fn assert_balanced(&self) {
        let (live, allocations) = LEDGER.with_borrow(|l| (l.live.len(), l.allocations));
        assert_eq!(live, 0, "{live} of {allocations} blocks never released");
    }
}
