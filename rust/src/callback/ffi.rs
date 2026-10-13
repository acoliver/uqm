//! FFI shims for the C-callable Alarm/Callback/Async API.
//!
//! Exports `#[no_mangle] extern "C"` functions matching the signatures
//! declared in `alarm.h`, `callback.h`, and `async.h`. C code calls these
//! directly; the linker resolves them from the Rust archive.

use std::collections::{BinaryHeap, HashSet};
use std::ffi::c_void;
use std::sync::OnceLock;

use parking_lot::Mutex;

// ===========================================================================
// SDL timing — same time base as C (`SDL_GetTicks`)
// ===========================================================================

extern "C" {
    fn SDL_GetTicks() -> u32;
}

fn now_ms() -> u32 {
    unsafe { SDL_GetTicks() }
}

// ===========================================================================
// Callback — FIFO queue with snapshot semantics
// ===========================================================================

/// A single queued callback. Stored in a `Box` so the raw pointer is stable
/// and can serve as the `CallbackID` (matching C's linked-list-node approach).
struct CallbackLink {
    func: extern "C" fn(*mut c_void),
    arg: *mut c_void,
    cancelled: bool,
}

// SAFETY: The callback queue is accessed from the main game thread and
// network threads. The raw pointer `arg` is opaque C state.
unsafe impl Send for CallbackLink {}

struct CallbackState {
    /// Pending callbacks, front = next to process.
    queue: std::collections::VecDeque<Box<CallbackLink>>,
    /// Snapshot of `queue.len()` at the start of `Callback_process`.
    /// Entries added during processing have index >= process_end and
    /// are deferred to the next call (matching C `callbacksProcessEnd`).
    process_end: usize,
}

static CALLBACKS: OnceLock<Mutex<CallbackState>> = OnceLock::new();

#[no_mangle]
pub extern "C" fn Callback_init() {
    let _ = CALLBACKS.set(Mutex::new(CallbackState {
        queue: std::collections::VecDeque::new(),
        process_end: 0,
    }));
}

#[no_mangle]
pub extern "C" fn Callback_uninit() {
    if let Some(state) = CALLBACKS.get() {
        let mut s = state.lock();
        s.queue.clear();
        s.process_end = 0;
    }
}

#[no_mangle]
pub extern "C" fn Callback_add(func: extern "C" fn(*mut c_void), arg: *mut c_void) -> *mut c_void {
    let state = match CALLBACKS.get() {
        Some(s) => s,
        None => return std::ptr::null_mut(),
    };
    let link = Box::new(CallbackLink {
        func,
        arg,
        cancelled: false,
    });
    let ptr = Box::as_ref(&link) as *const CallbackLink as *mut c_void;
    state.lock().queue.push_back(link);
    ptr
}

#[no_mangle]
pub extern "C" fn Callback_remove(id: *mut c_void) -> bool {
    let state = match CALLBACKS.get() {
        Some(s) => s,
        None => return false,
    };
    let target = id as *const CallbackLink;
    let mut s = state.lock();
    // Find and mark as cancelled (lazy deletion — preserves indices).
    for entry in s.queue.iter_mut() {
        if std::ptr::eq(&**entry, target) {
            entry.cancelled = true;
            return true;
        }
    }
    false
}

#[no_mangle]
pub extern "C" fn Callback_process() {
    let state = match CALLBACKS.get() {
        Some(s) => s,
        None => return,
    };

    // Snapshot the current queue length — only process entries up to here.
    // New callbacks added during processing are deferred (matching C
    // `callbacksProcessEnd` semantics).
    let process_end = {
        let s = state.lock();
        s.queue.len()
    };

    for _ in 0..process_end {
        // Pop the front entry under the lock, then release before calling.
        let entry = state.lock().queue.pop_front();
        match entry {
            Some(e) => {
                if !e.cancelled {
                    (e.func)(e.arg);
                }
                // Box is dropped here, freeing the CallbackLink.
            }
            None => break,
        }
    }

    // Clean up any cancelled entries that were added during processing
    // (they would have been beyond process_end, so not popped above).
    if let Some(s) = CALLBACKS.get() {
        let mut s = s.lock();
        s.queue.retain(|e| !e.cancelled);
    }
}

