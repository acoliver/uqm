//! Init-sequence wrappers for the UQM startup.
//!
//! Provides Rust-side equivalents of the C init functions called from
//! `uqm_c_do_init()`. Each is exported as `#[no_mangle] extern "C"` for
//! C to call when `RUST_OWNS_MAIN`.

use std::collections::BinaryHeap;
use std::sync::OnceLock;

use libc::{c_int, c_void};
use parking_lot::Mutex;

// ===========================================================================
// #1 TFB_PreInit — SDL video init
// ===========================================================================

extern "C" {
    fn SDL_Init(flags: u32) -> c_int;
    fn SDL_GetVersion(ver: *mut SdlVersion);
}

#[repr(C)]
struct SdlVersion {
    major: u8,
    minor: u8,
    patch: u8,
}

const SDL_INIT_VIDEO: u32 = 0x00000020;

/// Initialize SDL video subsystem from Rust.
///
/// Equivalent to C `TFB_PreInit()`: calls `SDL_Init(SDL_INIT_VIDEO)` and
/// logs the SDL version. SDL is cleaned up at process exit by the OS.
#[no_mangle]
pub extern "C" fn rust_tfb_preinit() -> c_int {
    tracing::info!("Initializing base SDL functionality.");

    let result = unsafe { SDL_Init(SDL_INIT_VIDEO) };
    if result != 0 {
        tracing::error!("Could not initialize SDL");
        return -1;
    }

    let mut ver = SdlVersion { major: 0, minor: 0, patch: 0 };
    unsafe { SDL_GetVersion(&mut ver) };
    tracing::info!(
        "Using SDL version {}.{}.{} (compiled with {}.{}.{})",
        ver.major, ver.minor, ver.patch, ver.major, ver.minor, ver.patch,
    );

    0
}

// ===========================================================================
// #2 log_initThreads — create logging lock
// ===========================================================================

/// Initialize the logging thread lock.
///
/// The C version creates a Mutex for the logging system. With tracing,
/// logging is already thread-safe, so this is a no-op that just ensures
/// compatibility with C code that checks the lock.
#[no_mangle]
pub extern "C" fn rust_log_init_threads() {
    // tracing::subscriber is already thread-safe; no mutex needed.
    tracing::debug!("log_initThreads: tracing handles thread safety natively");
}

// ===========================================================================
// #4 Alarm system — timer-based callback queue
// ===========================================================================

/// A timed alarm entry: fires `callback(arg)` at `deadline_ms`.
struct AlarmEntry {
    deadline_ms: u32,
    callback: extern "C" fn(*mut c_void),
    arg: *mut c_void,
}

// SAFETY: The alarm system is only accessed from the main game thread.
// Raw pointers in callbacks are opaque C-side state.
unsafe impl Send for AlarmEntry {}
unsafe impl Sync for AlarmEntry {}

impl PartialEq for AlarmEntry {
    fn eq(&self, other: &Self) -> bool {
        self.deadline_ms == other.deadline_ms
    }
}
impl Eq for AlarmEntry {}
impl PartialOrd for AlarmEntry {
    // Reverse ordering so BinaryHeap acts as a min-heap by deadline
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(other.deadline_ms.cmp(&self.deadline_ms))
    }
}
impl Ord for AlarmEntry {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        other.deadline_ms.cmp(&self.deadline_ms)
    }
}

static ALARM_HEAP: OnceLock<Mutex<BinaryHeap<AlarmEntry>>> = OnceLock::new();

/// Initialize the alarm system.
///
/// Creates an empty min-heap for timed callbacks. Equivalent to C
/// `Alarm_init()` which creates a `Heap` for `Alarm` structs.
#[no_mangle]
pub extern "C" fn rust_alarm_init() {
    let _ = ALARM_HEAP.set(Mutex::new(BinaryHeap::new()));
    tracing::debug!("Alarm system initialized");
}

/// Shut down the alarm system, draining all pending alarms.
#[no_mangle]
pub extern "C" fn rust_alarm_uninit() {
    if let Some(heap) = ALARM_HEAP.get() {
        heap.lock().clear();
        tracing::debug!("Alarm system uninitialized");
    }
}

