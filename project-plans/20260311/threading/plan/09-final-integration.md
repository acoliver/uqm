# Phase 09: Final Integration Verification

## Phase ID
`PLAN-20260314-THREADING.P09`

## Prerequisites
- Required: Phase 08a (TODO Cleanup Verification) completed
- Expected previous artifact: `project-plans/20260311/threading/.completed/P08a.md`
- All phases P03-P08 completed and verified
- All tests pass

## Requirements Verified

This phase verifies that all in-scope gaps are implemented or explicitly left open with rationale. No new code is written — this is a verification-only phase.

### Gap Status Summary

| Gap | Description | Addressed In | End-of-plan status | Traceability |
|-----|-------------|-------------|--------------------|-------------|
| G1 | Thread return value propagation | P03-P05 | **Implemented** | requirements.md thread-result / `WaitThread` obligations; spec §2.2, §10.2, §10.3 |
| G2 | SleepThreadUntil async pumping | P06 | **Implemented** | requirements.md `SleepThreadUntil` async-pumping obligations; spec §6.5 |
| G3 | StartThread_Core spawn routing documented honestly | P07 | **Clarified / documented** | non-joinable lifecycle cleanup obligations; spec §2.4 / §2.5 |
| G4 | Detached helper cleanup/documentation scope corrected | P07 | **Still open — blocked by detached ABI/design mismatch** | `StartThread` failure contract and non-joinable no-leak requirement; spec §2.5 |
| G5 | Stale TODO markers | P08 | **Implemented** | scoped plan cleanup |
| G6 | Lifecycle stub documentation | P08 | **Implemented** | requirements.md lifecycle obligations; spec §2.4 |
| G7 | Stale recursive mutex comment | P07 | **Implemented** | requirements.md recursive mutex obligations; spec §3.2 |

## Integration Verification Tasks

### 1. Full Rust verification suite

```bash
cd /Users/acoliver/projects/uqm/rust
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

- [ ] Format clean
- [ ] Clippy clean (zero warnings)
- [ ] All tests pass (baseline + 4 Rust-generic tests from P04 + adapter/public-API coverage from P05)

### 2. Full C build verification

```bash
cd /Users/acoliver/projects/uqm/sc2
make -f Makefile.build
```

- [ ] C project compiles without errors
- [ ] No warnings in `rust_thrcommon.c`
- [ ] No warnings in any file referencing `rust_thread_join`

### 3. ABI consistency audit

Verify all FFI signatures match between Rust, C header, and C adapter:

```bash
# rust_thread_join — must have out_status parameter in all three
grep -n "rust_thread_join" /Users/acoliver/projects/uqm/rust/src/threading/mod.rs
grep -n "rust_thread_join" /Users/acoliver/projects/uqm/sc2/src/libs/threads/rust_threads.h
grep -n "rust_thread_join" /Users/acoliver/projects/uqm/sc2/src/libs/threads/rust_thrcommon.c

# All other FFI functions — spot check
grep -n "rust_thread_spawn\b" /Users/acoliver/projects/uqm/rust/src/threading/mod.rs | head -3
grep -n "rust_thread_spawn\b" /Users/acoliver/projects/uqm/sc2/src/libs/threads/rust_threads.h
```

- [ ] `rust_thread_join` signature matches in all three files (2 params + return)
- [ ] All other FFI signatures are unchanged and consistent

### 4. Scoped TODO cleanup audit

```bash
grep -n -E "TODO|FIXME|HACK|placeholder|for now|will be implemented" \
  /Users/acoliver/projects/uqm/rust/src/threading/mod.rs
