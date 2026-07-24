// Generic doubly-linked list queue — port of C displist.c
// Uses #[repr(C)] QUEUE/LINK structs matching the exact C memory layout.
// QUEUE_TABLE mode: elements are allocated from a preallocated pool.

use std::os::raw::c_void;

/// HLINK — opaque handle to a link (actually a raw pointer to the LINK fields)
pub type HLink = *mut c_void;

/// LINK — the first two fields of any queue element
/// C: `typedef struct link { HLINK pred; HLINK succ; } LINK;`
#[repr(C)]
pub struct Link {
    pub pred: HLink,
    pub succ: HLink,
}

/// QUEUE — the queue control structure
/// C: `typedef struct queue { HLINK head; HLINK tail; BYTE *pq_tab;
///      HLINK free_list; COUNT object_size; BYTE num_objects; } QUEUE;`
#[repr(C)]
pub struct Queue {
    pub head: HLink,
    pub tail: HLink,
    pub pq_tab: *mut u8,
    pub free_list: HLink,
    pub object_size: u16,
    pub num_objects: u8,
}

/// OBJ_SIZE type alias
pub type ObjSize = u16;

extern "C" {
    fn rust_hmalloc(size: usize) -> *mut c_void;
    fn rust_hfree(ptr: *mut c_void);
}

// ---------------------------------------------------------------------------
// Internal helpers (replacing C macros)
// ---------------------------------------------------------------------------

#[inline]
unsafe fn lock_link(_pq: *const Queue, h: HLink) -> *mut Link {
    // LockLink in QUEUE_TABLE mode just does bounds-check asserts + cast.
    // We skip the assert for performance (C only asserts in debug).
    h as *mut Link
}

#[inline]
unsafe fn get_pred_link(lp: *const Link) -> HLink {
    (*lp).pred
}

#[inline]
unsafe fn set_pred_link(lp: *mut Link, h: HLink) {
    (*lp).pred = h;
}

#[inline]
unsafe fn get_succ_link(lp: *const Link) -> HLink {
    (*lp).succ
}

#[inline]
unsafe fn set_succ_link(lp: *mut Link, h: HLink) {
    (*lp).succ = h;
}

#[inline]
unsafe fn get_link_addr(pq: *const Queue, i: u16) -> HLink {
    let base = (*pq).pq_tab;
    let offset = ((*pq).object_size as usize) * ((i - 1) as usize);
    base.add(offset) as HLink
}

// ---------------------------------------------------------------------------
// Public API — matches C function signatures exactly
// ---------------------------------------------------------------------------

/// C: `BOOLEAN InitQueue(QUEUE *pq, COUNT num_elements, OBJ_SIZE size)`
#[no_mangle]
#[allow(
    clippy::not_unsafe_ptr_arg_deref,
    reason = "C ABI compatibility is fixed during the Rust migration; tracked by PLAN-20260723-RUNTIME-AUTOMATION.P00"
)]
pub extern "C" fn InitQueue(pq: *mut Queue, num_elements: u16, size: ObjSize) -> bool {
    unsafe {
        (*pq).head = std::ptr::null_mut();
        (*pq).tail = std::ptr::null_mut();
        (*pq).object_size = size;

        // AllocQueueTab: pq_tab = HMalloc(object_size * num_objects)
        (*pq).num_objects = num_elements as u8;
        let tab_size = (size as usize) * (num_elements as usize);
        (*pq).pq_tab = rust_hmalloc(tab_size) as *mut u8;

        if (*pq).pq_tab.is_null() {
            return false;
        }

        // Build free list: free each slot in reverse order
        (*pq).free_list = std::ptr::null_mut();
        let mut n = num_elements;
        while n > 0 {
            FreeLink(pq, get_link_addr(pq, n));
            n -= 1;
        }

        true
    }
}

