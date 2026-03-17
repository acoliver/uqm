# Threading subsystem initial state

## Scope and purpose

The threading subsystem provides the process-wide abstraction used by UQM C code for:

- worker thread creation and joining,
- per-thread local storage (`ThreadLocal`) with a graphics flush semaphore,
- blocking and recursive mutexes,
- semaphores,
- condition variables,
- cooperative yielding and sleep helpers (`TaskSwitch`, `HibernateThread`, `SleepThread*`), and
- lifecycle cleanup coordination for spawned threads.

The public C-facing API is declared in `/Users/acoliver/projects/uqm/sc2/src/libs/threadlib.h:56-180`. Important pieces include the opaque handle typedefs at `:61-65`, `ThreadLocal` at `:67-70`, creation macros at `:93-140`, lifecycle entry points at `:144-157`, and synchronization operations at `:164-180`.

This subsystem is an integration-critical dependency. Current call sites in C include graphics DCQ waiting and flush signaling, task management, mixer/audio locking, callbacks, logging, and drawable/image locking, e.g.:

- recursive mutex depth + condvar wait in `/Users/acoliver/projects/uqm/sc2/src/libs/graphics/dcqueue.c:55-60`,
- thread-local flush semaphore use in `/Users/acoliver/projects/uqm/sc2/src/libs/graphics/tfb_draw.c:205-212`,
- task spawning/yield loops in `/Users/acoliver/projects/uqm/sc2/src/libs/task/tasklib.c:40-42` and `:67-73`,
- stream thread throttling in `/Users/acoliver/projects/uqm/sc2/src/libs/sound/stream.c:571-576`, and
- mixer recursive mutex setup in `/Users/acoliver/projects/uqm/sc2/src/libs/sound/mixer/mixer.c:110-112`.

## Build and configuration wiring

Rust threading is enabled in the active Unix configuration by `#define USE_RUST_THREADS` in `/Users/acoliver/projects/uqm/sc2/config_unix.h:113-114`.

The Rust library exposes the threading module from `/Users/acoliver/projects/uqm/rust/src/lib.rs:7-22`, where `pub mod threading;` appears at `:21`.

The Rust crate is built as a static library and rlib in `/Users/acoliver/projects/uqm/rust/Cargo.toml:5-9`, which is the mechanism used by the C build to link the Rust FFI exports.

On the C side, the original common threading shim is compiled out when Rust threading is enabled: `/Users/acoliver/projects/uqm/sc2/src/libs/threads/thrcommon.c:17` opens with `#ifndef USE_RUST_THREADS`, and the file closes at `:455`.

The Rust replacement wrapper is compiled only when that define is active: `/Users/acoliver/projects/uqm/sc2/src/libs/threads/rust_thrcommon.c:9` opens with `#ifdef USE_RUST_THREADS`, and closes at `:432`.

## Current C structure

### Public abstraction layer

The canonical C API remains `threadlib.h`, not the native SDL/pthread backends directly. The rest of the codebase calls `CreateThread`, `CreateMutex`, `CreateRecursiveMutex`, `CreateCondVar`, `SetSemaphore`, `ClearSemaphore`, `WaitCondVar`, `TaskSwitch`, `HibernateThread`, and related wrappers from `/Users/acoliver/projects/uqm/sc2/src/libs/threadlib.h:93-180`.

### Original C common layer

The legacy implementation in `/Users/acoliver/projects/uqm/sc2/src/libs/threads/thrcommon.c` is still present and documents the pre-port semantics:

- initialization of `pendingBirth` and `pendingDeath` lifecycle arrays at `:47-57`,
- deferred thread creation through `FlagStartThread` and main-thread lifecycle processing at `:66-91` and `:111-153`,
- named and unnamed `Create*_Core` forwarding to native backends at `:159-255`,
- `ThreadLocal` allocation with `flushSem` creation at `:263-275`,
- `SleepThreadUntil` calling `Async_process()` before sleeping at `:333-362`, and
- thin wrappers around native mutex/semaphore/condvar/recursive-mutex operations at `:371-452`.

The split matters because some of those semantics are not preserved in the Rust wrapper.

### Native backend structure still present in C

The native implementations remain in the repository and provide the baseline semantics the Rust port is replacing:

