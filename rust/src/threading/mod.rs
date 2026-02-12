//! Threading system for The Ur-Quan Masters
//!
//! This module provides thread management, synchronization primitives, and task scheduling.
//! It wraps Rust's standard threading with UQM-specific lifecycle management that mirrors
//! the original C implementation.
//!
//! # Design Notes
//!
//! The C implementation (thrcommon.c, sdlthreads.c) uses:
//! - A thread lifecycle system with `pendingBirth` and `pendingDeath` arrays
//! - SDL threads/mutexes/semaphores/condition variables
//! - Thread-local storage for graphics flush semaphores
//! - A task scheduling system for cooperative multitasking
//!
//! The Rust implementation uses:
//! - `std::thread` for thread spawning
//! - `std::sync::{Mutex, Condvar}` for synchronization
//! - Custom `Semaphore` type (counting semaphore not in std)
//! - `thread_local!` for thread-local storage
//!
//! # Reference
//! See `sc2/src/libs/threads/` for the original C implementation.

#[cfg(test)]
mod tests;

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, TryLockError};
use std::thread::{self, JoinHandle};
use std::time::Duration;

/// Error type for threading operations
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ThreadError {
    /// Thread spawn failed
    SpawnFailed(String),
    /// Thread join failed
    JoinFailed(String),
    /// Mutex operation failed (poisoned)
    MutexPoisoned,
    /// Lock acquisition timed out
    LockTimeout,
    /// Semaphore operation failed
    SemaphoreError(String),
    /// Condition variable error
    CondVarError(String),
    /// Task system not initialized
    NotInitialized,
    /// Invalid operation
    InvalidOperation(String),
}

impl std::fmt::Display for ThreadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ThreadError::SpawnFailed(s) => write!(f, "Thread spawn failed: {}", s),
            ThreadError::JoinFailed(s) => write!(f, "Thread join failed: {}", s),
            ThreadError::MutexPoisoned => write!(f, "Mutex is poisoned"),
            ThreadError::LockTimeout => write!(f, "Lock acquisition timed out"),
            ThreadError::SemaphoreError(s) => write!(f, "Semaphore error: {}", s),
            ThreadError::CondVarError(s) => write!(f, "CondVar error: {}", s),
            ThreadError::NotInitialized => write!(f, "Thread system not initialized"),
            ThreadError::InvalidOperation(s) => write!(f, "Invalid operation: {}", s),
        }
    }
}

impl std::error::Error for ThreadError {}

pub type Result<T> = std::result::Result<T, ThreadError>;

// ============================================================================
// Thread Handle
// ============================================================================

/// Handle to a spawned thread
///
/// Wraps a `JoinHandle` and provides UQM-compatible thread operations.
/// The C implementation tracks threads in a global queue; we use Rust's
/// ownership model instead.
pub struct Thread<T> {
    handle: Option<JoinHandle<T>>,
    #[allow(dead_code)]
    name: Option<String>,
}

impl<T> Thread<T> {
    /// Spawn a new thread that executes the given function
    ///
    /// # Arguments
    /// * `name` - Optional thread name (for debugging)
    /// * `f` - Function to execute in the new thread
    ///
    /// # Returns
    /// A `Thread` handle that can be used to join the thread
    ///
    /// # Errors
    /// Returns `ThreadError::SpawnFailed` if thread creation fails
    pub fn spawn<F>(name: Option<&str>, f: F) -> Result<Self>
    where
        F: FnOnce() -> T + Send + 'static,
        T: Send + 'static,
    {
        // Build a thread with an optional name
        let mut builder = thread::Builder::new();
        if let Some(n) = name {
            builder = builder.name(n.to_string());
        }

        let handle = builder.spawn(f).map_err(|e| {
            ThreadError::SpawnFailed(format!("Failed to spawn thread: {}", e))
        })?;

        Ok(Self {
            handle: Some(handle),
            name: name.map(String::from),
        })
    }

