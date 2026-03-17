# Rust Threading Subsystem — Functional & Technical Specification

This document specifies the desired end-state behavior of the Rust threading subsystem at functional parity with the legacy C implementation. It covers functional semantics, technical structure, adapter ABI contracts, and integration points. Where behavior intentionally diverges from legacy, the divergence is identified as an approved behavioral change with its evidence basis stated.

Sections marked **[Normative]** define required observable behavior at the public API boundary (`threadlib.h`). Sections marked **[Adapter ABI]** define the C↔Rust function-signature and layout contract that the C adapter must match; these are stable but not public-API-visible. Sections marked **[Reference design]** describe one acceptable internal implementation approach and are not binding on alternative designs that satisfy the normative and adapter ABI requirements.

### Open audits (spec finalization only — does not affect downstream normative status)

The following decisions require call-site audits before this specification is marked as **finalized**:

1. **Plain-mutex recursion policy** (§3.1) — requires a call-site audit to determine whether plain `Mutex` must remain non-recursive or may share a recursive backing with `RecursiveMutex`.
2. **Stack-size handling** (§2.1) — requires a call-site audit to determine whether the `stackSize` parameter may be ignored.
3. **Deferred-creation compatibility** (§2.1) — requires a call-site audit to confirm that immediate thread creation is safe.

These audits block marking this spec as finalized. They do **not** block downstream cross-subsystem verification. The interim acceptance rules below are normative for downstream pass/fail today — they are not provisional, temporary, or advisory. Downstream subsystem specs (audio-heart, graphics, etc.) that depend on threading semantics shall verify against these interim rules exactly as though they were final. If an audit outcome later changes a rule, this specification will be revised and affected downstream specs will be updated; until such revision, the rules below are the settled contract.

**Interim acceptance rules for downstream consumers:** Until the above audits are completed, the following interim rules apply so that downstream subsystem specs (audio-heart, graphics, etc.) can normatively depend on threading behavior without waiting for final signoff:

- **Plain-mutex recursion (interim):** Downstream consumers shall treat plain `Mutex` as non-recursive. Code that re-locks a plain mutex on the same thread is a consumer bug regardless of the audit outcome. If the audit later reveals that existing call sites require recursive plain-mutex behavior, the threading spec will be amended and affected downstream specs will be notified.
- **Stack size (interim):** Downstream consumers shall not depend on specific stack sizes being honored. If a consumer requires non-default stack sizing, it shall document that dependency explicitly and flag it as blocked on the threading audit.
- **Deferred creation (interim):** Downstream consumers shall assume immediate thread creation (the current Rust behavior). If the audit reveals a call site that depends on main-thread deferred creation, the threading spec will be amended.

These interim rules are the controlling cross-subsystem contract for all downstream verification until explicitly amended by a revision to this specification. Downstream subsystem specs (audio-heart, graphics, etc.) that depend on threading semantics shall verify against these interim rules — not against an unspecified future post-audit outcome. If the audit outcome changes a rule, this specification will be revised and affected downstream specs will be updated. Until such revision, the interim rules are normative for cross-subsystem pass/fail determination.

---

## 1. Scope

The Rust threading subsystem replaces the native SDL/pthread backend and the `thrcommon.c` common layer. When `USE_RUST_THREADS` is defined, all synchronization primitives, thread lifecycle management, thread-local storage, and cooperative scheduling helpers are provided by Rust code linked as a static library, with a thin C adapter (`rust_thrcommon.c`) mapping the public `threadlib.h` API onto Rust FFI exports.

The public C API surface (`threadlib.h`) does not change. All existing call sites — graphics, audio, tasks, callbacks, logging — continue to call the same `threadlib.h` functions. The Rust subsystem is invisible to callers.

### 1.1 Terminology

The following terms are used consistently throughout this document:

- **Joinable thread** — a thread created via `CreateThread` that returns a handle the caller can later pass to `WaitThread`.
- **Non-joinable (fire-and-forget) thread** — a thread created via `StartThread` that does not return a joinable handle to the caller. The thread is still tracked for lifecycle cleanup.
- **Handle** — (in the thread context) the C-side `TrueThread` wrapper struct. This is the object passed to `WaitThread` and `DestroyThread`.

### 1.2 Document layers

This specification describes three distinct layers. Readers should be aware of which layer a given section addresses:

1. **Public contract** — the observable behavior visible through `threadlib.h`. This is the authoritative, implementation-agnostic behavioral contract. Marked **[Normative]**.
2. **Adapter ABI** — the C↔Rust FFI function signatures, opaque-pointer conventions, and layout guarantees that the C adapter (`rust_thrcommon.c`) must conform to. Marked **[Adapter ABI]**. The adapter ABI is a stable internal contract but is not visible to engine callers.
3. **Reference implementation** — Rust internal types, algorithms, and design patterns that satisfy the above contracts. Marked **[Reference design]**. Alternative implementations that preserve normative and adapter ABI behavior are acceptable.

---

## 2. Thread Lifecycle

### 2.1 Thread creation

Two creation paths exist in the public API:

| C function | Semantics |
|---|---|
| `CreateThread(func, data, stackSize, name)` | Spawns a joinable thread and returns a `Thread` handle. The caller can later call `WaitThread` on the handle. |
| `StartThread(func, data, stackSize, name)` | Spawns a non-joinable thread. The thread is still tracked for lifecycle cleanup. |

#### Deferred-creation compatibility decision

**[Normative]** The legacy `thrcommon.c` stages spawn requests in a `pendingBirth` array and performs actual native thread creation on the main thread inside `ProcessThreadLifecycles()`. `CreateThread` blocks the caller on a semaphore until the main thread has created the native thread. `StartThread` does not block.

The Rust subsystem spawns threads immediately rather than deferring to the main thread. This is classified as an **approved behavioral change**. Per the top-level interim acceptance rules, immediate thread creation is **normative for downstream cross-subsystem verification today**. Downstream specs (audio-heart, graphics, etc.) shall treat immediate creation as the settled contract for pass/fail purposes.

The following verification obligation remains for spec finalization (not for downstream pass/fail):

- Before this spec is marked as finalized, an explicit audit of all `CreateThread` and `StartThread` call sites must confirm that no caller depends on thread creation occurring on the main thread. The `threadlib.h` main-thread deadlock caveat (`:82-85`) must be evaluated for each call path.
- If any call site is found to depend on main-thread creation semantics, this decision must be revisited and either the Rust/C adapter must preserve deferred semantics for that path or the call site must be modified.
- Until the audit is complete, this spec cannot be marked as finalized. However, immediate creation is the controlling downstream contract now and will remain so unless the audit outcome explicitly requires revision.

#### Stack size handling

**[Normative]** The `stackSize` parameter is accepted by `CreateThread` and `StartThread`. Whether the implementation honors, ignores, or adapts this value is an open design decision pending a call-site audit. This is the canonical statement of that decision; other sections that reference stack size defer to this one.

**Verification obligation:** before treating `stackSize` as ignorable, an audit of all call sites must confirm that no call path passes a meaningful non-default stack size that the thread depends on. If any call site is found to require non-default stack sizing, the implementation must either honor the parameter or document why the specific call site is safe without it.

#### Thread naming

**[Normative]** The `name` parameter is accepted by `CreateThread` and `StartThread`. Thread names are observable through platform debugging tools (debuggers, crash reporters, profiling tools, `ps`/`top` on some platforms) and may be relied upon for diagnostic and operational workflows.

The implementation shall preserve provided thread names on a best-effort basis where the platform and runtime support it. Specifically:

- If the platform's thread API supports setting a thread name (e.g., `pthread_setname_np`, `std::thread::Builder::name`), the implementation should pass the provided name through.
- If the name cannot be set (e.g., platform does not support it, name exceeds platform length limits), the implementation shall proceed without error.

