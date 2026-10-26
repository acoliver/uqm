//! Game callback, alarm and asynchronous dispatch queues.

#![allow(non_snake_case)]

use std::collections::{BinaryHeap, HashSet, VecDeque};
use std::sync::OnceLock;

use parking_lot::Mutex;

type Callback = Box<dyn FnOnce() + Send>;

fn now_ms() -> u32 {
    crate::time::source::now_ms()
}

/// Identity of a queued callback, valid until it runs or is cancelled.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CallbackId(u64);

struct CallbackLink {
    id: CallbackId,
    func: Callback,
    cancelled: bool,
}

struct CallbackState {
    queue: VecDeque<CallbackLink>,
    next_id: u64,
}

static CALLBACKS: OnceLock<Mutex<CallbackState>> = OnceLock::new();

pub fn Callback_init() {
    let _ = CALLBACKS.set(Mutex::new(CallbackState {
        queue: VecDeque::new(),
        next_id: 1,
    }));
}

pub fn Callback_uninit() {
    if let Some(state) = CALLBACKS.get() {
        state.lock().queue.clear();
    }
}

pub fn Callback_add<T: Send + 'static>(
    func: impl FnOnce(T) + Send + 'static,
    arg: T,
) -> Option<CallbackId> {
    let mut state = CALLBACKS.get()?.lock();
    let id = CallbackId(state.next_id);
    state.next_id = state.next_id.checked_add(1).expect("callback ID exhausted");
    state.queue.push_back(CallbackLink {
        id,
        func: Box::new(move || func(arg)),
        cancelled: false,
    });
    Some(id)
}

pub fn Callback_remove(id: CallbackId) -> bool {
    let Some(state) = CALLBACKS.get() else {
        return false;
    };
    let mut state = state.lock();
    for entry in &mut state.queue {
        if entry.id == id {
            entry.cancelled = true;
            return true;
        }
    }
    false
}

pub fn Callback_process() {
    let Some(state) = CALLBACKS.get() else {
        return;
    };
    // Defer callbacks added during dispatch until the next snapshot.
    let process_end = state.lock().queue.len();
    for _ in 0..process_end {
        let entry = state.lock().queue.pop_front();
        match entry {
            Some(entry) => {
                if !entry.cancelled {
                    (entry.func)();
                }
            }
            None => break,
        }
    }
    state.lock().queue.retain(|entry| !entry.cancelled);
}

pub fn Callback_haveMore() -> bool {
    CALLBACKS
        .get()
        .is_some_and(|state| state.lock().queue.iter().any(|entry| !entry.cancelled))
}

/// Identity of a timed alarm. Cancellation does not dereference caller storage.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct AlarmId(u64);

struct AlarmEntry {
    deadline_ms: u32,
    callback: Callback,
    id: AlarmId,
}

impl PartialEq for AlarmEntry {
    fn eq(&self, other: &Self) -> bool {
        self.deadline_ms == other.deadline_ms
    }
}
impl Eq for AlarmEntry {}
impl PartialOrd for AlarmEntry {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for AlarmEntry {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other.deadline_ms.cmp(&self.deadline_ms)
    }
}

static ALARM_HEAP: OnceLock<Mutex<BinaryHeap<AlarmEntry>>> = OnceLock::new();
static ALARM_NEXT_ID: OnceLock<Mutex<u64>> = OnceLock::new();
static ALARM_CANCELLED: OnceLock<Mutex<HashSet<AlarmId>>> = OnceLock::new();

pub fn Alarm_init() {
    let _ = ALARM_HEAP.set(Mutex::new(BinaryHeap::new()));
    let _ = ALARM_NEXT_ID.set(Mutex::new(1));
    let _ = ALARM_CANCELLED.set(Mutex::new(HashSet::new()));
}

pub fn Alarm_uninit() {
    if let Some(heap) = ALARM_HEAP.get() {
        heap.lock().clear();
    }
}

fn alarm_add(deadline_ms: u32, callback: Callback) -> Option<AlarmId> {
    let heap = ALARM_HEAP.get()?;
    let mut next_id = ALARM_NEXT_ID.get()?.lock();
    let id = AlarmId(*next_id);
    *next_id = next_id.checked_add(1).expect("alarm ID exhausted");
    drop(next_id);
    heap.lock().push(AlarmEntry {
        deadline_ms,
        callback,
        id,
    });
    Some(id)
}

pub fn Alarm_addAbsoluteMs<T: Send + 'static>(
    ms: u32,
    callback: impl FnOnce(T) + Send + 'static,
    arg: T,
) -> Option<AlarmId> {
    alarm_add(ms, Box::new(move || callback(arg)))
}

