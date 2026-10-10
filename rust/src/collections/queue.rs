//! Pooled doubly linked queue shared with the remaining C callers.
//!
//! This is the only implementation of the `QUEUE`/`LINK` API declared in
//! `sc2/src/uqm/displist.h`. The header stays as the C declaration of this
//! module until the last C includer is ported (tracked by #158). It declares
//! the `QUEUE_TABLE` variant: every queue owns a fixed pool of equally sized
//! slots and the number of slots is a hard limit on the queue length. The
//! hyperspace encounter limit depends on that.
//!
//! Layout: [`Queue`] and [`Link`] are `repr(C)` mirrors of the C structs. The
//! unit tests compare them with bindings generated from the header at build
//! time, so a change to either side fails the build of the tests.
//!
//! Ownership: a [`Queue`] owns its pool. `init` allocates it and `uninit`
//! releases it. Slots are handed out as raw handles because C stores the
//! handles inside its own structs.

use std::ffi::{c_uint, c_void};
use std::mem::size_of;
use std::ptr;

use crate::logging::{log_add, LogLevel};
use crate::memory::{rust_hfree, rust_hmalloc};

#[cfg(test)]
mod tests;

/// `HLINK`: the address of a slot in a queue pool. Null is the empty handle.
pub type HLink = *mut c_void;

/// `OBJ_SIZE`: the byte size of one pool slot.
pub type ObjSize = u16;

/// `BOOLEAN`: a C enum, so it travels as an `unsigned int`.
pub type Boolean = c_uint;

/// `FALSE`.
pub const FALSE: Boolean = 0;

/// `TRUE`.
pub const TRUE: Boolean = 1;

/// The pool size is stored in a `BYTE`, which caps the queue length.
pub const MAX_QUEUE_OBJECTS: u16 = u8::MAX as u16;

/// `LINK`: the first two fields of every queue element.
#[repr(C)]
#[derive(Debug)]
pub struct Link {
    pub pred: HLink,
    pub succ: HLink,
}

/// `QUEUE`: queue control block, embedded by value in C structs.
///
/// A zeroed `Queue` (what C globals start as) is an uninitialised queue
/// without a pool. Fields are private so that only this module maintains the
/// pool invariants: `pq_tab` is null or points at `object_size * num_objects`
/// bytes from [`rust_hmalloc`], and `head`, `tail` and `free_list` are null or
/// slots of that pool.
#[repr(C)]
#[derive(Debug)]
pub struct Queue {
    head: HLink,
    tail: HLink,
    pq_tab: *mut u8,
    free_list: HLink,
    object_size: u16,
    num_objects: u8,
}

/// `void (*)(LINK *, void *)`, the callback type of [`ForAllLinks`].
pub type LinkCallback = extern "C" fn(*mut Link, *mut c_void);

/// Slot counts reported by [`Queue::audit`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QueueAudit {
    /// Slots in the pool.
    pub capacity: u8,
    /// Links reachable from the head.
    pub queued: u8,
    /// Links on the free list.
    pub free: u8,
    /// Slots that were allocated and are not queued: held by the caller.
    pub detached: u8,
}

/// A broken queue invariant found by [`Queue::audit`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueueFault {
    /// A list entry is not a slot of this queue's pool.
    ForeignLink(usize),
    /// The same slot is reachable twice in one list.
    Cycle(usize),
    /// A queued link's `pred` is not the link before it.
    BrokenPred(usize),
    /// `tail` is not the last reachable link, or `head` and `tail` disagree
    /// about emptiness.
    BrokenTail,
    /// A slot is both queued and on the free list.
    QueuedAndFree(usize),
}

impl Queue {
    /// An uninitialised queue without a pool.
    pub const fn new() -> Self {
        Self {
            head: ptr::null_mut(),
            tail: ptr::null_mut(),
            pq_tab: ptr::null_mut(),
            free_list: ptr::null_mut(),
            object_size: 0,
            num_objects: 0,
        }
    }

    /// First queued link, or null.
    pub fn head(&self) -> HLink {
        self.head
    }

    /// Last queued link, or null.
    pub fn tail(&self) -> HLink {
        self.tail
    }

    /// Next link [`alloc_link`](Self::alloc_link) will return, or null.
    pub fn free_list(&self) -> HLink {
        self.free_list
    }

    /// `GetLinkSize`: byte size of one slot.
    pub fn link_size(&self) -> ObjSize {
        self.object_size
    }

