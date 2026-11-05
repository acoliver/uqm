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

#[test]
fn recording_jobs_one_runs_scenarios_serially() {
    let args = ["--all", "--out", "record", "--jobs", "1"].map(str::to_owned);
    let options = options::Options::parse("record", &args).unwrap();
    let active = AtomicUsize::new(0);
    let order = std::sync::Mutex::new(Vec::new());
    let scenarios = ["comm", "samatra", "new-game"];
    let results = parallel::map(&scenarios, options.workers(), |scenario| {
        assert_eq!(active.fetch_add(1, Ordering::SeqCst), 0);
        order.lock().unwrap().push(*scenario);
        std::thread::yield_now();
        assert_eq!(active.fetch_sub(1, Ordering::SeqCst), 1);
        *scenario
    });
    assert_eq!(results, scenarios);
    assert_eq!(*order.lock().unwrap(), scenarios);
}

#[test]
fn jobs_requires_a_positive_count_and_defaults_to_available_parallelism() {
    let parse = |extra: &[&str]| {
        let args: Vec<_> = ["--all", "--out", "record"]
            .into_iter()
            .chain(extra.iter().copied())
            .map(str::to_owned)
            .collect();
        options::Options::parse("record", &args)
    };
    assert_eq!(parse(&[]).unwrap().workers(), parallel::workers());
    for value in ["0", "invalid", "-1"] {
        assert!(parse(&["--jobs", value]).is_err(), "accepted {value}");
    }
    assert!(parse(&["--jobs"]).is_err());
    assert_eq!(parse(&["--jobs", "2"]).unwrap().workers().get(), 2);
}
