/*
 *  Rust Threading System wrapper
 *  
 *  When USE_RUST_THREADS is defined, this file provides the threading
 *  implementation via the Rust FFI bindings, replacing the native
 *  SDL/POSIX threading in thrcommon.c.
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

#define LIFECYCLE_SIZE 8

typedef struct _rust_thread {
	RustThread *native;
#ifdef NAMED_SYNCHRO
	const char *name;
#endif
} *TrueThread;

typedef struct ThreadStartInfo {
	ThreadFunction func;
	void *data;
	TrueThread thread;
} ThreadStartInfo;

static Mutex lifecycleMutex;
static Thread pendingDeath[LIFECYCLE_SIZE];

/* Forward declarations of Rust FFI functions */
extern int rust_init_thread_system(void);
extern void rust_uninit_thread_system(void);
extern int rust_is_thread_system_initialized(void);
extern RustThread* rust_thread_spawn(const char* name, int (*func)(void*), void* data);
extern void rust_thread_spawn_detached(const char* name, int (*func)(void*), void* data);
extern int rust_thread_join(RustThread* thread, int* out_status);
extern void rust_thread_yield(void);
extern void rust_hibernate_thread(uint32 msecs);
extern RustMutex* rust_mutex_create(const char* name);
extern void rust_mutex_destroy(RustMutex* mutex);
extern void rust_mutex_lock(RustMutex* mutex);
extern int rust_mutex_try_lock(RustMutex* mutex);
extern void rust_mutex_unlock(RustMutex* mutex);
extern int rust_mutex_depth(RustMutex* mutex);
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
extern void* rust_thread_local_create(void);
extern void rust_thread_local_destroy(void* thread_local);
extern void* rust_get_my_thread_local(void);

static void
InitLifecycleState (void)
{
	int i;
	for (i = 0; i < LIFECYCLE_SIZE; ++i)
		pendingDeath[i] = NULL;
}

static TrueThread
AllocThreadHandle (const char *name)
{
	TrueThread thread;

	thread = (TrueThread) HMalloc (sizeof *thread);
	thread->native = NULL;
#ifdef NAMED_SYNCHRO
	thread->name = name;
#else
	(void) name;
#endif
	return thread;
}

static int
RustThreadHelper (void *opaque)
{
	ThreadStartInfo *startInfo = (ThreadStartInfo *) opaque;
	ThreadFunction func = startInfo->func;
	void *data = startInfo->data;
	TrueThread thread = startInfo->thread;
	int result;

	HFree (startInfo);
	result = (*func) (data);

#ifdef DEBUG_THREADS
	if (thread != NULL)
	{
		log_add (log_Debug, "Thread '%s' done (returned %d).",
				thread->name, result);
		fflush (stderr);
	}
#endif

	FinishThread ((Thread) thread);
	return result;
}

void
InitThreadSystem (void)
{
	rust_init_thread_system();
	InitLifecycleState ();
	lifecycleMutex = CreateMutex ("Thread Lifecycle Mutex", SYNC_CLASS_RESOURCE);
	log_add(log_Debug, "Rust thread system initialized");
}

void
UnInitThreadSystem (void)
{
	ProcessThreadLifecycles ();
	if (lifecycleMutex)
	{
		DestroyMutex (lifecycleMutex);
		lifecycleMutex = 0;
	}
	rust_uninit_thread_system();
}

Thread
CreateThread_Core (ThreadFunction func, void *data, SDWORD stackSize, const char *name)
{
	TrueThread thread;
	ThreadStartInfo *startInfo;

	(void)stackSize; /* Rust manages its own stack */
	thread = AllocThreadHandle (name);
	startInfo = (ThreadStartInfo *) HMalloc (sizeof (*startInfo));
	startInfo->func = func;
	startInfo->data = data;
	startInfo->thread = thread;

	thread->native = rust_thread_spawn (name, RustThreadHelper, startInfo);
	if (thread->native == NULL)
	{
		HFree (startInfo);
		HFree (thread);
		return NULL;
	}

	return (Thread) thread;
}