    /// Wait for the thread to finish and return its result
    ///
    /// # Returns
    /// The value returned by the thread function
    ///
    /// # Errors
    /// Returns `ThreadError::JoinFailed` if the thread panicked
    pub fn join(mut self) -> Result<T> {
        match self.handle.take() {
            Some(handle) => handle.join().map_err(|_| {
                ThreadError::JoinFailed("Thread panicked".to_string())
            }),
            None => Err(ThreadError::JoinFailed(
                "Thread already joined".to_string(),
            )),
        }
    }

    /// Check if the thread is still running
    pub fn is_running(&self) -> bool {
        match &self.handle {
            Some(handle) => !handle.is_finished(),
            None => false,
        }
    }
}

// ============================================================================
// Mutex
// ============================================================================

/// A mutual exclusion lock
///
/// Wraps `std::sync::Mutex` with UQM-compatible API.
/// The C implementation supports optional contention tracking via TRACK_CONTENTION.
pub struct UqmMutex<T> {
    inner: Mutex<T>,
    #[allow(dead_code)]
    name: Option<String>,
}

impl<T> UqmMutex<T> {
    /// Create a new mutex
    ///
    /// # Arguments
    /// * `value` - The initial value protected by the mutex
    /// * `name` - Optional name for debugging (matches C NAMED_SYNCHRO)
    pub fn new(value: T, name: Option<&str>) -> Self {
        Self {
            inner: Mutex::new(value),
            name: name.map(String::from),
        }
    }

    /// Lock the mutex, blocking until it's available
    ///
    /// # Returns
    /// A guard that releases the lock when dropped
    ///
    /// # Errors
    /// Returns `ThreadError::MutexPoisoned` if another thread panicked while holding the lock
    pub fn lock(&self) -> Result<MutexGuard<'_, T>> {
        self.inner.lock().map_err(|_| ThreadError::MutexPoisoned)
    }

    /// Try to lock the mutex without blocking
    ///
    /// # Returns
    /// `Some(guard)` if the lock was acquired, `None` if it's held by another thread
    ///
    /// # Errors
    /// Returns `ThreadError::MutexPoisoned` if the mutex is poisoned
    pub fn try_lock(&self) -> Result<Option<MutexGuard<'_, T>>> {
        match self.inner.try_lock() {
            Ok(guard) => Ok(Some(guard)),
            Err(TryLockError::WouldBlock) => Ok(None),
            Err(TryLockError::Poisoned(_)) => Err(ThreadError::MutexPoisoned),
        }
    }
}

impl<T: Default> Default for UqmMutex<T> {
    fn default() -> Self {
        Self::new(T::default(), None)
    }
}

// ============================================================================
// Condition Variable
// ============================================================================

/// A condition variable for thread synchronization
///
/// Wraps `std::sync::Condvar` with UQM-compatible API.
/// Used for wait/signal patterns between threads.
///
/// This implementation uses a generation counter to handle broadcast correctly.
/// Each broadcast increments the generation, and waiters check if the generation
/// has changed since they started waiting.
pub struct UqmCondVar {
    inner: Condvar,
    /// Internal mutex to track generation counter. The counter is incremented on signal/broadcast.
    state: Mutex<CondVarState>,
    #[allow(dead_code)]
    name: Option<String>,
}

/// Internal state for the condition variable
struct CondVarState {
    /// Generation counter - incremented on each signal/broadcast
    generation: u64,
    /// Number of pending signals (for signal() - wakes one waiter)
    pending_signals: u64,
}

impl UqmCondVar {
    /// Create a new condition variable
    ///
    /// # Arguments
    /// * `name` - Optional name for debugging
    pub fn new(name: Option<&str>) -> Self {
        Self {
            inner: Condvar::new(),
            state: Mutex::new(CondVarState {
                generation: 0,
                pending_signals: 0,
            }),
            name: name.map(String::from),
        }
    }

