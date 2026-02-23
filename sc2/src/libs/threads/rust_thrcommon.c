/*
 *  Rust Threading System wrapper
 *  
 *  When USE_RUST_THREADS is defined, this file provides the threading
 *  implementation via the Rust FFI bindings.
 */

#ifdef USE_RUST_THREADS

#include <stdio.h>
#include <stdlib.h>
#include "libs/threadlib.h"
#include "libs/log.h"
#include "rust_threads.h"

/* Forward declarations of Rust FFI functions */
extern int rust_init_thread_system(void);
extern void rust_uninit_thread_system(void);
extern int rust_is_thread_system_initialized(void);
extern RustThread* rust_thread_spawn(const char* name, void (*func)(void*), void* data);
extern int rust_thread_join(RustThread* thread);
extern void rust_thread_yield(void);
extern void rust_hibernate_thread(uint32 msecs);
extern RustMutex* rust_mutex_create(const char* name);
extern void rust_mutex_destroy(RustMutex* mutex);
extern void rust_mutex_lock(RustMutex* mutex);
extern int rust_mutex_try_lock(RustMutex* mutex);
extern void rust_mutex_unlock(RustMutex* mutex);
extern RustCondVar* rust_condvar_create(const char* name);
extern void rust_condvar_destroy(RustCondVar* cond);
extern void rust_condvar_wait(RustCondVar* cond, RustMutex* mutex);
extern int rust_condvar_wait_timeout(RustCondVar* cond, RustMutex* mutex, uint32 msecs);
extern void rust_condvar_signal(RustCondVar* cond);
extern void rust_condvar_broadcast(RustCondVar* cond);
extern RustSemaphore* rust_semaphore_create(uint32 initial, const char* name);
extern void rust_semaphore_destroy(RustSemaphore* sem);
extern void rust_semaphore_acquire(RustSemaphore* sem);
extern int rust_semaphore_try_acquire(RustSemaphore* sem);
extern void rust_semaphore_release(RustSemaphore* sem);
extern uint32 rust_semaphore_count(RustSemaphore* sem);
extern void rust_task_switch(void);

void
InitThreadSystem (void)
{
	rust_init_thread_system();
	log_add(log_Debug, "Rust thread system initialized");
}

void
UnInitThreadSystem (void)
{
	rust_uninit_thread_system();
}

Thread
CreateThread (ThreadFunction func, void *data, SDWORD stackSize, const char *name)
{
	(void)stackSize; /* Rust manages its own stack */
	return (Thread)rust_thread_spawn(name, (void (*)(void*))func, data);
}

void
SleepThread (TimeCount sleepTime)
{
	uint32 msecs = (uint32)(sleepTime * 1000 / ONE_SECOND);
	rust_hibernate_thread(msecs);
}

void
SleepThreadUntil (TimeCount wakeTime)
{
	TimeCount now = GetTimeCounter();
	if (wakeTime > now)
	{
		SleepThread(wakeTime - now);
	}
}

void
TaskSwitch (void)
{
	rust_task_switch();
}

void
WaitThread (Thread thread, int *status)
{
	(void)status; /* Rust doesn't return status this way */
	if (thread)
	{
		rust_thread_join((RustThread*)thread);
	}
}

Mutex
CreateMutex (const char *name, DWORD syncClass)
{
	(void)syncClass;
	return (Mutex)rust_mutex_create(name);
}

void
DestroyMutex (Mutex m)
{
	rust_mutex_destroy((RustMutex*)m);
}

void
LockMutex (Mutex m)
{
	rust_mutex_lock((RustMutex*)m);
}

void
UnlockMutex (Mutex m)
{
	rust_mutex_unlock((RustMutex*)m);
}

Semaphore
CreateSemaphore (DWORD initial, const char *name, DWORD syncClass)
{
	(void)syncClass;
	return (Semaphore)rust_semaphore_create((uint32)initial, name);
}

void
DestroySemaphore (Semaphore s)
{
	rust_semaphore_destroy((RustSemaphore*)s);
}

void
SetSemaphore (Semaphore s)
{
	rust_semaphore_acquire((RustSemaphore*)s);
}

void
ClearSemaphore (Semaphore s)
{
	rust_semaphore_release((RustSemaphore*)s);
}

CondVar
CreateCondVar (const char *name, DWORD syncClass)
{
	(void)syncClass;
	return (CondVar)rust_condvar_create(name);
}

void
DestroyCondVar (CondVar c)
{
	rust_condvar_destroy((RustCondVar*)c);
}

void
WaitCondVar (CondVar c)
{
	/* Note: Rust condvar doesn't need external mutex for this simplified API */
	rust_condvar_wait((RustCondVar*)c, NULL);
}

void
SignalCondVar (CondVar c)
{
	rust_condvar_signal((RustCondVar*)c);
}

void
BroadcastCondVar (CondVar c)
{
	rust_condvar_broadcast((RustCondVar*)c);
}

RecursiveMutex
CreateRecursiveMutex (const char *name, DWORD syncClass)
{
	/* Rust std::sync::Mutex is not recursive; for now use regular mutex */
	/* TODO: Implement proper recursive mutex if needed */
	(void)syncClass;
	return (RecursiveMutex)rust_mutex_create(name);
}

void
DestroyRecursiveMutex (RecursiveMutex m)
{
	rust_mutex_destroy((RustMutex*)m);
}

void
LockRecursiveMutex (RecursiveMutex m)
{
	rust_mutex_lock((RustMutex*)m);
}

void
UnlockRecursiveMutex (RecursiveMutex m)
{
	rust_mutex_unlock((RustMutex*)m);
}

int
TryLockRecursiveMutex (RecursiveMutex m)
{
	return rust_mutex_try_lock((RustMutex*)m);
}

#endif /* USE_RUST_THREADS */