void
StartThread_Core (ThreadFunction func, void *data, SDWORD stackSize, const char *name)
{
	TrueThread thread;
	ThreadStartInfo *startInfo;

	(void)stackSize;
	thread = AllocThreadHandle (name);
	startInfo = (ThreadStartInfo *) HMalloc (sizeof (*startInfo));
	startInfo->func = func;
	startInfo->data = data;
	startInfo->thread = thread;

	/* rust_thread_spawn (not rust_thread_spawn_detached) is intentional here.
	 * ProcessThreadLifecycles -> WaitThread -> rust_thread_join needs
	 * thread->native to hold a valid RustThread* for the current cleanup path.
	 * The detached-spawn failure contract in the spec would require a different
	 * ABI/design if adapter-owned wrapper cleanup on failure is to be guaranteed. */
	thread->native = rust_thread_spawn (name, RustThreadHelper, startInfo);
	if (thread->native == NULL)
	{
		HFree (startInfo);
		HFree (thread);
	}
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
	for (;;)
	{
		uint32 nextTimeMs;
		TimeCount nextTime;
		TimeCount now;

		Async_process ();

		now = GetTimeCounter ();
		if (wakeTime <= now)
			return;

		nextTimeMs = Async_timeBeforeNextMs ();
		nextTime = (nextTimeMs / 1000) * ONE_SECOND +
				((nextTimeMs % 1000) * ONE_SECOND / 1000);
				/* Overflow-safe conversion. */
		if (wakeTime < nextTime)
			nextTime = wakeTime;

		if (nextTime <= now)
			SleepThread (0);
		else
			SleepThread (nextTime - now);
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
	TrueThread t = (TrueThread) thread;

	if (status)
		*status = 0;

	if (t && t->native)
	{
		int out_status = 0;
		int result = rust_thread_join (t->native, &out_status);
		if (status)
		{
			if (result)
				*status = out_status;  /* actual thread return value */
			else
				*status = 0;           /* join failed */
		}
		t->native = NULL;
	}
}

void
FinishThread (Thread thread)
{
	int i;

	if (!thread || !lifecycleMutex)
		return;

	LockMutex (lifecycleMutex);
	for (i = 0; i < LIFECYCLE_SIZE; i++)
	{
		if (pendingDeath[i] == NULL)
		{
			pendingDeath[i] = thread;
			UnlockMutex (lifecycleMutex);
			return;
		}
	}
	UnlockMutex (lifecycleMutex);
	log_add (log_Fatal, "Thread Lifecycle array filled.  Make LIFECYCLE_SIZE larger than %d.", LIFECYCLE_SIZE);
	exit (EXIT_FAILURE);
}

void
ProcessThreadLifecycles (void)
{
	int i;

	if (!lifecycleMutex)
		return;

	LockMutex (lifecycleMutex);
	for (i = 0; i < LIFECYCLE_SIZE; i++)
	{
		Thread t = pendingDeath[i];
		if (t != NULL)
		{
			WaitThread (t, NULL);
			pendingDeath[i] = NULL;
			DestroyThread (t);
		}
	}
	UnlockMutex (lifecycleMutex);
}

void
DestroyThread (Thread t)
{
	if (t)
		HFree (t);
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
	/* RustFfiMutex supports recursive locking with owner tracking and depth counting.
	 * This comment describes the recursive-mutex path only; plain Mutex semantics
	 * remain governed by the separate audit blocker in the specification. */
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
	return rust_mutex_depth((RustMutex*)m);
}

ThreadLocal *
CreateThreadLocal (void)
{
	return (ThreadLocal *) rust_thread_local_create();
}

void
DestroyThreadLocal (ThreadLocal *tl)
{
	rust_thread_local_destroy((void *)tl);
}

ThreadLocal *
GetMyThreadLocal (void)
{
	return (ThreadLocal *) rust_get_my_thread_local();
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
