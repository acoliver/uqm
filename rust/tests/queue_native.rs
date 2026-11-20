//! Production draw queue ordering, batching and no-swap flush contract.
use std::sync::atomic::{AtomicUsize, Ordering};
use uqm_rust::engine::test_support::native_probe::{queue_sizes, rendered_frames, DrawQueue};
use uqm_rust::graphics::state::{
    batch, batch_depth, callback, reset_batch, unbatch, wait_for_signal,
};

static NEXT: AtomicUsize = AtomicUsize::new(0);

fn ordered(expected: usize) {
    callback(move || assert_eq!(NEXT.fetch_add(1, Ordering::SeqCst), expected));
}

#[test]
fn nested_batches_publish_in_order_and_wait_flushes_without_a_presenter() {
    let _queue = DrawQueue::init();
    assert_eq!(batch_depth(), 0);
    unbatch();
    assert_eq!(batch_depth(), 0);
    batch();
    ordered(0);
    batch();
    ordered(1);
    assert_eq!(batch_depth(), 2);
    assert_eq!(queue_sizes().1, 0);
    unbatch();
    assert_eq!(queue_sizes().1, 0);
    wait_for_signal();
    assert_eq!(batch_depth(), 1);
    assert_eq!(NEXT.load(Ordering::SeqCst), 2);
    assert_eq!(queue_sizes(), (0, 0));
    assert_eq!(rendered_frames(), 1);
    reset_batch();
    assert_eq!(batch_depth(), 0);
    unbatch();
    assert_eq!(batch_depth(), 0);
}
