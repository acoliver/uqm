#[test]
fn retired_slot_identity_does_not_require_a_live_value() {
    let mut pool = uqm_engine::collections::pool::OrderedPool::<u32, 1>::new();
    let id = pool.alloc(7).unwrap();
    let address = std::ptr::from_ref(pool.get(id)) as usize;
    assert_eq!(pool.retired_address(id.index()), address);
    pool.free(id);
    assert!(!pool.contains(id));
    assert_eq!(pool.retired_address(id.index()), address);
    pool.reinit();
    assert_eq!(pool.retired_address(id.index()), address);
    let replacement = pool.alloc(9).unwrap();
    assert_ne!(id, replacement);
    assert_eq!(pool.retired_address(id.index()), address);
    assert!(std::panic::catch_unwind(|| pool.get(id)).is_err());
}

use uqm_engine::collections::{pool::OrderedPool, queue};

#[test]
fn allocation_order() {
    let mut pool = OrderedPool::<_, 150>::new();
    let a = pool.alloc(1).unwrap();
    let b = pool.alloc(2).unwrap();
    let c = pool.alloc(3).unwrap();
    assert_eq!([a.index(), b.index(), c.index()], [0, 1, 2]);
    pool.put(a);
    assert_eq!(pool.succ(a), None);
    pool.put(b);
    assert_eq!(pool.succ(a), Some(b));
    pool.put(c);
    assert_eq!(pool.head(), Some(a));
    assert_eq!(pool.succ(a), Some(b));
    assert_eq!(pool.succ(b), Some(c));
    assert_eq!(pool.pred(c), Some(b));
    assert_eq!(pool.tail(), Some(c));
    pool.remove(b);
    assert_eq!(pool.succ(b), Some(c));
    assert_eq!(pool.succ(a), Some(c));
    pool.insert_before(b, Some(c));
    assert_eq!(pool.succ(a), Some(b));
    assert_eq!(pool.pred(c), Some(b));
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| pool.put(b)));
    assert!(result.is_err());
    assert_eq!(pool.count(), 3);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| pool.free(b)));
    assert!(result.is_err());
    assert_eq!(pool.succ(a), Some(b));
    assert_eq!(pool.pred(c), Some(b));
}

#[test]
fn lifo_reuse_invalidates_stale_ids() {
    let mut pool = OrderedPool::<_, 3>::new();
    let a = pool.alloc(1).unwrap();
    let b = pool.alloc(2).unwrap();
    let c = pool.alloc(3).unwrap();
    pool.free(a);
    pool.free(c);
    let reused_c = pool.alloc(4).unwrap();
    let reused_a = pool.alloc(5).unwrap();
    assert_eq!(reused_c.index(), c.index());
    assert_eq!(reused_a.index(), a.index());
    assert_ne!(reused_c, c);
    assert_ne!(reused_a, a);
    assert!(!pool.contains(c));
    assert!(std::panic::catch_unwind(|| pool.get(c)).is_err());
    assert_eq!(*pool.get(b), 2);
    pool.reinit();
    assert!(!pool.contains(b));
    let first = pool.alloc(6).unwrap();
    assert_eq!(first.index(), 0);
    assert_ne!(first, a);
}

#[test]
fn exhaustion_at_150() {
    let mut pool = OrderedPool::<_, 150>::new();
    for i in 0..150 {
        let id = pool.alloc(i).unwrap();
        assert_eq!(id.index(), i);
        pool.put(id);
    }
    assert!(pool.alloc(150).is_none());
    assert_eq!(pool.count(), 150);

    let mut bounded = OrderedPool::<_, 32>::with_capacity(14);
    for value in 0..14 {
        assert_eq!(bounded.alloc(value).unwrap().index(), value);
    }
    assert!(bounded.alloc(14).is_none());
    bounded.reinit();
    for value in 0..14 {
        let id = bounded.alloc(value).unwrap();
        assert_eq!(id.index(), value);
        assert_eq!(bounded.id_at(value), Some(id));
    }
    assert!(bounded.alloc(14).is_none());
    assert!(bounded.id_at(14).is_none());
}

#[test]
fn randomized_queue_equivalence() {
    const N: usize = 150;
    let mut raw = queue::Queue::default();
    let mut pool = OrderedPool::<usize, N>::new();
    let mut slots = [None; N];
    let mut linked = [false; N];
    let mut state = 0x4d59_5df4_d0f3_3173_u64;
    // SAFETY: this test owns the queue and all links until UninitQueue below.
    unsafe {
        assert!(queue::InitQueue(
            &mut raw,
            N as u16,
            size_of::<queue::Link>() as u16,
        ));
        for step in 0..50_000 {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            let slot = (state >> 8) as usize % N;
            let address = raw.pq_tab.add(slot * size_of::<queue::Link>()).cast();
            match (state % 6, slots[slot], linked[slot]) {
                (0, _, _) => {
                    let raw_id = queue::AllocLink(&mut raw);
                    let id = pool.alloc(step);
                    assert_eq!(raw_id.is_null(), id.is_none());
                    if let Some(id) = id {
                        let ix = (raw_id as usize - raw.pq_tab as usize) / size_of::<queue::Link>();
                        assert_eq!(id.index(), ix);
                        slots[ix] = Some(id);
                        linked[ix] = false;
                    }
                }
                (1, Some(id), false) => {
                    queue::FreeLink(&mut raw, address);
                    pool.free(id);
                    slots[slot] = None;
                }
                (2, Some(id), false) => {
                    queue::PutQueue(&mut raw, address);
                    pool.put(id);
                    linked[slot] = true;
                }
                (3, Some(id), true) => {
                    queue::RemoveQueue(&mut raw, address);
                    pool.remove(id);
                    linked[slot] = false;
                }
                (4, Some(id), false) => {
                    let before = (slot + 1..N).find(|&i| linked[i]);
                    let raw_before = before.map_or(std::ptr::null_mut(), |ix| {
                        raw.pq_tab.add(ix * size_of::<queue::Link>()).cast()
                    });
                    queue::InsertQueue(&mut raw, address, raw_before);
                    pool.insert_before(id, before.and_then(|ix| slots[ix]));
                    linked[slot] = true;
                }
                (5, _, _) if step % 997 == 0 => {
                    queue::ReinitQueue(&mut raw);
                    pool.reinit();
                    slots.fill(None);
                    linked.fill(false);
                }
                _ => {}
            }
            let mut raw_link = raw.head;
            let mut pool_link = pool.head();
            let mut previous = None;
            let mut count = 0;
            while !raw_link.is_null() {
                let ix = (raw_link as usize - raw.pq_tab as usize) / size_of::<queue::Link>();
                let id = pool_link.unwrap();
                assert_eq!(id.index(), ix, "visit order at step {step}");
                assert_eq!(pool.pred(id), previous);
                previous = Some(id);
                pool_link = pool.succ(id);
                raw_link = (*raw_link.cast::<queue::Link>()).succ;
                count += 1;
                assert!(count <= N);
            }
            assert_eq!(pool_link, None);
            assert_eq!(pool.tail(), previous);
            assert_eq!(pool.count(), count);
        }
        queue::UninitQueue(&mut raw);
    }
}
