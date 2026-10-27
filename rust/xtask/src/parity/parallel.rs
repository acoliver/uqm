use std::num::NonZeroUsize;
use std::sync::atomic::{AtomicUsize, Ordering};

pub(crate) fn workers() -> NonZeroUsize {
    std::thread::available_parallelism().unwrap_or(NonZeroUsize::MIN)
}

// A shared queue avoids waiting for the slowest scenario in each fixed batch.
// Results keep input order; completion order never affects the recording bytes.
pub(crate) fn map<T: Sync, R: Send>(
    items: &[T],
    workers: NonZeroUsize,
    run: impl Fn(&T) -> R + Sync,
) -> Vec<R> {
    let next = AtomicUsize::new(0);
    let mut results = std::thread::scope(|scope| {
        let children: Vec<_> = (0..workers.get().min(items.len()))
            .map(|_| {
                scope.spawn(|| {
                    let mut results = Vec::new();
                    loop {
                        let index = next.fetch_add(1, Ordering::Relaxed);
                        let Some(item) = items.get(index) else { break };
                        results.push((index, run(item)));
                    }
                    results
                })
            })
            .collect();
        children
            .into_iter()
            .flat_map(|child| {
                child
                    .join()
                    .unwrap_or_else(|panic| std::panic::resume_unwind(panic))
            })
            .collect::<Vec<_>>()
    });
    results.sort_unstable_by_key(|(index, _)| *index);
    results.into_iter().map(|(_, result)| result).collect()
}
