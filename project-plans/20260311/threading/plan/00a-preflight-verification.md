# Phase 00a: Preflight Verification

## Phase ID
`PLAN-20260314-THREADING.P00a`

## Purpose
Verify all assumptions about the codebase, toolchain, and integration boundaries before any implementation work begins.

## Toolchain Verification

- [ ] `cargo --version` (minimum 1.70+ expected)
- [ ] `rustc --version` (minimum 1.70+ expected)
- [ ] `cargo clippy --version`
- [ ] Verify Rust builds: `cd /Users/acoliver/projects/uqm/rust && cargo build 2>&1`
- [ ] Verify existing lib tests pass: `cd /Users/acoliver/projects/uqm/rust && cargo test --lib --all-features 2>&1` (integration tests have a pre-existing linker issue with `input_integration_tests` unrelated to threading)
- [ ] Record exact test count (expected: 1560+)

## Dependency Verification

- [ ] `std::thread`, `std::sync` — standard library, always available
- [ ] `std::ffi` — standard library, always available
- [ ] No new crate dependencies required for this plan

## Type/Interface Verification

### Rust-side types exist and match assumptions
- [ ] `Thread<T>` struct at `rust/src/threading/mod.rs` — has `handle: Option<JoinHandle<T>>`, `name: Option<String>`
- [ ] `RustFfiMutex` struct — has `state: Mutex<FfiMutexState>`, `condvar: Condvar`, `name: Option<String>`
- [ ] `UqmCondVar` struct — has `inner: Condvar`, `state: Mutex<CondVarState>`, `name: Option<String>`
- [ ] `Semaphore` struct — has `count: Mutex<u32>`, `condvar: Condvar`, `name: Option<String>`
- [ ] `FfiThreadLocal` struct — `#[repr(C)]`, has `flush_sem: *mut c_void`
- [ ] `ThreadLocalGuard` struct — has `created_rust: bool`, `created_ffi: bool`
- [ ] `spawn_c_thread` function — currently returns `Result<Thread<()>>` (gap G1: should be `Thread<c_int>`)

### FFI export signatures match current state
- [ ] `rust_thread_spawn` at mod.rs (~line 954) — `(name: *const c_char, func: ..., data: *mut c_void) -> *mut RustThread`
- [ ] `rust_thread_spawn_detached` at mod.rs (~line 972) — `(name: *const c_char, func: ..., data: *mut c_void)` (void return)
- [ ] `rust_thread_join` at mod.rs (~line 998) — `(thread: *mut RustThread) -> c_int` (gap G1: spec requires `out_status` param)
- [ ] `rust_hibernate_thread` at mod.rs (~line 1027) — `(msecs: u32)` (void return)
- [ ] `rust_task_switch` at mod.rs (~line 1403) — `()` (void return)

### C-side types and integration points exist
- [ ] `TrueThread` struct at `rust_thrcommon.c:23-28` — has `native: RustThread*`, `name: const char*`
- [ ] `ThreadStartInfo` struct at `rust_thrcommon.c:30-34` — has `func`, `data`, `thread`
- [ ] `RustThreadHelper` at `rust_thrcommon.c:94-117` — captures return value: `result = (*func)(data)`; `return result`
- [ ] `WaitThread` at `rust_thrcommon.c:208-223` — currently writes `rust_thread_join` return (1/0) to `*status` (gap G1)
- [ ] `SleepThreadUntil` at `rust_thrcommon.c:192-200` — currently just computes delta, no async loop (gap G2)
- [ ] `StartThread_Core` at `rust_thrcommon.c:164-183` — uses `rust_thread_spawn`, NOT `rust_thread_spawn_detached` (gap G3)

### C header declarations match
- [ ] `rust_threads.h:33` — `rust_thread_spawn` declaration matches Rust export
- [ ] `rust_threads.h:34` — `rust_thread_spawn_detached` declaration matches Rust export
- [ ] `rust_threads.h:36` — `rust_thread_join` declaration (gap G1: lacks `out_status` param)

## Call-Path Feasibility

### G1: Return value propagation path
- [ ] C worker function returns `int` via `RustThreadHelper` → `result = (*func)(data)` → `return result` (rust_thrcommon.c:104,116)
- [ ] Rust `spawn_c_thread` closure calls `func(data)` but assigns result to `_` (mod.rs:949) — **THIS IS THE GAP**
- [ ] `Thread<()>` wraps `JoinHandle<()>` — cannot recover `c_int` after join
- [ ] `rust_thread_join` casts to `Thread<()>` — can only report success/failure, not the worker status value
- [ ] `WaitThread` in C writes the 1/0 boolean into `*status` (rust_thrcommon.c:218-220) — not the worker's return code
- [ ] Fix path: change `spawn_c_thread` to `Thread<c_int>`, capture return value, add `out_status` to `rust_thread_join`

### G2: SleepThreadUntil async pumping path
- [ ] Legacy `thrcommon.c:333-362` calls `Async_process()` in a loop before each sleep
- [ ] `Async_process` declared in `sc2/src/libs/callback/async.h`
- [ ] `Async_timeBeforeNextMs` declared in `sc2/src/libs/callback/async.h`
- [ ] `rust_thrcommon.c` line 16 already includes `"libs/async.h"` — functions are accessible
- [ ] Current `rust_thrcommon.c:192-200` does NOT call either function — **THIS IS THE GAP**
- [ ] Fix is entirely C-side — no Rust changes needed

### G3/G4: Detached spawn path
- [ ] `rust_thread_spawn_detached` exists at mod.rs:971-984 — calls `spawn_c_thread` and drops result via `let _ = ...`
- [ ] That `let _ =` drops `Result<Thread<()>>` — if `Ok`, the `Thread<()>` drops, which drops the inner `JoinHandle` without joining
- [ ] Dropping a `JoinHandle` in Rust does NOT join — the thread runs but its return value is lost and resources may not be reclaimed promptly
- [ ] `StartThread_Core` at rust_thrcommon.c:177 calls `rust_thread_spawn` — returns a `RustThread*` that the C code stores but never joins via `rust_thread_join` (lifecycle cleanup joins via `WaitThread` which does call `rust_thread_join`)
- [ ] Current state: `StartThread_Core` uses `rust_thread_spawn` (not detached) — this is intentionally kept as-is per G3/G4 design decision in overview

## Test Infrastructure Verification

- [ ] Test file exists: `rust/src/threading/tests.rs`
- [ ] Tests module included via `#[cfg(test)] mod tests;` in `mod.rs`
- [ ] Existing test categories cover: thread spawn/join, mutex, condvar, semaphore, task, system init, sleep/yield, TLS, error handling
- [ ] New tests for G1 will be added in P04 to verify `c_int` return value propagation through `Thread<c_int>` (not expected to exist yet)
- [ ] Existing tests use `Thread<()>` and `Thread<i32>` (via generics) — both must continue working

## Identified TODOs in Active Code

- [ ] `mod.rs:596` — `// TODO: Implement state retrieval` — code actually works (AtomicU32 load is implemented below)
- [ ] `mod.rs:611` — `// TODO: Implement state setting` — code actually works (AtomicU32 store is implemented below)
- [ ] `mod.rs:681` — `// TODO: Implement lifecycle processing` — spec says keep C-owned (G6)
- [ ] `mod.rs:~697` — `// TODO: Implement thread hibernation` — `thread::sleep` IS the implementation (G5)

## Blocking Issues

[To be filled during execution. If any check fails, STOP and revise the plan.]

## Gate Decision

- [ ] PASS: proceed to Phase 01
- [ ] FAIL: revise plan (list blocking issues)
