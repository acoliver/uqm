use std::ffi::{c_uint, c_void};
use std::mem::{offset_of, size_of};

use proptest::collection::vec;
use proptest::prelude::*;

use super::*;

mod header_layout {
    include!(concat!(env!("OUT_DIR"), "/queue_abi_layout.rs"));
}

mod header_signatures {
    include!(concat!(env!("OUT_DIR"), "/queue_abi_signatures.rs"));
}

/// A queue element as C declares them: the two link fields first.
#[repr(C)]
struct Element {
    link: Link,
    value: u32,
}

const ELEMENT_SIZE: ObjSize = size_of::<Element>() as ObjSize;

/// A queue with a pool, released on drop.
struct Pool {
    queue: Box<Queue>,
}

impl Pool {
    fn new(capacity: u16) -> Self {
        let mut queue = Box::new(Queue::new());
        assert!(queue.init(capacity, ELEMENT_SIZE));
        Self { queue }
    }

    fn alloc(&mut self, value: u32) -> HLink {
        let h = self.queue.alloc_link();
        assert!(!h.is_null(), "pool exhausted");
        unsafe { (*h.cast::<Element>()).value = value };
        h
    }

    fn put(&mut self, value: u32) -> HLink {
        let h = self.alloc(value);
        unsafe { self.queue.put(h) };
        h
    }

    fn values(&self) -> Vec<u32> {
        let mut values = Vec::new();
        let mut h = self.queue.head();
        while !h.is_null() {
            values.push(unsafe { (*h.cast::<Element>()).value });
            h = unsafe { (*h.cast::<Link>()).succ };
        }
        values
    }

    fn values_from_tail(&self) -> Vec<u32> {
        let mut values = Vec::new();
        let mut h = self.queue.tail();
        while !h.is_null() {
            values.push(unsafe { (*h.cast::<Element>()).value });
            h = unsafe { (*h.cast::<Link>()).pred };
        }
        values.reverse();
        values
    }

    fn assert_consistent(&self, expected: &[u32]) {
        assert_eq!(self.values(), expected);
        assert_eq!(self.values_from_tail(), expected);
        assert_eq!(usize::from(self.queue.count()), expected.len());
        assert!(self.queue.audit().is_ok(), "{:?}", self.queue.audit());
    }
}

impl Drop for Pool {
    fn drop(&mut self) {
        self.queue.uninit();
    }
}

// ---------------------------------------------------------------------------
// Initialisation, exhaustion and teardown
// ---------------------------------------------------------------------------

#[test]
fn init_hands_out_slots_in_address_order() {
    let mut pool = Pool::new(4);
    let slots: Vec<usize> = (0..4).map(|n| pool.alloc(n).addr()).collect();
    assert!(slots
        .windows(2)
        .all(|w| w[1] - w[0] == size_of::<Element>()));
    assert_eq!(pool.queue.capacity(), 4);
    assert_eq!(pool.queue.link_size(), ELEMENT_SIZE);
}

#[test]
fn exhausted_pool_returns_null_until_a_slot_is_freed() {
    let mut pool = Pool::new(3);
    let held: Vec<HLink> = (0..3).map(|n| pool.alloc(n)).collect();
    assert!(pool.queue.alloc_link().is_null());
    assert!(pool.queue.alloc_link().is_null());
    unsafe { pool.queue.free_link(held[1]) };
    assert_eq!(pool.queue.alloc_link(), held[1]);
}

#[test]
fn largest_pool_is_255_slots_and_all_are_distinct() {
    let mut pool = Pool::new(MAX_QUEUE_OBJECTS);
    let mut seen = std::collections::BTreeSet::new();
    for n in 0..MAX_QUEUE_OBJECTS {
        assert!(seen.insert(pool.alloc(u32::from(n)).addr()));
    }
    assert!(pool.queue.alloc_link().is_null());
    assert_eq!(
        pool.queue.audit(),
        Ok(QueueAudit {
            capacity: 255,
            queued: 0,
            free: 0,
            detached: 255
        })
    );
}

#[test]
fn invalid_pool_geometry_is_refused_and_leaves_no_pool() {
    let link_size = size_of::<Link>() as ObjSize;
    for (count, size) in [(0, ELEMENT_SIZE), (256, ELEMENT_SIZE), (u16::MAX, 64)]
        .into_iter()
        .chain([(4, 0), (4, link_size - 1)])
    {
        let mut queue = Queue::new();
        assert!(!queue.init(count, size), "{count} x {size}");
        assert_eq!(queue.capacity(), 0);
        assert!(queue.alloc_link().is_null());
        assert_eq!(queue.count(), 0);
        assert_eq!(
            queue.audit(),
            Ok(QueueAudit {
                capacity: 0,
                queued: 0,
                free: 0,
                detached: 0
            })
        );
        queue.uninit();
    }
}