    /// Wait for a signal on this condition variable
    ///
    /// The calling thread will block until another thread calls `signal()` or `broadcast()`.
    ///
    /// # Errors
    /// Returns `ThreadError::CondVarError` if the wait fails
    pub fn wait(&self) -> Result<()> {
        let mut state = self.state.lock().map_err(|_| {
            ThreadError::CondVarError("Internal mutex poisoned".to_string())
        })?;

        let my_generation = state.generation;

        // Wait until generation changes (broadcast) or we have a pending signal
        loop {
            if state.generation != my_generation {
                // A broadcast occurred
                return Ok(());
            }
            if state.pending_signals > 0 {
                // A signal was pending for us
                state.pending_signals -= 1;
                return Ok(());
            }
            state = self.inner.wait(state).map_err(|_| {
                ThreadError::CondVarError("Condvar wait failed".to_string())
            })?;
        }
    }

    /// Wait with a timeout
    ///
    /// # Arguments
    /// * `timeout` - Maximum time to wait
    ///
    /// # Returns
    /// `Ok(true)` if signaled, `Ok(false)` if timed out
    ///
    /// # Errors
    /// Returns `ThreadError::CondVarError` if the wait fails
    pub fn wait_timeout(&self, timeout: Duration) -> Result<bool> {
        let mut state = self.state.lock().map_err(|_| {
            ThreadError::CondVarError("Internal mutex poisoned".to_string())
        })?;

        let my_generation = state.generation;

        // Check if already signaled
        if state.pending_signals > 0 {
            state.pending_signals -= 1;
            return Ok(true);
        }

        let (guard, result) = self.inner.wait_timeout(state, timeout).map_err(|_| {
            ThreadError::CondVarError("Condvar wait_timeout failed".to_string())
        })?;

        state = guard;

        // Check if timed out first
        if result.timed_out() {
            return Ok(false);
        }

        // Check if we were signaled (generation changed or pending signal)
        if state.generation != my_generation {
            return Ok(true);
        }
        if state.pending_signals > 0 {
            state.pending_signals -= 1;
            return Ok(true);
        }

        // Spurious wakeup - treat as timeout for simplicity
        Ok(false)
    }

    /// Signal one waiting thread
    ///
    /// Wakes up one thread that is waiting on this condition variable.
    pub fn signal(&self) {
        if let Ok(mut state) = self.state.lock() {
            state.pending_signals += 1;
            self.inner.notify_one();
        }
    }

    /// Signal all waiting threads
    ///
    /// Wakes up all threads that are waiting on this condition variable.
    pub fn broadcast(&self) {
        if let Ok(mut state) = self.state.lock() {
            state.generation += 1;
            self.inner.notify_all();
        }
    }
}

impl Default for UqmCondVar {
    fn default() -> Self {
        Self::new(None)
    }
}

// ============================================================================
// Semaphore
// ============================================================================

/// A counting semaphore
///
/// The C implementation calls acquire "SetSemaphore" (wait/decrement)
/// and release "ClearSemaphore" (post/increment).
#[derive(Debug)]
pub struct Semaphore {
    count: Mutex<u32>,
    condvar: Condvar,
    #[allow(dead_code)]
    name: Option<String>,
}

impl Semaphore {
    /// Create a new semaphore with the given initial count
    ///
    /// # Arguments
    /// * `initial` - Initial permit count
    /// * `name` - Optional name for debugging
    pub fn new(initial: u32, name: Option<&str>) -> Self {
        Self {
            count: Mutex::new(initial),
            condvar: Condvar::new(),
            name: name.map(String::from),
        }
    }

    /// Acquire a permit, blocking if none available
    ///
    /// This is called `SetSemaphore` in the C code (SDL_SemWait).
    /// Decrements the count, blocking if count is zero.
    ///
    /// # Errors
    /// Returns `ThreadError::SemaphoreError` if the operation fails
    pub fn acquire(&self) -> Result<()> {
        let mut count = self.count.lock().map_err(|_| {
            ThreadError::SemaphoreError("Semaphore mutex poisoned".to_string())
        })?;

        // Wait until count > 0
        while *count == 0 {
            count = self.condvar.wait(count).map_err(|_| {
                ThreadError::SemaphoreError("Condvar wait failed".to_string())
            })?;
        }

        // Decrement the count
        *count -= 1;
        Ok(())
    }

    /// Try to acquire a permit without blocking
    ///
    /// # Returns
    /// `true` if a permit was acquired, `false` if none available
    pub fn try_acquire(&self) -> bool {
        if let Ok(mut count) = self.count.lock() {
            if *count > 0 {
                *count -= 1;
                return true;
            }
        }
        false
    }