/// C: `BOOLEAN UninitQueue(QUEUE *pq)`
#[no_mangle]
#[allow(
    clippy::not_unsafe_ptr_arg_deref,
    reason = "C ABI compatibility is fixed during the Rust migration; tracked by PLAN-20260723-RUNTIME-AUTOMATION.P00"
)]
pub extern "C" fn UninitQueue(pq: *mut Queue) -> bool {
    unsafe {
        (*pq).head = std::ptr::null_mut();
        (*pq).tail = std::ptr::null_mut();
        (*pq).free_list = std::ptr::null_mut();

        if !(*pq).pq_tab.is_null() {
            rust_hfree((*pq).pq_tab as *mut c_void);
            (*pq).pq_tab = std::ptr::null_mut();
        }

        true
    }
}

/// C: `void ReinitQueue(QUEUE *pq)` — empty the queue, rebuild free list
#[no_mangle]
#[allow(
    clippy::not_unsafe_ptr_arg_deref,
    reason = "C ABI compatibility is fixed during the Rust migration; tracked by PLAN-20260723-RUNTIME-AUTOMATION.P00"
)]
pub extern "C" fn ReinitQueue(pq: *mut Queue) {
    unsafe {
        (*pq).head = std::ptr::null_mut();
        (*pq).tail = std::ptr::null_mut();

        (*pq).free_list = std::ptr::null_mut();
        let num_elements = (*pq).num_objects as u16;
        let mut n = num_elements;
        while n > 0 {
            FreeLink(pq, get_link_addr(pq, n));
            n -= 1;
        }
    }
}

/// C: `HLINK AllocLink(QUEUE *pq)` — allocate from free list
#[no_mangle]
#[allow(
    clippy::not_unsafe_ptr_arg_deref,
    reason = "C ABI compatibility is fixed during the Rust migration; tracked by PLAN-20260723-RUNTIME-AUTOMATION.P00"
)]
pub extern "C" fn AllocLink(pq: *mut Queue) -> HLink {
    unsafe {
        let h = (*pq).free_list;
        if !h.is_null() {
            let lp = lock_link(pq, h);
            (*pq).free_list = get_succ_link(lp);
        }
        h
    }
}

/// C: `void FreeLink(QUEUE *pq, HLINK hLink)` — return to free list
#[no_mangle]
#[allow(
    clippy::not_unsafe_ptr_arg_deref,
    reason = "C ABI compatibility is fixed during the Rust migration; tracked by PLAN-20260723-RUNTIME-AUTOMATION.P00"
)]
pub extern "C" fn FreeLink(pq: *mut Queue, h_link: HLink) {
    unsafe {
        let lp = lock_link(pq, h_link);
        set_succ_link(lp, (*pq).free_list);
        (*pq).free_list = h_link;
    }
}

/// C: `void PutQueue(QUEUE *pq, HLINK hLink)` — append to tail
#[no_mangle]
#[allow(
    clippy::not_unsafe_ptr_arg_deref,
    reason = "C ABI compatibility is fixed during the Rust migration; tracked by PLAN-20260723-RUNTIME-AUTOMATION.P00"
)]
pub extern "C" fn PutQueue(pq: *mut Queue, h_link: HLink) {
    unsafe {
        if (*pq).head.is_null() {
            (*pq).head = h_link;
        } else {
            let tail = (*pq).tail;
            let lp_tail = lock_link(pq, tail);
            set_succ_link(lp_tail, h_link);
        }

        let lp = lock_link(pq, h_link);
        set_pred_link(lp, (*pq).tail);
        set_succ_link(lp, std::ptr::null_mut());

        (*pq).tail = h_link;
    }
}

/// C: `void InsertQueue(QUEUE *pq, HLINK hLink, HLINK hRefLink)` — insert before ref
#[no_mangle]
#[allow(
    clippy::not_unsafe_ptr_arg_deref,
    reason = "C ABI compatibility is fixed during the Rust migration; tracked by PLAN-20260723-RUNTIME-AUTOMATION.P00"
)]
pub extern "C" fn InsertQueue(pq: *mut Queue, h_link: HLink, h_ref_link: HLink) {
    if h_ref_link.is_null() {
        PutQueue(pq, h_link);
        return;
    }

    unsafe {
        let lp = lock_link(pq, h_link);
        let lp_ref = lock_link(pq, h_ref_link);

        set_pred_link(lp, get_pred_link(lp_ref));
        set_pred_link(lp_ref, h_link);
        set_succ_link(lp, h_ref_link);

        if (*pq).head == h_ref_link {
            (*pq).head = h_link;
        } else {
            let h_pred = get_pred_link(lp);
            let lp_pred = lock_link(pq, h_pred);
            set_succ_link(lp_pred, h_link);
        }

        // UnlockLink is no-op in our mode
    }
}