#[test]
fn a_slot_exactly_as_large_as_a_link_is_accepted() {
    let mut queue = Queue::new();
    assert!(queue.init(2, size_of::<Link>() as ObjSize));
    let h = queue.alloc_link();
    assert!(!h.is_null());
    queue.uninit();
}

#[test]
fn c_entry_points_report_invalid_geometry_as_false() {
    let mut queue = Queue::new();
    let pq: *mut Queue = &mut queue;
    unsafe {
        assert_eq!(InitQueue(pq, 0, ELEMENT_SIZE), FALSE);
        assert_eq!(InitQueue(pq, 300, ELEMENT_SIZE), FALSE);
        assert_eq!(InitQueue(pq, 10, 3), FALSE);
        assert_eq!(InitQueue(pq, 10, ELEMENT_SIZE), TRUE);
        assert_eq!(UninitQueue(pq), TRUE);
    }
}

#[test]
fn uninit_is_repeatable_and_leaves_an_empty_queue() {
    let mut pool = Pool::new(3);
    pool.put(1);
    pool.queue.uninit();
    assert!(pool.queue.head().is_null());
    assert!(pool.queue.tail().is_null());
    assert!(pool.queue.free_list().is_null());
    assert_eq!(pool.queue.capacity(), 0);
    assert!(pool.queue.alloc_link().is_null());
    pool.queue.uninit();
    pool.queue.reinit();
    assert_eq!(pool.queue.count(), 0);
}

#[test]
fn zeroed_c_global_can_be_uninitialised_and_reinitialised() {
    // SAFETY: an all-zero `QUEUE` is the state of an untouched C global.
    let mut queue: Queue = unsafe { std::mem::zeroed() };
    let pq: *mut Queue = &mut queue;
    unsafe {
        assert_eq!(UninitQueue(pq), TRUE);
        ReinitQueue(pq);
        assert_eq!(CountLinks(pq), 0);
        assert!(AllocLink(pq).is_null());
    }
}

#[test]
fn restart_cycles_release_and_rebuild_the_pool() {
    let mut queue = Queue::new();
    for round in 0..50u32 {
        assert!(queue.init(5, ELEMENT_SIZE));
        let mut held = Vec::new();
        for n in 0..5 {
            let h = queue.alloc_link();
            unsafe { (*h.cast::<Element>()).value = round * 10 + n };
            unsafe { queue.put(h) };
            held.push(h);
        }
        assert!(queue.alloc_link().is_null());
        assert_eq!(queue.count(), 5);
        queue.uninit();
    }
}

// ---------------------------------------------------------------------------
// Ordering
// ---------------------------------------------------------------------------

#[test]
fn put_appends_in_order() {
    let mut pool = Pool::new(5);
    for value in 1..=4 {
        pool.put(value);
    }
    pool.assert_consistent(&[1, 2, 3, 4]);
}

#[test]
fn insert_places_before_the_reference_link() {
    let mut pool = Pool::new(6);
    let first = pool.put(10);
    let last = pool.put(30);
    let middle = pool.alloc(20);
    unsafe { pool.queue.insert(middle, last) };
    pool.assert_consistent(&[10, 20, 30]);

    let new_head = pool.alloc(5);
    unsafe { pool.queue.insert(new_head, first) };
    pool.assert_consistent(&[5, 10, 20, 30]);
    assert_eq!(pool.queue.head(), new_head);
}

#[test]
fn insert_with_a_null_reference_appends() {
    let mut pool = Pool::new(3);
    pool.put(1);
    let h = pool.alloc(2);
    unsafe { pool.queue.insert(h, std::ptr::null_mut()) };
    pool.assert_consistent(&[1, 2]);
    assert_eq!(pool.queue.tail(), h);
}

#[test]
fn insert_into_an_empty_queue_appends() {
    let mut pool = Pool::new(2);
    let h = pool.alloc(7);
    unsafe { pool.queue.insert(h, std::ptr::null_mut()) };
    pool.assert_consistent(&[7]);
}