- SDL backend declarations and macro selection: `/Users/acoliver/projects/uqm/sc2/src/libs/threads/sdl/sdlthreads.h:31-103`
- pthread backend declarations and macro selection: `/Users/acoliver/projects/uqm/sc2/src/libs/threads/pthread/posixthreads.h:28-100`
- SDL thread creation allocates `thread->localData = CreateThreadLocal()` in `/Users/acoliver/projects/uqm/sc2/src/libs/threads/sdl/sdlthreads.c:255-271`
- pthread condvar wait uses an internal condvar mutex, not the caller mutex, in `/Users/acoliver/projects/uqm/sc2/src/libs/threads/pthread/posixthreads.c:635-657`

Notably, the original C condvar API itself is atypical: `WaitCondVar` in `threadlib.h` takes only a `CondVar` handle, and the native backend uses condvar-internal mutexing rather than the usual external mutex discipline. That baseline reduces one potential mismatch with Rust, but call sites still assume specific wake/block behavior.

## Current Rust structure

All Rust threading code currently lives in one module, `/Users/acoliver/projects/uqm/rust/src/threading/mod.rs`, with unit tests in `/Users/acoliver/projects/uqm/rust/src/threading/tests.rs`.

### Internal Rust implementation

The module currently defines:

- generic `Thread<T>` wrapper over `std::thread::JoinHandle` at `/Users/acoliver/projects/uqm/rust/src/threading/mod.rs:81-143`,
- generic `UqmMutex<T>` over `std::sync::Mutex` at `:153-203`,
- FFI-facing recursive-owner/depth mutex implementation `RustFfiMutex` at `:205-289`,
- generation-counted `UqmCondVar` at `:303-441`,
- counting semaphore at `:452-537`,
- a task abstraction at `:543-634`,
- thread-system init/uninit and helpers at `:640-700`, and
- Rust + FFI thread-local storage support at `:706-817`.

### FFI export surface

The Rust module exports the C-facing threading API directly from the same file. Important exported symbols and locations:

- lifecycle: `rust_init_thread_system`, `rust_uninit_thread_system`, `rust_is_thread_system_initialized` at `/Users/acoliver/projects/uqm/rust/src/threading/mod.rs:887-917`
- thread spawn/join/yield/sleep: `rust_thread_spawn` `:953-969`, `rust_thread_spawn_detached` `:971-984`, `rust_thread_join` `:997-1007`, `rust_thread_yield` `:1014-1017`, `rust_hibernate_thread` `:1026-1029`
- thread-local: `rust_thread_local_create` `:1031-1040`, `rust_thread_local_destroy` `:1043-1055`, `rust_get_my_thread_local` `:1058-1060`
- mutex: `rust_mutex_create` `:1075-1085`, `rust_mutex_destroy` `:1095-1100`, `rust_mutex_lock` `:1109-1114`, `rust_mutex_try_lock` `:1127-1137`, `rust_mutex_unlock` `:1147-1153`, `rust_mutex_depth` `:1156-1164`
- condvar: `rust_condvar_create` `:1178-1188`, `rust_condvar_destroy` `:1197-1201`, `rust_condvar_wait` `:1213-1221`, `rust_condvar_wait_timeout` `:1235-1249`, `rust_condvar_signal` `:1259-1264`, `rust_condvar_broadcast` `:1274-1279`
- semaphore: `rust_semaphore_create` `:1295-1308`, `rust_semaphore_destroy` `:1317-1321`, `rust_semaphore_acquire` `:1331-1336`, `rust_semaphore_try_acquire` `:1349-1360`, `rust_semaphore_release` `:1370-1375`, `rust_semaphore_count` `:1388-1395`
- task switch: `rust_task_switch` at `:1402-1404`

The corresponding C declarations are in `/Users/acoliver/projects/uqm/sc2/src/libs/threads/rust_threads.h:27-68`.

## C↔Rust integration points

### C wrapper over Rust exports

`/Users/acoliver/projects/uqm/sc2/src/libs/threads/rust_thrcommon.c` is the active C-side adapter. It maps the historical `threadlib.h` surface to Rust FFI functions.

Important boundary points:

