use super::*;
use std::num::NonZeroUsize;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Barrier,
};

#[test]
fn workers_run_concurrently_once_each_and_return_input_order_including_errors() {
    let visits: Vec<_> = (0..10).map(|_| AtomicUsize::new(0)).collect();
    let barrier = Barrier::new(2);
    let active = AtomicUsize::new(0);
    let peak = AtomicUsize::new(0);
    let inputs: Vec<_> = (0..10).collect();
    let results = parallel::map(&inputs, NonZeroUsize::new(2).unwrap(), |&index| {
        visits[index].fetch_add(1, Ordering::SeqCst);
        let count = active.fetch_add(1, Ordering::SeqCst) + 1;
        peak.fetch_max(count, Ordering::SeqCst);
        if index < 2 {
            barrier.wait();
        }
        active.fetch_sub(1, Ordering::SeqCst);
        if index == 3 {
            Err(index)
        } else {
            Ok(index)
        }
    });
    assert_eq!(peak.load(Ordering::SeqCst), 2);
    assert!(visits.iter().all(|count| count.load(Ordering::SeqCst) == 1));
    assert_eq!(
        results,
        inputs
            .into_iter()
            .map(|index| if index == 3 { Err(index) } else { Ok(index) })
            .collect::<Vec<_>>()
    );
}

#[test]
fn empty_and_single_worker_queues_keep_every_result() {
    let workers = NonZeroUsize::new(8).unwrap();
    assert!(parallel::map::<usize, usize>(&[], workers, |value| *value).is_empty());
    assert_eq!(
        parallel::map(&[2, 4, 6], NonZeroUsize::MIN, |value| value * 2),
        [4, 8, 12]
    );
    assert_eq!(parallel::map(&[7], workers, |value| *value), [7]);
}