#[test]
fn remove_head_middle_tail_and_only_link() {
    let mut pool = Pool::new(5);
    let handles: Vec<HLink> = (1..=5).map(|v| pool.put(v)).collect();

    unsafe { pool.queue.remove(handles[2]) };
    pool.assert_consistent(&[1, 2, 4, 5]);
    unsafe { pool.queue.remove(handles[0]) };
    pool.assert_consistent(&[2, 4, 5]);
    unsafe { pool.queue.remove(handles[4]) };
    pool.assert_consistent(&[2, 4]);
    unsafe { pool.queue.remove(handles[1]) };
    pool.assert_consistent(&[4]);
    unsafe { pool.queue.remove(handles[3]) };
    pool.assert_consistent(&[]);
    assert!(pool.queue.head().is_null());
    assert!(pool.queue.tail().is_null());
}

#[test]
fn a_removed_link_can_be_requeued_elsewhere() {
    let mut pool = Pool::new(3);
    let a = pool.put(1);
    pool.put(2);
    pool.put(3);
    unsafe {
        pool.queue.remove(a);
        pool.queue.put(a);
    }
    pool.assert_consistent(&[2, 3, 1]);
}

#[test]
fn reinit_empties_the_queue_and_frees_every_slot() {
    let mut pool = Pool::new(3);
    pool.put(1);
    pool.put(2);
    let held = pool.alloc(3);
    pool.queue.reinit();
    assert_eq!(pool.queue.count(), 0);
    assert!(pool.queue.head().is_null());
    assert_eq!(
        pool.queue.audit(),
        Ok(QueueAudit {
            capacity: 3,
            queued: 0,
            free: 3,
            detached: 0
        })
    );
    let again: Vec<HLink> = (0..3).map(|n| pool.alloc(n)).collect();
    assert!(again.contains(&held));
}

// ---------------------------------------------------------------------------
// Callbacks and re-entry
// ---------------------------------------------------------------------------

struct Visit {
    queue: *mut Queue,
    seen: Vec<u32>,
    remove_current: bool,
    free_current: bool,
    count_during: Vec<u16>,
}

extern "C" fn visit(link: *mut Link, arg: *mut c_void) {
    let state = unsafe { &mut *arg.cast::<Visit>() };
    state.seen.push(unsafe { (*link.cast::<Element>()).value });
    // The callback may use the queue it is being run over.
    state.count_during.push(unsafe { CountLinks(state.queue) });
    if state.remove_current {
        unsafe { RemoveQueue(state.queue, link.cast()) };
    }
    if state.free_current {
        unsafe { FreeLink(state.queue, link.cast()) };
    }
}

fn run_visit(pool: &mut Pool, remove_current: bool, free_current: bool) -> Visit {
    let pq: *mut Queue = &mut *pool.queue;
    let mut state = Visit {
        queue: pq,
        seen: Vec::new(),
        remove_current,
        free_current,
        count_during: Vec::new(),
    };
    unsafe { ForAllLinks(pq, visit, (&mut state as *mut Visit).cast()) };
    state
}

#[test]
fn for_all_links_visits_every_link_in_order() {
    let mut pool = Pool::new(4);
    for value in [3, 1, 2] {
        pool.put(value);
    }
    let state = run_visit(&mut pool, false, false);
    assert_eq!(state.seen, [3, 1, 2]);
    assert_eq!(state.count_during, [3, 3, 3]);
}

#[test]
fn for_all_links_on_an_empty_queue_never_calls_back() {
    let mut pool = Pool::new(2);
    assert!(run_visit(&mut pool, false, false).seen.is_empty());
}

#[test]
fn callback_may_remove_the_link_it_is_given() {
    let mut pool = Pool::new(4);
    for value in 1..=4 {
        pool.put(value);
    }
    let state = run_visit(&mut pool, true, false);
    assert_eq!(state.seen, [1, 2, 3, 4]);
    assert_eq!(state.count_during, [4, 3, 2, 1]);
    pool.assert_consistent(&[]);
}

#[test]
fn callback_may_remove_and_free_the_link_it_is_given() {
    let mut pool = Pool::new(4);
    for value in 1..=4 {
        pool.put(value);
    }
    run_visit(&mut pool, true, true);
    assert_eq!(
        pool.queue.audit(),
        Ok(QueueAudit {
            capacity: 4,
            queued: 0,
            free: 4,
            detached: 0
        })
    );
}

// ---------------------------------------------------------------------------
// The C entry points, called as C would
// ---------------------------------------------------------------------------