    /// Number of slots in the pool.
    pub fn capacity(&self) -> u8 {
        self.num_objects
    }

    /// Whether `h` is the address of a slot of this queue's pool.
    pub fn owns(&self, h: HLink) -> bool {
        let size = usize::from(self.object_size);
        let pool_len = size * usize::from(self.num_objects);
        let offset = h.addr().wrapping_sub(self.pq_tab.addr());
        !h.is_null() && !self.pq_tab.is_null() && offset < pool_len && offset.is_multiple_of(size)
    }

    /// `InitQueue`: empty the queue and allocate a pool of `num_elements`
    /// slots of `size` bytes.
    ///
    /// Returns `false`, leaving a queue without a pool, when the geometry is
    /// invalid: `num_elements` must be 1 to [`MAX_QUEUE_OBJECTS`] and `size`
    /// must fit a [`Link`]. The pool of an earlier `init` is not released;
    /// use [`uninit`](Self::uninit) first, as C does.
    pub fn init(&mut self, num_elements: u16, size: ObjSize) -> bool {
        *self = Self::new();
        self.object_size = size;
        let valid_count = (1..=MAX_QUEUE_OBJECTS).contains(&num_elements);
        if !valid_count || usize::from(size) < size_of::<Link>() {
            log(
                LogLevel::Error,
                &format!("InitQueue(): invalid pool of {num_elements} elements of {size} bytes"),
            );
            return false;
        }

        let pool_len = usize::from(size) * usize::from(num_elements);
        // SAFETY: `rust_hmalloc` has no preconditions and aborts on failure.
        self.pq_tab = unsafe { rust_hmalloc(pool_len) }.cast();
        self.num_objects = num_elements as u8;
        self.rebuild_free_list();
        true
    }

    /// `UninitQueue`: empty the queue and release its pool. Queued links and
    /// handles into the pool are invalid afterwards.
    pub fn uninit(&mut self) {
        if !self.pq_tab.is_null() {
            // SAFETY: `pq_tab` came from `rust_hmalloc` in `init` and is
            // cleared below, so it is freed once.
            unsafe { rust_hfree(self.pq_tab.cast()) };
        }
        *self = Self {
            object_size: self.object_size,
            ..Self::new()
        };
    }

    /// `ReinitQueue`: empty the queue and return every slot to the free list.
    /// The slot contents are untouched.
    pub fn reinit(&mut self) {
        self.head = ptr::null_mut();
        self.tail = ptr::null_mut();
        self.rebuild_free_list();
    }

    /// `AllocLink`: take a slot off the free list, or null when the pool is
    /// exhausted. The slot contents are whatever the last user left.
    pub fn alloc_link(&mut self) -> HLink {
        let h = self.free_list;
        if h.is_null() {
            log(LogLevel::Debug, "AllocLink(): No more elements");
            return h;
        }
        // SAFETY: a non-null free list head is a pool slot.
        self.free_list = unsafe { (*self.link(h)).succ };
        h
    }

    /// `FreeLink`: return a slot to the free list.
    ///
    /// # Safety
    ///
    /// `h` must be a slot of this queue's pool that is not queued and not
    /// already free.
    pub unsafe fn free_link(&mut self, h: HLink) {
        (*self.link(h)).succ = self.free_list;
        self.free_list = h;
    }

    /// `PutQueue`: append `h` at the tail.
    ///
    /// # Safety
    ///
    /// `h` must be an allocated slot of this queue's pool that is not queued.
    pub unsafe fn put(&mut self, h: HLink) {
        if self.head.is_null() {
            self.head = h;
        } else {
            (*self.link(self.tail)).succ = h;
        }
        let link = self.link(h);
        (*link).pred = self.tail;
        (*link).succ = ptr::null_mut();
        self.tail = h;
    }

    /// `InsertQueue`: insert `h` before `h_ref`, or append when `h_ref` is
    /// null.
    ///
    /// # Safety
    ///
    /// `h` must be as for [`put`](Self::put), and `h_ref` must be null or a
    /// queued slot.
    pub unsafe fn insert(&mut self, h: HLink, h_ref: HLink) {
        if h_ref.is_null() {
            self.put(h);
            return;
        }
        let link = self.link(h);
        let ref_link = self.link(h_ref);
        (*link).pred = (*ref_link).pred;
        (*ref_link).pred = h;
        (*link).succ = h_ref;
        if self.head == h_ref {
            self.head = h;
        } else {
            (*self.link((*link).pred)).succ = h;
        }
    }