```

- [ ] The four targeted stale markers from Phase 08 remain removed
- [ ] Any remaining TODO-like strings are reviewed and are outside this plan's scoped cleanup work
- [ ] No new TODO-like markers were introduced by this plan's changes

### 5. Integration point verification

For each integration point from the specification:

#### Graphics — DCQ and flush semaphore (Spec §8.1)
- [ ] `GetRecursiveMutexDepth` returns accurate depth (unchanged)
- [ ] `WaitCondVar` blocks correctly with self-contained internal mutex (unchanged)
- [ ] `BroadcastCondVar` wakes all waiters (unchanged)
- [ ] `GetMyThreadLocal()->flushSem` returns valid semaphore (unchanged)
- [ ] `WaitThread` now propagates actual thread return value (G1 — NEW)

#### Audio — mixer recursive mutexes (Spec §8.2)
- [ ] Recursive mutexes support nested acquisition (unchanged)
- [ ] Non-owner unlock fails safely (unchanged)
- [ ] Works for threads not spawned by threading subsystem (SDL audio callback) (unchanged)

#### Audio — stream thread throttling (Spec §8.3)
- [ ] `HibernateThread` sleeps for requested duration (unchanged)
- [ ] `TaskSwitch` yields ~1ms (unchanged)

#### Task system (Spec §8.4)
- [ ] `CreateThread` returns valid joinable handle (unchanged)
- [ ] `TaskSwitch` yields for short cooperative delay (unchanged)

#### SleepThreadUntil — async pumping (Spec §6.5)
- [ ] `Async_process()` called in loop (G2 — NEW)
- [ ] Sleep interval bounded by next async event time (G2 — NEW)
- [ ] Returns when `wakeTime <= now` (G2 — NEW)
- [ ] Behavioral validation evidence from P06/P06a is present, not just grep/build review

#### Plain mutex blocker remains visible
- [ ] Callback/logging integration review does not treat the recursive-mutex implementation detail as settling the public recursion contract for plain `Mutex`
- [ ] Open plain-mutex audit blocker remains called out explicitly

### 6. Return value data flow end-to-end

Trace the complete path:
1. C code calls `CreateThread(func, data, stackSize, name)`
2. `CreateThread_Core` → `rust_thread_spawn(name, RustThreadHelper, startInfo)`
3. `RustThreadHelper` calls `func(data)` and captures `int result`
4. `RustThreadHelper` calls `FinishThread(thread)` and returns `result`
5. Rust closure in `spawn_c_thread` captures the `c_int` return
6. Thread completes; `JoinHandle<c_int>` holds the value
7. C code calls `WaitThread(t, &status)`
8. `WaitThread` calls `rust_thread_join(t->native, &out_status)`
9. `rust_thread_join` calls `thread.join()` → `Ok(status_value)`
10. `rust_thread_join` writes `status_value` to `*out_status`, returns 1
11. `WaitThread` writes `out_status` to `*status`

- [ ] Every step in this chain is implemented
- [ ] Return value is preserved through all 11 steps
- [ ] Adapter/public-API verification evidence exists for positive, zero, and negative status values

### 7. Existing test preservation

```bash
cd /Users/acoliver/projects/uqm/rust
cargo test --workspace --all-features 2>&1 | tail -5
```

- [ ] Total test count = baseline recorded in P00a + plan-added tests
- [ ] Zero failures

## Structural Verification Checklist
- [ ] All files modified per plan are accounted for:
  - `rust/src/threading/mod.rs` (P03, P05, P07, P08)
  - `rust/src/threading/tests.rs` (P04 and/or P05 if adapter-level Rust tests are added there)
  - `sc2/src/libs/threads/rust_thrcommon.c` (P05, P06, P07)
  - `sc2/src/libs/threads/rust_threads.h` (P05)
- [ ] No files outside the plan scope were modified

## Semantic Verification Checklist (Mandatory)
- [ ] G1: Thread return values propagate end-to-end (C func → WaitThread *status)
- [ ] G2: SleepThreadUntil pumps async queue matching legacy thrcommon.c
- [ ] G3: StartThread_Core design choice documented, uses rust_thread_spawn intentionally
- [ ] G4: rust_thread_spawn_detached has explicit error handling/docs, but detached-thread creation failure semantics remain an unresolved ABI/design blocker rather than a closed requirement
- [ ] G5: All targeted stale TODOs in active code removed
- [ ] G6: process_thread_lifecycles() documented as intentional no-op
- [ ] G7: Recursive mutex comment is accurate without overclaiming plain-mutex semantics
- [ ] All existing call sites (graphics, audio, task, callback, logging) unaffected within the scoped changes
- [ ] No new API surface exposed
- [ ] No behavioral regressions in the implemented slices

## Open Items (Not Addressed by This Plan)

These are spec-acknowledged open items. They remain open after this plan and must not be collapsed into a “fully closed subsystem” claim:

1. **Plain mutex recursion policy** (Spec §3.1) — blocked on call-site audit. Interim rule (non-recursive) applies.
2. **Stack size handling** (Spec §2.1) — blocked on call-site audit. Currently ignored.
3. **Deferred creation compatibility** (Spec §2.1) — blocked on call-site audit. Current behavior (immediate creation) is interim normative.
4. **Thread naming audit** (Spec §2.1) — best-effort currently. Audit not yet performed.
5. **Detached-thread creation failure contract vs current detached ABI** (Spec §2.5 / requirements.md) — current `rust_thread_spawn_detached() -> void` ABI cannot, by itself, guarantee adapter-owned wrapper cleanup before `StartThread` returns. This plan documents the mismatch but does not redesign the ABI.

## Success Criteria
- [ ] In-scope implemented gaps are verified implemented
- [ ] Open normative requirements are explicitly reported as open/blockers, not counted as closed
- [ ] All Rust tests pass
- [ ] C project builds cleanly
- [ ] ABI is consistent
- [ ] Scoped stale TODO cleanup verified
- [ ] Integration points verified
- [ ] Open items documented honestly and prominently

## Phase Completion Marker
Create: `project-plans/20260311/threading/.completed/P09.md` with phase ID, timestamp, files changed, verification commands run, outputs summary, semantic findings, and follow-up notes.