#[test]
fn c_entry_points_drive_a_queue_end_to_end() {
    let mut queue = Queue::new();
    let pq: *mut Queue = &mut queue;
    unsafe {
        assert_eq!(InitQueue(pq, 3, ELEMENT_SIZE), TRUE);
        let a = AllocLink(pq);
        let b = AllocLink(pq);
        let c = AllocLink(pq);
        assert!(AllocLink(pq).is_null());
        PutQueue(pq, a);
        PutQueue(pq, c);
        InsertQueue(pq, b, c);
        assert_eq!(CountLinks(pq), 3);
        RemoveQueue(pq, a);
        FreeLink(pq, a);
        assert_eq!(CountLinks(pq), 2);
        assert_eq!(AllocLink(pq), a);
        ReinitQueue(pq);
        assert_eq!(CountLinks(pq), 0);
        assert_eq!(UninitQueue(pq), TRUE);
    }
}

// ---------------------------------------------------------------------------
// Invariant audit
// ---------------------------------------------------------------------------

#[test]
fn audit_counts_queued_free_and_detached_slots() {
    let mut pool = Pool::new(6);
    pool.put(1);
    pool.put(2);
    pool.alloc(3);
    assert_eq!(
        pool.queue.audit(),
        Ok(QueueAudit {
            capacity: 6,
            queued: 2,
            free: 3,
            detached: 1
        })
    );
}

#[test]
fn audit_reports_a_slot_that_is_queued_and_free() {
    let mut pool = Pool::new(3);
    let h = pool.put(1);
    pool.queue.free_list = h;
    assert!(matches!(
        pool.queue.audit(),
        Err(QueueFault::QueuedAndFree(_))
    ));
    pool.queue.free_list = std::ptr::null_mut();
}

#[test]
fn audit_reports_a_foreign_link() {
    let mut pool = Pool::new(3);
    pool.put(1);
    let mut stray = Element {
        link: Link {
            pred: std::ptr::null_mut(),
            succ: std::ptr::null_mut(),
        },
        value: 0,
    };
    pool.queue.head = (&mut stray as *mut Element).cast();
    assert!(matches!(
        pool.queue.audit(),
        Err(QueueFault::ForeignLink(_))
    ));
    pool.queue.head = std::ptr::null_mut();
}

#[test]
fn audit_reports_a_cycle_and_a_broken_pred_and_tail() {
    let mut pool = Pool::new(3);
    let a = pool.put(1);
    let b = pool.put(2);

    unsafe { (*b.cast::<Link>()).succ = a };
    assert!(matches!(pool.queue.audit(), Err(QueueFault::Cycle(_))));
    unsafe { (*b.cast::<Link>()).succ = std::ptr::null_mut() };

    unsafe { (*b.cast::<Link>()).pred = std::ptr::null_mut() };
    assert!(matches!(pool.queue.audit(), Err(QueueFault::BrokenPred(_))));
    unsafe { (*b.cast::<Link>()).pred = a };

    pool.queue.tail = a;
    assert_eq!(pool.queue.audit(), Err(QueueFault::BrokenTail));
    pool.queue.tail = b;
    assert!(pool.queue.audit().is_ok());
}

#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "is not a slot of the queue pool")]
fn freeing_a_pointer_outside_the_pool_trips_the_membership_assertion() {
    let mut pool = Pool::new(2);
    let mut stray = Element {
        link: Link {
            pred: std::ptr::null_mut(),
            succ: std::ptr::null_mut(),
        },
        value: 0,
    };
    unsafe { pool.queue.free_link((&mut stray as *mut Element).cast()) };
}

#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "is not a slot of the queue pool")]
fn a_pointer_into_the_middle_of_a_slot_trips_the_membership_assertion() {
    let mut pool = Pool::new(2);
    let h = pool.alloc(1);
    unsafe { pool.queue.put(h.cast::<u8>().wrapping_add(1).cast()) };
}

// ---------------------------------------------------------------------------
// Layout and signatures against the retained C header
// ---------------------------------------------------------------------------

