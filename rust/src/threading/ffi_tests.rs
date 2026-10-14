//! Behaviour of the C-facing handles: recursive mutexes, condition
//! variables, semaphores, thread spawning and per-thread data, exercised
//! with real threads through the exported `rust_*` functions.

use super::tests::hold_thread_system;
use super::*;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

/// Raw handles are plain heap pointers; moving the address between threads
/// is how the C side shares them.
#[derive(Clone, Copy)]
struct Handle(usize);

impl Handle {
    fn of<T>(ptr: *mut T) -> Self {
        Handle(ptr as usize)
    }
    fn mutex(self) -> *mut RustMutex {
        self.0 as *mut RustMutex
    }
    fn condvar(self) -> *mut RustCondVar {
        self.0 as *mut RustCondVar
    }
    fn semaphore(self) -> *mut RustSemaphore {
        self.0 as *mut RustSemaphore
    }
}

fn wait_until(what: &str, condition: impl Fn() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !condition() {
        assert!(Instant::now() < deadline, "timed out waiting for {what}");
        thread::sleep(Duration::from_millis(1));
    }
}

// ---------------------------------------------------------------------------
// Mutex
// ---------------------------------------------------------------------------

#[test]
fn ffi_mutex_is_recursive_and_tracks_its_depth() {
    unsafe {
        let mutex = rust_mutex_create(c"recursive".as_ptr());
        assert_eq!(rust_mutex_depth(mutex), 0);

        rust_mutex_lock(mutex);
        rust_mutex_lock(mutex);
        assert_eq!(rust_mutex_depth(mutex), 2);
        assert_eq!(rust_mutex_try_lock(mutex), 1, "the owner may re-enter");
        assert_eq!(rust_mutex_depth(mutex), 3);

        rust_mutex_unlock(mutex);
        rust_mutex_unlock(mutex);
        assert_eq!(rust_mutex_depth(mutex), 1);
        rust_mutex_unlock(mutex);
        assert_eq!(rust_mutex_depth(mutex), 0);
        rust_mutex_destroy(mutex);
    }
}

#[test]
fn ffi_mutex_try_lock_fails_for_other_threads_until_fully_released() {
    unsafe {
        let mutex = Handle::of(rust_mutex_create(ptr::null()));
        rust_mutex_lock(mutex.mutex());
        rust_mutex_lock(mutex.mutex());

        let try_from_other_thread = move || {
            thread::spawn(move || rust_mutex_try_lock(mutex.mutex()))
                .join()
                .unwrap()
        };
        assert_eq!(try_from_other_thread(), 0);
        rust_mutex_unlock(mutex.mutex());
        assert_eq!(try_from_other_thread(), 0, "one level is still held");
        rust_mutex_unlock(mutex.mutex());
        assert_eq!(try_from_other_thread(), 1);
        rust_mutex_destroy(mutex.mutex());
    }
}

#[test]
fn ffi_mutex_lock_blocks_until_the_owner_releases() {
    unsafe {
        let mutex = Handle::of(rust_mutex_create(ptr::null()));
        let released = Arc::new(AtomicBool::new(false));
        let (about_to_lock_tx, about_to_lock_rx) = mpsc::channel();
        rust_mutex_lock(mutex.mutex());

        let waiter = {
            let released = Arc::clone(&released);
            thread::spawn(move || {
                about_to_lock_tx.send(()).unwrap();
                rust_mutex_lock(mutex.mutex());
                let saw_release = released.load(Ordering::SeqCst);
                rust_mutex_unlock(mutex.mutex());
                saw_release
            })
        };
        about_to_lock_rx.recv().unwrap();
        thread::sleep(Duration::from_millis(20));
        released.store(true, Ordering::SeqCst);
        rust_mutex_unlock(mutex.mutex());

        assert!(
            waiter.join().unwrap(),
            "the waiter got in before the release"
        );
        rust_mutex_destroy(mutex.mutex());
    }
}

#[test]
fn unlocking_a_mutex_from_a_thread_that_does_not_own_it_is_refused() {
    let mutex = Arc::new(RustFfiMutex::new(Some("owned")));
    mutex.lock().unwrap();

    let other = Arc::clone(&mutex);
    let result = thread::spawn(move || other.unlock()).join().unwrap();

    assert!(matches!(result, Err(ThreadError::InvalidOperation(_))));
    assert_eq!(mutex.depth(), 1, "the refused unlock changed nothing");
    mutex.unlock().unwrap();
    assert!(
        matches!(mutex.unlock(), Err(ThreadError::InvalidOperation(_))),
        "unlocking an unlocked mutex is refused"
    );
}