    /// `RemoveQueue`: unlink `h`. The slot stays allocated; its own links are
    /// left as they were.
    ///
    /// # Safety
    ///
    /// `h` must be a queued slot of this queue.
    pub unsafe fn remove(&mut self, h: HLink) {
        let link = self.link(h);
        if self.head == h {
            self.head = (*link).succ;
        } else {
            (*self.link((*link).pred)).succ = (*link).succ;
        }
        if self.tail == h {
            self.tail = (*link).pred;
        } else {
            (*self.link((*link).succ)).pred = (*link).pred;
        }
    }

    /// `CountLinks`: number of queued links.
    pub fn count(&self) -> u16 {
        let mut count = 0;
        let mut h = self.head;
        while !h.is_null() {
            count += 1;
            // SAFETY: queued handles are pool slots.
            h = unsafe { (*self.link(h)).succ };
        }
        count
    }

    /// Check the pool invariants and report the slot accounting.
    ///
    /// Every list entry must be a pool slot and appear once, the `pred`
    /// chain must mirror the `succ` chain, `tail` must be the last link, and
    /// no slot may be both queued and free.
    pub fn audit(&self) -> Result<QueueAudit, QueueFault> {
        let mut queued_slots = [false; MAX_QUEUE_OBJECTS as usize];
        let mut queued = 0u8;
        let mut prev: HLink = ptr::null_mut();
        let mut h = self.head;
        while !h.is_null() {
            let slot = self.slot_index(h)?;
            if queued_slots[slot] {
                return Err(QueueFault::Cycle(slot));
            }
            queued_slots[slot] = true;
            queued += 1;
            // SAFETY: `slot_index` verified that `h` is a pool slot.
            let link = unsafe { &*self.link(h) };
            if link.pred != prev {
                return Err(QueueFault::BrokenPred(slot));
            }
            prev = h;
            h = link.succ;
        }
        if prev != self.tail {
            return Err(QueueFault::BrokenTail);
        }

        let mut free_slots = [false; MAX_QUEUE_OBJECTS as usize];
        let mut free = 0u8;
        let mut h = self.free_list;
        while !h.is_null() {
            let slot = self.slot_index(h)?;
            if queued_slots[slot] {
                return Err(QueueFault::QueuedAndFree(slot));
            }
            if free_slots[slot] {
                return Err(QueueFault::Cycle(slot));
            }
            free_slots[slot] = true;
            free += 1;
            // SAFETY: `slot_index` verified that `h` is a pool slot.
            h = unsafe { (*self.link(h)).succ };
        }
        Ok(QueueAudit {
            capacity: self.num_objects,
            queued,
            free,
            detached: self.num_objects - queued - free,
        })
    }

    /// Zero-based slot number of `h`.
    fn slot_index(&self, h: HLink) -> Result<usize, QueueFault> {
        if self.owns(h) {
            Ok(h.addr().wrapping_sub(self.pq_tab.addr()) / usize::from(self.object_size))
        } else {
            Err(QueueFault::ForeignLink(h.addr()))
        }
    }

    /// `GetLinkAddr`: address of the slot with the one-based number `number`.
    fn slot_addr(&self, number: u16) -> HLink {
        let offset = usize::from(self.object_size) * usize::from(number - 1);
        self.pq_tab.wrapping_add(offset).cast()
    }

    /// `LockLink`: the link header of slot `h`.
    ///
    /// The debug assertion is the pool membership invariant of the C
    /// `LockLink`, with the slot alignment added.
    fn link(&self, h: HLink) -> *mut Link {
        debug_assert!(
            self.owns(h),
            "{h:p} is not a slot of the queue pool at {:p}",
            self.pq_tab
        );
        h.cast()
    }

    /// Chain every slot onto the free list so the lowest slot is allocated
    /// first.
    fn rebuild_free_list(&mut self) {
        self.free_list = ptr::null_mut();
        for number in (1..=u16::from(self.num_objects)).rev() {
            let slot = self.slot_addr(number);
            // SAFETY: `slot` is in the pool and not on the free list yet.
            unsafe { self.free_link(slot) };
        }
    }
}

impl Default for Queue {
    fn default() -> Self {
        Self::new()
    }
}

