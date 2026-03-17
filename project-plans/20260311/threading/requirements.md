# Threading Subsystem Requirements

## Purpose

This document defines the required externally observable behavior of the threading subsystem in EARS format. The requirements preserve the public `threadlib.h` contract and the integration behavior relied upon by existing engine subsystems while remaining implementation-agnostic except where ABI or integration compatibility requires specific visible behavior.

## Terminology

- **Joinable thread** — a thread created via `CreateThread` that returns a handle the caller can later pass to `WaitThread`.
- **Non-joinable (fire-and-forget) thread** — a thread created via `StartThread` that does not return a joinable handle to the caller. The thread is still tracked for lifecycle cleanup.
- **Handle** — (in the thread context) the C-side wrapper struct returned by `CreateThread`. This is the object passed to `WaitThread` and `DestroyThread`.

## Thread system lifecycle

### Initialization and shutdown

- **Ubiquitous:** The threading subsystem shall preserve the existing public C API surface and semantics exposed through `threadlib.h`.
- **Ubiquitous:** The threading subsystem shall remain opaque to existing callers, such that graphics, sound, task, callback, logging, and other engine code continue to use the established threadlib entry points without source-level API changes.
- **When** `InitThreadSystem` is called before the thread system is initialized, **the threading subsystem shall** initialize process-wide threading state and make thread-local storage available for the calling thread.
- **When** `InitThreadSystem` is called after the thread system is already initialized, **the threading subsystem shall** avoid reinitializing internal state or duplicating process-wide resources.
- **When** `UnInitThreadSystem` is called, **the threading subsystem shall** release process-wide threading resources. All joinable threads must have been joined via `WaitThread` and all non-joinable threads must have completed before this call. `UnInitThreadSystem` does not join threads.
- **Ubiquitous:** The threading subsystem shall permit synchronization primitives and thread handles created through the public API to be destroyed during orderly shutdown without requiring callers to know implementation-specific ownership details.

### Shutdown ordering

- **When** `UnInitThreadSystem` is called, **the threading subsystem shall** require that all joinable threads have been joined via `WaitThread` before shutdown completes. Lifecycle cleanup does not substitute for explicit joins on caller-owned joinable handles. `UnInitThreadSystem` does not perform joins.
- **Ubiquitous:** Non-joinable threads shall not outlive thread-system shutdown. The engine must ensure all non-joinable threads have completed before shutdown begins. Lifecycle cleanup reclaims their internal handles.
- **Ubiquitous:** No thread shall call any threadlib function after `UnInitThreadSystem` has completed. Behavior of any threading API call after shutdown is undefined.
- **Ubiquitous:** Behavior of threadlib calls made by other threads is undefined from the point `UnInitThreadSystem` begins executing. The engine must quiesce all cross-thread use of threading APIs before calling `UnInitThreadSystem`; the subsystem does not guarantee safe behavior for concurrent callers during teardown.
- **When** `UnInitThreadSystem` destroys main-thread thread-local storage, **the threading subsystem shall** do so only after all worker-thread cleanup has completed.

### Shutdown preconditions

- **Ubiquitous:** Correctness of shutdown requires that all worker threads created through the public API have exited their entry wrappers before `UnInitThreadSystem` is called.
- **Ubiquitous:** No queued cross-thread signal (e.g., a draw command referencing a flush semaphore) shall target thread-local storage that is being torn down. Engine shutdown sequencing is responsible for quiescing such activity before teardown.

### Lifecycle processing obligations

- **When** a thread created through the public API reaches the end of its entry function, **the threading subsystem shall** make that thread eligible for lifecycle cleanup through the existing cleanup path used by the engine.
- **When** lifecycle cleanup is processed for a non-joinable thread, **the threading subsystem shall** reclaim the internal resources associated with that thread without requiring a caller-held handle.
- **Ubiquitous:** Lifecycle cleanup shall not free, invalidate, or reclaim C-side wrapper structs for caller-owned joinable threads. Joinable thread wrappers are exclusively caller-owned from `CreateThread` through `WaitThread` + `DestroyThread`.
- **Ubiquitous:** The threading subsystem shall preserve the engine-visible distinction between joinable thread creation and non-joinable thread start operations.

## Thread creation, execution, and join semantics