pub fn Alarm_addRelativeMs<T: Send + 'static>(
    ms: u32,
    callback: impl FnOnce(T) + Send + 'static,
    arg: T,
) -> Option<AlarmId> {
    Alarm_addAbsoluteMs(now_ms().wrapping_add(ms), callback, arg)
}

pub fn Alarm_remove(alarm: Option<AlarmId>) {
    if let (Some(id), Some(cancelled)) = (alarm, ALARM_CANCELLED.get()) {
        cancelled.lock().insert(id);
    }
}

pub fn Alarm_processOne() -> bool {
    let Some(heap) = ALARM_HEAP.get() else {
        return false;
    };
    let cancelled = ALARM_CANCELLED.get();
    let now = now_ms();
    let mut guard = heap.lock();
    loop {
        match guard.peek() {
            None => return false,
            Some(top) => {
                if cancelled
                    .map(|c| c.lock().contains(&top.id))
                    .unwrap_or(false)
                {
                    guard.pop();
                    continue;
                }
                if now < top.deadline_ms {
                    return false;
                }
                let alarm = guard.pop().unwrap();
                drop(guard);
                (alarm.callback)();
                return true;
            }
        }
    }
}

/// Milliseconds until the next live alarm, zero if overdue, or `u32::MAX`
/// when no alarm is queued.
pub fn Alarm_timeBeforeNextMs() -> u32 {
    let Some(heap) = ALARM_HEAP.get() else {
        return u32::MAX;
    };
    let cancelled = ALARM_CANCELLED.get();
    let now = now_ms();
    heap.lock()
        .iter()
        .filter(|entry| {
            !cancelled
                .map(|c| c.lock().contains(&entry.id))
                .unwrap_or(false)
        })
        .map(|entry| entry.deadline_ms.saturating_sub(now))
        .min()
        .unwrap_or(u32::MAX)
}

pub fn Async_process() {
    Callback_process();
    loop {
        if !Alarm_processOne() {
            return;
        }
        Callback_process();
    }
}

pub fn Async_timeBeforeNextMs() -> u32 {
    if Callback_haveMore() {
        return 1;
    }
    Alarm_timeBeforeNextMs()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::sync::atomic::{AtomicU32, Ordering};

    static CALL_COUNT: AtomicU32 = AtomicU32::new(0);

    fn test_callback(_arg: ()) {
        CALL_COUNT.fetch_add(1, Ordering::SeqCst);
    }

    fn ensure_callbacks() -> &'static Mutex<CallbackState> {
        CALLBACKS.get_or_init(|| {
            Mutex::new(CallbackState {
                queue: VecDeque::new(),
                next_id: 1,
            })
        })
    }

    #[test]
    #[serial]
    fn callback_add_process_remove() {
        let state = ensure_callbacks();
        state.lock().queue.clear();

        CALL_COUNT.store(0, Ordering::SeqCst);

        let _id1 = Callback_add(test_callback, ());
        let id2 = Callback_add(test_callback, ()).unwrap();

        assert!(Callback_haveMore());

        // Remove id2 before processing.
        assert!(Callback_remove(id2));

        Callback_process();

        assert_eq!(CALL_COUNT.load(Ordering::SeqCst), 1);
        assert!(!Callback_haveMore());
    }

    #[test]
    #[serial]
    fn callback_process_snapshot() {
        let state = ensure_callbacks();
        state.lock().queue.clear();

        CALL_COUNT.store(0, Ordering::SeqCst);

        // This callback adds another callback during processing.
        fn chained_callback(_arg: ()) {
            CALL_COUNT.fetch_add(1, Ordering::SeqCst);
            Callback_add(chained_callback, ());
        }

        Callback_add(chained_callback, ());

        // First round: fires the chained callback (count=1), which adds
        // a new callback. The new one should NOT fire this round.
        Callback_process();
        assert_eq!(CALL_COUNT.load(Ordering::SeqCst), 1);
        assert!(Callback_haveMore());

        // Second round: fires the callback added during the first round.
        Callback_process();
        assert_eq!(CALL_COUNT.load(Ordering::SeqCst), 2);
    }
}

#[cfg(test)]
mod alarm_and_async_tests {
    use super::*;
    use serial_test::serial;
    use std::sync::Mutex as StdMutex;

    /// Argument values of the callbacks that fired, in firing order.
    static FIRED: StdMutex<Vec<usize>> = StdMutex::new(Vec::new());

    fn record(arg: usize) {
        FIRED.lock().unwrap().push(arg);
    }

    /// Queue another callback carrying `arg + 100`, from inside an alarm.
    fn record_and_queue_follow_up(arg: usize) {
        FIRED.lock().unwrap().push(arg);
        Callback_add(record, arg + 100);
    }

    fn fired() -> Vec<usize> {
        FIRED.lock().unwrap().clone()
    }