- initialization delegates to Rust then creates the C lifecycle mutex in `/Users/acoliver/projects/uqm/sc2/src/libs/threads/rust_thrcommon.c:119-138`
- thread creation goes through `rust_thread_spawn` in `:140-183`
- `WaitThread` uses `rust_thread_join` in `:208-223`
- C mutex API forwards to Rust mutex functions in `:277-300`
- C semaphore API forwards to Rust semaphore functions in `:302-325`
- C condvar API forwards to Rust condvar functions in `:327-357`
- recursive mutex API is implemented by reusing the same Rust mutex type in `:359-395`
- thread-local creation/destruction/get are forwarded in `:397-413`
- `HibernateThread` and `TaskSwitch` forward to Rust at `:415-430` and `:202-206`

### Split boundary with original C common layer

The partial-port boundary is explicit:

- original common layer excluded by `/Users/acoliver/projects/uqm/sc2/src/libs/threads/thrcommon.c:17-455`
- Rust replacement wrapper enabled by `/Users/acoliver/projects/uqm/sc2/src/libs/threads/rust_thrcommon.c:9-432`

The native SDL/pthread backend files remain in the repo but are no longer the active top-level implementation when `USE_RUST_THREADS` is on.

## What is already ported

### Ported and actively wired

The following are implemented in Rust and wired through the active C wrapper:

- thread system init/uninit state tracking: `/Users/acoliver/projects/uqm/rust/src/threading/mod.rs:640-669`, exported at `:887-917`
- thread spawn and join via `std::thread`: `:99-118`, `:127-134`, FFI bridge `:936-1007`
- owner-tracked recursive mutex behavior for the FFI-facing mutex type: `/Users/acoliver/projects/uqm/rust/src/threading/mod.rs:205-289`
- counting semaphore: `:452-537`, FFI `:1295-1395`
- generation/pending-signal condition variable implementation: `:303-441`, FFI `:1178-1279`
- thread-local allocation and exposure of `flush_sem`: `:730-876`, FFI `:1031-1060`
- cooperative task switch export: `:1402-1404`

Unit tests exist for the internal Rust abstractions in `/Users/acoliver/projects/uqm/rust/src/threading/tests.rs:33-726`, including tests for thread spawn/join, mutexes, condvars, semaphores, task state/callback behavior, initialization, sleep/yield, and thread-local flush semaphore handling.

## What remains C-owned

Even with `USE_RUST_THREADS` enabled, important ownership remains on the C side:

- the public API contract and call sites remain C-owned in `/Users/acoliver/projects/uqm/sc2/src/libs/threadlib.h:56-180` and numerous callers under `/Users/acoliver/projects/uqm/sc2/src/libs/**`
- lifecycle handle bookkeeping (`lifecycleMutex`, `pendingDeath`, `FinishThread`, `ProcessThreadLifecycles`) is still implemented in C in `/Users/acoliver/projects/uqm/sc2/src/libs/threads/rust_thrcommon.c:36-38` and `:225-268`
- C code still decides when to process lifecycles; Rust's own `process_thread_lifecycles()` is not used by the active adapter
- time conversion from UQM `TimeCount`/`TimePeriod` to milliseconds for sleep helpers remains in the C wrapper at `/Users/acoliver/projects/uqm/sc2/src/libs/threads/rust_thrcommon.c:186-200` and `:415-430`
- all engine-level synchronization policy remains driven by existing C call patterns in graphics, sound, tasks, logging, callbacks, and graphics resources

## Verified parity gaps and incomplete semantics

### 1. Original deferred main-thread thread creation is gone

The legacy common layer stages spawns in `pendingBirth` and performs actual `NativeCreateThread` from `ProcessThreadLifecycles()` on the main thread in `/Users/acoliver/projects/uqm/sc2/src/libs/threads/thrcommon.c:66-91` and `:111-139`.

The Rust wrapper does not preserve that behavior. `CreateThread_Core` and `StartThread_Core` call `rust_thread_spawn` immediately in `/Users/acoliver/projects/uqm/sc2/src/libs/threads/rust_thrcommon.c:140-183`, and the Rust export immediately calls `Thread::spawn` in `/Users/acoliver/projects/uqm/rust/src/threading/mod.rs:944-950` and `:965-967`.

This is a real semantic split boundary because `threadlib.h` explicitly documents a main-thread deadlock caveat around `CreateThread` at `/Users/acoliver/projects/uqm/sc2/src/libs/threadlib.h:82-85`, and the original implementation existed to avoid main-thread direct native creation.

### 2. `StartThread_Core` does not use the detached Rust export

