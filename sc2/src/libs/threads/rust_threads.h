/*
 *  Rust Threading System header
 *  
 *  Provides extern declarations for the Rust-implemented threading system.
 *  When USE_RUST_THREADS is defined, this system is used instead of
 *  the C SDL threading implementation.
 */

#ifndef LIBS_THREADS_RUST_THREADS_H_
#define LIBS_THREADS_RUST_THREADS_H_

#include "types.h"

#ifdef USE_RUST_THREADS

/* Opaque handles to Rust threading primitives */
typedef struct RustThread RustThread;
typedef struct RustMutex RustMutex;
typedef struct RustCondVar RustCondVar;
typedef struct RustSemaphore RustSemaphore;

/*
 * Rust Threading FFI functions
 * Defined in rust/src/threading/mod.rs and exported via staticlib
 */

/* Thread System Lifecycle */
extern int rust_init_thread_system(void);
extern void rust_uninit_thread_system(void);
extern int rust_is_thread_system_initialized(void);

/* Thread operations */
extern RustThread* rust_thread_spawn(const char* name, void (*func)(void*), void* data);
extern int rust_thread_join(RustThread* thread);
extern void rust_thread_yield(void);
extern void rust_hibernate_thread(uint32 msecs);

/* Mutex operations */
extern RustMutex* rust_mutex_create(const char* name);
extern void rust_mutex_destroy(RustMutex* mutex);
extern void rust_mutex_lock(RustMutex* mutex);
extern int rust_mutex_try_lock(RustMutex* mutex);
extern void rust_mutex_unlock(RustMutex* mutex);

/* Condition Variable operations */
extern RustCondVar* rust_condvar_create(const char* name);
extern void rust_condvar_destroy(RustCondVar* cond);
extern void rust_condvar_wait(RustCondVar* cond, RustMutex* mutex);
extern int rust_condvar_wait_timeout(RustCondVar* cond, RustMutex* mutex, uint32 msecs);
extern void rust_condvar_signal(RustCondVar* cond);
extern void rust_condvar_broadcast(RustCondVar* cond);

/* Semaphore operations */
extern RustSemaphore* rust_semaphore_create(uint32 initial, const char* name);
extern void rust_semaphore_destroy(RustSemaphore* sem);
extern void rust_semaphore_acquire(RustSemaphore* sem);
extern int rust_semaphore_try_acquire(RustSemaphore* sem);
extern void rust_semaphore_release(RustSemaphore* sem);
extern uint32 rust_semaphore_count(RustSemaphore* sem);

/* Task switch (cooperative yield) */
extern void rust_task_switch(void);

#endif /* USE_RUST_THREADS */

#endif /* LIBS_THREADS_RUST_THREADS_H_ */