fn log(level: LogLevel, message: &str) {
    // SAFETY: `log_add` only formats the message to the diagnostic stream.
    unsafe { log_add(level, message) };
}

// ---------------------------------------------------------------------------
// C ABI, as declared in displist.h
// ---------------------------------------------------------------------------

/// `BOOLEAN InitQueue (QUEUE *pq, COUNT num_elements, OBJ_SIZE size)`.
///
/// Returns `FALSE` for a pool of zero or more than 255 elements, or for a
/// slot size that cannot hold a `LINK`.
///
/// # Safety
///
/// `pq` must point to a writable `QUEUE`. A pool it already owns is leaked.
#[no_mangle]
pub unsafe extern "C" fn InitQueue(pq: *mut Queue, num_elements: u16, size: ObjSize) -> Boolean {
    Boolean::from((*pq).init(num_elements, size))
}

/// `BOOLEAN UninitQueue (QUEUE *pq)`.
///
/// # Safety
///
/// `pq` must point to a `QUEUE` that is zeroed or was set up by [`InitQueue`].
#[no_mangle]
pub unsafe extern "C" fn UninitQueue(pq: *mut Queue) -> Boolean {
    (*pq).uninit();
    TRUE
}

/// `void ReinitQueue (QUEUE *pq)`.
///
/// # Safety
///
/// `pq` must point to a `QUEUE` that is zeroed or was set up by [`InitQueue`].
#[no_mangle]
pub unsafe extern "C" fn ReinitQueue(pq: *mut Queue) {
    (*pq).reinit();
}

/// `HLINK AllocLink (QUEUE *pq)`.
///
/// # Safety
///
/// `pq` must point to a `QUEUE` that is zeroed or was set up by [`InitQueue`].
#[no_mangle]
pub unsafe extern "C" fn AllocLink(pq: *mut Queue) -> HLink {
    (*pq).alloc_link()
}

/// `void FreeLink (QUEUE *pq, HLINK hLink)`.
///
/// # Safety
///
/// See [`Queue::free_link`]; `pq` must point to a set-up `QUEUE`.
#[no_mangle]
pub unsafe extern "C" fn FreeLink(pq: *mut Queue, h_link: HLink) {
    (*pq).free_link(h_link);
}

/// `void PutQueue (QUEUE *pq, HLINK hLink)`.
///
/// # Safety
///
/// See [`Queue::put`]; `pq` must point to a set-up `QUEUE`.
#[no_mangle]
pub unsafe extern "C" fn PutQueue(pq: *mut Queue, h_link: HLink) {
    (*pq).put(h_link);
}

/// `void InsertQueue (QUEUE *pq, HLINK hLink, HLINK hRefLink)`.
///
/// # Safety
///
/// See [`Queue::insert`]; `pq` must point to a set-up `QUEUE`.
#[no_mangle]
pub unsafe extern "C" fn InsertQueue(pq: *mut Queue, h_link: HLink, h_ref_link: HLink) {
    (*pq).insert(h_link, h_ref_link);
}

/// `void RemoveQueue (QUEUE *pq, HLINK hLink)`.
///
/// # Safety
///
/// See [`Queue::remove`]; `pq` must point to a set-up `QUEUE`.
#[no_mangle]
pub unsafe extern "C" fn RemoveQueue(pq: *mut Queue, h_link: HLink) {
    (*pq).remove(h_link);
}

/// `COUNT CountLinks (QUEUE *pq)`.
///
/// # Safety
///
/// `pq` must point to a `QUEUE` that is zeroed or was set up by [`InitQueue`].
#[no_mangle]
pub unsafe extern "C" fn CountLinks(pq: *mut Queue) -> u16 {
    (*pq).count()
}

/// `void ForAllLinks (QUEUE *pq, void (*callback)(LINK *, void *), void *arg)`.
///
/// The successor is read before the callback runs, so the callback may remove
/// or free the link it is given. It may also use `pq`; the queue is not
/// borrowed while the callback runs.
///
/// # Safety
///
/// `pq` must point to a set-up `QUEUE`, and `callback` must uphold the
/// contracts of the queue operations it performs.
#[no_mangle]
pub unsafe extern "C" fn ForAllLinks(pq: *mut Queue, callback: LinkCallback, arg: *mut c_void) {
    let mut h = (*pq).head;
    while !h.is_null() {
        let link = (*pq).link(h);
        let next = (*link).succ;
        callback(link, arg);
        h = next;
    }
}