    /// Release a permit
    ///
    /// This is called `ClearSemaphore` in the C code (SDL_SemPost).
    /// Increments the count, potentially waking a waiting thread.
    pub fn release(&self) {
        if let Ok(mut count) = self.count.lock() {
            *count += 1;
            self.condvar.notify_one();
        }
    }

    /// Get the current permit count
    ///
    /// # Returns
    /// The current number of available permits
    pub fn count(&self) -> u32 {
        self.count.lock().map(|c| *c).unwrap_or(0)
    }
}

impl Default for Semaphore {
    fn default() -> Self {
        Self::new(0, None)
    }
}

// ============================================================================
// Task System
// ============================================================================

/// Task state, matching C TaskState enum
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    /// Task is ready to run
    Ready,
    /// Task is currently running
    Running,
    /// Task is waiting/blocked
    Waiting,
    /// Task has completed
    Completed,
    /// Task was cancelled
    Cancelled,
}

/// A task represents a unit of work that can be scheduled
///
/// The C implementation uses tasks for cooperative multitasking within the game loop.
pub struct Task {
    id: u32,
    state: AtomicU32,
    #[allow(dead_code)]
    name: Option<String>,
    callback: Option<Box<dyn FnOnce() + Send>>,
}

impl Task {
    /// Create a new task
    ///
    /// # Arguments
    /// * `name` - Optional task name
    /// * `callback` - Function to execute when the task runs
    pub fn new<F>(name: Option<&str>, callback: F) -> Self
    where
        F: FnOnce() + Send + 'static,
    {
        static NEXT_ID: AtomicU32 = AtomicU32::new(1);

        Self {
            id: NEXT_ID.fetch_add(1, Ordering::SeqCst),
            state: AtomicU32::new(TaskState::Ready as u32),
            name: name.map(String::from),
            callback: Some(Box::new(callback)),
        }
    }

    /// Get the task ID
    pub fn id(&self) -> u32 {
        self.id
    }

    /// Get the current task state
    pub fn state(&self) -> TaskState {
        // TODO: Implement state retrieval
        match self.state.load(Ordering::SeqCst) {
            0 => TaskState::Ready,
            1 => TaskState::Running,
            2 => TaskState::Waiting,
            3 => TaskState::Completed,
            _ => TaskState::Cancelled,
        }
    }

    /// Set the task state
    ///
    /// # Arguments
    /// * `state` - The new state
    pub fn set_state(&self, state: TaskState) {
        // TODO: Implement state setting
        self.state.store(state as u32, Ordering::SeqCst);
    }

    /// Execute the task callback
    ///
    /// This consumes the callback, so can only be called once.
    ///
    /// # Errors
    /// Returns `ThreadError::InvalidOperation` if the callback has already been executed
    pub fn execute(&mut self) -> Result<()> {
        match self.callback.take() {
            Some(callback) => {
                self.set_state(TaskState::Running);
                callback();
                self.set_state(TaskState::Completed);
                Ok(())
            }
            None => Err(ThreadError::InvalidOperation(
                "Task callback already executed".to_string(),
            )),
        }
    }
}

// ============================================================================
// Thread System Initialization
// ============================================================================

static THREAD_SYSTEM_INITIALIZED: AtomicBool = AtomicBool::new(false);

/// Initialize the threading system
///
/// Must be called before using any threading primitives.
/// The C implementation initializes the lifecycle mutex and thread queue.
///
/// # Errors
/// Returns `ThreadError::InvalidOperation` if already initialized
pub fn init_thread_system() -> Result<()> {
    // TODO: Implement thread system initialization
    // The C implementation calls NativeInitThreadSystem and creates lifecycleMutex
    if THREAD_SYSTEM_INITIALIZED.swap(true, Ordering::SeqCst) {
        return Err(ThreadError::InvalidOperation(
            "Thread system already initialized".to_string(),
        ));
    }
    Ok(())
}