`rust_threads.h` declares `rust_thread_spawn_detached` at `/Users/acoliver/projects/uqm/sc2/src/libs/threads/rust_threads.h:34`, and Rust exports it at `/Users/acoliver/projects/uqm/rust/src/threading/mod.rs:971-984`.

However, the active C wrapper's `StartThread_Core` calls `rust_thread_spawn`, not `rust_thread_spawn_detached`, in `/Users/acoliver/projects/uqm/sc2/src/libs/threads/rust_thrcommon.c:165-183`.

So the detached export exists but is not the integration path currently used by the C adapter.

### 3. `WaitThread` status semantics are not preserved

The C API expects thread exit status via `WaitThread(Thread, int *status)` in `/Users/acoliver/projects/uqm/sc2/src/libs/threadlib.h:152-154`.

The Rust-side spawn bridge discards the C thread function's return value: `/Users/acoliver/projects/uqm/rust/src/threading/mod.rs:944-950` calls the C function but stores its result into `_` and returns `Thread<()>`. `rust_thread_join` then returns only success/failure as `1/0` at `:997-1007`.

The active C wrapper writes that boolean-like result into `*status` in `/Users/acoliver/projects/uqm/sc2/src/libs/threads/rust_thrcommon.c:213-220`, not the worker function's actual return code.

That is a concrete parity gap versus the legacy model, which forwards native thread status through `NativeWaitThread` in `/Users/acoliver/projects/uqm/sc2/src/libs/threads/thrcommon.c:284-288`.

### 4. `SleepThreadUntil` lost `Async_process()` behavior

Original `SleepThreadUntil` repeatedly calls `Async_process()` until wake time in `/Users/acoliver/projects/uqm/sc2/src/libs/threads/thrcommon.c:333-362`.

The Rust-enabled wrapper's `SleepThreadUntil` in `/Users/acoliver/projects/uqm/sc2/src/libs/threads/rust_thrcommon.c:193-200` only computes `wakeTime - now` and delegates to `SleepThread`, and `SleepThread` just calls `rust_hibernate_thread` after millisecond conversion at `:186-190`.

On the Rust side, `hibernate_thread` is explicitly marked incomplete with `// TODO: Implement thread hibernation` in `/Users/acoliver/projects/uqm/rust/src/threading/mod.rs:696-699`, and currently just uses `thread::sleep(duration)` at `:699`.

So the main-thread asynchronous pumping behavior from the original C layer is absent under Rust threading.

### 5. Condvar wait ignores the passed mutex handle

The C FFI declaration includes a mutex parameter in `/Users/acoliver/projects/uqm/sc2/src/libs/threads/rust_threads.h:54-55`, and the C wrapper comments that the API is simplified and passes `NULL` in `/Users/acoliver/projects/uqm/sc2/src/libs/threads/rust_thrcommon.c:341-345`.

Rust also names the parameter `_mutex` and ignores it in `/Users/acoliver/projects/uqm/rust/src/threading/mod.rs:1214-1221` and `:1236-1249`.

This differs from conventional condvar semantics and from the signature advertised in the Rust FFI header. It is less divergent from the historical UQM backend than it first appears because the native pthread implementation also uses an internal condvar mutex in `/Users/acoliver/projects/uqm/sc2/src/libs/threads/pthread/posixthreads.c:637-657`; however, the active Rust/C split still means the mutex argument is not part of current behavior.

The practical risk is visible at call sites that manually release and reacquire a recursive mutex around `WaitCondVar`, such as `/Users/acoliver/projects/uqm/sc2/src/libs/graphics/dcqueue.c:55-60`.

### 6. Recursive mutexes are emulated by the regular Rust FFI mutex type

The active C wrapper says this directly: `/Users/acoliver/projects/uqm/sc2/src/libs/threads/rust_thrcommon.c:359-365` comments `Rust std::sync::Mutex is not recursive; using regular mutex` and returns `rust_mutex_create(name)` for `CreateRecursiveMutex_Core`.

In practice, the exported Rust mutex implementation is not actually a plain `std::sync::Mutex`; `RustFfiMutex` tracks owner and depth to emulate recursive locking in `/Users/acoliver/projects/uqm/rust/src/threading/mod.rs:205-289`. So the wrapper comment is stale/misleading, but the code path still collapses both `Mutex` and `RecursiveMutex` onto the same exported handle type.