### Thread creation

- **When** `CreateThread` is called with valid arguments, **the threading subsystem shall** create a joinable worker thread that begins executing the supplied entry function with the supplied user data.
- **When** `CreateThread` cannot create the requested thread, **the threading subsystem shall** return the existing public failure result for thread creation (`NULL`/`0`) without requiring callers to handle implementation-specific exceptions.
- **When** `StartThread` is called with valid arguments, **the threading subsystem shall** create a non-joinable worker thread that begins executing the supplied entry function with the supplied user data without returning a joinable handle to the caller.
- **When** `StartThread` cannot create the requested thread, **the threading subsystem shall** satisfy the following minimal end-state contract: no joinable handle is returned, no language-native exception (panic, C++ exception, or equivalent) is exposed to the caller, and no internal resources allocated for the creation attempt are leaked. Failed detached creation shall leave no lifecycle-visible worker state behind: `FinishThread` shall not become reachable, no completed-thread registration shall be created for the failed attempt, and any adapter-owned wrapper allocated for the failed attempt shall be reclaimed before `StartThread` returns. The implementation may log the failure or report it through a diagnostic side channel; such ancillary reporting is optional. *Note (verification required): the exact legacy caller-visible failure reporting for detached thread creation is not fully established in the current evidence set. If a future audit proves that legacy callers depend on a specific observable failure indicator beyond the guarantees above, the contract must be strengthened to match.*
- **Ubiquitous:** The threading subsystem should preserve provided thread names on a best-effort basis where the platform and runtime support setting thread names as an operational/diagnostic compatibility obligation. If the platform does not support thread naming or the name exceeds platform limits, the subsystem shall proceed without error. Before treating thread names as unconditionally ignorable, an audit must confirm that no diagnostic, profiling, or crash-analysis workflow depends on thread names being preserved. Until that audit is complete, best-effort preservation remains the expected implementation behavior.
- **Ubiquitous:** The threading subsystem shall treat stack-size parameters as optional metadata unless an audit of call sites identifies a call path that depends on non-default stack sizing. If such a dependency is found, stack-size handling must be revisited. This is the canonical statement of that decision; other requirements that reference stack size defer to this one.
- **Ubiquitous:** The threading subsystem shall support use from the engine main thread and from worker threads.

#### Deferred-creation compatibility

- **Ubiquitous:** The legacy thread creation model deferred actual native thread creation to the main thread. If the threading subsystem changes this to immediate creation, it shall do so only after verifying that no call site depends on threads being created from the main thread. This verification is a prerequisite, not an assumption.

### Thread completion and result propagation

- **When** a thread entry function returns an integer status, **the threading subsystem shall** preserve that status until the corresponding join operation consumes it.
- **When** `WaitThread` is called with a valid joinable thread handle and a non-null status pointer, **the threading subsystem shall** block until the target thread has terminated and shall store the thread entry function's integer return value in the caller-provided status location.
- **When** `WaitThread` is called with a valid joinable thread handle and a null status pointer, **the threading subsystem shall** wait for thread termination without requiring status retrieval.
- **When** `WaitThread` is called on a thread that cannot produce a normal completion result, **the threading subsystem shall** write `0` to the caller-provided status location (if non-null) and shall not expose implementation-specific panic or exception details through the public API.
- **Ubiquitous:** The public join API cannot distinguish a normal thread exit with status `0` from a join failure. This is a legacy limitation that the threading subsystem need not resolve. The adapter ABI uses a two-value return convention (success/failure result plus out-status parameter) so that the adapter can reliably detect join failure at the FFI boundary. The adapter maps both outcomes to the single `*status` value at the public API boundary, preserving the legacy limitation.
- **When** a joinable thread has been successfully joined, **the threading subsystem shall** make the consumed join capability no longer usable for a second successful join. The C-side wrapper handle shall remain allocated until explicitly freed by `DestroyThread`.

### Thread handle ownership and state model

- **Ubiquitous:** Joinable thread handles follow the caller-destroy model. The handle returned by `CreateThread` is exclusively caller-owned through all state transitions.

