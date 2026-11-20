//! Game-thread callback queue and timed alarms, owned by the platform.
//!
//! Callbacks are a FIFO: `process_callbacks` runs the entries queued when it
//! starts and defers any queued while it runs to the next call. Alarms are a
//! min-heap by deadline; cancellation is lazy because alarms are few.

use std::collections::{binary_heap::PeekMut, BinaryHeap, HashSet, VecDeque};

use parking_lot::Mutex;

type Task = Box<dyn FnOnce() + Send>;

/// Identity of a queued callback, valid until it runs or is cancelled.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CallbackId(u64);

/// Identity of a timed alarm. Cancellation does not dereference caller storage.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct AlarmId(u64);

struct CallbackLink {
    id: CallbackId,
    task: Task,
    cancelled: bool,
}

struct AlarmEntry {
    deadline_ms: u32,
    task: Task,
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

#[derive(Default)]
struct Callbacks {
    queue: VecDeque<CallbackLink>,
    next_id: u64,
}

#[derive(Default)]
struct Alarms {
    heap: BinaryHeap<AlarmEntry>,
    cancelled: HashSet<AlarmId>,
    next_id: u64,
}

/// The callback queue and alarm heap of one game.
#[derive(Default)]
pub struct Dispatch {
    callbacks: Mutex<Callbacks>,
    alarms: Mutex<Alarms>,
}

impl Dispatch {
    pub fn new() -> Self {
        Self::default()
    }

    /// Drop every queued callback and alarm.
    pub fn clear(&self) {
        self.callbacks.lock().queue.clear();
        let mut alarms = self.alarms.lock();
        alarms.heap.clear();
        alarms.cancelled.clear();
    }

    pub fn add_callback<T: Send + 'static>(
        &self,
        func: impl FnOnce(T) + Send + 'static,
        arg: T,
    ) -> CallbackId {
        let mut callbacks = self.callbacks.lock();
        callbacks.next_id += 1;
        let id = CallbackId(callbacks.next_id);
        callbacks.queue.push_back(CallbackLink {
            id,
            task: Box::new(move || func(arg)),
            cancelled: false,
        });
        id
    }

    /// Cancel a queued callback; `false` when it already ran or never existed.
    pub fn remove_callback(&self, id: CallbackId) -> bool {
        let mut callbacks = self.callbacks.lock();
        match callbacks.queue.iter_mut().find(|entry| entry.id == id) {
            Some(entry) => {
                entry.cancelled = true;
                true
            }
            None => false,
        }
    }

    pub fn process_callbacks(&self) {
        let process_end = self.callbacks.lock().queue.len();
        for _ in 0..process_end {
            let Some(entry) = self.callbacks.lock().queue.pop_front() else {
                break;
            };
            if !entry.cancelled {
                (entry.task)();
            }
        }
        self.callbacks.lock().queue.retain(|entry| !entry.cancelled);
    }

    pub fn has_callbacks(&self) -> bool {
        self.callbacks
            .lock()
            .queue
            .iter()
            .any(|entry| !entry.cancelled)
    }