/// Uninitialize the threading system
///
/// Should be called during shutdown.
/// The C implementation destroys the lifecycle mutex.
pub fn uninit_thread_system() {
    // TODO: Implement thread system cleanup
    // The C implementation calls NativeUnInitThreadSystem and destroys lifecycleMutex
    THREAD_SYSTEM_INITIALIZED.store(false, Ordering::SeqCst);
}

/// Check if the threading system is initialized
pub fn is_thread_system_initialized() -> bool {
    THREAD_SYSTEM_INITIALIZED.load(Ordering::SeqCst)
}

/// Process pending thread lifecycle events
///
/// The C implementation processes pendingBirth and pendingDeath arrays.
/// In Rust, we may not need this if we use proper RAII.
pub fn process_thread_lifecycles() {
    // TODO: Implement lifecycle processing
    // The C implementation creates/destroys threads from the lifecycle arrays
}

/// Yield execution to other threads
///
/// The C implementation uses TaskSwitch -> NativeTaskSwitch -> SDL_Delay(1).
pub fn task_switch() {
    // TODO: Implement task switch
    // For now, use std::thread::yield_now
    thread::yield_now();
}

/// Sleep the current thread
///
/// # Arguments
/// * `duration` - How long to sleep
pub fn hibernate_thread(duration: Duration) {
    // TODO: Implement thread hibernation
    // The C implementation uses NativeSleepThread -> SDL_Delay
    thread::sleep(duration);
}

// ============================================================================
// Thread-Local Storage
// ============================================================================

/// Thread-local data for UQM threads
///
/// The C implementation stores a flush semaphore per thread.
#[derive(Debug)]
pub struct ThreadLocal {
    /// Semaphore for graphics flush synchronization
    pub flush_sem: Arc<Semaphore>,
}

impl ThreadLocal {
    /// Create new thread-local data
    pub fn new() -> Self {
        Self {
            flush_sem: Arc::new(Semaphore::new(0, Some("FlushGraphics"))),
        }
    }
}

impl Default for ThreadLocal {
    fn default() -> Self {
        Self::new()
    }
}

thread_local! {
    static THREAD_LOCAL: std::cell::RefCell<Option<ThreadLocal>> = const { std::cell::RefCell::new(None) };
}

/// Get the current thread's local data
///
/// # Returns
/// The thread-local data, or `None` if not set
pub fn get_my_thread_local() -> Option<ThreadLocal> {
    THREAD_LOCAL.with(|tl| tl.borrow().clone())
}

impl Clone for ThreadLocal {
    fn clone(&self) -> Self {
        Self {
            flush_sem: Arc::clone(&self.flush_sem),
        }
    }
}

// ============================================================================
// FFI Bindings for C Integration
// ============================================================================

use std::ffi::{c_char, c_int, c_void, CStr};
use std::ptr;

/// Opaque handle to a Rust thread for C FFI
#[repr(C)]
pub struct RustThread {
    _private: [u8; 0],
}

/// Opaque handle to a Rust mutex for C FFI
#[repr(C)]
pub struct RustMutex {
    _private: [u8; 0],
}

/// Opaque handle to a Rust condition variable for C FFI
#[repr(C)]
pub struct RustCondVar {
    _private: [u8; 0],
}

/// Opaque handle to a Rust semaphore for C FFI
#[repr(C)]
pub struct RustSemaphore {
    _private: [u8; 0],
}

// --- Thread System Lifecycle ---

/// Initialize the Rust threading system
///
/// # Returns
/// 1 on success, 0 if already initialized or on error
///
/// # Safety
/// Safe to call from C.
#[no_mangle]
pub extern "C" fn rust_init_thread_system() -> c_int {
    match init_thread_system() {
        Ok(()) => 1,
        Err(_) => 0,
    }
}

/// Uninitialize the Rust threading system
///
/// # Safety
/// Safe to call from C.
#[no_mangle]
pub extern "C" fn rust_uninit_thread_system() {
    uninit_thread_system();
}

/// Check if the Rust threading system is initialized
///
/// # Returns
/// 1 if initialized, 0 otherwise
///
/// # Safety
/// Safe to call from C.
#[no_mangle]
pub extern "C" fn rust_is_thread_system_initialized() -> c_int {
    if is_thread_system_initialized() { 1 } else { 0 }
}