#[test]
fn ffi_calls_on_null_handles_do_nothing_and_report_failure() {
    unsafe {
        rust_mutex_lock(ptr::null_mut());
        rust_mutex_unlock(ptr::null_mut());
        rust_mutex_destroy(ptr::null_mut());
        assert_eq!(rust_mutex_try_lock(ptr::null_mut()), 0);
        assert_eq!(rust_mutex_depth(ptr::null_mut()), 0);

        rust_condvar_wait(ptr::null_mut(), ptr::null_mut());
        assert_eq!(
            rust_condvar_wait_timeout(ptr::null_mut(), ptr::null_mut(), 1),
            0
        );
        rust_condvar_signal(ptr::null_mut());
        rust_condvar_broadcast(ptr::null_mut());
        rust_condvar_destroy(ptr::null_mut());

        rust_semaphore_acquire(ptr::null_mut());
        assert_eq!(rust_semaphore_try_acquire(ptr::null_mut()), 0);
        rust_semaphore_release(ptr::null_mut());
        assert_eq!(rust_semaphore_count(ptr::null_mut()), 0);
        rust_semaphore_destroy(ptr::null_mut());

        rust_thread_local_destroy(ptr::null_mut());
    }
}

// ---------------------------------------------------------------------------
// Semaphore
// ---------------------------------------------------------------------------

#[test]
fn ffi_semaphore_hands_out_exactly_its_permits() {
    unsafe {
        let sem = rust_semaphore_create(2, c"permits".as_ptr());
        assert_eq!(rust_semaphore_count(sem), 2);

        assert_eq!(rust_semaphore_try_acquire(sem), 1);
        assert_eq!(rust_semaphore_try_acquire(sem), 1);
        assert_eq!(rust_semaphore_try_acquire(sem), 0);
        assert_eq!(rust_semaphore_count(sem), 0);

        rust_semaphore_release(sem);
        assert_eq!(rust_semaphore_count(sem), 1);
        rust_semaphore_acquire(sem);
        assert_eq!(rust_semaphore_count(sem), 0);
        rust_semaphore_destroy(sem);
    }
}

#[test]
fn ffi_semaphore_acquire_waits_for_a_release_from_another_thread() {
    unsafe {
        let sem = Handle::of(rust_semaphore_create(0, ptr::null()));
        let acquired = Arc::new(AtomicBool::new(false));

        let waiter = {
            let acquired = Arc::clone(&acquired);
            thread::spawn(move || {
                rust_semaphore_acquire(sem.semaphore());
                acquired.store(true, Ordering::SeqCst);
            })
        };
        thread::sleep(Duration::from_millis(20));
        assert!(
            !acquired.load(Ordering::SeqCst),
            "no permit has been released"
        );
        rust_semaphore_release(sem.semaphore());
        waiter.join().unwrap();

        assert!(acquired.load(Ordering::SeqCst));
        assert_eq!(rust_semaphore_count(sem.semaphore()), 0);
        rust_semaphore_destroy(sem.semaphore());
    }
}

// ---------------------------------------------------------------------------
// Condition variable
// ---------------------------------------------------------------------------

#[test]
fn ffi_condvar_wait_times_out_without_a_signal() {
    unsafe {
        let cond = rust_condvar_create(c"idle".as_ptr());
        let mutex = rust_mutex_create(ptr::null());

        let started = Instant::now();
        assert_eq!(rust_condvar_wait_timeout(cond, mutex, 30), 0);
        assert!(started.elapsed() >= Duration::from_millis(25));

        rust_condvar_destroy(cond);
        rust_mutex_destroy(mutex);
    }
}

#[test]
fn ffi_condvar_signal_raised_before_the_wait_is_not_lost() {
    unsafe {
        let cond = rust_condvar_create(ptr::null());
        let mutex = rust_mutex_create(ptr::null());

        rust_condvar_signal(cond);

        assert_eq!(rust_condvar_wait_timeout(cond, mutex, 1000), 1);
        assert_eq!(
            rust_condvar_wait_timeout(cond, mutex, 10),
            0,
            "the latched signal was consumed by the first wait"
        );
        rust_condvar_destroy(cond);
        rust_mutex_destroy(mutex);
    }
}

#[test]
fn ffi_condvar_broadcast_releases_every_waiter() {
    unsafe {
        let cond = Handle::of(rust_condvar_create(ptr::null()));
        let mutex = Handle::of(rust_mutex_create(ptr::null()));
        let woken = Arc::new(AtomicI32::new(0));

        let waiters: Vec<_> = (0..3)
            .map(|_| {
                let woken = Arc::clone(&woken);
                thread::spawn(move || {
                    rust_condvar_wait(cond.condvar(), mutex.mutex());
                    woken.fetch_add(1, Ordering::SeqCst);
                })
            })
            .collect();

        // A broadcast only reaches threads already waiting, so keep sending.
        wait_until("all three waiters to wake", || {
            rust_condvar_broadcast(cond.condvar());
            woken.load(Ordering::SeqCst) == 3
        });
        for waiter in waiters {
            waiter.join().unwrap();
        }
        rust_condvar_destroy(cond.condvar());
        rust_mutex_destroy(mutex.mutex());
    }
}