    /// Start every test from empty queues, whatever ran before it.
    fn fresh() {
        FIRED.lock().unwrap().clear();
        Callback_init();
        Alarm_init();
        Callback_uninit();
        ALARM_HEAP.get().unwrap().lock().clear();
        ALARM_CANCELLED.get().unwrap().lock().clear();
        // Alarms use the SDL tick clock; make sure it has moved past the
        // small deadlines the tests use.
        while now_ms() < 20 {
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
    }
    #[test]
    #[serial]
    fn owned_arguments_and_captures_are_dispatched_once() {
        fresh();
        let (sender, receiver) = std::sync::mpsc::channel();
        Callback_add(
            move |text| sender.send(text).unwrap(),
            String::from("callback"),
        );
        let (sender, alarm_receiver) = std::sync::mpsc::channel();
        Alarm_addAbsoluteMs(
            1,
            move |text| sender.send(text).unwrap(),
            String::from("alarm"),
        );

        Async_process();

        assert_eq!(receiver.recv().unwrap(), "callback");
        assert_eq!(alarm_receiver.recv().unwrap(), "alarm");
        assert!(receiver.try_recv().is_err());
        assert!(alarm_receiver.try_recv().is_err());
        assert!(!Callback_haveMore());
        assert!(!Alarm_processOne());
    }

    #[test]
    #[serial]
    fn stale_callback_ids_cannot_cancel_later_callbacks() {
        fresh();
        let stale = Callback_add(record, id(1)).unwrap();
        Callback_process();
        let current = Callback_add(record, id(2)).unwrap();

        assert_ne!(stale, current);
        assert!(!Callback_remove(stale));
        Callback_process();
        assert_eq!(fired(), [1, 2]);
    }

    fn id(n: usize) -> usize {
        n
    }

    const FAR: u32 = 1_000_000;

    #[test]
    #[serial]
    fn due_alarms_fire_in_deadline_order_one_per_call() {
        fresh();
        Alarm_addAbsoluteMs(3, record, id(3));
        Alarm_addAbsoluteMs(1, record, id(1));
        Alarm_addAbsoluteMs(2, record, id(2));

        assert!(Alarm_processOne());
        assert!(Alarm_processOne());
        assert!(Alarm_processOne());
        assert!(!Alarm_processOne(), "nothing is left");

        assert_eq!(fired(), [1, 2, 3]);
    }

    #[test]
    #[serial]
    fn an_alarm_that_is_not_due_stays_queued() {
        fresh();
        Alarm_addAbsoluteMs(now_ms() + FAR, record, id(9));

        assert!(!Alarm_processOne());

        assert!(fired().is_empty());
        let wait = Alarm_timeBeforeNextMs();
        assert!(wait > FAR - 10_000 && wait <= FAR, "wait was {wait}");
    }

    #[test]
    #[serial]
    fn a_removed_alarm_never_fires_and_later_ones_still_do() {
        fresh();
        let doomed = Alarm_addAbsoluteMs(1, record, id(1));
        Alarm_addAbsoluteMs(2, record, id(2));

        Alarm_remove(doomed);

        assert!(Alarm_processOne());
        assert!(!Alarm_processOne());
        assert_eq!(fired(), [2]);
    }

    #[test]
    #[serial]
    fn removing_a_null_alarm_is_harmless() {
        fresh();
        Alarm_addAbsoluteMs(1, record, id(1));

        Alarm_remove(None);

        assert!(Alarm_processOne());
        assert_eq!(fired(), [1]);
    }

    #[test]
    #[serial]
    fn removing_the_earliest_alarm_does_not_lengthen_the_wait_for_the_next() {
        fresh();
        let base = now_ms() + FAR;
        let earliest = Alarm_addAbsoluteMs(base, record, id(1));
        Alarm_addAbsoluteMs(base + 300, record, id(2));
        Alarm_addAbsoluteMs(base + 100, record, id(3));

        Alarm_remove(earliest);

        // The next alarm that can fire is the one at base + 100.
        let wait = Alarm_timeBeforeNextMs();
        assert!(
            wait <= FAR + 100,
            "waiting {wait} ms would oversleep the alarm due after {} ms",
            FAR + 100
        );
        assert!(wait > FAR - 10_000);
    }

    #[test]
    #[serial]
    fn relative_alarms_are_due_that_many_milliseconds_from_now() {
        fresh();
        let handle = Alarm_addRelativeMs(FAR, record, id(5));
        assert!(handle.is_some());

        let wait = Alarm_timeBeforeNextMs();
        assert!(wait > FAR - 10_000 && wait <= FAR, "wait was {wait}");

        Alarm_remove(handle);
        assert_eq!(
            Alarm_timeBeforeNextMs(),
            u32::MAX,
            "only a cancelled alarm remained"
        );
        assert!(!Alarm_processOne());
    }

    #[test]
    #[serial]
    fn an_overdue_alarm_has_no_wait_left() {
        fresh();
        Alarm_addAbsoluteMs(5, record, id(1));

        assert_eq!(Alarm_timeBeforeNextMs(), 0);
        assert_eq!(Async_timeBeforeNextMs(), 0);
    }

    #[test]
    #[serial]
    fn the_wait_is_a_duration_not_a_clock_time() {
        fresh();
        let wait = 60_000;
        Alarm_addAbsoluteMs(now_ms() + wait, record, id(1));

        let measured = Alarm_timeBeforeNextMs();

        assert!(
            measured <= wait && measured > wait - 5_000,
            "expected about {wait} ms, got {measured}"
        );
    }

    #[test]
    #[serial]
    fn queued_callbacks_report_a_one_millisecond_wait_ahead_of_any_alarm() {
        fresh();
        Alarm_addAbsoluteMs(now_ms() + FAR, record, id(1));
        Callback_add(record, id(2));

        assert_eq!(Async_timeBeforeNextMs(), 1);

        Callback_process();
        let wait = Async_timeBeforeNextMs();
        assert!(wait > FAR - 10_000 && wait <= FAR, "wait was {wait}");
    }

    #[test]
    #[serial]
    fn with_no_alarms_the_wait_is_unbounded() {
        fresh();

        assert_eq!(Alarm_timeBeforeNextMs(), u32::MAX);
        assert!(!Alarm_processOne());
        assert_eq!(Async_timeBeforeNextMs(), u32::MAX);
    }

    #[test]
    #[serial]
    fn uninit_drops_every_pending_alarm() {
        fresh();
        Alarm_addAbsoluteMs(1, record, id(1));

        Alarm_uninit();

        assert!(!Alarm_processOne());
        assert!(fired().is_empty());
    }

    #[test]
    #[serial]
    fn alarm_handles_are_distinct_per_alarm() {
        fresh();
        let a = Alarm_addAbsoluteMs(now_ms() + FAR, record, id(1));
        let b = Alarm_addAbsoluteMs(now_ms() + FAR, record, id(2));

        assert!(a.is_some() && b.is_some() && a != b);
        assert_ne!(a, b);

        Alarm_remove(a);
        Alarm_remove(b);
    }

    #[test]
    #[serial]
    fn async_process_runs_callbacks_then_each_due_alarm_with_its_follow_ups() {
        fresh();
        Callback_add(record, id(1));
        Alarm_addAbsoluteMs(1, record_and_queue_follow_up, id(10));
        Alarm_addAbsoluteMs(2, record_and_queue_follow_up, id(20));

        Async_process();

        // The pending callback first, then each alarm followed by the
        // callback it queued before the next alarm fires.
        assert_eq!(fired(), [1, 10, 110, 20, 120]);
        assert!(!Callback_haveMore());
        assert!(!Alarm_processOne());
    }

    #[test]
    #[serial]
    fn async_wait_is_one_while_callbacks_are_pending_otherwise_the_next_alarm() {
        fresh();
        Alarm_addAbsoluteMs(now_ms() + FAR, record, id(1));
        let wait = Async_timeBeforeNextMs();
        assert!(wait > FAR - 10_000 && wait <= FAR, "wait was {wait}");

        Callback_add(record, id(2));
        assert_eq!(
            Async_timeBeforeNextMs(),
            1,
            "never 0, which callers reserve"
        );
    }

    #[test]
    #[serial]
    fn removing_an_unknown_callback_reports_failure() {
        fresh();
        let queued = Callback_add(record, id(1)).unwrap();

        assert!(!Callback_remove(CallbackId(u64::MAX)));
        assert!(Callback_remove(queued));

        Callback_process();
        assert!(fired().is_empty());
    }

    #[test]
    #[serial]
    fn callback_uninit_discards_what_was_queued() {
        fresh();
        Callback_add(record, id(1));
        Callback_add(record, id(2));
        assert!(Callback_haveMore());

        Callback_uninit();

        assert!(!Callback_haveMore());
        Callback_process();
        assert!(fired().is_empty());
    }

    #[test]
    #[serial]
    fn a_callback_cancelled_while_processing_is_swept_afterwards() {
        fresh();
        fn queue_and_cancel(_arg: ()) {
            let late = Callback_add(record, id(7)).unwrap();
            assert!(Callback_remove(late));
        }
        Callback_add(queue_and_cancel, ());

        Callback_process();

        assert!(fired().is_empty(), "the cancelled callback never ran");
        assert!(
            !Callback_haveMore(),
            "and it is not left behind for the next round"
        );
        assert_eq!(
            CALLBACKS.get().unwrap().lock().queue.len(),
            0,
            "the cancelled entry was swept out of the queue"
        );
    }
}