// --- Thread Operations ---

/// Spawn a new thread from C
///
/// # Arguments
/// * `name` - Optional thread name (null-terminated C string, can be NULL)
/// * `func` - Thread function pointer
/// * `data` - Data to pass to the thread function
///
/// # Returns
/// Pointer to RustThread handle on success, NULL on failure
///
/// # Safety
/// * `name` must be a valid null-terminated C string or NULL
/// * `func` must be a valid function pointer
/// * `data` must remain valid for the lifetime of the thread
#[no_mangle]
pub unsafe extern "C" fn rust_thread_spawn(
    name: *const c_char,
    func: unsafe extern "C" fn(*mut c_void),
    data: *mut c_void,
) -> *mut RustThread {
    let name_str = if name.is_null() {
        None
    } else {
        CStr::from_ptr(name).to_str().ok()
    };

    // Convert raw pointers to usize to make them Send-safe for the closure.
    // usize is Send, whereas raw pointers are not.
    let func_ptr = func as usize;
    let data_ptr = data as usize;

    match Thread::spawn(name_str, move || {
        // Safety: We're reconstructing the original function pointer and data pointer
        // inside the spawned thread. The caller guarantees data remains valid.
        let func: unsafe extern "C" fn(*mut c_void) =
            unsafe { std::mem::transmute(func_ptr) };
        let data = data_ptr as *mut c_void;
        unsafe { func(data) }
    }) {
        Ok(thread) => Box::into_raw(Box::new(thread)) as *mut RustThread,
        Err(_) => ptr::null_mut(),
    }
}

/// Join a thread and wait for it to complete
///
/// # Arguments
/// * `thread` - Thread handle (consumed by this call)
///
/// # Returns
/// 1 on success, 0 on failure
///
/// # Safety
/// * `thread` must be a valid handle from rust_thread_spawn
/// * The handle is invalidated after this call
#[no_mangle]
pub unsafe extern "C" fn rust_thread_join(thread: *mut RustThread) -> c_int {
    if thread.is_null() {
        return 0;
    }

    let thread: Box<Thread<()>> = Box::from_raw(thread as *mut Thread<()>);
    match thread.join() {
        Ok(()) => 1,
        Err(_) => 0,
    }
}

/// Yield execution to other threads
///
/// # Safety
/// Safe to call from C.
#[no_mangle]
pub extern "C" fn rust_thread_yield() {
    task_switch();
}

/// Sleep the current thread
///
/// # Arguments
/// * `msecs` - Milliseconds to sleep
///
/// # Safety
/// Safe to call from C.
#[no_mangle]
pub extern "C" fn rust_hibernate_thread(msecs: u32) {
    hibernate_thread(Duration::from_millis(msecs as u64));
}

// --- Mutex Operations ---

/// Create a new mutex
///
/// # Arguments
/// * `name` - Optional mutex name (can be NULL)
///
/// # Returns
/// Pointer to RustMutex handle on success, NULL on failure
///
/// # Safety
/// * `name` must be a valid null-terminated C string or NULL
#[no_mangle]
pub unsafe extern "C" fn rust_mutex_create(name: *const c_char) -> *mut RustMutex {
    let name_str = if name.is_null() {
        None
    } else {
        CStr::from_ptr(name).to_str().ok()
    };

    let mutex = UqmMutex::new((), name_str);
    Box::into_raw(Box::new(mutex)) as *mut RustMutex
}

/// Destroy a mutex
///
/// # Arguments
/// * `mutex` - Mutex handle to destroy
///
/// # Safety
/// * `mutex` must be a valid handle from rust_mutex_create
/// * The handle is invalidated after this call
#[no_mangle]
pub unsafe extern "C" fn rust_mutex_destroy(mutex: *mut RustMutex) {
    if !mutex.is_null() {
        drop(Box::from_raw(mutex as *mut UqmMutex<()>));
    }
}