#[test]
fn layout_matches_the_c_struct_generated_from_displist_h() {
    use header_layout as c;

    assert_eq!(size_of::<Queue>(), size_of::<c::Queue>());
    assert_eq!(align_of::<Queue>(), align_of::<c::Queue>());
    assert_eq!(offset_of!(Queue, head), offset_of!(c::Queue, head));
    assert_eq!(offset_of!(Queue, tail), offset_of!(c::Queue, tail));
    assert_eq!(offset_of!(Queue, pq_tab), offset_of!(c::Queue, pq_tab));
    assert_eq!(
        offset_of!(Queue, free_list),
        offset_of!(c::Queue, free_list)
    );
    assert_eq!(
        offset_of!(Queue, object_size),
        offset_of!(c::Queue, object_size)
    );
    assert_eq!(
        offset_of!(Queue, num_objects),
        offset_of!(c::Queue, num_objects)
    );

    assert_eq!(size_of::<Link>(), size_of::<c::Link>());
    assert_eq!(align_of::<Link>(), align_of::<c::Link>());
    assert_eq!(offset_of!(Link, pred), offset_of!(c::Link, pred));
    assert_eq!(offset_of!(Link, succ), offset_of!(c::Link, succ));
}

#[cfg(target_pointer_width = "64")]
#[test]
fn layout_is_the_documented_64_bit_layout() {
    assert_eq!(size_of::<Queue>(), 40);
    assert_eq!(offset_of!(Queue, head), 0);
    assert_eq!(offset_of!(Queue, tail), 8);
    assert_eq!(offset_of!(Queue, pq_tab), 16);
    assert_eq!(offset_of!(Queue, free_list), 24);
    assert_eq!(offset_of!(Queue, object_size), 32);
    assert_eq!(offset_of!(Queue, num_objects), 34);
    assert_eq!(size_of::<Link>(), 16);
    assert_eq!(offset_of!(Link, pred), 0);
    assert_eq!(offset_of!(Link, succ), 8);
}

#[test]
fn exports_have_the_signatures_declared_in_displist_h() {
    use header_signatures as c;

    let _: unsafe extern "C" fn(*mut Queue, u16, ObjSize) -> Boolean = c::InitQueue;
    let _: unsafe extern "C" fn(*mut Queue, u16, ObjSize) -> Boolean = InitQueue;
    let _: unsafe extern "C" fn(*mut Queue) -> Boolean = c::UninitQueue;
    let _: unsafe extern "C" fn(*mut Queue) -> Boolean = UninitQueue;
    let _: unsafe extern "C" fn(*mut Queue) = c::ReinitQueue;
    let _: unsafe extern "C" fn(*mut Queue) = ReinitQueue;
    let _: unsafe extern "C" fn(*mut Queue) -> HLink = c::AllocLink;
    let _: unsafe extern "C" fn(*mut Queue) -> HLink = AllocLink;
    let _: unsafe extern "C" fn(*mut Queue, HLink) = c::FreeLink;
    let _: unsafe extern "C" fn(*mut Queue, HLink) = FreeLink;
    let _: unsafe extern "C" fn(*mut Queue, HLink) = c::PutQueue;
    let _: unsafe extern "C" fn(*mut Queue, HLink) = PutQueue;
    let _: unsafe extern "C" fn(*mut Queue, HLink, HLink) = c::InsertQueue;
    let _: unsafe extern "C" fn(*mut Queue, HLink, HLink) = InsertQueue;
    let _: unsafe extern "C" fn(*mut Queue, HLink) = c::RemoveQueue;
    let _: unsafe extern "C" fn(*mut Queue, HLink) = RemoveQueue;
    let _: unsafe extern "C" fn(*mut Queue) -> u16 = c::CountLinks;
    let _: unsafe extern "C" fn(*mut Queue) -> u16 = CountLinks;

    // The header takes a nullable callback; `Option` of a function pointer has
    // the same representation as the non-null pointer used here.
    type HeaderCallback = Option<unsafe extern "C" fn(*mut Link, *mut c_void)>;
    let _: unsafe extern "C" fn(*mut Queue, HeaderCallback, *mut c_void) = c::ForAllLinks;
    let _: unsafe extern "C" fn(*mut Queue, LinkCallback, *mut c_void) = ForAllLinks;
    assert_eq!(size_of::<LinkCallback>(), size_of::<HeaderCallback>());
    assert_eq!(size_of::<Boolean>(), size_of::<c_uint>());
}

// ---------------------------------------------------------------------------
// Model check against a plain vector
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
enum Op {
    Alloc,
    Free(usize),
    Put(usize),
    Insert(usize, usize),
    Remove(usize),
    Reinit,
}

fn op() -> impl Strategy<Value = Op> {
    prop_oneof![
        3 => Just(Op::Alloc),
        2 => any::<usize>().prop_map(Op::Free),
        3 => any::<usize>().prop_map(Op::Put),
        3 => (any::<usize>(), any::<usize>()).prop_map(|(a, b)| Op::Insert(a, b)),
        3 => any::<usize>().prop_map(Op::Remove),
        1 => Just(Op::Reinit),
    ]
}

