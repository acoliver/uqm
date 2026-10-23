//! Thread handles and legacy entry points, ported from rust_thrcommon.c.
#![allow(non_snake_case)]

use super::*;
use std::{
    ffi::{c_char, c_int, c_void},
    ptr,
};

pub struct LegacyThread {
    native: *mut RustThread,
}
struct StartInfo {
    func: unsafe extern "C" fn(*mut c_void) -> c_int,
    data: *mut c_void,
    thread: *mut LegacyThread,
}

static PENDING_DEATH: Mutex<[usize; 8]> = Mutex::new([0; 8]);
static LIFECYCLE_ACTIVE: AtomicBool = AtomicBool::new(false);
static MAIN_THREAD: Mutex<Option<ThreadId>> = Mutex::new(None);

unsafe extern "C" fn thread_helper(opaque: *mut c_void) -> c_int {
    let start = *Box::from_raw(opaque.cast::<StartInfo>());
    let result = (start.func)(start.data);
    FinishThread(start.thread);
    result
}

/// # Safety
/// Initialize once on the game thread before spawning workers.
#[no_mangle]
pub unsafe extern "C" fn InitThreadSystem() {
    rust_init_thread_system();
    *PENDING_DEATH.lock().unwrap() = [0; 8];
    *MAIN_THREAD.lock().unwrap() = Some(thread::current().id());
    LIFECYCLE_ACTIVE.store(true, Ordering::Release);
    log::debug!("Rust thread system initialized");
}

/// # Safety
/// All workers must have stopped before final subsystem teardown.
#[no_mangle]
pub unsafe extern "C" fn UnInitThreadSystem() {
    ProcessThreadLifecycles();
    LIFECYCLE_ACTIVE.store(false, Ordering::Release);
    rust_uninit_thread_system();
}

/// # Safety
/// The callback and data must remain valid until the thread exits.
#[no_mangle]
pub unsafe extern "C" fn CreateThread_Core(
    func: unsafe extern "C" fn(*mut c_void) -> c_int,
    data: *mut c_void,
    _stack_size: i32,
    name: *const c_char,
) -> *mut LegacyThread {
    let thread = Box::into_raw(Box::new(LegacyThread {
        native: ptr::null_mut(),
    }));
    let info = Box::into_raw(Box::new(StartInfo { func, data, thread }));
    (*thread).native = rust_thread_spawn(name, thread_helper, info.cast());
    if (*thread).native.is_null() {
        drop(Box::from_raw(info));
        drop(Box::from_raw(thread));
        ptr::null_mut()
    } else {
        thread
    }
}

/// # Safety
/// The callback and data must remain valid until the thread exits.
#[no_mangle]
pub unsafe extern "C" fn StartThread_Core(
    func: unsafe extern "C" fn(*mut c_void) -> c_int,
    data: *mut c_void,
    stack_size: i32,
    name: *const c_char,
) {
    CreateThread_Core(func, data, stack_size, name);
}

/// # Safety
/// Thread is null or a live handle, joined at most once; status is null or writable.
#[no_mangle]
pub unsafe extern "C" fn WaitThread(thread: *mut LegacyThread, status: *mut c_int) {
    if !status.is_null() {
        *status = 0;
    }
    if !thread.is_null() && !(*thread).native.is_null() {
        let mut result = 0;
        let joined = rust_thread_join((*thread).native, &mut result);
        if !status.is_null() && joined != 0 {
            *status = result;
        }
        (*thread).native = ptr::null_mut();
    }
}

/// # Safety
/// Called once by the thread whose handle is being retired.
#[no_mangle]
pub unsafe extern "C" fn FinishThread(thread: *mut LegacyThread) {
    if thread.is_null() || !LIFECYCLE_ACTIVE.load(Ordering::Acquire) {
        return;
    }
    let mut pending = PENDING_DEATH.lock().unwrap();
    if let Some(slot) = pending.iter_mut().find(|slot| **slot == 0) {
        *slot = thread as usize;
        return;
    }
    log::error!("Thread Lifecycle array filled. Make LIFECYCLE_SIZE larger than 8.");
    std::process::exit(1);
}

/// # Safety
/// Called by the lifecycle owner, not by a worker pending retirement.
#[no_mangle]
pub unsafe extern "C" fn ProcessThreadLifecycles() {
    if !LIFECYCLE_ACTIVE.load(Ordering::Acquire) {
        return;
    }
    let mut pending = PENDING_DEATH.lock().unwrap();
    for slot in pending.iter_mut() {
        if *slot != 0 {
            let thread = *slot as *mut LegacyThread;
            WaitThread(thread, ptr::null_mut());
            *slot = 0;
            DestroyThread(thread);
        }
    }
}