**Verification obligation:** before treating thread names as unconditionally ignorable, an audit must confirm that no diagnostic, profiling, or crash-analysis workflow in the project's supported configurations depends on thread names being preserved. Until that audit is complete, the implementation must make a best-effort attempt to preserve names.

#### Thread wrapper

**[Reference design]** Each spawned thread executes inside a `ThreadLocalGuard` scope that:

1. Initializes the Rust-side `thread_local!` TLS (`ThreadLocal` with `flush_sem`).
2. Initializes the FFI-side `FfiThreadLocal` struct (heap-allocated, containing a `flush_sem` pointer).
3. On thread exit (guard drop), destroys the FFI TLS and clears the Rust TLS.

The C adapter wraps the caller's `ThreadFunction` in a `RustThreadHelper` that:

1. Extracts `func`, `data`, and `thread` from a `ThreadStartInfo`.
2. Calls `func(data)` and captures the integer return value.
3. Calls `FinishThread(thread)` to register the thread handle for deferred cleanup.

### 2.2 Thread join — `WaitThread` contract

**[Normative]** `WaitThread(thread, &status)` joins the thread and writes its result.

The full observable contract:

- **On successful join with non-null `status`:** the thread entry function's integer return value is written to `*status`.
- **On successful join with null `status`:** the join completes without writing a result.
- **On join failure with non-null `status`:** `0` is written to `*status`.
- **Distinguishability limitation:** the public API cannot distinguish a normal thread exit with status `0` from a join failure. This matches the legacy API and is accepted as a legacy limitation. The adapter ABI (§10.2) uses a two-value return convention (success/failure result plus separate out-status) to give the adapter enough information to populate `WaitThread` behavior correctly, but the adapter maps both cases to the single `*status` value at the public API boundary, preserving the legacy limitation.
- **Handle consumption on success:** after a successful `WaitThread`, the internal join capability is consumed and must not be used for a second join. The C-side handle (wrapper struct) remains allocated; see §2.6 for the complete handle-state model.
- **Handle state after failure (approved end-state contract choice):** if `WaitThread` fails (returns the failure indicator), the end-state contract for the Rust-backed subsystem is that the handle is thereafter treated as no longer joinable. The only legal subsequent operation is `DestroyThread`; a second `WaitThread` call is undefined behavior. This rule is an explicit forward contract choice to keep ownership unambiguous when the join boundary reports failure. It is not presented as proven historical legacy behavior. **Rationale:** preserving retryability across the ABI boundary would require stronger ownership and state guarantees than this subsystem intends to provide after a failed join attempt. The design therefore chooses a destroy-only post-failure state so that exactly one join attempt is permitted, exactly one destroy call reclaims the wrapper, and no implementation is required to preserve retry capability across the FFI boundary. The current Rust `JoinHandle` behavior is one concrete example of why this simpler ownership model is attractive, not the sole reason for the rule.


### 2.3 Thread destruction

**[Normative]** `DestroyThread(thread)` frees the C-side `TrueThread` wrapper.

The caller-destroy model applies:

- **Joinable threads:** the caller owns the handle returned by `CreateThread`. `DestroyThread` is safe to call only after `WaitThread` has consumed the join capability. Calling `DestroyThread` on a joinable thread that has not been joined via `WaitThread` is not a supported operation, regardless of whether the thread has finished executing.
- **Non-joinable threads:** the caller does not hold a join handle. The C-side `TrueThread` wrapper created internally by `StartThread` is reclaimed by `ProcessThreadLifecycles` after the thread completes and calls `FinishThread`. No caller action is needed.

**[Reference design]** On the Rust side, `DestroyThread` is a no-op — the Rust thread object is consumed by `join()` and its memory is freed on drop.

### 2.4 Lifecycle cleanup (`FinishThread` / `ProcessThreadLifecycles`)

**[Normative]** Threads that complete register themselves for cleanup via `FinishThread`, which enqueues the thread handle into a fixed-size `pendingDeath` array under `lifecycleMutex`. The main thread calls `ProcessThreadLifecycles()` periodically to process completed threads in the `pendingDeath` queue.

Lifecycle cleanup serves the following purpose:

- **Non-joinable threads:** `ProcessThreadLifecycles` reclaims the C-side `TrueThread` wrapper for threads started via `StartThread`. This is the primary resource-reclamation path for non-joinable threads, since no caller holds a handle to join or destroy.

`ProcessThreadLifecycles` **must not** free or invalidate C-side wrappers for caller-owned joinable threads. Joinable thread wrappers are exclusively caller-owned from `CreateThread` through `WaitThread` + `DestroyThread`. See §2.6 for the complete handle-state model.

**[Reference design]** This lifecycle bookkeeping remains C-owned in `rust_thrcommon.c`. The Rust subsystem does not need to replicate the `pendingBirth`/`pendingDeath` arrays. The Rust-side `process_thread_lifecycles()` stub may be removed or left as a no-op.

Rationale: lifecycle processing is policy, not mechanism. It depends on the main loop's call cadence and on C-owned handle memory. Keeping it in C avoids a redundant FFI round-trip and keeps handle ownership unambiguous.

### 2.5 Non-joinable threads and lifecycle participation

**[Normative]** `StartThread` creates a non-joinable thread. The caller does not receive a join handle. However, the thread still participates in lifecycle cleanup:

- `FinishThread` is called by the thread wrapper on completion, registering the thread handle in `pendingDeath`.
- `ProcessThreadLifecycles` processes the handle to reclaim resources.

The Rust-side join handle for a non-joinable thread must not leak. If the Rust join handle is not needed for lifecycle cleanup (because the C adapter tracks the handle via `pendingDeath`), the Rust handle must be detached or otherwise disposed of without leaking memory.

**[Reference design]** `StartThread_Core` should use `rust_thread_spawn_detached` (not `rust_thread_spawn`) since the caller does not receive a join handle. The current adapter incorrectly uses `rust_thread_spawn`, which leaks the Rust-side boxed handle. When `rust_thread_spawn_detached` is used, the Rust join handle is dropped immediately; TLS cleanup still occurs via `ThreadLocalGuard`. The C-side lifecycle cleanup operates on the C `TrueThread` wrapper, which is independent of the Rust join handle.

#### Detached-thread creation failure contract

**[Normative]** The minimal end-state contract for `StartThread` creation failure is:

- **No caller-visible join handle:** `StartThread` does not return a joinable handle on either success or failure; the caller has no handle to inspect or act on.
- **No language-native exception exposure:** a creation failure must not propagate a Rust panic, C++ exception, or any other language-native exception type to the caller. The failure must be contained within the subsystem boundary.
- **No leaked internal resources:** if thread creation fails, any internal resources allocated for the attempt (C-side wrapper, Rust-side handle, TLS structures) must be reclaimed before `StartThread` returns.
- **Allocation order is implementation choice:** whether the adapter allocates lifecycle bookkeeping before or after calling the Rust detached-spawn entry point is not part of the contract. To avoid requiring synchronous failure knowledge at the C boundary, one valid strategy is to allocate any adapter-owned wrapper only after detached spawn has crossed the point where failure would be reported or contained. Regardless of allocation order between adapter and Rust internals, failed detached creation must leave no surviving worker state, no lifecycle-visible registration, and no leaked wrapper, handle, or TLS object.
- **Ancillary logging or reporting:** the implementation may log the failure or report it through a diagnostic side channel. Such reporting is optional unless a future legacy verification audit proves that callers depend on stronger caller-visible failure behavior.

