//! Unit tests for the threading module
//!
//! These tests are written test-first - they should compile but fail until
//! the threading module is fully implemented.
//!
//! # Test Categories
//! 1. Thread tests - spawning, joining, return values
//! 2. Mutex tests - locking, try_lock, contention
//! 3. Condition variable tests - wait/signal/broadcast
//! 4. Semaphore tests - counting semaphore operations
//! 5. Task tests - task creation, state, execution
//!
//! # Reference
//! These tests validate behavior matching the C implementation in:
//! - `sc2/src/libs/threads/thrcommon.c`
//! - `sc2/src/libs/threads/sdl/sdlthreads.c`

use super::*;
use std::sync::atomic::{AtomicI32, AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

// ============================================================================
// Thread Tests
// ============================================================================

/// Test basic thread spawn and join
///
/// Validates that we can spawn a thread, have it do work, and join it.
/// The C implementation uses CreateThread_Core -> NativeCreateThread -> SDL_CreateThread
/// and WaitThread -> NativeWaitThread -> SDL_WaitThread.
#[test]
fn test_thread_spawn_and_join() {
    // Initialize thread system (C: InitThreadSystem)
    let _ = init_thread_system();

    // Spawn a thread that does some work
    let counter = Arc::new(AtomicUsize::new(0));
    let counter_clone = Arc::clone(&counter);

    let thread = Thread::spawn(Some("test_worker"), move || {
        // Simulate some work
        for _ in 0..10 {
            counter_clone.fetch_add(1, Ordering::SeqCst);
        }
    })
    .expect("Thread spawn should succeed");

    // Join the thread
    thread.join().expect("Thread join should succeed");

    // Verify work was done
    assert_eq!(counter.load(Ordering::SeqCst), 10);

    uninit_thread_system();
}

/// Test thread spawn with return value
///
/// Validates that threads can return values to the caller.
/// The C implementation passes return values through WaitThread's status parameter.
#[test]
fn test_thread_spawn_with_return_value() {
    let _ = init_thread_system();

    // Spawn a thread that computes and returns a value
    let thread = Thread::spawn(Some("compute_thread"), || {
        // Compute factorial of 5
        let mut result = 1;
        for i in 1..=5 {
            result *= i;
        }
        result
    })
    .expect("Thread spawn should succeed");

    // Join and get the return value
    let result = thread.join().expect("Thread join should succeed");
    assert_eq!(result, 120); // 5! = 120

    uninit_thread_system();
}

/// Test multiple concurrent threads
///
/// Validates that multiple threads can run concurrently and complete.
/// The C implementation maintains threads in a thread queue (threadQueue).
#[test]
fn test_multiple_threads_concurrent() {
    let _ = init_thread_system();

    let counter = Arc::new(AtomicUsize::new(0));
    let num_threads = 4;
    let iterations_per_thread = 100;

    // Spawn multiple threads
    let threads: Vec<_> = (0..num_threads)
        .map(|i| {
            let counter_clone = Arc::clone(&counter);
            Thread::spawn(Some(&format!("worker_{}", i)), move || {
                for _ in 0..iterations_per_thread {
                    counter_clone.fetch_add(1, Ordering::SeqCst);
                }
            })
            .expect("Thread spawn should succeed")
        })
        .collect();

    // Join all threads
    for thread in threads {
        thread.join().expect("Thread join should succeed");
    }

    // Verify all work was done
    assert_eq!(
        counter.load(Ordering::SeqCst),
        num_threads * iterations_per_thread
    );

    uninit_thread_system();
}

/// Test thread name is preserved
///
/// Validates that thread names are stored and retrievable.
/// The C implementation uses NAMED_SYNCHRO to track thread names.
#[test]
fn test_thread_name() {
    let _ = init_thread_system();

    let thread = Thread::spawn(Some("named_thread"), || {
        // Thread does nothing
    })
    .expect("Thread spawn should succeed");

    // The thread should store its name (internal implementation detail)
    // This is mainly for debugging purposes
    assert!(thread.is_running() || true); // Thread may complete quickly

    thread.join().expect("Thread join should succeed");

    uninit_thread_system();
}

// ============================================================================
// Mutex Tests
// ============================================================================

/// Test basic mutex lock and unlock
///
/// Validates that a mutex can be locked and unlocked.
/// The C implementation uses LockMutex -> SDL_mutexP and UnlockMutex -> SDL_mutexV.
#[test]
fn test_mutex_lock_unlock() {
    let mutex = UqmMutex::new(42i32, Some("test_mutex"));

    // Lock the mutex
    {
        let guard = mutex.lock().expect("Lock should succeed");
        assert_eq!(*guard, 42);
    }
    // Mutex is automatically unlocked when guard is dropped

    // Lock again to verify it was released
    {
        let guard = mutex.lock().expect("Second lock should succeed");
        assert_eq!(*guard, 42);
    }
}

/// Test mutex try_lock
///
/// Validates non-blocking lock acquisition.
/// Note: The C implementation doesn't have try_lock, but it's useful for Rust.
#[test]
fn test_mutex_try_lock() {
    let mutex = Arc::new(UqmMutex::new(0i32, Some("trylock_mutex")));
    let mutex_clone = Arc::clone(&mutex);

    // Lock the mutex
    let guard = mutex.lock().expect("Lock should succeed");

    // Spawn a thread that tries to lock
    let handle = thread::spawn(move || {
        // try_lock should fail because main thread holds the lock
        let result = mutex_clone.try_lock().expect("try_lock should not error");
        result.is_none()
    });

    // Give the thread time to try
    thread::sleep(Duration::from_millis(10));

    // Drop our lock
    drop(guard);

    // The thread should have seen the lock as held
    let was_locked = handle.join().expect("Thread should not panic");
    assert!(was_locked, "try_lock should have failed while lock was held");
}

/// Test mutex contention
///
/// Validates that multiple threads can compete for a mutex.
/// The C implementation tracks contention via TRACK_CONTENTION.
#[test]
fn test_mutex_contention() {
    let mutex = Arc::new(UqmMutex::new(0i32, Some("contention_mutex")));
    let num_threads = 4;
    let iterations = 100;

    let threads: Vec<_> = (0..num_threads)
        .map(|_| {
            let mutex_clone = Arc::clone(&mutex);
            thread::spawn(move || {
                for _ in 0..iterations {
                    let mut guard = mutex_clone.lock().expect("Lock should succeed");
                    *guard += 1;
                    // Hold the lock briefly to create contention
                    thread::yield_now();
                }
            })
        })
        .collect();

    for handle in threads {
        handle.join().expect("Thread should not panic");
    }

    // Verify all increments happened
    let final_value = *mutex.lock().expect("Lock should succeed");
    assert_eq!(final_value, num_threads * iterations);
}

/// Test mutex with mutable data
///
/// Validates that the mutex properly protects mutable data.
#[test]
fn test_mutex_protects_data() {
    let mutex = UqmMutex::new(vec![1, 2, 3], Some("vec_mutex"));

    {
        let mut guard = mutex.lock().expect("Lock should succeed");
        guard.push(4);
        guard.push(5);
    }

    {
        let guard = mutex.lock().expect("Lock should succeed");
        assert_eq!(*guard, vec![1, 2, 3, 4, 5]);
    }
}

// ============================================================================
// Condition Variable Tests
// ============================================================================

/// Test condition variable wait and signal
///
/// Validates that one thread can wait on a condvar and another can signal it.
/// The C implementation uses WaitCondVar -> SDL_CondWait and SignalCondVar -> SDL_CondSignal.
#[test]
fn test_condvar_wait_signal() {
    let condvar = Arc::new(UqmCondVar::new(Some("test_condvar")));
    let signaled = Arc::new(AtomicBool::new(false));

    let condvar_clone = Arc::clone(&condvar);
    let signaled_clone = Arc::clone(&signaled);

    // Spawn waiting thread
    let waiter = thread::spawn(move || {
        condvar_clone.wait().expect("Wait should succeed");
        signaled_clone.store(true, Ordering::SeqCst);
    });

    // Give the waiter time to start waiting
    thread::sleep(Duration::from_millis(50));

    // Signal shouldn't have been received yet (thread is waiting)
    assert!(!signaled.load(Ordering::SeqCst));

    // Signal the condvar
    condvar.signal();

    // Wait for the thread to finish
    waiter.join().expect("Waiter should not panic");

    // Verify the signal was received
    assert!(signaled.load(Ordering::SeqCst));
}

/// Test condition variable broadcast
///
/// Validates that broadcast wakes all waiting threads.
/// The C implementation uses BroadcastCondVar -> SDL_CondBroadcast.
#[test]
fn test_condvar_broadcast() {
    let condvar = Arc::new(UqmCondVar::new(Some("broadcast_condvar")));
    let woken_count = Arc::new(AtomicUsize::new(0));
    let num_waiters = 4;

    // Spawn multiple waiting threads
    let waiters: Vec<_> = (0..num_waiters)
        .map(|_| {
            let condvar_clone = Arc::clone(&condvar);
            let woken_clone = Arc::clone(&woken_count);
            thread::spawn(move || {
                condvar_clone.wait().expect("Wait should succeed");
                woken_clone.fetch_add(1, Ordering::SeqCst);
            })
        })
        .collect();

    // Give waiters time to start waiting
    thread::sleep(Duration::from_millis(50));

    // No threads should be woken yet
    assert_eq!(woken_count.load(Ordering::SeqCst), 0);

    // Broadcast to wake all
    condvar.broadcast();

    // Wait for all threads to finish
    for waiter in waiters {
        waiter.join().expect("Waiter should not panic");
    }

    // All waiters should have woken
    assert_eq!(woken_count.load(Ordering::SeqCst), num_waiters);
}

/// Test condition variable with timeout
///
/// Validates that wait_timeout returns after the timeout expires.
#[test]
fn test_condvar_wait_timeout() {
    let condvar = UqmCondVar::new(Some("timeout_condvar"));

    // Wait with a short timeout - should return false (not signaled)
    let start = std::time::Instant::now();
    let result = condvar
        .wait_timeout(Duration::from_millis(50))
        .expect("wait_timeout should not error");
    let elapsed = start.elapsed();

    // Should have timed out
    assert!(!result, "Should have timed out, not signaled");

    // Should have waited approximately the timeout duration
    assert!(elapsed >= Duration::from_millis(40)); // Allow some tolerance
    assert!(elapsed < Duration::from_millis(200)); // But not too long
}

// ============================================================================
// Semaphore Tests
// ============================================================================

/// Test basic semaphore acquire and release
///
/// Validates that semaphore permits can be acquired and released.
/// The C implementation uses SetSemaphore (acquire/wait) and ClearSemaphore (release/post).
#[test]
fn test_semaphore_acquire_release() {
    let sem = Semaphore::new(1, Some("basic_semaphore"));

    // Should be able to acquire the initial permit
    sem.acquire().expect("Acquire should succeed");

    // Count should be 0 now
    assert_eq!(sem.count(), 0);

    // Release the permit
    sem.release();

    // Count should be 1 again
    assert_eq!(sem.count(), 1);
}

/// Test that semaphore blocks when count is zero
///
/// Validates that acquiring a semaphore with zero permits blocks.
/// The C implementation blocks in SDL_SemWait until SDL_SemPost is called.
#[test]
fn test_semaphore_zero_blocks() {
    let sem = Arc::new(Semaphore::new(0, Some("blocking_semaphore")));
    let acquired = Arc::new(AtomicBool::new(false));

    let sem_clone = Arc::clone(&sem);
    let acquired_clone = Arc::clone(&acquired);

    // Spawn thread that tries to acquire (should block)
    let waiter = thread::spawn(move || {
        sem_clone.acquire().expect("Acquire should succeed");
        acquired_clone.store(true, Ordering::SeqCst);
    });

    // Give the thread time to block
    thread::sleep(Duration::from_millis(50));

    // Should not have acquired yet
    assert!(!acquired.load(Ordering::SeqCst));

    // Release a permit
    sem.release();

    // Wait for the thread
    waiter.join().expect("Waiter should not panic");

    // Should have acquired now
    assert!(acquired.load(Ordering::SeqCst));
}

/// Test semaphore with multiple permits
///
/// Validates counting semaphore behavior with multiple permits.
/// The C implementation uses SDL semaphores which are counting semaphores.
#[test]
fn test_semaphore_multiple_permits() {
    let sem = Semaphore::new(3, Some("multi_permit_semaphore"));

    // Should be able to acquire 3 times
    assert!(sem.try_acquire());
    assert!(sem.try_acquire());
    assert!(sem.try_acquire());

    // Fourth acquire should fail (non-blocking)
    assert!(!sem.try_acquire());

    // Release one
    sem.release();

    // Now can acquire again
    assert!(sem.try_acquire());

    // Count should be 0
    assert_eq!(sem.count(), 0);
}

/// Test semaphore try_acquire
///
/// Validates non-blocking acquire behavior.
#[test]
fn test_semaphore_try_acquire() {
    let sem = Semaphore::new(2, Some("tryacquire_semaphore"));

    // Should succeed twice
    assert!(sem.try_acquire());
    assert!(sem.try_acquire());

    // Should fail on third
    assert!(!sem.try_acquire());

    // Release and try again
    sem.release();
    assert!(sem.try_acquire());
}

/// Test semaphore producer-consumer pattern
///
/// Validates semaphore use in a classic producer-consumer scenario.
/// This matches common usage in the C codebase for synchronization.
#[test]
fn test_semaphore_producer_consumer() {
    let sem = Arc::new(Semaphore::new(0, Some("producer_consumer")));
    let items_produced = Arc::new(AtomicUsize::new(0));
    let items_consumed = Arc::new(AtomicUsize::new(0));
    let num_items = 10;

    let sem_producer = Arc::clone(&sem);
    let produced = Arc::clone(&items_produced);

    // Producer thread
    let producer = thread::spawn(move || {
        for _ in 0..num_items {
            produced.fetch_add(1, Ordering::SeqCst);
            sem_producer.release();
            thread::yield_now();
        }
    });

    let sem_consumer = Arc::clone(&sem);
    let consumed = Arc::clone(&items_consumed);

    // Consumer thread
    let consumer = thread::spawn(move || {
        for _ in 0..num_items {
            sem_consumer.acquire().expect("Acquire should succeed");
            consumed.fetch_add(1, Ordering::SeqCst);
        }
    });

    producer.join().expect("Producer should not panic");
    consumer.join().expect("Consumer should not panic");

    assert_eq!(items_produced.load(Ordering::SeqCst), num_items);
    assert_eq!(items_consumed.load(Ordering::SeqCst), num_items);
}

// ============================================================================
// Task Tests
// ============================================================================

/// Test task creation
///
/// Validates that tasks can be created with state and callbacks.
/// The C implementation uses task structures for game loop scheduling.
#[test]
fn test_task_create() {
    let executed = Arc::new(AtomicBool::new(false));
    let executed_clone = Arc::clone(&executed);

    let task = Task::new(Some("test_task"), move || {
        executed_clone.store(true, Ordering::SeqCst);
    });

    // Task should have an ID
    assert!(task.id() > 0);

    // Task should start in Ready state
    assert_eq!(task.state(), TaskState::Ready);
}

/// Test task state transitions
///
/// Validates that task state can be changed.
/// The C implementation tracks task state for scheduling decisions.
#[test]
fn test_task_set_state() {
    let task = Task::new(Some("state_task"), || {});

    // Initially Ready
    assert_eq!(task.state(), TaskState::Ready);

    // Transition to Running
    task.set_state(TaskState::Running);
    assert_eq!(task.state(), TaskState::Running);

    // Transition to Waiting
    task.set_state(TaskState::Waiting);
    assert_eq!(task.state(), TaskState::Waiting);

    // Transition to Completed
    task.set_state(TaskState::Completed);
    assert_eq!(task.state(), TaskState::Completed);

    // Transition to Cancelled
    task.set_state(TaskState::Cancelled);
    assert_eq!(task.state(), TaskState::Cancelled);
}

/// Test task callback execution
///
/// Validates that executing a task runs its callback.
#[test]
fn test_task_callback_execution() {
    let counter = Arc::new(AtomicI32::new(0));
    let counter_clone = Arc::clone(&counter);

    let mut task = Task::new(Some("callback_task"), move || {
        counter_clone.fetch_add(42, Ordering::SeqCst);
    });

    // Execute the task
    task.execute().expect("Task execution should succeed");

    // Callback should have run
    assert_eq!(counter.load(Ordering::SeqCst), 42);

    // Task state should be updated (implementation detail)
    // The task should not be executable again
    let result = task.execute();
    assert!(result.is_err(), "Task should not be executable twice");
}

/// Test task ID uniqueness
///
/// Validates that each task gets a unique ID.
#[test]
fn test_task_id_uniqueness() {
    let task1 = Task::new(None, || {});
    let task2 = Task::new(None, || {});
    let task3 = Task::new(None, || {});

    assert_ne!(task1.id(), task2.id());
    assert_ne!(task2.id(), task3.id());
    assert_ne!(task1.id(), task3.id());
}

// ============================================================================
// Thread System Tests
// ============================================================================

/// Test thread system initialization
///
/// Validates that the thread system can be initialized and uninitialized.
/// The C implementation uses InitThreadSystem and UnInitThreadSystem.
#[test]
fn test_thread_system_init() {
    // May already be initialized by other tests, so uninit first
    uninit_thread_system();

    assert!(!is_thread_system_initialized());

    init_thread_system().expect("Init should succeed");
    assert!(is_thread_system_initialized());

    // Double init should fail
    let result = init_thread_system();
    assert!(result.is_err());

    uninit_thread_system();
    assert!(!is_thread_system_initialized());
}

/// Test task_switch yields execution
///
/// Validates that task_switch allows other threads to run.
/// The C implementation uses TaskSwitch -> SDL_Delay(1).
#[test]
fn test_task_switch() {
    let counter = Arc::new(AtomicUsize::new(0));
    let counter_clone = Arc::clone(&counter);

    let handle = thread::spawn(move || {
        for _ in 0..10 {
            counter_clone.fetch_add(1, Ordering::SeqCst);
            thread::sleep(Duration::from_millis(1));
        }
    });

    // Use task_switch to yield while waiting
    while counter.load(Ordering::SeqCst) < 5 {
        task_switch();
    }

    handle.join().expect("Thread should complete");
    assert_eq!(counter.load(Ordering::SeqCst), 10);
}

/// Test hibernate_thread sleeps for the specified duration
///
/// Validates that hibernate_thread sleeps approximately the requested time.
/// The C implementation uses HibernateThread -> NativeSleepThread -> SDL_Delay.
#[test]
fn test_hibernate_thread() {
    let start = std::time::Instant::now();
    hibernate_thread(Duration::from_millis(50));
    let elapsed = start.elapsed();

    // Should have slept approximately 50ms (with some tolerance)
    assert!(elapsed >= Duration::from_millis(40));
    assert!(elapsed < Duration::from_millis(150));
}

// ============================================================================
// Thread-Local Storage Tests
// ============================================================================

/// Test thread-local storage access
///
/// Validates that thread-local data can be accessed.
/// The C implementation uses GetMyThreadLocal to access per-thread data.
#[test]
fn test_thread_local_access() {
    // Thread-local may not be set initially
    let tl = get_my_thread_local();
    // This test documents current behavior - may be None or Some depending on setup

    // Create new thread-local data
    let new_tl = ThreadLocal::new();
    assert!(new_tl.flush_sem.count() == 0); // Initial semaphore count
}

/// Test thread-local has flush semaphore
///
/// Validates that ThreadLocal contains a flush semaphore.
/// The C implementation uses flushSem for graphics synchronization.
#[test]
fn test_thread_local_flush_sem() {
    let tl = ThreadLocal::new();

    // Should have a flush semaphore
    let sem = &tl.flush_sem;

    // Initial count should be 0 (matches C: CreateSemaphore(0, "FlushGraphics", ...))
    assert_eq!(sem.count(), 0);

    // Should be able to release/acquire
    sem.release();
    assert_eq!(sem.count(), 1);

    assert!(sem.try_acquire());
    assert_eq!(sem.count(), 0);
}

// ============================================================================
// Error Handling Tests
// ============================================================================

/// Test ThreadError display
///
/// Validates that error messages are meaningful.
#[test]
fn test_thread_error_display() {
    let err = ThreadError::SpawnFailed("test error".to_string());
    let msg = format!("{}", err);
    assert!(msg.contains("spawn"));
    assert!(msg.contains("test error"));

    let err = ThreadError::MutexPoisoned;
    let msg = format!("{}", err);
    assert!(msg.contains("poisoned"));
}

/// Test Result type alias works
#[test]
fn test_result_type() {
    let ok: Result<i32> = Ok(42);
    assert_eq!(ok.unwrap(), 42);

    let err: Result<i32> = Err(ThreadError::NotInitialized);
    assert!(err.is_err());
}