/// # Safety
/// Handle must be null or allocated here, no longer used by its worker, and not freed.
#[no_mangle]
pub unsafe extern "C" fn DestroyThread(thread: *mut LegacyThread) {
    if !thread.is_null() {
        drop(Box::from_raw(thread));
    }
}

unsafe fn pump(flush: bool) {
    if *MAIN_THREAD.lock().unwrap() == Some(thread::current().id()) {
        crate::graphics::sdl::lifecycle_abi::TFB_ProcessEvents();
        if flush {
            crate::graphics::dcqueue_abi::TFB_FlushGraphics();
        }
    }
}

/// # Safety
/// Thread system is initialized; SDL is accessed only by its owner.
#[no_mangle]
pub unsafe extern "C" fn TaskSwitch() {
    // A flush here would recurse through the empty DCQ's TaskSwitch.
    pump(false);
    rust_task_switch();
}

/// # Safety
/// Thread system is initialized; game thread owns pending graphics work.
#[no_mangle]
pub unsafe extern "C" fn HibernateThread(period: u32) {
    let ms = period.wrapping_mul(1000) / 840;
    pump(true);
    rust_hibernate_thread(ms);
}

/// # Safety
/// Thread system and monotonic game clock are initialized.
#[no_mangle]
pub unsafe extern "C" fn HibernateThreadUntil(wake: u32) {
    let now = crate::time::counter_abi::GetTimeCounter();
    if wake > now {
        HibernateThread(wake - now);
    }
}

macro_rules! forward {
    ($name:ident($($arg:ident: $ty:ty),*) $(-> $ret:ty)? = $call:expr) => {
        /// # Safety
        /// Pointers and handles obey the corresponding threading provider contract.
        #[no_mangle]
        pub unsafe extern "C" fn $name($($arg: $ty),*) $(-> $ret)? { $call }
    };
}

forward!(CreateMutex_Core(name: *const c_char, _class: u32) -> *mut RustMutex = rust_mutex_create(name));
forward!(DestroyMutex(m: *mut RustMutex) = rust_mutex_destroy(m));
forward!(LockMutex(m: *mut RustMutex) = rust_mutex_lock(m));
forward!(UnlockMutex(m: *mut RustMutex) = rust_mutex_unlock(m));
forward!(CreateRecursiveMutex_Core(name: *const c_char, _class: u32) -> *mut RustMutex = rust_mutex_create(name));
forward!(DestroyRecursiveMutex(m: *mut RustMutex) = rust_mutex_destroy(m));
forward!(LockRecursiveMutex(m: *mut RustMutex) = rust_mutex_lock(m));
forward!(UnlockRecursiveMutex(m: *mut RustMutex) = rust_mutex_unlock(m));
forward!(TryLockRecursiveMutex(m: *mut RustMutex) -> c_int = rust_mutex_try_lock(m));
forward!(GetRecursiveMutexDepth(m: *mut RustMutex) -> c_int = rust_mutex_depth(m));
forward!(CreateSemaphore_Core(initial: u32, name: *const c_char, _class: u32) -> *mut RustSemaphore = rust_semaphore_create(initial, name));
forward!(DestroySemaphore(s: *mut RustSemaphore) = rust_semaphore_destroy(s));
forward!(SetSemaphore(s: *mut RustSemaphore) = rust_semaphore_acquire(s));
forward!(ClearSemaphore(s: *mut RustSemaphore) = rust_semaphore_release(s));
forward!(CreateCondVar_Core(name: *const c_char, _class: u32) -> *mut RustCondVar = rust_condvar_create(name));
forward!(DestroyCondVar(c: *mut RustCondVar) = rust_condvar_destroy(c));
forward!(WaitCondVar(c: *mut RustCondVar) = rust_condvar_wait(c, ptr::null_mut()));
forward!(SignalCondVar(c: *mut RustCondVar) = rust_condvar_signal(c));
forward!(BroadcastCondVar(c: *mut RustCondVar) = rust_condvar_broadcast(c));
forward!(CreateThreadLocal() -> *mut c_void = rust_thread_local_create());
forward!(DestroyThreadLocal(t: *mut c_void) = rust_thread_local_destroy(t));
forward!(GetMyThreadLocal() -> *mut c_void = rust_get_my_thread_local());