This is sufficient for current `GetRecursiveMutexDepth` usage because the wrapper forwards to `rust_mutex_depth` in `/Users/acoliver/projects/uqm/sc2/src/libs/threads/rust_thrcommon.c:391-395`, and Rust reports tracked depth in `/Users/acoliver/projects/uqm/rust/src/threading/mod.rs:1156-1164`.

This shared backing is an observed implementation fact only. It does **not** establish acceptable end-state public semantics for plain `Mutex`; that remains an unresolved design/signoff question for the specification and requirements.

### 7. Rust lifecycle processing is stubbed and unused

Rust exposes a lifecycle helper but marks it incomplete: `/Users/acoliver/projects/uqm/rust/src/threading/mod.rs:676-683` contains `// TODO: Implement lifecycle processing`.

The active integration avoids this by keeping lifecycle cleanup on the C side in `/Users/acoliver/projects/uqm/sc2/src/libs/threads/rust_thrcommon.c:225-268`. That means the subsystem is only partially ported: primitives and thread creation are in Rust, while lifecycle policy is still owned by the C wrapper.

### 8. Task abstraction in Rust is not integrated with C tasklib

Rust contains a task model at `/Users/acoliver/projects/uqm/rust/src/threading/mod.rs:543-634`, but it is internal only. The C task system still uses `CreateThread` / `TaskSwitch` from `threadlib.h`, e.g. `/Users/acoliver/projects/uqm/sc2/src/libs/task/tasklib.c:40-42` and `:67-73`.

The Rust task model also includes explicit TODO markers for state retrieval and setting at `/Users/acoliver/projects/uqm/rust/src/threading/mod.rs:596-612`.

So the presence of `Task` in Rust should not be read as a port of `tasklib.c`; it is currently an unintegrated internal abstraction.

### 9. Thread-local destruction clears Rust TLS unconditionally

`rust_thread_local_destroy` destroys the passed FFI object and then always calls `clear_rust_thread_local()` in `/Users/acoliver/projects/uqm/rust/src/threading/mod.rs:1043-1055`.

That coupling differs from the original C structure, where `ThreadLocal` is a C heap object with `flushSem` created and destroyed in `/Users/acoliver/projects/uqm/sc2/src/libs/threads/thrcommon.c:263-275`. The current behavior may be acceptable, but it is a semantic consolidation of two TLS layers that did not exist in the legacy code.

## Guards, TODOs, and evidence of partial port boundaries

### Compile/guard boundaries

- Original common C implementation disabled under Rust threads: `/Users/acoliver/projects/uqm/sc2/src/libs/threads/thrcommon.c:17-18`
- Rust common wrapper enabled only under Rust threads: `/Users/acoliver/projects/uqm/sc2/src/libs/threads/rust_thrcommon.c:9-10`
- Feature define enabling Rust threading: `/Users/acoliver/projects/uqm/sc2/config_unix.h:113-114`

### TODOs in active Rust threading code

All verified TODO markers in the Rust threading module:

- task state retrieval: `/Users/acoliver/projects/uqm/rust/src/threading/mod.rs:596-603`
- task state setting: `/Users/acoliver/projects/uqm/rust/src/threading/mod.rs:611-612`
- lifecycle processing: `/Users/acoliver/projects/uqm/rust/src/threading/mod.rs:681-683`
- thread hibernation: `/Users/acoliver/projects/uqm/rust/src/threading/mod.rs:696-699`

### Explicit simplification/stale-comment markers in C wrapper

- condvar API simplification comment: `/Users/acoliver/projects/uqm/sc2/src/libs/threads/rust_thrcommon.c:343-345`
- recursive mutex comment claiming regular mutex fallback: `/Users/acoliver/projects/uqm/sc2/src/libs/threads/rust_thrcommon.c:362-364`

These comments are useful evidence that maintainers already recognized semantic mismatch points.

## Notable integration points in the wider engine

The following C subsystems currently depend on threading behavior that should be considered part of parity verification:

- graphics DCQ wait/broadcast/depth handling: `/Users/acoliver/projects/uqm/sc2/src/libs/graphics/dcqueue.c:55-60`, `:149-152`, `:353`, `:623`
- graphics thread-local flush semaphore signal path: `/Users/acoliver/projects/uqm/sc2/src/libs/graphics/tfb_draw.c:201-212`
- task manager thread lifecycle and yield loop: `/Users/acoliver/projects/uqm/sc2/src/libs/task/tasklib.c:40-42`, `:61-73`, `:125-136`
- audio stream sleep/yield throttling: `/Users/acoliver/projects/uqm/sc2/src/libs/sound/stream.c:571-576`
- mixer recursive mutex use: `/Users/acoliver/projects/uqm/sc2/src/libs/sound/mixer/mixer.c:110-112`, `:145-147`, `:198-200`
- callback list lock: `/Users/acoliver/projects/uqm/sc2/src/libs/callback/callback.c:43-73`
- log mutex: `/Users/acoliver/projects/uqm/sc2/src/libs/log/uqmlog.c:56-162`

These are the places most likely to expose semantic drift in recursive locking, wakeup behavior, thread-local lifetime, or sleep/yield behavior.

## Risks and unknowns

- **Main-thread behavior risk:** the original deferred thread-creation semantics are absent; if any caller relied on `ProcessThreadLifecycles()` creating threads from the main thread, Rust behavior is different now. Evidence: legacy `/Users/acoliver/projects/uqm/sc2/src/libs/threads/thrcommon.c:111-139` vs current `/Users/acoliver/projects/uqm/sc2/src/libs/threads/rust_thrcommon.c:140-183`.
- **Return-code parity risk:** `WaitThread` no longer returns the worker function's real exit code. Evidence: Rust spawn bridge drops the return value at `/Users/acoliver/projects/uqm/rust/src/threading/mod.rs:949`, join returns success/failure at `:1003-1007`, and C stores that into `status` at `/Users/acoliver/projects/uqm/sc2/src/libs/threads/rust_thrcommon.c:218-220`.
- **Sleep semantics risk:** `SleepThreadUntil` no longer pumps async work. Evidence: legacy `/Users/acoliver/projects/uqm/sc2/src/libs/threads/thrcommon.c:343-361` vs current Rust wrapper `/Users/acoliver/projects/uqm/sc2/src/libs/threads/rust_thrcommon.c:193-200` and Rust TODO at `/Users/acoliver/projects/uqm/rust/src/threading/mod.rs:696-699`.
- **Condvar semantics risk:** the mutex parameter exists in the FFI API but is ignored by the active Rust implementation and wrapper. Evidence: `/Users/acoliver/projects/uqm/sc2/src/libs/threads/rust_threads.h:54-55`, `/Users/acoliver/projects/uqm/sc2/src/libs/threads/rust_thrcommon.c:341-345`, `/Users/acoliver/projects/uqm/rust/src/threading/mod.rs:1214-1249`.
- **Partial-port maintenance risk:** thread primitives are in Rust, but lifecycle policy is still in C, and Rust contains unused/stubbed task/lifecycle pieces. Evidence: `/Users/acoliver/projects/uqm/rust/src/threading/mod.rs:676-683` and `/Users/acoliver/projects/uqm/sc2/src/libs/threads/rust_thrcommon.c:225-268`.
- **Detached-thread integration unknown:** `rust_thread_spawn_detached` exists but is not used by the active adapter. Evidence: export at `/Users/acoliver/projects/uqm/rust/src/threading/mod.rs:971-984`, unused by `/Users/acoliver/projects/uqm/sc2/src/libs/threads/rust_thrcommon.c:165-183`. This leaves two open current-state questions for the end-state design package to resolve explicitly: whether detached-thread creation should route through this entry point at all, and whether detached creation failure requires stronger adapter-visible reporting than the currently documented minimal no-result contract.

## Bottom line

The threading subsystem is partially ported, not fully Rust-owned.

What is fully Rust-backed today is the primitive implementation surface exposed through FFI: thread spawn/join, owner-tracked mutexes, condvars, semaphores, thread-local flush semaphore allocation, and task-switch/sleep entry points.

What remains non-trivially C-owned is the public API contract, lifecycle processing policy, all integration call sites, and several semantics inherited from the old `thrcommon.c` layer that are not yet reproduced by the Rust path. The clearest evidence of incompleteness is the combination of:

- active compile split between `thrcommon.c` and `rust_thrcommon.c`,
- Rust TODOs in `/Users/acoliver/projects/uqm/rust/src/threading/mod.rs:596-699`, and
- behavior differences around deferred creation, thread return status, and `SleepThreadUntil` async processing.