*Note (verification required):* the exact legacy caller-visible failure reporting for detached thread creation is not fully established in the current evidence set. If a future audit reveals that legacy callers depend on a specific observable failure indicator beyond the guarantees above (e.g., a specific return value, a flag set, or a callback invoked), the contract must be strengthened to match.

### 2.6 Handle-state model

**[Normative]** The following table defines the complete state model for thread handles. A handle is the C-side `TrueThread` wrapper struct.

#### Joinable thread handle states

| State | Entry condition | Legal operations | Illegal operations |
|---|---|---|---|
| **Active** | `CreateThread` returns handle | `WaitThread` (blocks until thread finishes) | `DestroyThread` |
| **Joined** | `WaitThread` succeeds | `DestroyThread` | Second `WaitThread` |
| **Join-failed** | `WaitThread` fails | `DestroyThread` | Second `WaitThread` |
| **Destroyed** | `DestroyThread` called (after join or join-failure) | None — handle is freed | Any use |

State transitions are strictly sequential: **Active → Joined → Destroyed** or **Active → Join-failed → Destroyed**. No transition may be skipped. Both the **Joined** and **Join-failed** states consume the join capability; the handle is no longer joinable in either case. `ProcessThreadLifecycles` does not participate in any joinable-handle state transition; it must not reclaim or invalidate a caller-owned joinable handle.

Postconditions of `WaitThread` (success):
- The internal join capability is consumed.
- The C-side wrapper struct remains allocated and caller-owned.
- The handle is valid only for `DestroyThread`; all other operations (including a second `WaitThread`) are undefined.

Postconditions of `WaitThread` (failure):
- `0` has been written to `*status` if the caller provided a non-null status pointer.
- The public contract after a failed join attempt is the approved end-state rule defined in §2.2: the handle is treated as no longer joinable and remains valid only for `DestroyThread`.
- This post-failure state is a deliberate forward contract choice for the Rust-backed subsystem, not a claim that legacy code proved the same state transition. See the rationale in §2.2 for why consuming joinability on failure is the least-risk design at the ABI boundary.

Postconditions of `DestroyThread` (on a joined or join-failed handle):
- The C-side wrapper struct is freed.
- The handle pointer is invalid. Any subsequent use is undefined behavior.

#### Non-joinable thread handle states

| State | Entry condition | Legal operations |
|---|---|---|
| **Active (internal)** | `StartThread` spawns thread | None — handle is not caller-visible |
| **Finished (pending cleanup)** | Thread calls `FinishThread` | `ProcessThreadLifecycles` reclaims wrapper |
| **Reclaimed** | `ProcessThreadLifecycles` processes handle | None — handle is freed |

Non-joinable handles are never caller-visible. The caller does not hold a reference and cannot call `WaitThread` or `DestroyThread` on them.

---

## 3. Synchronization Primitives

### 3.1 Mutex

**[Normative]** Created via `CreateMutex(name, syncClass)`. The C API provides `LockMutex`, `UnlockMutex`, `DestroyMutex`.

In the legacy code, `Mutex` and `RecursiveMutex` are distinct types with separate backends. A plain `Mutex` is non-recursive: locking it twice from the same thread deadlocks (SDL backend) or is undefined behavior (pthread backend).

#### Mutex recursion policy —  unresolved blocker

**[Normative]** The end-state contract for plain `Mutex` recursion semantics is an **unresolved decision that blocks final signoff** of this specification. The decision depends on the outcome of a mandatory call-site audit:

1. **If the audit finds any call site that depends on non-recursive semantics** — i.e., any code path that relies on same-thread re-lock producing a deadlock or error as a bug-detection mechanism — then `Mutex` must remain non-recursive. A non-recursive `Mutex` that deadlocks or errors on same-thread re-lock is the required behavior.

2. **If the audit finds no such dependency**, then a unified recursive implementation backing both `Mutex` and `RecursiveMutex` is explicitly accepted as a compatibility-preserving simplification. In this case, the project accepts the tradeoff that accidental same-thread re-lock bugs will no longer be caught at runtime. This decision and its rationale must be recorded in the project's decision log.

Until the audit is complete, downstream consumers shall treat plain `Mutex` as non-recursive (per the interim acceptance rules in the top-level section). The interim non-recursive rule is normative for downstream pass/fail today. However, the specific same-thread re-lock *implementation behavior* (deadlock vs. error vs. recursive success) must not be cited as a committed contract in downstream design, test, or review artifacts — only the interim rule that re-locking is a consumer bug applies. The audit outcome will finalize whether the re-lock behavior is enforced or merely undefined.

**[Normative — cross-layer clarification]** The adapter ABI (§10.2) currently exposes a single opaque `RustMutex*` handle type with lock/unlock/depth operations. This single-handle ABI shape is an implementation convenience, not a commitment to unified recursive behavior. It does not constrain the public recursion semantics of plain `Mutex`. If the §3.1 audit requires non-recursive plain mutexes, the adapter ABI permits several compatible responses — including internal mode flags, separate backing types behind the same opaque pointer, or distinct ABI entry points — without changing the public `threadlib.h` contract. The current ABI shape is acceptable for both audit outcomes.

**[Normative]** Regardless of the recursion policy chosen, all mutexes must:

- Block callers from other threads until the owner unlocks to depth 0.
- Track ownership so that unlock by a non-owner thread fails safely (no-op or error return, not a crash).
- Accept and store a name string for debugging (see §14 for naming policy).
- Tolerate internal implementation errors gracefully at the FFI boundary (return error, do not panic or expose implementation-specific error states through the public API).

**[Normative]** Recursive mutexes (and plain mutexes only if the unresolved §3.1 audit eventually approves the unified-recursive path) must additionally:

- Allow re-entrant locking from the owning thread (depth increments).
- Track the current lock depth (queryable via `rust_mutex_depth`).
- On unlock from depth > 1, decrement depth without releasing.
- On unlock from depth 1, release ownership and wake one blocked waiter.

### 3.2 Recursive Mutex

**[Normative]** Created via `CreateRecursiveMutex(name, syncClass)`. The C API provides `LockRecursiveMutex`, `UnlockRecursiveMutex`, `DestroyRecursiveMutex`, `GetRecursiveMutexDepth`.

Behavioral requirements are as specified in §3.1 for recursive behavior.

`GetRecursiveMutexDepth(m)` returns the current lock depth. This value is `0` when the mutex is unlocked. Call sites depend on this to save and restore lock state across condvar waits (see §3.4 and §6.1).

### 3.3 Semaphore (counting)

**[Normative]** Created via `CreateSemaphore(initial, name, syncClass)`. The C API uses `SetSemaphore` (acquire/wait/decrement) and `ClearSemaphore` (release/post/increment).

**Specified behavior:**

- **`SetSemaphore` / `acquire`:** Atomically decrements the count. If the count is 0, the calling thread blocks until another thread calls `ClearSemaphore`.
- **`ClearSemaphore` / `release`:** Atomically increments the count. If threads are blocked in `SetSemaphore`, wakes exactly one.
- **`try_acquire`:** Non-blocking. Returns success if a permit was consumed, failure otherwise.
- **`count`:** Returns the current permit count. This is a snapshot; the value may change immediately after the call.
- **Initial count:** Set at creation time. A semaphore created with `initial=0` starts fully blocking.
- **No upper bound.** The count is a `u32` and may grow without limit (within `u32` range).
- **Fairness:** No strict FIFO ordering is required and no stronger fairness contract than the legacy backend provides is guaranteed.

### 3.4 Condition Variable

**[Normative]** Created via `CreateCondVar(name, syncClass)`. The C API provides `WaitCondVar`, `SignalCondVar`, `BroadcastCondVar`.

