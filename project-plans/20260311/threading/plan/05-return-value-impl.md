# Phase 05: Return Value Propagation — Implementation

## Phase ID
`PLAN-20260314-THREADING.P05`

## Prerequisites
- Required: Phase 04a (TDD Verification) completed
- Expected previous artifact: `project-plans/20260311/threading/.completed/P04a.md`
- `Thread<c_int>` tests passing
- `spawn_c_thread` already returns `Thread<c_int>` (from P03)
- All existing tests pass

## Requirements Implemented (Expanded)

### Thread return value propagation
**Full requirement**: The `rust_thread_join` FFI function must use a two-value return convention: a `c_int` return value (1=success, 0=failure) and an `out_status: *mut c_int` through which the thread function's return value is written on success.

### Adapter ABI update for `rust_thread_join`
**Full requirement**: The `rust_thread_join` signature must change from `(RustThread*) -> int` to `(RustThread*, int* out_status) -> int` per spec §10.2.

## Implementation Tasks

This phase changes BOTH the Rust FFI function AND the C adapter/header simultaneously to maintain ABI compatibility.

### Files to modify

#### 1. `rust/src/threading/mod.rs` — Update `rust_thread_join`

**Current** (around line 997-1007):
```rust
#[no_mangle]
pub unsafe extern "C" fn rust_thread_join(thread: *mut RustThread) -> c_int {
    if thread.is_null() {
        return 0;
    }
    let thread: Box<Thread<()>> = Box::from_raw(thread as *mut Thread<()>);
    match thread.join() {
        Ok(()) => 1,
        Err(_) => 0,
    }
}
```

**Target** (pseudocode lines 28-41):
```rust
/// @plan PLAN-20260314-THREADING.P05
#[no_mangle]
pub unsafe extern "C" fn rust_thread_join(
    thread: *mut RustThread,
    out_status: *mut c_int,
) -> c_int {
    if thread.is_null() {
        if !out_status.is_null() {
            *out_status = 0;
        }
        return 0;
    }
    let thread: Box<Thread<c_int>> = Box::from_raw(thread as *mut Thread<c_int>);
    match thread.join() {
        Ok(status) => {
            if !out_status.is_null() {
                *out_status = status;
            }
            1
        }
        Err(_) => {
            if !out_status.is_null() {
                *out_status = 0;
            }
            0
        }
    }
}
```

#### 2. `sc2/src/libs/threads/rust_threads.h` — Update declaration

**Current** (line 36):
```c
extern int rust_thread_join(RustThread* thread);
```

**Target** (pseudocode line 53):
```c
extern int rust_thread_join(RustThread* thread, int* out_status);
```

#### 3. `sc2/src/libs/threads/rust_thrcommon.c` — Update `WaitThread`

**Current** (lines 208-223):
```c
void
WaitThread (Thread thread, int *status)
{
    TrueThread t = (TrueThread) thread;
    if (status)
        *status = 0;
    if (t && t->native)
    {
        int result = rust_thread_join (t->native);
        if (status)
            *status = result;
        t->native = NULL;
    }
}
```

**Target** (pseudocode lines 42-52):
```c
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
```

#### 4. `sc2/src/libs/threads/rust_thrcommon.c` — Update forward declaration

**Current** (line 45):
```c
extern int rust_thread_join(RustThread* thread);
```

**Target**:
```c
extern int rust_thread_join(RustThread* thread, int* out_status);
```

#### 5. Adapter/public-API verification additions

Because the original regression lived at the FFI boundary, this phase must verify the adapter path directly rather than relying only on Rust-generic tests from P04.

Acceptable verification options for execution:
1. Add a Rust test that invokes exported `rust_thread_spawn` / `rust_thread_join` unsafely with an `extern "C"` callback returning controlled `c_int` values.
2. Or add a focused C-side test/harness under the existing build/test structure that exercises `CreateThread` / `WaitThread(&status)` end-to-end.

Minimum required cases for whichever path is used:
- worker returns `42` → adapter/public API observes `42`
- worker returns `0` → adapter/public API observes `0`, while the adapter-level success signal remains distinguishable internally
- worker returns `-1` → adapter/public API observes `-1`
- `WaitThread(thread, NULL)` remains valid

### Pseudocode traceability
- Uses pseudocode lines: 28-41 (rust_thread_join), 42-52 (WaitThread), 53 (header)

## Verification Commands

```bash
# Rust tests
cd /Users/acoliver/projects/uqm/rust
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features

# C project build (verify ABI compatibility)
cd /Users/acoliver/projects/uqm/sc2
make -f Makefile.build
```

## Structural Verification Checklist
- [ ] `rust_thread_join` has `out_status: *mut c_int` parameter
- [ ] `rust_thread_join` casts to `Thread<c_int>` (not `Thread<()>`)
- [ ] `rust_thread_join` writes actual status on success, 0 on failure
- [ ] `rust_threads.h` declaration matches Rust signature
- [ ] `rust_thrcommon.c` forward declaration matches Rust signature
- [ ] `WaitThread` passes `&out_status` to `rust_thread_join`
- [ ] `WaitThread` writes `out_status` (actual thread return) to `*status` on success
- [ ] At least one adapter/public-API verification path exists in addition to the P04 Rust-generic tests

## Semantic Verification Checklist (Mandatory)
- [ ] A thread returning 42 → `WaitThread` writes 42 to `*status`
- [ ] A thread returning 0 → `WaitThread` writes 0 to `*status` (indistinguishable from failure at public API — this is the spec-acknowledged legacy limitation §2.2)
- [ ] A thread returning -1 → `WaitThread` writes -1 to `*status`
- [ ] Adapter-level verification demonstrates the two-value convention still distinguishes success-with-zero from join failure at the FFI boundary
- [ ] `WaitThread(t, NULL)` works without crash (null status pointer)
- [ ] `ProcessThreadLifecycles → WaitThread(t, NULL)` still works
- [ ] All existing tests pass
- [ ] No placeholder/deferred implementation patterns

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/threading/mod.rs | grep -v "test"
```

No NEW TODOs. Pre-existing TODOs (to be cleaned in P08) are expected.

## Success Criteria
- [ ] Thread return values propagate through the full path: C func → Rust closure → Thread<c_int> → join → out_status → WaitThread *status
- [ ] Adapter/public-API behavior is verified directly, not inferred only from generic `Thread<T>` tests
- [ ] All Rust tests pass (baseline + 4 from P04 + adapter/public-API coverage added here)
- [ ] C project compiles cleanly with updated header
- [ ] ABI is consistent across all three files (mod.rs, rust_threads.h, rust_thrcommon.c)

## Failure Recovery
- rollback: `git checkout -- rust/src/threading/mod.rs sc2/src/libs/threads/rust_threads.h sc2/src/libs/threads/rust_thrcommon.c`
- blocking: ABI mismatch between C and Rust — all three files must be updated atomically

## Phase Completion Marker
Create: `project-plans/20260311/threading/.completed/P05.md` with phase ID, timestamp, files changed, verification commands run, outputs summary, semantic findings, and follow-up notes.