- **Ubiquitous:** A joinable thread handle shall progress through exactly three states in order:
  1. **Active** — created by `CreateThread`; legal operations: `WaitThread`.
  2. **Joined** (or **Join-failed**) — `WaitThread` has been called (whether it succeeded or failed); the internal join capability is consumed; legal operations: `DestroyThread` only.
  3. **Destroyed** — `DestroyThread` has been called; the wrapper is freed; no further use is legal.

- **When** `WaitThread` fails (returns the failure indicator), **the threading subsystem shall** follow the approved end-state contract choice for the Rust-backed subsystem: the handle is treated as having consumed the join capability, only `DestroyThread` remains legal, and a second `WaitThread` is undefined behavior. *Note: This is an explicit forward contract choice for the end-state subsystem, not a claim that the historical legacy implementation proved the same post-failure state transition.* **Rationale:** preserving retryability across the ABI boundary would require stronger ownership and state guarantees than this subsystem intends to provide after a failed join attempt. The design therefore chooses a destroy-only post-failure state so that exactly one join attempt is permitted, exactly one destroy call reclaims the wrapper, and no implementation is required to preserve retry capability across the FFI boundary.

- **Ubiquitous:** `ProcessThreadLifecycles` shall not participate in any joinable-handle state transition. It shall not reclaim, free, or invalidate a caller-owned joinable thread handle.

- **When** `DestroyThread` is called on a joinable thread handle that has already been through `WaitThread` (whether `WaitThread` succeeded or failed), **the threading subsystem shall** free the C-side wrapper without requiring callers to understand internal allocation strategies.
- **Ubiquitous:** `DestroyThread` is not safe to call on a joinable thread that has not been through `WaitThread`. Behavior in this case is undefined.
- **Ubiquitous:** Non-joinable threads do not expose a handle to the caller. Their internal resources are reclaimed by lifecycle cleanup after the thread completes.

### Non-joinable thread lifecycle

- **When** a non-joinable thread completes, **the threading subsystem shall** make it eligible for lifecycle cleanup without requiring a caller-held join handle.
- **Ubiquitous:** The threading subsystem shall not leak internal thread objects for non-joinable threads. Any implementation-internal handle that is not returned to the caller must be disposed of or detached without resource leakage.

## Synchronization semantics

### General synchronization obligations

- **Ubiquitous:** The threading subsystem shall provide blocking behavior, wakeup behavior, and ownership behavior that are safe for concurrent use by multiple OS threads.
- **Ubiquitous:** The threading subsystem shall not require engine callers to know whether a primitive is backed by SDL, pthreads, Rust, or any other internal mechanism.
- **Ubiquitous:** The threading subsystem shall tolerate use by threads not originally spawned through the public thread creation API whenever the legacy engine already relies on such use.

### Mutexes and recursive mutexes

- **When** `CreateMutex` is called successfully, **the threading subsystem shall** return a mutex handle that can be used to serialize access to shared state.
- **When** `CreateRecursiveMutex` is called successfully, **the threading subsystem shall** return a mutex handle that supports repeated locking by its current owning thread.
- **When** a thread locks an unlocked mutex, **the threading subsystem shall** grant ownership to that thread.
- **When** a thread attempts to lock a mutex that is currently owned by another thread, **the threading subsystem shall** block the caller until ownership becomes available or until a non-blocking API explicitly reports failure.
- **When** the owning thread re-locks a recursive mutex, **the threading subsystem shall** succeed without deadlocking and shall increment the observable ownership depth.
- **When** the owning thread unlocks a recursive mutex whose ownership depth is greater than one, **the threading subsystem shall** decrement the observable ownership depth without releasing ownership to another thread.
- **When** the owning thread unlocks a recursive mutex whose ownership depth becomes zero, **the threading subsystem shall** release ownership and make the mutex available to other waiting threads.
- **When** a thread that does not own a mutex attempts to unlock it, **the threading subsystem shall** fail safely without corrupting mutex ownership state.
- **Ubiquitous:** The threading subsystem shall provide `GetRecursiveMutexDepth` semantics that accurately report the current recursive lock depth observable through the public API.
- **Ubiquitous:** The threading subsystem shall preserve the engine-visible behavior relied upon by code that saves recursive lock depth, fully unlocks, waits, and then reacquires to the same depth.

#### Plain mutex recursion policy — unresolved blocker