**Critical design note:** UQM's condvar API is non-standard. `WaitCondVar(cv)` takes only a `CondVar` handle — there is no external mutex parameter. Both the SDL and pthread backends create an internal mutex per condvar and lock/unlock it around the `pthread_cond_wait` / `SDL_CondWait` call. The caller never interacts with this internal mutex.

#### Non-standard condvar contract

**[Normative]** UQM condition variables differ from standard condvar semantics in the following way:

- **Public contract on signal buffering:** The public `threadlib.h` contract does **not** guarantee signal buffering. Callers must tolerate lost notifications — that is, a `SignalCondVar` call made when no thread is waiting may have no effect on a future `WaitCondVar` call. Callers must use predicate loops to guard against missed and spurious wakeups. This is the standard condvar discipline and is the only behavior callers may rely on.

- **Signal buffering as permitted strengthening (not guaranteed public contract):** The public contract does **not** require or recommend remembered-signal behavior. Implementations are merely permitted to exhibit it as an internal artifact. Callers and compatibility analysis must treat both remembered-signal and lost-signal behavior as valid and must not depend on buffering. No currently identified call site has been proven to require buffered signals. Any current Rust pending-signal mechanism is reference-design detail, not endorsed public semantics.

**[Normative]** Specified behavior:

- **`WaitCondVar` / `wait`:** Blocks the calling thread awaiting a wake event from `SignalCondVar` or `BroadcastCondVar` on the same condvar. Callers must use predicate loops to guard against spurious or missed wakeups; the public contract does not guarantee that every signal or broadcast will wake a specific waiter, nor that a signal issued with no waiter present will be remembered.
- **`SignalCondVar` / `signal`:** Wakes at most one waiting thread, if any are waiting.
- **`BroadcastCondVar` / `broadcast`:** Wakes all currently waiting threads.
- **`wait_timeout`:** Blocks for at most the specified duration and returns an advisory non-timeout/timeout result. A `true` result means only that the wait ended before the timeout expired; a `false` result means the timeout expired before the wait completed. Callers must continue to use predicate loops; the contract does not require proving an exact wake cause or distinguishing remembered signals, current-waiter signals, broadcasts, or spurious wakeups.
- **Mutex parameter in FFI:** The `rust_condvar_wait` and `rust_condvar_wait_timeout` FFI signatures include a `mutex` parameter for potential future use, but it is currently `NULL`-passed and ignored. This matches the legacy UQM condvar design where the condvar owns its own internal mutex.

**Interaction with recursive mutexes at call sites.** The DCQ code (`dcqueue.c:55-60`) manually saves `GetRecursiveMutexDepth(DCQ_Mutex)`, unlocks that many times, calls `WaitCondVar(RenderingCond)`, then re-locks that many times. This pattern works correctly because `WaitCondVar` uses its own internal synchronization — the caller's mutex is fully released before blocking and fully reacquired after waking.

**[Reference design]** One acceptable implementation uses an internal mutex + generation counter + pending-signal counter. The pending-signal counter provides signal-buffering behavior as an internal strengthening; this is an implementation detail that callers must not depend on. Broadcast increments a generation counter so all waiters see the change. This buffered example is illustrative only and is not intended as an endorsement over an equally valid lost-signal implementation.

---

## 4. Synchronization Primitive Destruction and Lifetime

### 4.1 Destruction preconditions

**[Normative]** The following preconditions apply to destruction of synchronization primitives through the public API:

- **Mutexes:** callers must not destroy a mutex while another thread owns it or is blocked waiting to acquire it. If violated, behavior is undefined.
- **Condition variables:** callers must not destroy a condvar while any thread is blocked in `WaitCondVar` on it. If violated, behavior is undefined.
- **Semaphores:** callers must not destroy a semaphore while any thread is blocked in `SetSemaphore` on it. If violated, behavior is undefined.
- **Thread-local storage:** see §4.2.
- **Joinable thread handles:** callers must not destroy a joinable thread handle before the thread has been joined (or join has been attempted) via `WaitThread`. See §2.6 for the complete handle-state model.
- **Non-joinable thread handles:** callers do not hold handles to non-joinable threads. Resource reclamation is handled by lifecycle cleanup.

These preconditions are consistent with the legacy backends (SDL, pthreads) and with standard practice for threading primitives.

**[Normative]** Destroying a null handle through a public destroy path that historically tolerated it shall be a safe no-op.

### 4.2 TLS destruction authority

**[Normative]** Thread-local storage ownership and destruction rules:

- **Ownership:** each thread has at most one authoritative `ThreadLocal` object. That object is owned by the thread it belongs to.
- **Destruction authority:** only the owning thread should destroy its own `ThreadLocal`. Destroying another thread's TLS from a different thread is not a supported operation.
- **`DestroyThreadLocal` semantics:** `DestroyThreadLocal(tl)` destroys the flush semaphore associated with the passed object and frees the object. It also clears the calling thread's registered TLS slot. The passed pointer and the calling thread's registered TLS must refer to the same object.
- **Pointer-level double-destroy:** calling `DestroyThreadLocal` on an already-destroyed object is not defined as safe. Callers must not attempt to free the same TLS object twice.
- **Slot-cleared tolerance for automatic cleanup:** threads spawned through the public API receive automatic TLS initialization and cleanup. If C code also calls `CreateThreadLocal`/`DestroyThreadLocal` for such a thread, the behavior depends on the idempotency of `CreateThreadLocal` (which returns the existing object if one exists). Manually destroying TLS for an automatically-managed thread before the thread exits will leave the thread without TLS for the remainder of its execution. The automatic cleanup path must tolerate the TLS slot already being cleared, but that slot-cleared tolerance does not imply that freeing the same object twice is safe.
- **Cross-thread `flushSem` use:** the `flushSem` semaphore within a `ThreadLocal` is designed to be signaled from other threads (e.g., the render thread calls `ClearSemaphore` on a game thread's `flushSem`). This cross-thread use is valid only while the owning thread's TLS is live. Callers that signal a `flushSem` must ensure the owning thread has not yet destroyed its TLS.

---

## 5. Thread-Local Storage

### 5.1 C-visible structure

**[Normative]**

```c
typedef struct _threadLocal {
    Semaphore flushSem;
} ThreadLocal;
```

Three functions manage TLS:

| Function | Behavior |
|---|---|
| `CreateThreadLocal()` | Allocates a `ThreadLocal` and creates a `flushSem` semaphore with initial count 0. Returns a pointer to the struct. Idempotent: returns the existing object if one already exists for the calling thread. |
| `DestroyThreadLocal(tl)` | Destroys the `flushSem` semaphore, frees the struct, and clears the calling thread's TLS slot. See §4.2 for ownership and preconditions. |
| `GetMyThreadLocal()` | Returns the `ThreadLocal*` for the calling thread, or `NULL` if none is set. |

### 5.2 Rust TLS layers

**[Reference design]** The Rust subsystem maintains two parallel TLS layers:

1. **Rust-native TLS** (`THREAD_LOCAL: RefCell<Option<ThreadLocal>>`): Stores a `ThreadLocal` struct containing an `Arc<Semaphore>` named `flush_sem`. Used by pure-Rust callers.

2. **FFI TLS** (`FFI_THREAD_LOCAL: Cell<*mut FfiThreadLocal>`): A heap-allocated `#[repr(C)]` struct containing a `*mut c_void` pointer to a `RustSemaphore`. This is the pointer returned to C code by `rust_thread_local_create` and `rust_get_my_thread_local`.

**Specified behavior:**

- `rust_thread_local_create`: Returns the existing FFI TLS pointer if one already exists for this thread (idempotent). Otherwise, initializes both Rust-native and FFI TLS, then returns the FFI pointer.
- `rust_thread_local_destroy`: Destroys the FFI TLS struct and its contained semaphore. Clears both the FFI and Rust TLS slots. Must tolerate being called when the slots are already cleared (for interaction with automatic cleanup).
- `rust_get_my_thread_local`: Returns the current thread's FFI TLS pointer, or null if none is set.

### 5.3 Automatic TLS lifecycle

**[Normative]** Threads spawned via the public thread creation API automatically have TLS initialized on entry and cleaned up on exit. C callers using `CreateThreadLocal` / `DestroyThreadLocal` directly manage TLS for threads not spawned through the threading subsystem (e.g., the main thread).

**[Reference design]** Automatic lifecycle is implemented via the `ThreadLocalGuard` RAII type.

### 5.4 FfiThreadLocal layout

**[Normative]** The `FfiThreadLocal` struct must be layout-compatible with the C `ThreadLocal`:

```rust
#[repr(C)]
struct FfiThreadLocal {
    flush_sem: *mut c_void,  // Points to a RustSemaphore
}
```

C code accesses `tl->flushSem` at offset 0. The Rust struct's `flush_sem` field must be at the same offset. The pointer must be directly usable as a `Semaphore` (i.e., `RustSemaphore*`) by C callers — e.g., `SetSemaphore(tl->flushSem)` and `ClearSemaphore(tl->flushSem)` must work.

---

## 6. Timing, Yield, and Sleep Helpers

### 6.1 `TaskSwitch`

**[Normative]** **C API:** `void TaskSwitch(void)`

**Legacy behavior:** Calls `SDL_Delay(1)` (SDL backend) or `sched_yield()` + nanosleep fallback (pthread backend).

**Specified behavior:** Yields execution in a way that gives other runnable work an opportunity to proceed and remains compatible with existing engine polling loops that expect a short cooperative delay rather than a busy spin. The exact mechanism need not be a fixed ~1ms sleep so long as the observable behavior remains compatible with legacy polling usage.

**[Reference design]** `thread::sleep(Duration::from_millis(1))` is one acceptable implementation strategy.

### 6.2 `HibernateThread(timePeriod)`

**[Normative]** **C API:** `void HibernateThread(TimePeriod timePeriod)`

**Legacy behavior:** Converts `timePeriod` from UQM time units to milliseconds, calls `SDL_Delay(ms)`.

**Specified behavior:** The C adapter converts `TimePeriod` to milliseconds (`msecs = timePeriod * 1000 / ONE_SECOND` where `ONE_SECOND = 840`), then the implementation sleeps for that duration.

**[Reference design]** Calls `rust_hibernate_thread(msecs)`. The Rust side calls `thread::sleep(Duration::from_millis(msecs))`.

### 6.3 `HibernateThreadUntil(wakeTime)`

**[Normative]** **C API:** `void HibernateThreadUntil(TimeCount wakeTime)`

**Legacy behavior:** Computes `wakeTime - now` and sleeps that duration.

**Specified behavior:** Same computation in the C adapter. If `wakeTime <= now`, returns immediately (no sleep).

### 6.4 `SleepThread(timePeriod)`

**[Normative]** **C API:** `void SleepThread(TimePeriod timePeriod)`

**Legacy behavior:** Computes `now + timePeriod` and delegates to `SleepThreadUntil`.

**Specified behavior:** Same delegation.

### 6.5 `SleepThreadUntil(wakeTime)`

**[Normative]** **C API:** `void SleepThreadUntil(TimeCount wakeTime)`

**Legacy behavior (critical).** This function is **not** a simple sleep. It runs a loop that:

1. Calls `Async_process()` to pump the asynchronous callback queue.
2. Checks the current time; returns if `wakeTime <= now`.
3. Queries `Async_timeBeforeNextMs()` for the next scheduled async event.
4. Sleeps until `min(wakeTime, nextAsyncTime)`.
5. Repeats from step 1.

This ensures that async callbacks (timer-driven operations) are serviced even while the main thread is "sleeping." This is important for game timing — audio callbacks, input processing, and other deferred operations depend on `Async_process()` being called regularly from the main-thread sleep path.

**Specified behavior.** `SleepThreadUntil` must preserve the legacy observable behavior of servicing asynchronous engine work while waiting for the target wake time. The current implementation that simply delegates to `SleepThread` (a single `rust_hibernate_thread` call) is therefore a parity gap.

**Current architecture note (non-normative):** In the present architecture, the natural place to restore this behavior is the C adapter (`rust_thrcommon.c`), because `Async_process()` and `Async_timeBeforeNextMs()` are C functions. That implementation placement is architecture guidance, not the normative requirement itself.

**[Reference design]** The specified implementation:

```c
void SleepThreadUntil(TimeCount wakeTime) {
    for (;;) {
        uint32 nextTimeMs;
        TimeCount nextTime;
        TimeCount now;

        Async_process();

        now = GetTimeCounter();
        if (wakeTime <= now)
            return;

        nextTimeMs = Async_timeBeforeNextMs();
        nextTime = (nextTimeMs / 1000) * ONE_SECOND +
                ((nextTimeMs % 1000) * ONE_SECOND / 1000);
        if (wakeTime < nextTime)
            nextTime = wakeTime;

        SleepThread(nextTime - now);  // delegates to rust_hibernate_thread
    }
}
```

This matches the legacy `thrcommon.c` implementation.

### 6.6 Time unit conversions

**[Normative]** UQM uses `TimeCount` / `TimePeriod` (both `DWORD` / `uint32`) with `ONE_SECOND = 840`. Conversion to milliseconds: `ms = time_units * 1000 / 840`. This conversion is performed in the C adapter, not in Rust. The Rust FFI functions accept milliseconds as `u32`.

---

## 7. Shutdown Sequencing

### 7.1 Shutdown contract

**[Normative]** The following ordering constraints apply to thread system shutdown:

- **Joinable threads:** all joinable threads must be joined via `WaitThread` before `UnInitThreadSystem` completes. It is the caller's responsibility to ensure all joinable worker threads have terminated and been joined before initiating shutdown. Lifecycle cleanup does not substitute for explicit joins on caller-owned joinable handles. `UnInitThreadSystem` does not join threads.
- **Non-joinable threads:** non-joinable (fire-and-forget) threads must not outlive thread-system shutdown. The engine must ensure all non-joinable threads have completed before `UnInitThreadSystem` is called. Lifecycle cleanup reclaims their internal handles.
- **Post-shutdown API use:** no thread may call any `threadlib.h` function after `UnInitThreadSystem` has completed. Behavior of any threading API call after shutdown is undefined.
- **Teardown window:** behavior of `threadlib.h` calls made by other threads is undefined from the point `UnInitThreadSystem` begins executing. The engine must quiesce all cross-thread use of threading APIs before calling `UnInitThreadSystem`; the subsystem does not guarantee safe behavior for concurrent callers during teardown.
- **Main-thread TLS:** main-thread TLS is destroyed during `UnInitThreadSystem`. All worker-thread cleanup must be complete before this point.
- **Process-wide resources:** `UnInitThreadSystem` releases process-wide resources (lifecycle mutex, initialization state). These resources must not be in use by any thread at the time of shutdown.

### 7.2 Shutdown preconditions

**[Normative]** Correctness of shutdown relies on the engine satisfying these preconditions before calling `UnInitThreadSystem`:

- All worker threads created through the public API must have exited their entry wrappers (completed execution and called `FinishThread`).
- No queued cross-thread signal (e.g., a `SENDSIGNAL` draw command referencing a `flushSem`) may target TLS that is being torn down.
- Engine shutdown sequencing is responsible for quiescing all such cross-thread activity before TLS destruction and thread-system teardown.

These are not enforced by the threading subsystem; they are obligations on the engine's shutdown path.

---

## 8. Integration Points

### 8.1 Graphics — DCQ and flush semaphore

**Draw Command Queue (DCQ).** The DCQ uses a `RecursiveMutex` (`DCQ_Mutex`) and a `CondVar` (`RenderingCond`) for producer-consumer coordination between game threads and the rendering thread.

**Producer side (game threads):**

1. `Lock_DCQ(slots)` — locks `DCQ_Mutex`, waits in a loop if the queue is full.
2. If the queue is full, `TFB_WaitForSpace` saves the recursive lock depth, unlocks `DCQ_Mutex` N times, calls `WaitCondVar(RenderingCond)`, then re-locks N times.
3. Enqueues draw commands.
4. `Unlock_DCQ()` — unlocks `DCQ_Mutex`.

**Consumer side (render thread):**

1. Dequeues and processes draw commands.
2. Calls `BroadcastCondVar(RenderingCond)` after processing or when idle.

**[Normative]** Specified requirements on the Rust subsystem:

- `GetRecursiveMutexDepth` must return the accurate current lock depth so the DCQ save/restore pattern works.
- `WaitCondVar` must block correctly even though the caller has manually released an unrelated mutex.
- `BroadcastCondVar` must wake all waiters, not just one.

**Flush semaphore.** `TFB_DrawScreen_WaitForSignal()` enqueues a `SENDSIGNAL` draw command containing the current thread's `flushSem`, then blocks on `SetSemaphore(flushSem)`. The render thread executes the command by calling `ClearSemaphore(sem)`, waking the game thread.

**[Normative]** Specified requirements:

- `GetMyThreadLocal()->flushSem` must return a valid, usable `Semaphore` pointer from any thread that was spawned through the threading subsystem.
- The semaphore must work correctly across thread boundaries (render thread posts, game thread waits).

### 8.2 Audio — mixer recursive mutexes

The mixer uses three `RecursiveMutex` instances (`src_mutex`, `buf_mutex`, `act_mutex`) with a strict lock ordering. The audio callback (`mixer_MixChannels`) acquires all three in order, processes samples, then releases in reverse order.

**[Normative]** Specified requirements:

- Recursive mutexes must support acquisition by the same thread in nested order without deadlock.
- The lock/unlock sequence must be correctly paired — unlocking a mutex that isn't owned by the caller must fail safely (no-op with optional log), not crash.
- SDL audio callbacks may call from a thread not spawned by the threading subsystem. The mutex implementation must work for any OS thread, not only threads spawned via the public thread creation API.

### 8.3 Audio — stream thread throttling

The stream decoder thread (`stream.c`) uses `HibernateThread(ONE_SECOND / 10)` when idle and `TaskSwitch()` when active.

**[Normative]** Specified requirements: `HibernateThread` and `TaskSwitch` must yield the CPU for approximately the requested duration. No async pumping is needed here (this is a worker thread, not the main thread).

### 8.4 Task system (`tasklib.c`)

The task system is a C-owned layer that sits on top of `threadlib.h`. Each task:

1. Is allocated from a fixed array of `TASK_MAX` (64) slots.
2. Has a `state_mutex` (plain `Mutex`) protecting a `state` bitmask.
3. Is backed by a `Thread` created via `CreateThread`.
4. Uses `TaskSwitch()` for cooperative polling in `ConcludeTask`.

**[Normative]** Specified requirements:

- `CreateThread` must return a valid, joinable handle.
- `TaskSwitch` must yield for a short cooperative delay consistent with existing engine polling loops.
- `Mutex` (used for `state_mutex`) must work as a basic lock. No currently identified task code re-locks `state_mutex`, so the recursive-vs-non-recursive distinction does not affect this call site given the current codebase. Stack size handling for task threads defers to the canonical rule in §2.1.

The Rust-side `Task` struct is an internal abstraction not integrated with `tasklib.c`. It does not need to be part of the FFI surface and should not be confused with the C task system.

### 8.5 Callback system

The callback system (`callback.c`) uses a plain `Mutex` (`callbackListLock`) to protect a linked list of deferred callbacks. `Async_process()` iterates and executes callbacks under this lock.

**[Normative]** Specified requirements: Standard mutex lock/unlock semantics. No special threading behavior needed.

### 8.6 Logging

The log system (`uqmlog.c`) uses a plain `Mutex` (`qmutex`) created via `CreateMutex`. The mutex is created after `InitThreadSystem` and destroyed before shutdown. A `qlock` flag gates whether locking is active (pre-init logging is single-threaded).

**[Normative]** Specified requirements: Standard mutex semantics. The mutex is created via the same `CreateMutex` path as all other mutexes.

---

## 9. Fairness Policy

**[Normative]** No blocking primitive (mutex, semaphore, condvar) provides a strict FIFO ordering guarantee. No stronger fairness contract than the legacy backend provides is guaranteed for any primitive type.

---

## 10. Adapter ABI Contract

This section defines the C↔Rust FFI boundary. These are stable internal contracts between the C adapter (`rust_thrcommon.c`) and the Rust static library. They are not part of the public `threadlib.h` contract and are not visible to engine callers.

### 10.1 Opaque handle types

**[Adapter ABI]** All Rust objects are exposed to C as opaque pointers. The C side must not alias, copy, or dereference these pointers — they are opaque handles carrying the semantics needed by the adapter.

| C type | Required semantics |
|---|---|
| `RustThread*` | Opaque joinable-thread handle. Consumed by `rust_thread_join`. |
| `RustMutex*` | Opaque mutex handle. Supports lock/unlock/depth/destroy. |
| `RustCondVar*` | Opaque condvar handle. Supports wait/signal/broadcast/destroy. |
| `RustSemaphore*` | Opaque semaphore handle. Supports acquire/release/count/destroy. |

The concrete Rust representation behind these opaque pointers is an implementation detail described in §11.

**[Adapter ABI — cross-layer clarification]** The single `RustMutex*` opaque type is used for both plain-mutex and recursive-mutex handles. This is an ABI-level convenience: it means the adapter can route both `CreateMutex` and `CreateRecursiveMutex` through the same creation and operation FFI entry points. However, this shared ABI shape does not settle the public recursion semantics of plain `Mutex`, which remain governed by the unresolved blocker in §3.1. If the audit outcome requires distinct behavior for plain mutexes, the implementation may differentiate internally (e.g., via a mode flag or distinct backing type behind the same opaque pointer) without breaking the adapter ABI.

### 10.2 FFI function catalog

**[Adapter ABI]**

**Thread system lifecycle:**

| FFI function | Signature | Semantics |
|---|---|---|
| `rust_init_thread_system` | `() -> c_int` | Initialize Rust TLS for main thread. Returns 1 on success, 0 if already initialized. |
| `rust_uninit_thread_system` | `()` | Destroy main thread TLS, reset initialized flag. |
| `rust_is_thread_system_initialized` | `() -> c_int` | Returns 1 if initialized, 0 otherwise. |

**Thread operations:**

| FFI function | Signature | Semantics |
|---|---|---|
| `rust_thread_spawn` | `(name: *const c_char, func: extern "C" fn(*mut c_void) -> c_int, data: *mut c_void) -> *mut RustThread` | Spawn a joinable thread. Returns handle or NULL on failure. |
| `rust_thread_spawn_detached` | `(name: *const c_char, func: extern "C" fn(*mut c_void) -> c_int, data: *mut c_void)` | Spawn a thread with no joinable handle. The current end-state adapter ABI intentionally provides no synchronous success/failure result to the C adapter for detached-thread creation. On creation failure, the function must still return normally across the FFI boundary, must not expose a panic/exception, and must reclaim any transient internal resources allocated for the failed attempt before returning. If the C adapter allocated a `TrueThread` wrapper or other lifecycle bookkeeping before calling this entry point, the adapter-side postcondition is that failed detached creation leaves no lifecycle-visible worker state behind: no `pendingDeath` registration occurs, `FinishThread` is unreachable, and any adapter-owned wrapper allocated for the failed attempt must be destroyed before `StartThread` returns. Caller-visible reporting beyond optional diagnostics is not required unless a future legacy audit proves a stronger observable contract. If such an audit shows that the C adapter must observe detached-thread creation failure synchronously, this ABI entry must be revised before signoff. |
| `rust_thread_join` | `(thread: *mut RustThread, out_status: *mut c_int) -> c_int` | Join thread. On success, writes the thread function's `c_int` return value to `*out_status` (if `out_status` is non-null) and returns `1`. On failure, writes `0` to `*out_status` (if non-null) and returns `0`. Consumes the handle regardless of outcome. |
| `rust_thread_yield` | `()` | Yield via short cooperative sleep. |
| `rust_hibernate_thread` | `(msecs: u32)` | Sleep for `msecs` milliseconds. |

**Thread-local storage:**

| FFI function | Signature | Semantics |
|---|---|---|
| `rust_thread_local_create` | `() -> *mut c_void` | Create or return existing FFI TLS for current thread. |
| `rust_thread_local_destroy` | `(tl: *mut c_void)` | Destroy FFI TLS and clear Rust TLS. Tolerates already-cleared state. |
| `rust_get_my_thread_local` | `() -> *mut c_void` | Return current thread's FFI TLS pointer, or null. |

**Mutex operations:**

| FFI function | Signature | Semantics |
|---|---|---|
| `rust_mutex_create` | `(name: *const c_char) -> *mut RustMutex` | Create a mutex. The single `RustMutex*` handle type is an ABI convenience shared by plain and recursive mutex paths; it does not settle the public recursion semantics of plain `Mutex` (see §3.1). |
| `rust_mutex_destroy` | `(m: *mut RustMutex)` | Destroy the mutex. Preconditions per §4.1. |
| `rust_mutex_lock` | `(m: *mut RustMutex)` | Lock (blocks if held by another thread; same-thread relock semantics for plain `Mutex` remain governed by unresolved blocker §3.1). |
| `rust_mutex_try_lock` | `(m: *mut RustMutex) -> c_int` | Non-blocking lock attempt. Returns 1 if acquired, 0 otherwise. |
| `rust_mutex_unlock` | `(m: *mut RustMutex)` | Unlock. Returns silently if caller is not the owner. |
| `rust_mutex_depth` | `(m: *mut RustMutex) -> u32` | Current recursive lock depth. Meaningful for recursive-mutex callers; presence of this entry point does not imply that the plain-`Mutex` public contract is settled as recursive. |

**Condition variable operations:**

| FFI function | Signature | Semantics |
|---|---|---|
| `rust_condvar_create` | `(name: *const c_char) -> *mut RustCondVar` | Create a condvar. |
| `rust_condvar_destroy` | `(cv: *mut RustCondVar)` | Destroy the condvar. Preconditions per §4.1. |
| `rust_condvar_wait` | `(cv: *mut RustCondVar, mutex: *mut RustMutex)` | Block awaiting a wake event. `mutex` is accepted for signature compatibility but currently ignored. The adapter ABI does not require remembered-signal behavior; both lost-signal and remembered-signal implementations are ABI-compatible so long as the public `threadlib.h` semantics in §3.4 are preserved. |
| `rust_condvar_wait_timeout` | `(cv: *mut RustCondVar, mutex: *mut RustMutex, msecs: u32) -> c_int` | Block with timeout awaiting a wake condition. Returns 1 when the wait completes before timeout expiry under the implementation's wake-condition rules, 0 when the timeout expires first. The adapter ABI does not require remembered-signal behavior; both lost-signal and remembered-signal implementations are ABI-compatible so long as the public `threadlib.h` semantics in §3.4 are preserved. |
| `rust_condvar_signal` | `(cv: *mut RustCondVar)` | Wake at most one waiter. The adapter ABI does not require buffered delivery semantics. |
| `rust_condvar_broadcast` | `(cv: *mut RustCondVar)` | Wake all current waiters. The adapter ABI does not require buffered delivery semantics. |

**Semaphore operations:**

| FFI function | Signature | Semantics |
|---|---|---|
| `rust_semaphore_create` | `(initial: u32, name: *const c_char) -> *mut RustSemaphore` | Create a counting semaphore. |
| `rust_semaphore_destroy` | `(s: *mut RustSemaphore)` | Destroy the semaphore. Preconditions per §4.1. |
| `rust_semaphore_acquire` | `(s: *mut RustSemaphore)` | Blocking acquire (decrement). |
| `rust_semaphore_try_acquire` | `(s: *mut RustSemaphore) -> c_int` | Non-blocking acquire. Returns 1 if acquired, 0 otherwise. |
| `rust_semaphore_release` | `(s: *mut RustSemaphore)` | Release (increment), wake one blocked waiter. |
| `rust_semaphore_count` | `(s: *mut RustSemaphore) -> u32` | Snapshot of current count. |

**Task switch:**

| FFI function | Signature | Semantics |
|---|---|---|
| `rust_task_switch` | `()` | Alias for `rust_thread_yield`. |

### 10.3 Thread return value propagation

**[Normative]** At parity, the spawn/join contract must propagate the C thread function's `c_int` return value.

**[Adapter ABI]** The `rust_thread_join` function uses a two-value return convention: a `c_int` return value indicating success (`1`) or failure (`0`), and an `out_status` pointer through which the thread function's return value is written on success. This gives the C adapter enough information to distinguish join success from join failure at the ABI boundary, even when the thread returned `0`. The C adapter then maps this to the public `WaitThread` contract, which writes either the thread's status or `0` (on failure) to the caller's `*status` and does not expose the success/failure distinction separately (see §2.2).

**[Reference design]**

```rust
// spawn_c_thread captures the return value
Thread::spawn(name, move || -> c_int {
    let _guard = ThreadLocalGuard::attach();
    unsafe { func(data) }  // returns c_int
})

// rust_thread_join writes the status via out-param and returns success/failure
#[no_mangle]
pub extern "C" fn rust_thread_join(
    thread: *mut RustThread,
    out_status: *mut c_int,
) -> c_int {
    // ... null checks ...
    match thread.join() {
        Ok(status) => {
            if !out_status.is_null() {
                unsafe { *out_status = status; }
            }
            1  // success
        }
        Err(_) => {
            if !out_status.is_null() {
                unsafe { *out_status = 0; }
            }
            0  // failure
        }
    }
}
```

### 10.4 Null-safety

**[Adapter ABI]** All FFI functions that receive a pointer must check for null and return gracefully (no-op or 0/NULL return). This is already implemented and must be maintained.

---

## 11. Rust Internal Types

**[Reference design]** This entire section describes the internal Rust implementation. These types, algorithms, and layouts are not part of the public contract or the adapter ABI. Alternative implementations that satisfy the normative and adapter ABI requirements are acceptable.

### 11.1 Opaque handle representation

One current-compatible implementation allocates each handle via `Box::into_raw` and reclaims it via `Box::from_raw`. The concrete Rust types behind the opaque C pointers are:

| C opaque type | Rust backing type |
|---|---|
| `RustThread*` | `Box<Thread<c_int>>` |
| `RustMutex*` | `Box<RustFfiMutex>` |
| `RustCondVar*` | `Box<UqmCondVar>` |
| `RustSemaphore*` | `Box<Semaphore>` |

These specific representations are not required by the adapter ABI (§10.1). Any Rust-side representation that preserves opaque-pointer semantics and the adapter ABI contract is acceptable.

### 11.2 `RustFfiMutex`

**[Reference design]** `RustFfiMutex` is one acceptable implementation strategy for the mutex ABI. It is an owner-tracked recursive mutex built on `Mutex<FfiMutexState>` + `Condvar`. It currently provides recursive-lock behavior for all mutex handles, which satisfies the `RecursiveMutex` contract and is compatible with the plain-`Mutex` contract only if the §3.1 audit approves unified recursive behavior.

If the §3.1 audit requires non-recursive plain mutexes, this reference design must be revised. Possible approaches include adding an internal mode flag that controls whether same-thread relock increments depth or returns an error, using a distinct backing type for plain mutexes, or any other strategy that preserves the adapter ABI while enforcing non-recursive semantics where required. The choice of revision strategy is an implementation decision, not a public contract concern.

```rust
struct FfiMutexState {
    owner: Option<ThreadId>,
    depth: u32,
}

struct RustFfiMutex {
    state: Mutex<FfiMutexState>,
    condvar: Condvar,
    name: Option<String>,
}
```

**Lock algorithm:**

1. Acquire `state` inner lock.
2. If `owner` is `Some(other_thread)`, wait on `condvar` in a loop.
3. Set `owner` to current thread, increment `depth`.

**Unlock algorithm:**

1. Acquire `state` inner lock.
2. Verify caller is the owner. If not, return error.
3. Decrement `depth`. If depth reaches 0, clear `owner` and notify one waiter.

**Depth query:** Acquire `state` inner lock, return `depth`.

### 11.3 `UqmCondVar`

Self-synchronizing condition variable with generation counter.

```rust
struct CondVarState {
    generation: u64,
    pending_signals: u64,
}

struct UqmCondVar {
    inner: Condvar,
    state: Mutex<CondVarState>,
    name: Option<String>,
}
```

**Wait algorithm:**

1. Lock `state`, capture `my_generation`.
2. Loop: if `generation != my_generation` (broadcast occurred), return. If `pending_signals > 0`, decrement and return. Otherwise, wait on `inner` condvar.

**Signal:** Lock `state`, increment `pending_signals`, notify one.

**Broadcast:** Lock `state`, increment `generation`, notify all.

**Wait with timeout:** Same as wait but uses `Condvar::wait_timeout`. Returns false when the timeout expires before the implementation observes a qualifying wake condition. A particular implementation may also return false after a spurious wakeup if no qualifying state change is observed before timeout accounting completes; this is an implementation detail, not additional public contract.

### 11.4 `Semaphore`

Counting semaphore built on `Mutex<u32>` + `Condvar`.

```rust
struct Semaphore {
    count: Mutex<u32>,
    condvar: Condvar,
    name: Option<String>,
}
```

**Acquire:** Lock `count`, loop while `*count == 0` (wait on condvar), decrement.

**Release:** Lock `count`, increment, notify one.

### 11.5 `Thread<T>`

Thin wrapper over `JoinHandle<T>`.

```rust
struct Thread<T> {
    handle: Option<JoinHandle<T>>,
    name: Option<String>,
}
```

At parity, the FFI-facing thread type should be `Thread<c_int>` so that `join()` returns the C function's exit code.

### 11.6 `ThreadLocal` and `FfiThreadLocal`

```rust
struct ThreadLocal {
    flush_sem: Arc<Semaphore>,
}

#[repr(C)]
struct FfiThreadLocal {
    flush_sem: *mut c_void,  // *mut RustSemaphore
}
```

Two `thread_local!` slots per thread store `Option<ThreadLocal>` and `*mut FfiThreadLocal` respectively.

### 11.7 `ThreadLocalGuard`

RAII guard that initializes TLS on construction and cleans up on drop:

```rust
struct ThreadLocalGuard {
    created_rust: bool,
    created_ffi: bool,
}
```

`attach()` calls `ensure_rust_thread_local()` and `ensure_ffi_thread_local()`, recording which were newly created. `drop()` destroys only what was newly created.

---

## 12. Thread System Initialization

### 12.1 `InitThreadSystem`

**[Normative]** Called once at startup. The C adapter:

1. Calls `rust_init_thread_system()` — sets the initialized flag, creates Rust/FFI TLS for the main thread.
2. Initializes `pendingDeath` array to null.
3. Creates `lifecycleMutex` via `CreateMutex`.

### 12.2 `UnInitThreadSystem`

**[Normative]** Called once at shutdown. The C adapter:

1. Calls `ProcessThreadLifecycles()` to reclaim resources for completed non-joinable threads.
2. Destroys `lifecycleMutex`.
3. Calls `rust_uninit_thread_system()` — destroys main thread TLS, resets initialized flag.

At the point `UnInitThreadSystem` is called, all joinable threads must already have been joined via `WaitThread` and their handles destroyed via `DestroyThread`. All non-joinable threads must have completed. `ProcessThreadLifecycles` at shutdown reclaims any remaining non-joinable thread wrappers that completed but were not yet processed; it does not join threads and does not reclaim caller-owned joinable handles. Shutdown ordering constraints are defined in §7.

### 12.3 Double-init guard

**[Normative]** `rust_init_thread_system` uses an `AtomicBool` flag. Calling it twice returns 0 (failure) without re-initializing. This prevents corruption if the C side has a bug.

---

## 13. Error Handling Strategy

**[Normative]** At the FFI boundary, errors are translated as follows:

| FFI return type | Success | Failure |
|---|---|---|
| `*mut T` (creation) | Valid pointer | `NULL` |
| `c_int` (boolean) | `1` | `0` |
| `c_int` (join result) | `1` (thread status written to out-param) | `0` (0 written to out-param) |
| `void` | Normal return | No-op (error is logged or silently discarded) |

**[Normative]** The threading subsystem shall not expose implementation-specific error states (such as mutex poisoning, panics, or internal exception types) through the public API. At the FFI boundary, all such conditions are mapped to the return conventions above.

**[Reference design]** The Rust subsystem uses a `Result<T, ThreadError>` type internally. If a `std::sync::Mutex` becomes poisoned (a thread panicked while holding it), the `RustFfiMutex`, `Semaphore`, and `UqmCondVar` implementations return `ThreadError::MutexPoisoned`. The FFI layer translates this to a 0 return or no-op. This is a defensive measure — mutex poisoning in production means the process is in an inconsistent state and is likely to crash regardless.

---

## 14. Naming and Debug Support

**[Normative]** When `NAMED_SYNCHRO` is defined (the default when `TRACK_CONTENTION` is active), all synchronization objects carry a `const char *name`. This name is passed through FFI to Rust and stored for debugging use.

Thread names provided to `CreateThread` and `StartThread` are subject to the naming policy defined in §2.1. Synchronization primitive names (mutexes, condvars, semaphores) are diagnostic metadata stored for debugging and contention analysis.

**[Reference design]** Names are stored as `Option<String>`. Names are borrowed from C (static string literals) and copied into Rust `String`s for ownership safety. The Rust subsystem does not currently log contention but stores names to enable it later.

---

## 15. Items Explicitly Out of Scope

The following are **not** part of this specification:

- **C task system port.** `tasklib.c` / `tasklib.h` remain C-owned. The Rust `Task` struct is an internal abstraction that does not replace or integrate with the C task system.
- **Async callback system port.** `callback.c` / `async.h` remain C-owned. `Async_process()` is called from C, not from Rust.
- **Graphics/audio/resource subsystem ports.** These consume `threadlib.h` but are not part of the threading subsystem.
- **Windows/non-Unix platform support.** This specification assumes the Unix build path (`config_unix.h` with `USE_RUST_THREADS`).
- **`PROFILE_THREADS` / `PrintThreadsStats`.** Thread profiling is not part of the Rust subsystem. If needed in the future, it can be added as a separate concern.