/// Process at most one expired alarm. Returns 1 if an alarm fired, 0 if not.
///
/// Uses `SDL_GetTicks()` for the current time (matching C behavior).
#[no_mangle]
pub extern "C" fn rust_alarm_process_one() -> c_int {
    let heap = match ALARM_HEAP.get() {
        Some(h) => h,
        None => return 0,
    };

    extern "C" {
        fn SDL_GetTicks() -> u32;
    }
    let mut guard = heap.lock();
    let now = unsafe { SDL_GetTicks() };

    if let Some(top) = guard.peek() {
        if now >= top.deadline_ms {
            let alarm = guard.pop().unwrap();
            drop(guard); // release lock before callback
            (alarm.callback)(alarm.arg);
            return 1;
        }
    }
    0
}

/// Returns the milliseconds before the next alarm fires, or u32::MAX if none.
#[no_mangle]
pub extern "C" fn rust_alarm_time_before_next_ms() -> u32 {
    let heap = match ALARM_HEAP.get() {
        Some(h) => h,
        None => return u32::MAX,
    };
    extern "C" {
        fn SDL_GetTicks() -> u32;
    }
    let guard = heap.lock();
    let now = unsafe { SDL_GetTicks() };

    match guard.peek() {
        Some(top) => {
            if now >= top.deadline_ms {
                0
            } else {
                top.deadline_ms - now
            }
        }
        None => u32::MAX,
    }
}

// ===========================================================================
// #4 Callback system — deferred callback queue
// ===========================================================================

struct CallbackEntry {
    func: extern "C" fn(*mut c_void),
    arg: *mut c_void,
}

// SAFETY: The callback system is only accessed from the main game thread.
unsafe impl Send for CallbackEntry {}

static CALLBACK_QUEUE: OnceLock<Mutex<std::collections::VecDeque<CallbackEntry>>> = OnceLock::new();

/// Initialize the callback system.
///
/// Creates an empty callback queue with a mutex. Equivalent to C
/// `Callback_init()` which initializes a linked list + mutex.
#[no_mangle]
pub extern "C" fn rust_callback_init() {
    let _ = CALLBACK_QUEUE.set(Mutex::new(std::collections::VecDeque::new()));
    tracing::debug!("Callback system initialized");
}

/// Shut down the callback system, draining all pending callbacks.
#[no_mangle]
pub extern "C" fn rust_callback_uninit() {
    if let Some(queue) = CALLBACK_QUEUE.get() {
        queue.lock().clear();
        tracing::debug!("Callback system uninitialized");
    }
}

/// Add a callback to the queue. Returns a non-null opaque ID.
#[no_mangle]
pub extern "C" fn rust_callback_add(
    func: extern "C" fn(*mut c_void),
    arg: *mut c_void,
) -> *mut c_void {
    let queue = match CALLBACK_QUEUE.get() {
        Some(q) => q,
        None => return std::ptr::null_mut(),
    };
    queue.lock().push_back(CallbackEntry { func, arg });
    // Return a non-null sentinel as the "ID" (C uses pointer to link struct)
    1 as *mut c_void
}

/// Process all queued callbacks in FIFO order.
#[no_mangle]
pub extern "C" fn rust_callback_process_all() -> c_int {
    let queue = match CALLBACK_QUEUE.get() {
        Some(q) => q,
        None => return 0,
    };

    let entries: Vec<_> = queue.lock().drain(..).collect();
    let count = entries.len() as c_int;
    for entry in entries {
        (entry.func)(entry.arg);
    }
    count
}

// ===========================================================================
// #6 InitColorMaps — delegate to existing Rust colormap system
// ===========================================================================

extern "C" {
    fn rust_cmap_init() -> c_int;
}

/// Initialize the colormap system via the existing Rust implementation.
///
/// Equivalent to C `InitColorMaps()` but delegates to `rust_cmap_init()`
/// from `graphics::cmap_ffi`.
#[no_mangle]
pub extern "C" fn rust_init_color_maps() -> c_int {
    let result = unsafe { rust_cmap_init() };
    if result == 0 {
        tracing::debug!("ColorMap system initialized (Rust)");
    } else {
        tracing::error!("ColorMap system init failed (code {})", result);
    }
    result
}

