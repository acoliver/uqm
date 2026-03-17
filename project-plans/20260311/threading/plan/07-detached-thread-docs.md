# Phase 07: Detached Thread Documentation + Scoped Helper Cleanup

## Phase ID
`PLAN-20260314-THREADING.P07`

## Prerequisites
- Required: Phase 06a (SleepThreadUntil Verification) completed
- Expected previous artifact: `project-plans/20260311/threading/.completed/P06a.md`
- All tests pass
- C project builds cleanly

## Scope Note

This phase is intentionally narrower than earlier drafts. It does **not** claim to satisfy the full detached-thread creation failure contract from `requirements.md` / spec §2.5, because that contract is not implementable through the current `rust_thread_spawn_detached() -> void` ABI alone. This phase only:
- documents why `StartThread_Core` keeps using `rust_thread_spawn`
- makes detached-helper intent explicit in Rust
- fixes the stale recursive-mutex comment

## Requirements Implemented (Expanded)

### StartThread_Core lifecycle-handle decision
**Traceability**: spec §2.4 and §2.5; requirements.md lifecycle cleanup obligations for non-joinable threads.

`StartThread_Core` correctly keeps using `rust_thread_spawn` because:
1. `RustThreadHelper` calls `FinishThread(thread)` → enqueues in `pendingDeath`
2. `ProcessThreadLifecycles` calls `WaitThread(t, NULL)` → calls `rust_thread_join(t->native, NULL)`
3. Without `t->native`, the join is skipped and the current lifecycle path loses its cleanup handle

This phase documents that design choice. It does not change the runtime behavior.

### Detached helper cleanup/style improvement
**Traceability**: requirements.md non-joinable no-leak rule; spec §2.5 detached helper discussion.

The current `rust_thread_spawn_detached` silently swallows spawn failures via `let _ = spawn_c_thread(...)`. This phase improves code clarity by making detach intent explicit via `match`. However, this is only a style/documentation cleanup plus bounded error handling at the Rust entry point. It does **not** solve adapter-owned wrapper cleanup on detached creation failure.

### Stale recursive-mutex comment
**Traceability**: requirements.md recursive mutex behavior requirements; spec §3.2.

The recursive-mutex comment fix must remain narrow. The spec still treats plain `Mutex` recursion policy as an unresolved blocker until audit completion, so this phase must not word the comment in a way that implies the public contract for plain `Mutex` is now settled.

## Implementation Tasks

### Files to modify

#### 1. `rust/src/threading/mod.rs` — Improve `rust_thread_spawn_detached`

**Current** (lines 971-984):
```rust
#[no_mangle]
pub unsafe extern "C" fn rust_thread_spawn_detached(
    name: *const c_char,
    func: unsafe extern "C" fn(*mut c_void) -> c_int,
    data: *mut c_void,
) {
    let name_str = if name.is_null() {
        None
    } else {
        CStr::from_ptr(name).to_str().ok()
    };

    let _ = spawn_c_thread(name_str, func, data);
}
```

**Target** (pseudocode lines 79-92):
```rust
/// Spawn a thread with no caller-visible join handle.
///
/// Dropping the JoinHandle intentionally detaches the Rust thread.
/// This helper contains spawn failure within the subsystem boundary,
/// but the current ABI does not provide synchronous failure reporting
/// back to the C caller for reclaiming adapter-owned detached-thread
/// wrapper allocations.
#[no_mangle]
pub unsafe extern "C" fn rust_thread_spawn_detached(
    name: *const c_char,
    func: unsafe extern "C" fn(*mut c_void) -> c_int,
    data: *mut c_void,
) {
    let name_str = if name.is_null() {
        None
    } else {
        CStr::from_ptr(name).to_str().ok()
    };

    match spawn_c_thread(name_str, func, data) {
        Ok(_thread) => {
            /* intentional detach via drop at scope end */
        }
        Err(_) => {
            /* contained failure; no panic/exception exposure */
        }
    }
}
```

#### 2. `sc2/src/libs/threads/rust_thrcommon.c` — Document `StartThread_Core` design choice

Add a comment to `StartThread_Core` explaining why `rust_thread_spawn` (not `rust_thread_spawn_detached`) is used:

```c
    /* rust_thread_spawn (not rust_thread_spawn_detached) is intentional here.
     * ProcessThreadLifecycles -> WaitThread -> rust_thread_join needs
     * thread->native to hold a valid RustThread* for the current cleanup path.
     * The detached-spawn failure contract in the spec would require a different
     * ABI/design if adapter-owned wrapper cleanup on failure is to be guaranteed. */
    thread->native = rust_thread_spawn (name, RustThreadHelper, startInfo);
```

#### 3. `sc2/src/libs/threads/rust_thrcommon.c` — Fix stale recursive mutex comment (G7)

**Current**:
```c
RecursiveMutex
CreateRecursiveMutex_Core (const char *name, DWORD syncClass)
{
    /* Rust std::sync::Mutex is not recursive; using regular mutex */
    (void)syncClass;
    return (RecursiveMutex)rust_mutex_create(name);
}
```

**Target:**
```c
RecursiveMutex
CreateRecursiveMutex_Core (const char *name, DWORD syncClass)
{
    /* RustFfiMutex supports recursive locking with owner tracking and depth counting.
     * This comment describes the recursive-mutex path only; plain Mutex semantics
     * remain governed by the separate audit blocker in the specification. */
    (void)syncClass;
    return (RecursiveMutex)rust_mutex_create(name);
}
```

### Detached-thread parity follow-up recorded here

This phase does not implement the detached-thread failure contract, but it must record concrete next-step options for the required follow-up design work:
- revise `rust_thread_spawn_detached` to return success/failure synchronously,
- change wrapper-allocation order so adapter-owned objects are not committed before the last synchronous failure point,
- or move detached-start ownership/publication semantics so failed starts cannot leave adapter-visible wrapper state behind.

### Pseudocode traceability
- Uses pseudocode lines: 66-78 (StartThread_Core documentation), 79-92 (detached helper scope note)

## Verification Commands

```bash
# Rust tests
cd /Users/acoliver/projects/uqm/rust
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features

# C build
cd /Users/acoliver/projects/uqm/sc2
make -f Makefile.build
```

## Structural Verification Checklist
- [ ] `rust_thread_spawn_detached` uses explicit `match` instead of `let _ =`
- [ ] `rust_thread_spawn_detached` has doc comment explaining detach semantics and current ABI limitation
- [ ] `StartThread_Core` has comment explaining why `rust_thread_spawn` is used
- [ ] `StartThread_Core` comment explicitly notes detached-failure contract remains an ABI/design issue
- [ ] Recursive mutex comment corrected in `CreateRecursiveMutex_Core`
- [ ] Recursive mutex comment does not imply the public contract for plain `Mutex` is settled
- [ ] Follow-up ABI/design options for detached-thread parity are explicitly recorded

## Semantic Verification Checklist (Mandatory)
- [ ] `rust_thread_spawn_detached` functional behavior is unchanged for successful detached spawn (drop = detach)
- [ ] Error path in `rust_thread_spawn_detached` does not panic or expose exceptions
- [ ] `StartThread_Core` call path is UNCHANGED (still calls `rust_thread_spawn`)
- [ ] `ProcessThreadLifecycles` → `WaitThread` → `rust_thread_join` path still works
- [ ] This phase does NOT claim detached-thread creation failure cleanup is solved
- [ ] All tests pass
- [ ] C project compiles cleanly

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/threading/mod.rs | grep -v "test"
```

Pre-existing TODOs (cleaned in P08) expected. No NEW TODOs.

## Success Criteria
- [ ] Detached helper intent is explicit and documented
- [ ] StartThread_Core design choice is documented honestly
- [ ] Recursive mutex comment is accurate without overclaiming plain-mutex semantics
- [ ] Detached-thread ABI/design follow-up options are concretely recorded
- [ ] All tests pass
- [ ] No false claim that detached-failure requirement is closed

## Failure Recovery
- rollback: `git checkout -- rust/src/threading/mod.rs sc2/src/libs/threads/rust_thrcommon.c`

## Phase Completion Marker
Create: `project-plans/20260311/threading/.completed/P07.md` with phase ID, timestamp, files changed, verification commands run, outputs summary, semantic findings, and follow-up notes.