#[no_mangle]
pub extern "C" fn Callback_haveMore() -> bool {
    let state = match CALLBACKS.get() {
        Some(s) => s,
        None => return false,
    };
    let s = state.lock();
    s.queue.iter().any(|e| !e.cancelled)
}

// ===========================================================================
// Alarm — min-heap of timed callbacks
// ===========================================================================

/// A timed alarm entry. When `deadline_ms` is reached, `callback(arg)` fires.
struct AlarmEntry {
    deadline_ms: u32,
    callback: extern "C" fn(*mut c_void),
    arg: *mut c_void,
    /// Unique ID used for cancellation lookup.
    id: u64,
}

// SAFETY: Alarms are accessed from the main game thread.
unsafe impl Send for AlarmEntry {}
unsafe impl Sync for AlarmEntry {}

impl PartialEq for AlarmEntry {
    fn eq(&self, other: &Self) -> bool {
        self.deadline_ms == other.deadline_ms
    }
}
impl Eq for AlarmEntry {}
impl PartialOrd for AlarmEntry {
    // Reverse ordering → BinaryHeap acts as min-heap by deadline.
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
static ALARM_CANCELLED: OnceLock<Mutex<HashSet<u64>>> = OnceLock::new();

/// The opaque `Alarm*` handle returned to C. Contains the unique ID for
/// cancellation lookup. Packed into a pointer-sized value so C can store
/// it as `Alarm*`.
#[repr(C)]
pub struct AlarmHandle {
    id: u64,
}

#[no_mangle]
pub extern "C" fn Alarm_init() {
    let _ = ALARM_HEAP.set(Mutex::new(BinaryHeap::new()));
    let _ = ALARM_NEXT_ID.set(Mutex::new(1));
    let _ = ALARM_CANCELLED.set(Mutex::new(HashSet::new()));
}

#[no_mangle]
pub extern "C" fn Alarm_uninit() {
    if let Some(heap) = ALARM_HEAP.get() {
        heap.lock().clear();
    }
}

fn alarm_add(
    deadline_ms: u32,
    callback: extern "C" fn(*mut c_void),
    arg: *mut c_void,
) -> *mut AlarmHandle {
    let heap = match ALARM_HEAP.get() {
        Some(h) => h,
        None => return std::ptr::null_mut(),
    };
    let next_id = match ALARM_NEXT_ID.get() {
        Some(n) => {
            let mut guard = n.lock();
            let id = *guard;
            *guard += 1;
            id
        }
        None => return std::ptr::null_mut(),
    };

    heap.lock().push(AlarmEntry {
        deadline_ms,
        callback,
        arg,
        id: next_id,
    });

    // Return a heap-allocated handle that C can store and pass to Alarm_remove.
    Box::into_raw(Box::new(AlarmHandle { id: next_id }))
}

#[no_mangle]
pub extern "C" fn Alarm_addAbsoluteMs(
    ms: u32,
    callback: extern "C" fn(*mut c_void),
    arg: *mut c_void,
) -> *mut AlarmHandle {
    alarm_add(ms, callback, arg)
}

#[no_mangle]
pub extern "C" fn Alarm_addRelativeMs(
    ms: u32,
    callback: extern "C" fn(*mut c_void),
    arg: *mut c_void,
) -> *mut AlarmHandle {
    alarm_add(now_ms().wrapping_add(ms), callback, arg)
}

#[no_mangle]
#[allow(
    clippy::not_unsafe_ptr_arg_deref,
    reason = "C ABI compatibility is fixed during the Rust migration; tracked by PLAN-20260723-RUNTIME-AUTOMATION.P00"
)]
pub extern "C" fn Alarm_remove(alarm: *mut AlarmHandle) {
    if alarm.is_null() {
        return;
    }
    let handle = unsafe { &*alarm };
    let id = handle.id;

    // Free the handle.
    unsafe {
        drop(Box::from_raw(alarm));
    }

    // Mark the alarm as cancelled (lazy deletion — BinaryHeap doesn't
    // support arbitrary removal, so we skip cancelled entries on pop).
    if let Some(cancelled) = ALARM_CANCELLED.get() {
        cancelled.lock().insert(id);
    }
}

#[no_mangle]
pub extern "C" fn Alarm_processOne() -> bool {
    let heap = match ALARM_HEAP.get() {
        Some(h) => h,
        None => return false,
    };
    let cancelled = ALARM_CANCELLED.get();
    let now = now_ms();

    // Pop entries until we find a non-cancelled one that's due, or stop.
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
                // Due — pop and fire.
                let alarm = guard.pop().unwrap();
                let callback = alarm.callback;
                let arg = alarm.arg;
                drop(guard); // release lock before callback
                (callback)(arg);
                return true;
            }
        }
    }
}