- **Ubiquitous:** The end-state contract for plain `Mutex` recursion semantics is an **unresolved decision that blocks final signoff**. The decision shall be determined by a mandatory call-site audit:
  - **If** the audit finds any call site that depends on non-recursive semantics (same-thread re-lock producing deadlock or error as a bug-detection mechanism), **then** `Mutex` shall remain non-recursive and same-thread re-lock shall deadlock or return an error.
  - **If** the audit finds no such dependency, **then** a unified recursive implementation backing both `Mutex` and `RecursiveMutex` is accepted as a compatibility-preserving simplification. The project explicitly accepts the tradeoff that accidental same-thread re-lock bugs will no longer be caught at runtime.
- **Ubiquitous:** Until the audit is complete, any currently observed implementation behavior for plain-`Mutex` same-thread re-lock is provisional only and shall not be cited as the contract in downstream design, test, or review artifacts. The audit outcome is required before any compatibility claim is made for this behavior.
- **Ubiquitous:** If the implementation uses a single opaque handle type or a single backing implementation for both plain and recursive mutexes at the adapter ABI layer, that internal convenience shall not be treated as settling the public recursion semantics of plain `Mutex`. The adapter ABI shape is compatible with both audit outcomes and does not constrain the public contract.

### Semaphores

- **When** `CreateSemaphore` is called with an initial count, **the threading subsystem shall** create a counting semaphore whose initial number of available permits equals that count.
- **When** `SetSemaphore` is called and the semaphore has at least one available permit, **the threading subsystem shall** consume one permit and return without indefinite blocking.
- **When** `SetSemaphore` is called and the semaphore has no available permits, **the threading subsystem shall** block until a permit becomes available.
- **When** `ClearSemaphore` is called, **the threading subsystem shall** add one permit to the semaphore and make at least one blocked waiter eligible to resume.
- **When** a non-blocking semaphore acquisition API is used, **the threading subsystem shall** report success only if a permit was actually consumed.
- **Ubiquitous:** The threading subsystem shall permit a semaphore created in one thread to be waited in that thread and signaled from another thread.
- **Ubiquitous:** The threading subsystem shall preserve semaphore behavior required for graphics flush signaling and other cross-thread handoff paths.

### Condition variables

- **When** `CreateCondVar` is called successfully, **the threading subsystem shall** return a condition variable handle that supports wait, signal, and broadcast operations through the public API.
- **When** `WaitCondVar` is called, **the threading subsystem shall** block the caller awaiting a wake event from a matching `SignalCondVar` or `BroadcastCondVar` on the same condvar. Callers must use predicate loops to guard against spurious or missed wakeups; the public contract does not guarantee that every signal or broadcast will wake a specific waiter, nor that a signal issued with no waiter present will be remembered.
- **When** `SignalCondVar` is called and one or more threads are waiting on that condition variable, **the threading subsystem shall** make at most one waiter eligible to resume.
- **When** `BroadcastCondVar` is called and one or more threads are waiting on that condition variable, **the threading subsystem shall** make all current waiters eligible to resume.
- **Ubiquitous:** The threading subsystem shall preserve the legacy threadlib condition-variable contract in which callers do not supply an external mutex to `WaitCondVar`.
- **Ubiquitous:** The threading subsystem shall provide condvar semantics compatible with existing call sites that release caller-managed recursive mutex state before waiting and restore that state after wakeup.

#### Signal buffering (permitted strengthening — not public contract)

- **Ubiquitous:** The public `threadlib.h` contract does not guarantee signal buffering. Callers must tolerate lost notifications — a `SignalCondVar` call made when no thread is waiting may have no effect on a future `WaitCondVar` call. Callers must use predicate loops to guard against missed and spurious wakeups.
- **Ubiquitous:** The implementation may provide signal buffering (where `SignalCondVar` with no waiter causes the next `WaitCondVar` to return immediately) as a permitted strengthening. This behavior is tolerated as an implementation artifact, not required or recommended by the public contract. Callers and compatibility analysis shall treat both remembered-signal and lost-signal behavior as valid and shall not rely on signal-buffering behavior. No currently identified call site has been proven to depend on buffered signals.
- **Ubiquitous:** The return value of timed condvar waits is advisory only. Callers shall use predicate loops and shall not rely on a precise causal interpretation of a non-timeout return; the public contract does not require distinguishing remembered signals, current-waiter signals, broadcasts, or other implementation-internal wake bookkeeping as separate success causes.


