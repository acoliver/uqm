//! Ship records are allocated, cleared and linked through the production queue.
use std::{mem, ptr};
use uqm_rust::collections::queue::{CountLinks, InitQueue, Link, Queue, UninitQueue};
use uqm_rust::ships::{ffi::build_ship, ffi_contract::CStarship, native::abi};

#[test]
fn ship_construction_clears_payload_and_preserves_allocation_order() {
    for size in [
        mem::size_of::<abi::STARSHIP>(),
        mem::size_of::<abi::SHIP_FRAGMENT>(),
    ] {
        let mut queue = Queue {
            head: ptr::null_mut(),
            tail: ptr::null_mut(),
            pq_tab: ptr::null_mut(),
            free_list: ptr::null_mut(),
            object_size: 0,
            num_objects: 0,
        };
        // SAFETY: queue is uninitialized writable storage and both record sizes include Link.
        assert!(unsafe { InitQueue(&mut queue, 2, size as u16) });
        unsafe {
            let prefix = mem::size_of::<Link>();
            for index in 0..2 {
                ptr::write_bytes(queue.pq_tab.add(index * size + prefix), 0xa5, size - prefix);
            }
            let first = build_ship(&mut queue, 3);
            let second = build_ship(&mut queue, 7);
            assert_eq!(first, queue.pq_tab.cast());
            assert_eq!(second, queue.pq_tab.add(size).cast());
            assert_eq!(queue.head, first);
            assert_eq!(queue.tail, second);
            assert_eq!((*first.cast::<Link>()).succ, second);
            assert_eq!((*second.cast::<Link>()).pred, first);
            assert_eq!((*first.cast::<CStarship>()).species_id, 3);
            assert_eq!((*second.cast::<CStarship>()).species_id, 7);
            let species_start = mem::offset_of!(CStarship, species_id);
            let species_end = species_start + mem::size_of::<i32>();
            for handle in [first, second] {
                let bytes = std::slice::from_raw_parts(handle.cast::<u8>(), size);
                assert!(bytes[prefix..species_start].iter().all(|byte| *byte == 0));
                assert!(bytes[species_end..].iter().all(|byte| *byte == 0));
            }
            assert!(build_ship(&mut queue, 9).is_null());
            assert_eq!(CountLinks(&queue), 2);
            assert!(UninitQueue(&mut queue));
        }
    }
}