/// What the queue must contain, tracked with plain vectors of slot addresses.
struct Model {
    capacity: usize,
    queued: Vec<usize>,
    detached: Vec<usize>,
}

impl Model {
    fn free(&self) -> usize {
        self.capacity - self.queued.len() - self.detached.len()
    }
}

proptest! {
    /// Any sequence of valid operations keeps the queue equal to a vector
    /// model, conserves slots, and never breaks a queue invariant.
    #[test]
    fn queue_matches_a_vector_model(capacity in 1u16..=255, ops in vec(op(), 0..160)) {
        let mut pool = Pool::new(capacity);
        let mut model = Model { capacity: usize::from(capacity), queued: vec![], detached: vec![] };
        let mut next_value = 0u32;
        let mut values: std::collections::BTreeMap<usize, u32> = Default::default();

        for op in ops {
            match op {
                Op::Alloc => {
                    let h = pool.queue.alloc_link();
                    if model.free() == 0 {
                        prop_assert!(h.is_null());
                    } else {
                        prop_assert!(!h.is_null());
                        prop_assert!(!model.queued.contains(&h.addr()));
                        prop_assert!(!model.detached.contains(&h.addr()));
                        next_value += 1;
                        unsafe { (*h.cast::<Element>()).value = next_value };
                        values.insert(h.addr(), next_value);
                        model.detached.push(h.addr());
                    }
                }
                Op::Free(i) if !model.detached.is_empty() => {
                    let addr = model.detached.remove(i % model.detached.len());
                    unsafe { pool.queue.free_link(std::ptr::with_exposed_provenance_mut(addr)) };
                }
                Op::Put(i) if !model.detached.is_empty() => {
                    let addr = model.detached.remove(i % model.detached.len());
                    unsafe { pool.queue.put(std::ptr::with_exposed_provenance_mut(addr)) };
                    model.queued.push(addr);
                }
                Op::Insert(i, j) if !model.detached.is_empty() => {
                    let addr = model.detached.remove(i % model.detached.len());
                    let position = j % (model.queued.len() + 1);
                    let reference = model.queued.get(position).copied().map_or(std::ptr::null_mut(), std::ptr::with_exposed_provenance_mut);
                    unsafe { pool.queue.insert(std::ptr::with_exposed_provenance_mut(addr), reference) };
                    model.queued.insert(position, addr);
                }
                Op::Remove(i) if !model.queued.is_empty() => {
                    let addr = model.queued.remove(i % model.queued.len());
                    unsafe { pool.queue.remove(std::ptr::with_exposed_provenance_mut(addr)) };
                    model.detached.push(addr);
                }
                Op::Reinit => {
                    pool.queue.reinit();
                    model.queued.clear();
                    model.detached.clear();
                }
                _ => {}
            }

            let expected: Vec<u32> = model.queued.iter().map(|a| values[a]).collect();
            prop_assert_eq!(pool.values(), expected.clone());
            prop_assert_eq!(pool.values_from_tail(), expected);
            prop_assert_eq!(usize::from(pool.queue.count()), model.queued.len());
            prop_assert_eq!(
                pool.queue.audit(),
                Ok(QueueAudit {
                    capacity: capacity as u8,
                    queued: model.queued.len() as u8,
                    free: model.free() as u8,
                    detached: model.detached.len() as u8,
                })
            );
        }
    }
}

#[test]
fn allocation_and_free_cycles_conserve_every_slot() {
    let mut pool = Pool::new(MAX_QUEUE_OBJECTS);
    for round in 0..200u32 {
        let mut held = Vec::new();
        while let Some(h) = Some(pool.queue.alloc_link()).filter(|h| !h.is_null()) {
            unsafe { (*h.cast::<Element>()).value = round };
            held.push(h);
        }
        assert_eq!(held.len(), usize::from(MAX_QUEUE_OBJECTS));
        for (n, h) in held.into_iter().enumerate() {
            if n % 2 == 0 {
                unsafe { pool.queue.put(h) };
            } else {
                unsafe { pool.queue.free_link(h) };
            }
        }
        let audit = pool.queue.audit().expect("consistent");
        assert_eq!(audit.queued, 128);
        assert_eq!(audit.free, 127);
        pool.queue.reinit();
    }
}