/// C: `void RemoveQueue(QUEUE *pq, HLINK hLink)` — unlink from queue
#[no_mangle]
#[allow(
    clippy::not_unsafe_ptr_arg_deref,
    reason = "C ABI compatibility is fixed during the Rust migration; tracked by PLAN-20260723-RUNTIME-AUTOMATION.P00"
)]
pub extern "C" fn RemoveQueue(pq: *mut Queue, h_link: HLink) {
    unsafe {
        let lp = lock_link(pq, h_link);

        if (*pq).head == h_link {
            (*pq).head = get_succ_link(lp);
        } else {
            let h_pred = get_pred_link(lp);
            let lp_pred = lock_link(pq, h_pred);
            set_succ_link(lp_pred, get_succ_link(lp));
        }

        if (*pq).tail == h_link {
            (*pq).tail = get_pred_link(lp);
        } else {
            let h_succ = get_succ_link(lp);
            let lp_succ = lock_link(pq, h_succ);
            set_pred_link(lp_succ, get_pred_link(lp));
        }
    }
}

/// C: `COUNT CountLinks(QUEUE *pq)` — count elements
#[no_mangle]
#[allow(
    clippy::not_unsafe_ptr_arg_deref,
    reason = "C ABI compatibility is fixed during the Rust migration; tracked by PLAN-20260723-RUNTIME-AUTOMATION.P00"
)]
pub extern "C" fn CountLinks(pq: *const Queue) -> u16 {
    unsafe {
        let mut count: u16 = 0;
        let mut h = (*pq).head;
        while !h.is_null() {
            count += 1;
            h = get_succ_link(lock_link(pq, h));
        }
        count
    }
}

/// C: `void ForAllLinks(QUEUE *pq, void (*callback)(LINK*, void*), void *arg)`
pub type LinkCallback = extern "C" fn(*mut Link, *mut c_void);

