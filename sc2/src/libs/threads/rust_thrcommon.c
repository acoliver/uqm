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
#include "libs/timelib.h"
#include "libs/log.h"
#include "libs/async.h"
#include "libs/memlib.h"
#include "thrcommon.h"
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
CreateThread_Core (ThreadFunction func, void *data, SDWORD stackSize, const char *name)
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
CreateMutex_Core (const char *name, DWORD syncClass)
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
CreateSemaphore_Core (DWORD initial, const char *name, DWORD syncClass)
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
CreateCondVar_Core (const char *name, DWORD syncClass)
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
CreateRecursiveMutex_Core (const char *name, DWORD syncClass)
{
	/* Rust std::sync::Mutex is not recursive; for now use regular mutex */
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

int
GetRecursiveMutexDepth (RecursiveMutex m)
{
	/* Rust mutexes don't track recursion depth */
	(void)m;
	return 0;
}

void
StartThread_Core (ThreadFunction func, void *data, SDWORD stackSize, const char *name)
{
	/* Fire-and-forget thread spawn */
	(void)stackSize;
	rust_thread_spawn(name, (void (*)(void*))func, data);
}

void
FinishThread (Thread thread)
{
	/* Mark thread for cleanup; in Rust threads clean up on drop */
	(void)thread;
}

void
ProcessThreadLifecycles (void)
{
	/* Rust threads are self-managing; nothing to do */
}

void
DestroyThread (Thread t)
{
	/* Rust threads clean up on join/drop */
	(void)t;
}

ThreadLocal *
CreateThreadLocal (void)
{
	ThreadLocal *tl = HMalloc (sizeof (ThreadLocal));
	tl->flushSem = CreateSemaphore (0, "FlushGraphics", SYNC_CLASS_VIDEO);
	return tl;
}

void
DestroyThreadLocal (ThreadLocal *tl)
{
	DestroySemaphore (tl->flushSem);
	HFree (tl);
}

ThreadLocal *
GetMyThreadLocal (void)
{
	/* Fall through to the native SDL implementation for TLS */
	return NativeGetMyThreadLocal ();
}

void
HibernateThread (TimePeriod timePeriod)
{
	uint32 msecs = (uint32)(timePeriod * 1000 / ONE_SECOND);
	rust_hibernate_thread(msecs);
}

void
HibernateThreadUntil (TimeCount wakeTime)
{
	TimeCount now = GetTimeCounter();
	if (wakeTime > now)
	{
		HibernateThread(wakeTime - now);
	}
}

#endif /* USE_RUST_THREADS */
