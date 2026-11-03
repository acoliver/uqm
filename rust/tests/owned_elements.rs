use uqm_rust::engine::battle::ElementTable;

#[test]
fn owned_storage_preserves_order_reuse_and_reset_generations() {
    let mut elements = ElementTable::with_capacity(3);
    let a = elements.alloc().unwrap();
    let b = elements.alloc().unwrap();
    let c = elements.alloc().unwrap();
    assert_eq!([a.slot(), b.slot(), c.slot()], [0, 1, 2]);
    assert!(elements.alloc().is_none());
    elements.put(b);
    elements.put(a);
    elements.insert_before(c, Some(a));
    assert_eq!(elements.head(), Some(b));
    assert_eq!(elements.succ(b), Some(c));
    assert_eq!(elements.pred(a), Some(c));
    assert_eq!(elements.tail(), Some(a));
    elements.remove(c);
    elements.free(c);
    let reused = elements.alloc().unwrap();
    assert_eq!(reused.slot(), c.slot());
    assert_ne!(reused, c);
    assert!(!elements.contains(c));
    elements.reinit();
    assert!(elements.head().is_none());
    assert!(!elements.contains(a));
    let reset = elements.alloc().unwrap();
    assert_eq!(reset.slot(), a.slot());
    assert_ne!(reset, a);
}

#[test]
fn owned_storage_visits_appends_in_the_same_pass() {
    let mut elements = ElementTable::with_capacity(3);
    let first = elements.alloc().unwrap();
    elements.put(first);
    assert!(elements.succ(first).is_none());
    let second = elements.alloc().unwrap();
    elements.put(second);
    assert_eq!(elements.succ(first), Some(second));
    elements.remove(first);
    assert_eq!(elements.succ(first), Some(second));
}

#[test]
fn resizing_invalidates_old_ids_and_preserves_slot_addresses() {
    let mut elements = ElementTable::with_capacity(3);
    let old = elements.alloc().unwrap();
    let address = elements.raw(old);
    elements.reset_capacity(0);
    assert!(!elements.contains(old));
    assert!(elements.alloc().is_none());
    elements.reset_capacity(1);
    let replacement = elements.alloc().unwrap();
    assert_eq!(replacement.slot(), old.slot());
    assert_ne!(replacement.generation(), old.generation());
    assert_eq!(elements.raw(replacement), address);
    assert!(elements.alloc().is_none());
}

#[test]
#[should_panic(expected = "stale element id")]
fn freed_ids_cannot_read_a_reused_slot() {
    let mut elements = ElementTable::with_capacity(1);
    let old = elements.alloc().unwrap();
    elements.free(old);
    let replacement = elements.alloc().unwrap();
    assert_ne!(replacement, old);
    let _ = &elements[old];
}