#[no_mangle]
#[allow(
    clippy::not_unsafe_ptr_arg_deref,
    reason = "C ABI compatibility is fixed during the Rust migration; tracked by PLAN-20260723-RUNTIME-AUTOMATION.P00"
)]
pub extern "C" fn ForAllLinks(pq: *mut Queue, callback: LinkCallback, arg: *mut c_void) {
    unsafe {
        let mut h = (*pq).head;
        while !h.is_null() {
            let lp = lock_link(pq, h);
            let next = get_succ_link(lp);
            callback(lp, arg);
            h = next;
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // Test element: LINK fields followed by a payload
    #[repr(C)]
    struct TestElement {
        pred: HLink,
        succ: HLink,
        value: i32,
    }

    /// Allocate a QUEUE + elements, perform operations, verify behavior
    fn setup_queue(n: u16) -> (*mut Queue, Vec<*mut TestElement>) {
        let object_size = std::mem::size_of::<TestElement>() as u16;

        // Allocate queue struct
        let pq = Box::into_raw(Box::new(Queue {
            head: std::ptr::null_mut(),
            tail: std::ptr::null_mut(),
            pq_tab: std::ptr::null_mut(),
            free_list: std::ptr::null_mut(),
            object_size,
            num_objects: 0,
        }));

        // Use InitQueue to allocate the pool
        InitQueue(pq, n, object_size);

        // Alloc n elements
        let mut elems = Vec::new();
        for i in 0..n as i32 {
            let h = AllocLink(pq);
            assert!(!h.is_null());
            unsafe {
                (*(h as *mut TestElement)).value = i;
            }
            elems.push(h as *mut TestElement);
        }

        (pq, elems)
    }

    fn teardown_queue(pq: *mut Queue) {
        unsafe {
            UninitQueue(pq);
            drop(Box::from_raw(pq));
        }
    }

    #[test]
    fn test_init_uninit() {
        let (pq, _) = setup_queue(4);
        teardown_queue(pq);
    }

    #[test]
    fn test_alloc_exhaustion() {
        let (pq, _) = setup_queue(3);
        // Pool of 3 exhausted
        let h = AllocLink(pq);
        assert!(
            h.is_null(),
            "AllocLink should return NULL when pool exhausted"
        );
        teardown_queue(pq);
    }

    #[test]
    fn test_free_realloc() {
        let (pq, elems) = setup_queue(2);
        // Free one, then alloc should succeed again
        FreeLink(pq, elems[0] as HLink);
        let h = AllocLink(pq);
        assert!(!h.is_null(), "AllocLink should succeed after FreeLink");
        teardown_queue(pq);
    }

    #[test]
    fn test_put_count_remove() {
        let (pq, elems) = setup_queue(3);
        unsafe {
            assert_eq!(CountLinks(pq), 0);

            PutQueue(pq, elems[0] as HLink);
            PutQueue(pq, elems[1] as HLink);
            PutQueue(pq, elems[2] as HLink);
            assert_eq!(CountLinks(pq), 3);

            RemoveQueue(pq, elems[1] as HLink);
            assert_eq!(CountLinks(pq), 2);

            // Head should be elems[0], tail should be elems[2]
            assert_eq!((*pq).head, elems[0] as HLink);
            assert_eq!((*pq).tail, elems[2] as HLink);
        }
        teardown_queue(pq);
    }

    #[test]
    fn test_reinit() {
        let (pq, elems) = setup_queue(3);
        PutQueue(pq, elems[0] as HLink);
        PutQueue(pq, elems[1] as HLink);
        assert_eq!(CountLinks(pq), 2);
        ReinitQueue(pq);
        assert_eq!(CountLinks(pq), 0);
        // After reinit, pool should be fully available again
        let h = AllocLink(pq);
        assert!(!h.is_null());
        let h2 = AllocLink(pq);
        assert!(!h2.is_null());
        let h3 = AllocLink(pq);
        assert!(!h3.is_null());
        teardown_queue(pq);
    }

    #[test]
    fn test_insert_in_order() {
        let (pq, elems) = setup_queue(3);
        unsafe {
            // Put [0], then [2], then insert [1] before [2]
            PutQueue(pq, elems[0] as HLink);
            PutQueue(pq, elems[2] as HLink);
            InsertQueue(pq, elems[1] as HLink, elems[2] as HLink);

            // Walk the list: should be 0 → 1 → 2
            let mut h = (*pq).head;
            let order = [0i32, 1, 2];
            let mut idx = 0;
            while !h.is_null() {
                let val = (*(h as *mut TestElement)).value;
                assert_eq!(val, order[idx], "element {} in wrong position", idx);
                h = get_succ_link(lock_link(pq, h));
                idx += 1;
            }
            assert_eq!(idx, 3);
        }
        teardown_queue(pq);
    }

    #[test]
    fn test_insert_at_head() {
        let (pq, elems) = setup_queue(3);
        unsafe {
            PutQueue(pq, elems[0] as HLink);
            InsertQueue(pq, elems[1] as HLink, elems[0] as HLink);

            // Head should now be elems[1]
            assert_eq!((*pq).head, elems[1] as HLink);
        }
        teardown_queue(pq);
    }

    #[test]
    fn test_insert_null_ref_appends() {
        let (pq, elems) = setup_queue(3);
        unsafe {
            PutQueue(pq, elems[0] as HLink);
            InsertQueue(pq, elems[1] as HLink, std::ptr::null_mut());

            // Should append → tail is elems[1]
            assert_eq!((*pq).tail, elems[1] as HLink);
        }
        teardown_queue(pq);
    }

    #[test]
    fn test_for_all_links() {
        extern "C" fn count_cb(_lp: *mut Link, arg: *mut c_void) {
            unsafe {
                let counter = arg as *mut u16;
                *counter += 1;
            }
        }

        let (pq, elems) = setup_queue(4);
        PutQueue(pq, elems[0] as HLink);
        PutQueue(pq, elems[1] as HLink);
        PutQueue(pq, elems[2] as HLink);
        let mut counter: u16 = 0;
        ForAllLinks(pq, count_cb, &mut counter as *mut u16 as *mut c_void);
        assert_eq!(counter, 3);
        teardown_queue(pq);
    }
}