/// Milliseconds from now until the earliest live alarm is due.
///
/// The value is RELATIVE: 0 when an alarm is already overdue, `u32::MAX` when
/// none is queued. The original C returned the alarm's absolute tick time
/// here, which `netInputBlocking` then used as a timeout and `SleepThreadUntil`
/// compared against the clock; callers in this port (`pacing::sleep_until`,
/// `netInputBlocking`) all use it as a duration.
#[no_mangle]
pub extern "C" fn Alarm_timeBeforeNextMs() -> u32 {
    let heap = match ALARM_HEAP.get() {
        Some(h) => h,
        None => return u32::MAX,
    };
    let cancelled = ALARM_CANCELLED.get();
    let now = now_ms();
    let guard = heap.lock();

    // Heap iteration order is storage order, not deadline order, so the
    // earliest alarm that can still fire has to be found by comparing them.
    guard
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

// ===========================================================================
// Async — combined callback + alarm processing
// ===========================================================================

#[no_mangle]
pub extern "C" fn Async_process() {
    // Call pending callbacks first.
    Callback_process();

    // Then fire due alarms, processing callbacks after each alarm.
    loop {
        if !Alarm_processOne() {
            return;
        }
        Callback_process();
    }
}

#[no_mangle]
pub extern "C" fn Async_timeBeforeNextMs() -> u32 {
    if Callback_haveMore() {
        // Return 1 (not 0) so callers can use 0 as a special value.
        return 1;
    }
    Alarm_timeBeforeNextMs()
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::sync::atomic::{AtomicU32, Ordering};

    static CALL_COUNT: AtomicU32 = AtomicU32::new(0);

    extern "C" fn test_callback(_arg: *mut c_void) {
        CALL_COUNT.fetch_add(1, Ordering::SeqCst);
    }

    fn ensure_callbacks() -> &'static Mutex<CallbackState> {
        CALLBACKS.get_or_init(|| {
            Mutex::new(CallbackState {
                queue: std::collections::VecDeque::new(),
                process_end: 0,
            })
        })
    }

    #[test]
    #[serial]
    fn callback_add_process_remove() {
        let state = ensure_callbacks();
        state.lock().queue.clear();
        state.lock().process_end = 0;

        CALL_COUNT.store(0, Ordering::SeqCst);

        let _id1 = Callback_add(test_callback, std::ptr::null_mut());
        let id2 = Callback_add(test_callback, std::ptr::null_mut());

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
        state.lock().process_end = 0;

        CALL_COUNT.store(0, Ordering::SeqCst);

        // This callback adds another callback during processing.
        extern "C" fn chained_callback(_arg: *mut c_void) {
            CALL_COUNT.fetch_add(1, Ordering::SeqCst);
            Callback_add(chained_callback, std::ptr::null_mut());
        }

        Callback_add(chained_callback, std::ptr::null_mut());

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

    extern "C" fn record(arg: *mut c_void) {
        FIRED.lock().unwrap().push(arg as usize);
    }

    /// Queue another callback carrying `arg + 100`, from inside an alarm.
    extern "C" fn record_and_queue_follow_up(arg: *mut c_void) {
        FIRED.lock().unwrap().push(arg as usize);
        Callback_add(record, (arg as usize + 100) as *mut c_void);
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

    fn id(n: usize) -> *mut c_void {
        n as *mut c_void
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

        Alarm_remove(std::ptr::null_mut());

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
        assert!(!handle.is_null());

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

        assert!(!a.is_null() && !b.is_null() && a != b);
        assert_ne!(unsafe { (*a).id }, unsafe { (*b).id });

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
        let queued = Callback_add(record, id(1));

        assert!(!Callback_remove(id(0x10)));
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
        static LATE: StdMutex<usize> = StdMutex::new(0);
        extern "C" fn queue_and_cancel(_arg: *mut c_void) {
            let late = Callback_add(record, id(7));
            *LATE.lock().unwrap() = late as usize;
            assert!(Callback_remove(late));
        }
        Callback_add(queue_and_cancel, std::ptr::null_mut());

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
