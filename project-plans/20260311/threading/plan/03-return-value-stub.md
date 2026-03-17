# Phase 03: Return Value Propagation — Stub

## Phase ID
`PLAN-20260314-THREADING.P03`

## Prerequisites
- Required: Phase 02a (Pseudocode Verification) completed
- Expected previous artifact: `project-plans/20260311/threading/.completed/P02a.md`
- All existing tests pass (1547+)

## Requirements Implemented (Expanded)

### Thread entry function return value propagation
**Requirement text**: “When a thread entry function returns an integer status, the threading subsystem shall preserve that status until the corresponding join operation consumes it.”

Behavior contract:
- GIVEN: A thread spawned via `CreateThread` with a `ThreadFunction` that returns `c_int`
- WHEN: The thread completes and `WaitThread` is called with a non-null status pointer
- THEN: `*status` contains the thread function's actual return value

Why it matters:
- C callers (task system, debug logging) expect the worker's return code, not a boolean success indicator
- Spec §10.3: the spawn/join contract must propagate the C thread function's `c_int` return value

### Adapter ABI staging rule for `rust_thread_join`
**Requirement text**: spec §10.2 defines the final adapter ABI as `rust_thread_join(thread: *mut RustThread, out_status: *mut c_int) -> c_int`

This phase does **not** change the exported ABI yet. It only makes the Rust-internal thread type carry `c_int` so Phase 05 can update Rust/C/header atomically.

## Implementation Tasks

### Final stub goal
Capture `c_int` in Rust now and defer the FFI signature change until Phase 05, where Rust/C/header are updated together. This phase makes the type system correct without introducing an ABI mismatch.

### Files to modify

#### `rust/src/threading/mod.rs`

1. **Change `spawn_c_thread` return type and closure** (around line 936-951):
   - Change signature: `-> Result<Thread<()>>` becomes `-> Result<Thread<c_int>>`
   - Change closure: `let _ = unsafe { func(data) };` becomes `unsafe { func(data) }` (returns c_int)
   - Pseudocode traceability: lines 01-09
   - marker: `@plan PLAN-20260314-THREADING.P03`

2. **Update `rust_thread_spawn` cast** (around line 953-969):
   - `Box::into_raw(Box::new(thread))` now stores `Thread<c_int>` instead of `Thread<()>`
   - No external ABI change in this phase

3. **Update `rust_thread_spawn_detached` compatibility** (around line 971-984):
   - Now drops `Result<Thread<c_int>>` instead of `Result<Thread<()>>`
   - No functional change in this phase

4. **Keep `rust_thread_join` external signature unchanged in this phase** (around line 997-1007):
   - Keep signature as `(thread: *mut RustThread) -> c_int`
   - Change internal cast: `Box::from_raw(thread as *mut Thread<()>)` becomes `Box::from_raw(thread as *mut Thread<c_int>)`
   - On `Ok(status)`, ignore `status` for now and still return 1/0 only
   - This preserves ABI safety until Phase 05 updates Rust/C/header simultaneously

### Pseudocode traceability
- Uses pseudocode lines: 01-10 (spawn_c_thread), 11-18 (rust_thread_spawn), 28-41 conceptually, with ABI change deferred to P05

## Verification Commands

```bash
cd /Users/acoliver/projects/uqm/rust
cargo fmt --all --check
cargo clippy --lib --all-features 2>&1 | grep -A2 "threading/mod.rs"  # scoped to threading
cargo test --lib --all-features
```

Notes:
- `--lib` is used instead of `--workspace` due to a pre-existing `input_integration_tests` linker failure (`ld: library 'uqm_rust' not found`) unrelated to this plan. Established in P00a.
- Clippy is scoped to threading module output because there are pre-existing FFI declaration conflicts in `wav_ffi.rs`/`io/ffi.rs` unrelated to this plan.

## Structural Verification Checklist
- [ ] `spawn_c_thread` returns `Result<Thread<c_int>>`
- [ ] `spawn_c_thread` closure returns the `c_int` value from `func(data)` (not `let _ =`)
- [ ] `rust_thread_spawn` compiles with `Thread<c_int>`
- [ ] `rust_thread_spawn_detached` compiles with `Thread<c_int>`
- [ ] `rust_thread_join` casts to `Thread<c_int>` not `Thread<()>`
- [ ] `rust_thread_join` signature is UNCHANGED (still `(thread: *mut RustThread) -> c_int`)
- [ ] No ABI mismatch between Rust and C
- [ ] No new compilation warnings
- [ ] All existing tests pass

## Semantic Verification Checklist (Mandatory)
- [ ] The Rust-internal `Thread<T>` generic still works for both `T=()` and `T=c_int`
- [ ] Existing Rust tests that use `Thread<()>` or `Thread<i32>` are unaffected
- [ ] The FFI export `rust_thread_join` is still callable from C without changes
- [ ] The `c_int` return value from the C function is now captured by the closure
- [ ] No placeholder/deferred implementation patterns introduced

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/threading/mod.rs | grep -v "test"
```

Pre-existing TODOs (to be cleaned in P08) are expected at lines ~587, ~602, ~672, ~688. No NEW TODOs should be introduced by this phase. The grep check should confirm the set is unchanged from preflight.

## Success Criteria
- [ ] Type change from `Thread<()>` to `Thread<c_int>` compiles cleanly
- [ ] All existing 1547+ tests pass
- [ ] `cargo clippy` clean
- [ ] `cargo fmt` clean
- [ ] No ABI break with C adapter

## Failure Recovery
- rollback: `git checkout -- rust/src/threading/mod.rs`
- blocking issues: if `Thread<c_int>` breaks existing test code, those tests need examination (they shouldn't — tests use `Thread::spawn` directly, not `spawn_c_thread`)

## Phase Completion Marker
Create: `project-plans/20260311/threading/.completed/P03.md` with phase ID, timestamp, files changed, verification commands run, outputs summary, semantic findings, and follow-up notes.
