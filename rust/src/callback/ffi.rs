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

#[no_mangle]
pub extern "C" fn Alarm_timeBeforeNextMs() -> u32 {
    let heap = match ALARM_HEAP.get() {
        Some(h) => h,
        None => return u32::MAX,
    };
    let cancelled = ALARM_CANCELLED.get();
    let now = now_ms();
    let guard = heap.lock();

    // Find the earliest non-cancelled alarm.
    for entry in guard.iter() {
        if cancelled
            .map(|c| c.lock().contains(&entry.id))
            .unwrap_or(false)
        {
            continue;
        }
        return entry.deadline_ms.saturating_sub(now);
    }
    u32::MAX
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