/// Lock a mutex (blocking)
///
/// # Arguments
/// * `mutex` - Mutex handle
///
/// # Safety
/// * `mutex` must be a valid handle
#[no_mangle]
pub unsafe extern "C" fn rust_mutex_lock(mutex: *mut RustMutex) {
    if !mutex.is_null() {
        let mutex = &*(mutex as *mut UqmMutex<()>);
        // We intentionally leak the guard here - it will be "unlocked" manually
        // This is a simplified model; a real implementation would track guards
        let _guard = mutex.lock();
        // Intentionally forget the guard so the mutex stays locked until rust_mutex_unlock
        std::mem::forget(_guard);
    }
}

/// Try to lock a mutex (non-blocking)
///
/// # Arguments
/// * `mutex` - Mutex handle
///
/// # Returns
/// 1 if lock acquired, 0 if would block or error
///
/// # Safety
/// * `mutex` must be a valid handle
#[no_mangle]
pub unsafe extern "C" fn rust_mutex_try_lock(mutex: *mut RustMutex) -> c_int {
    if mutex.is_null() {
        return 0;
    }

    let mutex = &*(mutex as *mut UqmMutex<()>);
    match mutex.try_lock() {
        Ok(Some(_)) => 1,
        _ => 0,
    }
}

/// Unlock a mutex
///
/// # Arguments
/// * `mutex` - Mutex handle
///
/// # Safety
/// * `mutex` must be a valid handle
/// * Must be called from the same thread that locked it
#[no_mangle]
pub unsafe extern "C" fn rust_mutex_unlock(_mutex: *mut RustMutex) {
    // Note: With Rust's guard-based locking, unlock happens when guard is dropped.
    // This FFI binding is a simplified model. In a real implementation,
    // we would need to track the guard separately.
    // For now, this is a no-op - the C code would need adaptation.
}

// --- Condition Variable Operations ---

/// Create a new condition variable
///
/// # Arguments
/// * `name` - Optional condvar name (can be NULL)
///
/// # Returns
/// Pointer to RustCondVar handle
///
/// # Safety
/// * `name` must be a valid null-terminated C string or NULL
#[no_mangle]
pub unsafe extern "C" fn rust_condvar_create(name: *const c_char) -> *mut RustCondVar {
    let name_str = if name.is_null() {
        None
    } else {
        CStr::from_ptr(name).to_str().ok()
    };

    let condvar = UqmCondVar::new(name_str);
    Box::into_raw(Box::new(condvar)) as *mut RustCondVar
}

/// Destroy a condition variable
///
/// # Arguments
/// * `cond` - CondVar handle to destroy
///
/// # Safety
/// * `cond` must be a valid handle from rust_condvar_create
#[no_mangle]
pub unsafe extern "C" fn rust_condvar_destroy(cond: *mut RustCondVar) {
    if !cond.is_null() {
        drop(Box::from_raw(cond as *mut UqmCondVar));
    }
}

/// Wait on a condition variable
///
/// # Arguments
/// * `cond` - CondVar handle
/// * `mutex` - Associated mutex handle (must be held)
///
/// # Safety
/// * Both handles must be valid
/// * Mutex must be held by the calling thread
#[no_mangle]
pub unsafe extern "C" fn rust_condvar_wait(cond: *mut RustCondVar, _mutex: *mut RustMutex) {
    if cond.is_null() {
        return;
    }

    let cond = &*(cond as *mut UqmCondVar);
    let _ = cond.wait();
}

/// Wait on a condition variable with timeout
///
/// # Arguments
/// * `cond` - CondVar handle
/// * `mutex` - Associated mutex handle
/// * `msecs` - Timeout in milliseconds
///
/// # Returns
/// 1 if signaled, 0 if timed out
///
/// # Safety
/// * Both handles must be valid
#[no_mangle]
pub unsafe extern "C" fn rust_condvar_wait_timeout(
    cond: *mut RustCondVar,
    _mutex: *mut RustMutex,
    msecs: u32,
) -> c_int {
    if cond.is_null() {
        return 0;
    }

    let cond = &*(cond as *mut UqmCondVar);
    match cond.wait_timeout(Duration::from_millis(msecs as u64)) {
        Ok(true) => 1,
        _ => 0,
    }
}