#[test]
fn ffi_condvar_signal_wakes_a_waiting_thread() {
    unsafe {
        let cond = Handle::of(rust_condvar_create(ptr::null()));
        let mutex = Handle::of(rust_mutex_create(ptr::null()));
        let done = Arc::new(AtomicBool::new(false));

        let waiter = {
            let done = Arc::clone(&done);
            thread::spawn(move || {
                rust_condvar_wait(cond.condvar(), mutex.mutex());
                done.store(true, Ordering::SeqCst);
            })
        };
        wait_until("the waiter to wake", || {
            rust_condvar_signal(cond.condvar());
            done.load(Ordering::SeqCst)
        });
        waiter.join().unwrap();
        rust_condvar_destroy(cond.condvar());
        rust_mutex_destroy(mutex.mutex());
    }
}

// ---------------------------------------------------------------------------
// Threads
// ---------------------------------------------------------------------------

unsafe extern "C" fn double_the_counter(data: *mut c_void) -> c_int {
    let counter = &*(data as *const AtomicI32);
    let doubled = counter.load(Ordering::SeqCst) * 2;
    counter.store(doubled, Ordering::SeqCst);
    doubled
}

unsafe extern "C" fn report_thread_local(data: *mut c_void) -> c_int {
    let seen = &*(data as *const AtomicBool);
    seen.store(!rust_get_my_thread_local().is_null(), Ordering::SeqCst);
    0
}

#[test]
fn ffi_spawned_thread_runs_the_c_function_on_its_data_and_returns_its_status() {
    unsafe {
        let counter = AtomicI32::new(21);
        let handle = rust_thread_spawn(
            c"doubler".as_ptr(),
            double_the_counter,
            &counter as *const AtomicI32 as *mut c_void,
        );
        assert!(!handle.is_null());

        let mut status = -1;
        assert_eq!(rust_thread_join(handle, &mut status), 1);

        assert_eq!(status, 42);
        assert_eq!(counter.load(Ordering::SeqCst), 42);
    }
}

#[test]
fn ffi_detached_thread_still_runs_to_completion() {
    static DETACHED_COUNTER: AtomicI32 = AtomicI32::new(5);
    unsafe {
        rust_thread_spawn_detached(
            ptr::null(),
            double_the_counter,
            &DETACHED_COUNTER as *const AtomicI32 as *mut c_void,
        );
    }

    wait_until("the detached thread to finish", || {
        DETACHED_COUNTER.load(Ordering::SeqCst) == 10
    });
}

#[test]
fn ffi_spawned_threads_get_their_own_thread_local_for_the_duration_of_the_call() {
    unsafe {
        let seen = AtomicBool::new(false);
        let handle = rust_thread_spawn(
            ptr::null(),
            report_thread_local,
            &seen as *const AtomicBool as *mut c_void,
        );
        let mut status = -1;
        assert_eq!(rust_thread_join(handle, &mut status), 1);

        assert!(seen.load(Ordering::SeqCst));
        assert!(
            rust_get_my_thread_local().is_null(),
            "the spawning thread never attached"
        );
    }
}

#[test]
fn ffi_thread_local_is_created_once_per_thread_and_cleared_on_destroy() {
    thread::spawn(|| unsafe {
        assert!(rust_get_my_thread_local().is_null());

        let created = rust_thread_local_create();
        assert!(!created.is_null());
        assert_eq!(rust_get_my_thread_local(), created);
        assert_eq!(
            rust_thread_local_create(),
            created,
            "a second create reuses it"
        );
        assert!(get_my_thread_local().is_some());

        rust_thread_local_destroy(created);
        assert!(rust_get_my_thread_local().is_null());
        assert!(get_my_thread_local().is_none());
    })
    .join()
    .unwrap();
}

#[test]
fn ffi_thread_system_lifecycle_reports_each_transition() {
    let _guard = hold_thread_system();
    unsafe {
        rust_uninit_thread_system();
        assert_eq!(rust_is_thread_system_initialized(), 0);

        assert_eq!(rust_init_thread_system(), 1);
        assert_eq!(rust_is_thread_system_initialized(), 1);
        assert_eq!(rust_init_thread_system(), 0, "a second init is refused");

        rust_uninit_thread_system();
        assert_eq!(rust_is_thread_system_initialized(), 0);
    }
}

#[test]
fn ffi_yield_and_hibernate_return_after_the_requested_pause() {
    let started = Instant::now();
    unsafe {
        rust_thread_yield();
        rust_task_switch();
        rust_hibernate_thread(20);
    }
    assert!(started.elapsed() >= Duration::from_millis(20));
}