    pub fn add_alarm_absolute_ms<T: Send + 'static>(
        &self,
        deadline_ms: u32,
        callback: impl FnOnce(T) + Send + 'static,
        arg: T,
    ) -> AlarmId {
        let mut alarms = self.alarms.lock();
        alarms.next_id += 1;
        let id = AlarmId(alarms.next_id);
        alarms.heap.push(AlarmEntry {
            deadline_ms,
            task: Box::new(move || callback(arg)),
            id,
        });
        id
    }

    pub fn remove_alarm(&self, id: AlarmId) {
        self.alarms.lock().cancelled.insert(id);
    }

    /// Run the earliest alarm due at `now_ms`; `false` when none is due.
    pub fn process_one_alarm(&self, now_ms: u32) -> bool {
        let mut guard = self.alarms.lock();
        loop {
            let alarms = &mut *guard;
            let Some(top) = alarms.heap.peek_mut() else {
                return false;
            };
            if alarms.cancelled.remove(&top.id) {
                PeekMut::pop(top);
                continue;
            }
            if now_ms < top.deadline_ms {
                return false;
            }
            let alarm = PeekMut::pop(top);
            drop(guard);
            (alarm.task)();
            return true;
        }
    }

    /// Milliseconds until the next live alarm, zero if overdue, or `u32::MAX`
    /// when no alarm is queued.
    pub fn time_before_next_alarm_ms(&self, now_ms: u32) -> u32 {
        let alarms = self.alarms.lock();
        alarms
            .heap
            .iter()
            .filter(|entry| !alarms.cancelled.contains(&entry.id))
            .map(|entry| entry.deadline_ms.saturating_sub(now_ms))
            .min()
            .unwrap_or(u32::MAX)
    }

    /// Run the pending callbacks, then every due alarm, each followed by the
    /// callbacks it queued. `now_ms` is read once per alarm attempt.
    pub fn process(&self, mut now_ms: impl FnMut() -> u32) {
        self.process_callbacks();
        while self.process_one_alarm(now_ms()) {
            self.process_callbacks();
        }
    }

    /// One millisecond while callbacks are pending (zero is reserved by
    /// callers), otherwise the wait for the next alarm.
    pub fn time_before_next_ms(&self, now_ms: impl FnOnce() -> u32) -> u32 {
        if self.has_callbacks() {
            return 1;
        }
        self.time_before_next_alarm_ms(now_ms())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    type Fired = Arc<Mutex<Vec<usize>>>;

    fn recorder(fired: &Fired) -> impl Fn(usize) + Clone + Send + 'static {
        let fired = fired.clone();
        move |arg| fired.lock().push(arg)
    }

    fn fired(fired: &Fired) -> Vec<usize> {
        fired.lock().clone()
    }

    const FAR: u32 = 1_000_000;

    #[test]
    fn callbacks_run_once_in_order_and_cancelled_ones_never_run() {
        let dispatch = Dispatch::new();
        let log = Fired::default();
        dispatch.add_callback(recorder(&log), 1);
        let doomed = dispatch.add_callback(recorder(&log), 2);
        dispatch.add_callback(recorder(&log), 3);
        assert!(dispatch.has_callbacks());
        assert!(dispatch.remove_callback(doomed));
        dispatch.process_callbacks();
        assert_eq!(fired(&log), [1, 3]);
        assert!(!dispatch.has_callbacks());
    }

    #[test]
    fn callbacks_queued_while_processing_wait_for_the_next_round() {
        let dispatch = Arc::new(Dispatch::new());
        let log = Fired::default();
        let chained = {
            let (dispatch, log) = (dispatch.clone(), log.clone());
            move |arg: usize| {
                log.lock().push(arg);
                dispatch.add_callback(recorder(&log), arg + 100);
            }
        };
        dispatch.add_callback(chained, 1);
        dispatch.process_callbacks();
        assert_eq!(fired(&log), [1]);
        assert!(dispatch.has_callbacks());
        dispatch.process_callbacks();
        assert_eq!(fired(&log), [1, 101]);
    }

    #[test]
    fn stale_callback_ids_cannot_cancel_later_callbacks() {
        let dispatch = Dispatch::new();
        let log = Fired::default();
        let stale = dispatch.add_callback(recorder(&log), 1);
        dispatch.process_callbacks();
        let current = dispatch.add_callback(recorder(&log), 2);
        assert_ne!(stale, current);
        assert!(!dispatch.remove_callback(stale));
        dispatch.process_callbacks();
        assert_eq!(fired(&log), [1, 2]);
    }

    #[test]
    fn a_callback_cancelled_while_processing_is_swept_afterwards() {
        let dispatch = Arc::new(Dispatch::new());
        let log = Fired::default();
        let queue_and_cancel = {
            let (dispatch, log) = (dispatch.clone(), log.clone());
            move |_: ()| {
                let late = dispatch.add_callback(recorder(&log), 7);
                assert!(dispatch.remove_callback(late));
            }
        };
        dispatch.add_callback(queue_and_cancel, ());
        dispatch.process_callbacks();
        assert!(fired(&log).is_empty());
        assert!(!dispatch.has_callbacks());
        assert_eq!(dispatch.callbacks.lock().queue.len(), 0);
    }

    #[test]
    fn owned_arguments_are_dispatched_once() {
        let dispatch = Dispatch::new();
        let (sender, receiver) = std::sync::mpsc::channel();
        let alarm_sender = sender.clone();
        dispatch.add_callback(
            move |text| sender.send(text).unwrap(),
            String::from("callback"),
        );
        dispatch.add_alarm_absolute_ms(
            1,
            move |text| alarm_sender.send(text).unwrap(),
            String::from("alarm"),
        );
        dispatch.process(|| 20);
        assert_eq!(receiver.recv().unwrap(), "callback");
        assert_eq!(receiver.recv().unwrap(), "alarm");
        assert!(receiver.try_recv().is_err());
    }

    #[test]
    fn due_alarms_fire_in_deadline_order_one_per_call() {
        let dispatch = Dispatch::new();
        let log = Fired::default();
        for deadline in [3, 1, 2] {
            dispatch.add_alarm_absolute_ms(deadline, recorder(&log), deadline as usize);
        }
        assert!(dispatch.process_one_alarm(10));
        assert!(dispatch.process_one_alarm(10));
        assert!(dispatch.process_one_alarm(10));
        assert!(!dispatch.process_one_alarm(10));
        assert_eq!(fired(&log), [1, 2, 3]);
    }

    #[test]
    fn an_alarm_that_is_not_due_stays_queued() {
        let dispatch = Dispatch::new();
        let log = Fired::default();
        dispatch.add_alarm_absolute_ms(FAR, recorder(&log), 9);
        assert!(!dispatch.process_one_alarm(20));
        assert!(fired(&log).is_empty());
        assert_eq!(dispatch.time_before_next_alarm_ms(20), FAR - 20);
    }

    #[test]
    fn a_removed_alarm_never_fires_and_later_ones_still_do() {
        let dispatch = Dispatch::new();
        let log = Fired::default();
        let doomed = dispatch.add_alarm_absolute_ms(1, recorder(&log), 1);
        dispatch.add_alarm_absolute_ms(2, recorder(&log), 2);
        dispatch.remove_alarm(doomed);
        assert!(dispatch.process_one_alarm(20));
        assert!(!dispatch.process_one_alarm(20));
        assert_eq!(fired(&log), [2]);
    }

    #[test]
    fn removing_the_earliest_alarm_does_not_lengthen_the_wait_for_the_next() {
        let dispatch = Dispatch::new();
        let log = Fired::default();
        let earliest = dispatch.add_alarm_absolute_ms(FAR, recorder(&log), 1);
        dispatch.add_alarm_absolute_ms(FAR + 300, recorder(&log), 2);
        dispatch.add_alarm_absolute_ms(FAR + 100, recorder(&log), 3);
        dispatch.remove_alarm(earliest);
        assert_eq!(dispatch.time_before_next_alarm_ms(0), FAR + 100);
    }

    #[test]
    fn a_cancelled_alarm_alone_leaves_no_wait() {
        let dispatch = Dispatch::new();
        let log = Fired::default();
        let only = dispatch.add_alarm_absolute_ms(FAR, recorder(&log), 5);
        dispatch.remove_alarm(only);
        assert_eq!(dispatch.time_before_next_alarm_ms(0), u32::MAX);
        assert!(!dispatch.process_one_alarm(FAR + 1));
        assert!(dispatch.alarms.lock().heap.is_empty());
    }

    #[test]
    fn an_overdue_alarm_has_no_wait_left() {
        let dispatch = Dispatch::new();
        dispatch.add_alarm_absolute_ms(5, |_: ()| {}, ());
        assert_eq!(dispatch.time_before_next_alarm_ms(20), 0);
        assert_eq!(dispatch.time_before_next_ms(|| 20), 0);
    }

    #[test]
    fn queued_callbacks_report_a_one_millisecond_wait_ahead_of_any_alarm() {
        let dispatch = Dispatch::new();
        let log = Fired::default();
        dispatch.add_alarm_absolute_ms(FAR, recorder(&log), 1);
        dispatch.add_callback(recorder(&log), 2);
        assert_eq!(dispatch.time_before_next_ms(|| unreachable!()), 1);
        dispatch.process_callbacks();
        assert_eq!(dispatch.time_before_next_ms(|| 0), FAR);
    }

    #[test]
    fn with_no_alarms_the_wait_is_unbounded() {
        let dispatch = Dispatch::new();
        assert_eq!(dispatch.time_before_next_alarm_ms(0), u32::MAX);
        assert!(!dispatch.process_one_alarm(0));
        assert_eq!(dispatch.time_before_next_ms(|| 0), u32::MAX);
    }

    #[test]
    fn clear_drops_every_pending_callback_and_alarm() {
        let dispatch = Dispatch::new();
        let log = Fired::default();
        dispatch.add_alarm_absolute_ms(1, recorder(&log), 1);
        dispatch.add_callback(recorder(&log), 2);
        dispatch.clear();
        assert!(!dispatch.has_callbacks());
        assert!(!dispatch.process_one_alarm(20));
        dispatch.process_callbacks();
        assert!(fired(&log).is_empty());
    }

    #[test]
    fn alarm_handles_are_distinct_per_alarm() {
        let dispatch = Dispatch::new();
        let a = dispatch.add_alarm_absolute_ms(FAR, |_: ()| {}, ());
        let b = dispatch.add_alarm_absolute_ms(FAR, |_: ()| {}, ());
        assert_ne!(a, b);
    }

    #[test]
    fn process_runs_callbacks_then_each_due_alarm_with_its_follow_ups() {
        let dispatch = Arc::new(Dispatch::new());
        let log = Fired::default();
        let queue_follow_up = {
            let (dispatch, log) = (dispatch.clone(), log.clone());
            move |arg: usize| {
                log.lock().push(arg);
                dispatch.add_callback(recorder(&log), arg + 100);
            }
        };
        dispatch.add_callback(recorder(&log), 1);
        dispatch.add_alarm_absolute_ms(1, queue_follow_up.clone(), 10);
        dispatch.add_alarm_absolute_ms(2, queue_follow_up, 20);
        dispatch.process(|| 20);
        assert_eq!(fired(&log), [1, 10, 110, 20, 120]);
        assert!(!dispatch.has_callbacks());
        assert!(!dispatch.process_one_alarm(20));
    }
}