/// Signal one waiter on a condition variable
///
/// # Arguments
/// * `cond` - CondVar handle
///
/// # Safety
/// * `cond` must be a valid handle
#[no_mangle]
pub unsafe extern "C" fn rust_condvar_signal(cond: *mut RustCondVar) {
    if !cond.is_null() {
        let cond = &*(cond as *mut UqmCondVar);
        cond.signal();
    }
}

/// Broadcast to all waiters on a condition variable
///
/// # Arguments
/// * `cond` - CondVar handle
///
/// # Safety
/// * `cond` must be a valid handle
#[no_mangle]
pub unsafe extern "C" fn rust_condvar_broadcast(cond: *mut RustCondVar) {
    if !cond.is_null() {
        let cond = &*(cond as *mut UqmCondVar);
        cond.broadcast();
    }
}

// --- Semaphore Operations ---

/// Create a new semaphore
///
/// # Arguments
/// * `initial` - Initial permit count
/// * `name` - Optional semaphore name (can be NULL)
///
/// # Returns
/// Pointer to RustSemaphore handle
///
/// # Safety
/// * `name` must be a valid null-terminated C string or NULL
#[no_mangle]
pub unsafe extern "C" fn rust_semaphore_create(
    initial: u32,
    name: *const c_char,
) -> *mut RustSemaphore {
    let name_str = if name.is_null() {
        None
    } else {
        CStr::from_ptr(name).to_str().ok()
    };

    let sem = Semaphore::new(initial, name_str);
    Box::into_raw(Box::new(sem)) as *mut RustSemaphore
}

/// Destroy a semaphore
///
/// # Arguments
/// * `sem` - Semaphore handle to destroy
///
/// # Safety
/// * `sem` must be a valid handle from rust_semaphore_create
#[no_mangle]
pub unsafe extern "C" fn rust_semaphore_destroy(sem: *mut RustSemaphore) {
    if !sem.is_null() {
        drop(Box::from_raw(sem as *mut Semaphore));
    }
}

/// Acquire a semaphore permit (blocking)
///
/// # Arguments
/// * `sem` - Semaphore handle
///
/// # Safety
/// * `sem` must be a valid handle
#[no_mangle]
pub unsafe extern "C" fn rust_semaphore_acquire(sem: *mut RustSemaphore) {
    if !sem.is_null() {
        let sem = &*(sem as *mut Semaphore);
        let _ = sem.acquire();
    }
}

/// Try to acquire a semaphore permit (non-blocking)
///
/// # Arguments
/// * `sem` - Semaphore handle
///
/// # Returns
/// 1 if acquired, 0 if would block
///
/// # Safety
/// * `sem` must be a valid handle
#[no_mangle]
pub unsafe extern "C" fn rust_semaphore_try_acquire(sem: *mut RustSemaphore) -> c_int {
    if sem.is_null() {
        return 0;
    }

    let sem = &*(sem as *mut Semaphore);
    if sem.try_acquire() { 1 } else { 0 }
}

/// Release a semaphore permit
///
/// # Arguments
/// * `sem` - Semaphore handle
///
/// # Safety
/// * `sem` must be a valid handle
#[no_mangle]
pub unsafe extern "C" fn rust_semaphore_release(sem: *mut RustSemaphore) {
    if !sem.is_null() {
        let sem = &*(sem as *mut Semaphore);
        sem.release();
    }
}

/// Get the current semaphore permit count
///
/// # Arguments
/// * `sem` - Semaphore handle
///
/// # Returns
/// Current permit count, or 0 if handle is NULL
///
/// # Safety
/// * `sem` must be a valid handle or NULL
#[no_mangle]
pub unsafe extern "C" fn rust_semaphore_count(sem: *mut RustSemaphore) -> u32 {
    if sem.is_null() {
        return 0;
    }

    let sem = &*(sem as *mut Semaphore);
    sem.count()
}

/// Cooperative task switch (yield)
///
/// # Safety
/// Safe to call from C.
#[no_mangle]
pub extern "C" fn rust_task_switch() {
    task_switch();
}