### Fairness

- **Ubiquitous:** No blocking primitive (mutex, semaphore, condvar) is required to provide strict FIFO ordering. No stronger fairness guarantee than the legacy backend provides is required for any primitive type.

## Thread-local storage

### TLS object behavior

- **When** `CreateThreadLocal` is called for a thread that does not already have a thread-local object through the public API, **the threading subsystem shall** create a thread-local object containing a flush semaphore initialized to the blocked state expected by existing engine code.
- **When** `CreateThreadLocal` is called for a thread that already has a valid thread-local object through the public API, **the threading subsystem shall** preserve a single usable thread-local object for that thread rather than exposing conflicting duplicates.
- **When** `GetMyThreadLocal` is called by a thread with an established public thread-local object, **the threading subsystem shall** return that calling thread's thread-local object.
- **When** `GetMyThreadLocal` is called by a thread with no established public thread-local object, **the threading subsystem shall** return the existing no-object result expected by callers.
- **When** `DestroyThreadLocal` is called on a valid thread-local object, **the threading subsystem shall** destroy the flush semaphore associated with that object, reclaim the object, and clear the calling thread's registered TLS slot.
- **Ubiquitous:** The thread-local object layout exposed to C shall remain ABI-compatible with existing engine code that accesses the flush semaphore field.

### TLS ownership and lifetime

- **Ubiquitous:** Each thread shall have at most one authoritative thread-local object. That object is owned by the thread it belongs to.
- **Ubiquitous:** Only the owning thread should destroy its own thread-local object. Destroying another thread's TLS from a different thread is not a supported operation.
- **Ubiquitous:** Pointer-level double-destroy of a thread-local object is not defined as safe. Callers must not destroy the same thread-local object twice.
- **When** automatic TLS cleanup runs for a thread whose TLS slot has already been cleared by prior valid destruction of that thread's TLS object, **the threading subsystem shall** tolerate the already-cleared slot state without error. This slot-cleared tolerance does not require object-level double-free safety.
- **Ubiquitous:** The `flushSem` within a thread-local object may be signaled from threads other than the owner. This cross-thread use is valid only while the owning thread's TLS is live. Callers that signal a `flushSem` must ensure the owning thread has not yet destroyed its TLS.

### Automatic TLS availability

- **When** a thread is created through the public thread creation API, **the threading subsystem shall** make a valid thread-local object available to that thread for the duration of its execution.
- **When** a thread created through the public thread creation API terminates, **the threading subsystem shall** clean up any automatically managed thread-local object associated with that thread.
- **Ubiquitous:** The threading subsystem shall preserve `GetMyThreadLocal()->flushSem` behavior required by graphics synchronization paths.

## Yield and sleep behavior

### Cooperative yield

- **When** `TaskSwitch` is called, **the threading subsystem shall** yield execution in a manner that gives other runnable work an opportunity to proceed.
- **Ubiquitous:** The externally observable behavior of `TaskSwitch` shall remain compatible with existing engine polling loops that expect a short cooperative delay rather than a busy spin.

### Sleep and hibernation

- **When** `HibernateThread` is called with a positive duration, **the threading subsystem shall** block the calling thread for approximately that duration, subject to scheduler granularity.
- **When** `HibernateThread` is called with a zero duration, **the threading subsystem shall** return promptly without requiring callers to special-case the request.
- **When** `HibernateThreadUntil` is called with a wake time in the future, **the threading subsystem shall** block until the wake time is reached or passed.
- **When** `HibernateThreadUntil` is called with a wake time that is already reached or passed, **the threading subsystem shall** return promptly.
- **When** `SleepThread` is called, **the threading subsystem shall** preserve the public behavior of sleeping relative to the current engine time base rather than requiring callers to convert to implementation-native time units.

### Async pumping during `SleepThreadUntil`