// ===========================================================================
// #5 TFB_InitGraphics — delegate to C (Rust driver is configured by C)
// ===========================================================================

extern "C" {
    fn TFB_InitGraphics(
        driver: c_int,
        flags: c_int,
        backend: *const libc::c_char,
        width: c_int,
        height: c_int,
    ) -> c_int;
}

/// Initialize the graphics subsystem.
///
/// Currently delegates to C `TFB_InitGraphics()` which configures the SDL
/// driver. The Rust graphics driver is selected via build config.
#[no_mangle]
pub unsafe extern "C" fn rust_tfb_init_graphics(
    driver: c_int,
    flags: c_int,
    backend: *const libc::c_char,
    width: c_int,
    height: c_int,
) -> c_int {
    TFB_InitGraphics(driver, flags, backend, width, height)
}

// ===========================================================================
// #7 init_communication — delegate to C
// ===========================================================================

extern "C" {
    fn init_communication();
}

/// Initialize the communication system.
///
/// Delegates to C `init_communication()` which sets up comm globals.
/// The Rust comm module (`comm::`) has its own FFI layer that runs alongside.
#[no_mangle]
pub extern "C" fn rust_init_communication() {
    unsafe { init_communication() };
    tracing::debug!("Communication system initialized");
}

// ===========================================================================
// #8 TFB_InitInput — delegate to C
// ===========================================================================

extern "C" {
    fn TFB_InitInput(driver: c_int, flags: c_int);
}

/// Initialize the input subsystem.
///
/// Delegates to C `TFB_InitInput()` which configures the SDL input driver.
#[no_mangle]
pub extern "C" fn rust_tfb_init_input(driver: c_int, flags: c_int) {
    unsafe { TFB_InitInput(driver, flags) };
    tracing::debug!("Input system initialized (driver={}, flags={})", driver, flags);
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    static CALLBACK_COUNT: AtomicU32 = AtomicU32::new(0);

    extern "C" fn test_callback(_arg: *mut c_void) {
        CALLBACK_COUNT.fetch_add(1, Ordering::SeqCst);
    }

    #[test]
    fn test_callback_queue() {
        CALLBACK_QUEUE.get_or_init(|| Mutex::new(std::collections::VecDeque::new()));
        let queue = CALLBACK_QUEUE.get().unwrap();
        queue.lock().clear();

        queue.lock().push_back(CallbackEntry {
            func: test_callback,
            arg: std::ptr::null_mut(),
        });
        queue.lock().push_back(CallbackEntry {
            func: test_callback,
            arg: std::ptr::null_mut(),
        });

        let processed = rust_callback_process_all();
        assert_eq!(processed, 2);
        assert_eq!(CALLBACK_COUNT.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn test_callback_empty_queue() {
        CALLBACK_QUEUE.get_or_init(|| Mutex::new(std::collections::VecDeque::new()));
        let queue = CALLBACK_QUEUE.get().unwrap();
        queue.lock().clear();

        let processed = rust_callback_process_all();
        assert_eq!(processed, 0);
    }

    #[test]
    fn test_alarm_ordering() {
        // Test that AlarmEntry min-heap ordering works
        let a1 = AlarmEntry {
            deadline_ms: 100,
            callback: test_callback,
            arg: std::ptr::null_mut(),
        };
        let a2 = AlarmEntry {
            deadline_ms: 50,
            callback: test_callback,
            arg: std::ptr::null_mut(),
        };
        let a3 = AlarmEntry {
            deadline_ms: 200,
            callback: test_callback,
            arg: std::ptr::null_mut(),
        };

        let mut heap = BinaryHeap::new();
        heap.push(a1);
        heap.push(a2);
        heap.push(a3);

        // Should pop earliest deadline first
        assert_eq!(heap.pop().unwrap().deadline_ms, 50);
        assert_eq!(heap.pop().unwrap().deadline_ms, 100);
        assert_eq!(heap.pop().unwrap().deadline_ms, 200);
    }
}