- **While** `SleepThreadUntil` is waiting for a future wake time, **the threading subsystem shall** preserve the legacy behavior of servicing pending asynchronous engine work required by the main-thread sleep path before and between blocking intervals.
- **While** `SleepThreadUntil` is waiting and asynchronous work is scheduled earlier than the final requested wake time, **the threading subsystem shall** wake in time to service that asynchronous work before continuing to wait.
- **Ubiquitous:** The externally visible behavior of `SleepThreadUntil` shall remain compatible with the legacy engine contract in which sleeping on the main thread also advances asynchronous processing obligations.

## Destruction preconditions

### Primitive lifetime

- **When** a mutex, recursive mutex, semaphore, condvar, or thread-local object is destroyed through the public API, **the threading subsystem shall** reclaim the implementation object associated with that handle without requiring callers to free internal allocations directly.
- **When** a destroy operation is invoked on a null or otherwise absent handle through a public path that historically tolerated it, **the threading subsystem shall** fail safely without process termination.
- **Ubiquitous:** The threading subsystem shall not require callers to pair public API cleanup with implementation-specific cleanup calls.

### In-use destruction

- **Ubiquitous:** Callers must not destroy a mutex while another thread owns it or is blocked waiting to acquire it. Behavior in this case is undefined.
- **Ubiquitous:** Callers must not destroy a condition variable while any thread is blocked in `WaitCondVar` on it. Behavior in this case is undefined.
- **Ubiquitous:** Callers must not destroy a semaphore while any thread is blocked in `SetSemaphore` on it. Behavior in this case is undefined.
- **Ubiquitous:** Callers must not destroy a joinable thread handle before `WaitThread` has been called on it (whether `WaitThread` succeeded or failed). See the thread handle ownership and state model section for the complete handle-state model. Behavior in this case is undefined.
- **Ubiquitous:** Non-joinable thread handles are not caller-visible. Their reclamation is handled by lifecycle cleanup.

### Cross-boundary ownership

- **Ubiquitous:** The threading subsystem shall maintain a single, unambiguous ownership model for thread handles and synchronization handles across the C-to-implementation boundary.
- **When** an object handle is passed through the public C ABI, **the threading subsystem shall** keep that handle valid for the lifetime implied by the public API regardless of the internal implementation language.
- **Ubiquitous:** The threading subsystem shall not expose internal exceptions, panics, poison states, or allocator details as new externally visible API requirements.

## Integration obligations

### Graphics integration

- **When** graphics code uses a thread-local flush semaphore to request render-thread acknowledgement, **the threading subsystem shall** support blocking in the requesting thread and signaling from the render thread using the same semaphore handle.
- **When** graphics queue management code saves recursive mutex depth, fully unlocks, waits on a condvar, and reacquires to the same depth, **the threading subsystem shall** preserve correct blocking and re-lock behavior for that sequence.
- **When** graphics code broadcasts queue-state changes, **the threading subsystem shall** make all relevant waiting threads eligible to resume.

### Audio integration

- **When** audio mixer code uses recursive mutexes from callback and non-callback threads, **the threading subsystem shall** preserve safe ownership tracking and nested lock behavior needed by those code paths.
- **When** audio streaming code uses `HibernateThread` and `TaskSwitch` to throttle worker activity, **the threading subsystem shall** preserve sufficiently similar scheduling behavior for those loops to function correctly.

### Task, callback, and logging integration

- **When** task-management code creates worker threads and later joins them, **the threading subsystem shall** preserve joinability and result propagation required by that task-management layer. Stack-size handling for task threads defers to the canonical rule in the thread creation section.
- **When** callback processing code uses plain mutex locking to serialize access to callback state, **the threading subsystem shall** preserve ordinary mutex behavior suitable for that serialization.
- **When** logging code uses a mutex created through the public API, **the threading subsystem shall** preserve ordinary mutex behavior suitable for multi-threaded logging.

## ABI and externally visible compatibility

- **Ubiquitous:** Public handle types exposed through the C ABI shall remain layout-compatible and usage-compatible with existing engine code.
- **Ubiquitous:** The C-visible thread-local structure shall expose the flush semaphore at the field location expected by existing compiled C code.
- **Ubiquitous:** Time-based public APIs shall preserve the existing engine-facing time-unit behavior even if the underlying implementation uses different native time units internally.
- **Ubiquitous:** Public wait, signal, join, yield, and sleep operations shall preserve engine-observable semantics required by existing call sites even if internal implementation strategies differ.
