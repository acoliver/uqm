# Phase 01: Analysis

## Phase ID
`PLAN-20260314-THREADING.P01`

## Prerequisites
- Required: Phase 00a (Preflight Verification) completed and passed
- Expected previous artifact: `project-plans/20260311/threading/.completed/P00a.md`
- All 1547+ existing tests pass

## Purpose

Map each identified gap to exact code locations, data flows, and integration boundaries. Produce the domain analysis artifacts required before pseudocode.

## Gap Analysis

### G1: Thread Return Value Propagation (Critical)

**Current data flow (broken):**
```text
C caller → CreateThread(func, data, ...) → RustThreadHelper(opaque)
  → func(data) returns int `result`
  → RustThreadHelper returns `result` to Rust
  → spawn_c_thread closure: `let _ = unsafe { func(data) };`  ← DISCARDS result
  → Thread<()> wraps JoinHandle<()>
  → rust_thread_join(thread) → thread.join() → Ok(()) → returns 1 to C
  → WaitThread writes 1 to *status  ← WRONG: should be `result`
```

**Required data flow (spec §10.3):**
```text
C caller → CreateThread(func, data, ...) → RustThreadHelper(opaque)
  → func(data) returns int `result`
  → RustThreadHelper returns `result` to Rust
  → spawn_c_thread closure: `unsafe { func(data) }`  ← CAPTURES result as c_int
  → Thread<c_int> wraps JoinHandle<c_int>
  → rust_thread_join(thread, out_status) → thread.join() → Ok(42) → writes 42 to *out_status, returns 1
  → WaitThread reads out_status into *status  ← CORRECT: actual return value
```

**Requirements traceability:**
- `requirements.md`: “When a thread entry function returns an integer status, the threading subsystem shall preserve that status until the corresponding join operation consumes it.”
- `requirements.md`: both `WaitThread` status-pointer requirements
- `requirements.md`: two-value adapter ABI limitation note

**Files requiring changes:**
- `rust/src/threading/mod.rs`:
  - `spawn_c_thread()` (line ~936): change return to `Result<Thread<c_int>>`, capture `func(data)` return
  - `rust_thread_spawn()` (line ~953): cast change from `Thread<()>` to `Thread<c_int>`
  - `rust_thread_spawn_detached()` (line ~971): now drops `Thread<c_int>`
  - `rust_thread_join()` (line ~997): add `out_status: *mut c_int` param, write actual value
- `sc2/src/libs/threads/rust_threads.h`:
  - Line 36: add `int* out_status` parameter to `rust_thread_join` declaration
- `sc2/src/libs/threads/rust_thrcommon.c`:
  - `WaitThread()` (line ~208): pass `&result` to `rust_thread_join`, then write `result` to `*status`

**Integration callers of `WaitThread` (must verify behavior preserved):**
- `rust_thrcommon.c:262` — `ProcessThreadLifecycles` calls `WaitThread(t, NULL)` — null status, no change
- `thrcommon.c:147` — legacy lifecycle cleanup, also calls `WaitThread(t, NULL)` — guarded out by `USE_RUST_THREADS`
- No current repository callers pass a non-NULL status pointer to `WaitThread`. The API supports it, but no engine code exercises that path today.

**Additional ABI touchpoint for G1:** The local `extern int rust_thread_join(RustThread* thread);` declaration at `rust_thrcommon.c:45` must also be updated to match the new signature with `out_status`.

**Risk assessment:** Moderate verification risk unless adapter/public-API behavior is tested directly. The type change from `Thread<()>` to `Thread<c_int>` is internal to the FFI path, but the parity bug lived at the C↔Rust adapter boundary rather than in Rust generics themselves.

### G2: SleepThreadUntil Async Pumping (Critical)

**Current behavior:** Single-shot sleep — compute `wakeTime - now`, call `SleepThread`, return.

**Required behavior (spec §6.5, legacy thrcommon.c:333-362):** Loop that:
1. Calls `Async_process()` to service pending async work
2. Checks if `wakeTime <= now` — if so, return
3. Queries `Async_timeBeforeNextMs()` for next scheduled event
4. Sleeps until `min(wakeTime, nextAsyncTime)`
5. Repeats

**Requirements traceability:**
- `requirements.md`: all three `SleepThreadUntil` async-pumping obligations

**Files requiring changes:**
- `sc2/src/libs/threads/rust_thrcommon.c`:
  - `SleepThreadUntil()` (lines 192-200): replace body with async-pumping loop

**No Rust changes needed.** `Async_process()` and `Async_timeBeforeNextMs()` are C functions declared in `libs/async.h`, already included at rust_thrcommon.c:16.

**Integration impact:** This restores behavior relied on by the main-thread sleep path. Audio callbacks, input processing, and timer-driven operations depend on `Async_process()` being called regularly.

**Verification implication:** build/grep comparison is not enough by itself because the regression is behavioral. The plan must include either an executable harness or a concrete manual validation procedure that demonstrates repeated pumping and wake-time clamping semantics.

### G3: StartThread_Core Detached Reference-Design Mismatch (High)

**Current behavior:** `StartThread_Core` at rust_thrcommon.c:177 calls `rust_thread_spawn()`, which allocates a `RustThread*` handle. The C code stores this in `thread->native` and later consumes it through `ProcessThreadLifecycles → WaitThread → rust_thread_join`.

**Reference-design tension:** spec §2.5 says `StartThread_Core` should use `rust_thread_spawn_detached`, because the caller does not receive a joinable handle.

**Why the current design cannot simply switch today:**
1. `RustThreadHelper` calls `FinishThread(thread)` which enqueues the C-side `TrueThread`
2. `ProcessThreadLifecycles` calls `WaitThread(t, NULL)`
3. `WaitThread` only joins when `t->native != NULL`
4. Therefore the existing lifecycle path depends on `StartThread_Core` storing a valid `RustThread*`

**Plan conclusion:** keep `StartThread_Core` on `rust_thread_spawn` and document why. This closes the misleading-spec/reference-design tension in the plan, but it does **not** mean detached-failure semantics are solved.

**Requirements traceability:**
- `requirements.md`: non-joinable thread lifecycle cleanup obligations
- `requirements.md`: `StartThread` must preserve distinction from joinable creation
- spec §2.5 `[Reference design]` note is documented but not treated as normative by itself

### G4: Detached-thread creation failure semantics are not closed by a Rust-only helper cleanup (High)

**Current behavior:** `rust_thread_spawn_detached` at mod.rs:971-984:
```rust
let _ = spawn_c_thread(name_str, func, data);
```
This obscures intent and swallows the `Result`.

**What a trivial fix can do:**
- Make detach intent explicit via `match`
- Avoid silent error swallowing in code style terms
- Improve documentation

**What it cannot do:**
It cannot satisfy the normative detached-thread creation failure contract from spec §2.5 / requirements.md, because `rust_thread_spawn_detached` returns `void`. If the C adapter allocated `thread` / `startInfo` before calling into Rust and spawn fails, the adapter has no synchronous failure signal with which to reclaim those allocations before `StartThread` returns.

**Why this matters:**
- `requirements.md`: failed `StartThread` creation must leave no leaked internal resources and no lifecycle-visible worker state behind
- spec §2.5 says adapter-owned wrapper cleanup must happen before `StartThread` returns
- a Rust-side `Err(_) => { ... }` comment saying “caller must handle cleanup” does not make that possible under the current ABI

**Plan conclusion:**
- P07 may improve `rust_thread_spawn_detached` readability/documentation
- P07 must not claim detached-creation failure semantics are solved
- The detached-failure contract mismatch remains an acknowledged ABI/design blocker outside this plan’s implementation scope
- Full parity requires a follow-up design decision with concrete ABI/ownership changes, not just comment cleanup

**Requirements traceability:**
- `requirements.md`: `StartThread` failure contract paragraph
- `requirements.md`: non-joinable thread no-leak requirement
- spec §2.5 detached-thread creation failure contract

### G5: Stale TODO Markers (Medium)

**Locations:**
1. `mod.rs:596` — `// TODO: Implement state retrieval` — code below DOES implement it via `AtomicU32::load`
2. `mod.rs:611` — `// TODO: Implement state setting` — code below DOES implement it via `AtomicU32::store`
3. `mod.rs:681` — `// TODO: Implement lifecycle processing` — spec says keep C-owned
4. `mod.rs:696` — `// TODO: Implement thread hibernation` — `thread::sleep(duration)` IS the implementation

**Fix:** Remove stale TODOs. For lifecycle processing, add documentation explaining why it's a no-op.

### G6: Lifecycle Processing Stub (Low)

`process_thread_lifecycles()` at mod.rs:676-683 has a TODO suggesting it needs implementation. Per spec §2.4: “This lifecycle bookkeeping remains C-owned in rust_thrcommon.c. The Rust subsystem does not need to replicate the pendingBirth/pendingDeath arrays. The Rust-side process_thread_lifecycles() stub may be removed or left as a no-op.”

**Fix:** Document as intentional no-op, remove TODO.

### G7: Stale Comment in C Adapter (Low)

`rust_thrcommon.c:362-364`:
```c
/* Rust std::sync::Mutex is not recursive; using regular mutex */
```
This is factually wrong in the context of `CreateRecursiveMutex_Core`. `RustFfiMutex` at mod.rs:205-289 supports recursive locking with owner tracking and depth counting for the recursive-mutex path.

**Requirements traceability:**
- `requirements.md`: recursive mutex behavior requirements
- `requirements.md`: plain `Mutex` recursion policy remains unresolved until audit completion

**Fix:** Update comment to reflect recursive-mutex behavior without implying that the public contract for plain `Mutex` is now settled.

## Entity/State Summary

### Thread handle states (per spec §2.6)
- **Active** → `WaitThread` → **Joined** → `DestroyThread` → **Destroyed**
- **Active** → `WaitThread` (fails) → **Join-failed** → `DestroyThread` → **Destroyed**

### Data flow through FFI boundary
```text
C ThreadFunction returns int
  → RustThreadHelper captures int, returns to Rust
  → Rust closure returns c_int (after G1 fix)
  → Thread<c_int>.join() → Ok(c_int)
  → rust_thread_join writes c_int to *out_status
  → WaitThread reads out_status, writes to caller's *status
```

## Explicit "Old Code to Replace" List

| Location | Current | Target |
|----------|---------|--------|
| `mod.rs:949` | `let _ = unsafe { func(data) };` | `unsafe { func(data) }` (capture return) |
| `mod.rs:936` | `-> Result<Thread<()>>` | `-> Result<Thread<c_int>>` |
| `mod.rs:997-1007` | `rust_thread_join(thread) -> c_int` | `rust_thread_join(thread, out_status) -> c_int` |
| `rust_threads.h:36` | `int rust_thread_join(RustThread*)` | `int rust_thread_join(RustThread*, int* out_status)` |
| `rust_thrcommon.c:218-220` | `*status = result` (where result = 1/0) | `*status = out_status` (where out_status = actual value) |
| `rust_thrcommon.c:192-200` | Single-shot `SleepThread` | Async-pumping loop |
| `rust_thrcommon.c:177` | `rust_thread_spawn(name, ...)` with no rationale | same call, but documented as intentional because lifecycle join path depends on handle |
| `mod.rs:971-984` | detached helper hides `Result` via `let _ =` | explicit `match` plus scope note that detached-failure contract is unresolved at current ABI |
| `rust_thrcommon.c:362-364` | Stale "not recursive" comment | Accurate recursive-mutex-specific comment that does not settle plain `Mutex` semantics |
| `mod.rs:596,611,681,696` | Stale TODO comments | Removed or replaced with docs |

## Integration Touchpoints

All integration callers that may be affected (verified safe for each gap):

1. **DCQ** (`dcqueue.c:55-60`): Uses `GetRecursiveMutexDepth`, `WaitCondVar`, `BroadcastCondVar` — NOT affected by any gap
2. **TFB Draw** (`tfb_draw.c:201-212`): Uses `GetMyThreadLocal()->flushSem` — NOT affected
3. **Task system** (`tasklib.c:40-42`): Uses `CreateThread` and `TaskSwitch` — does NOT directly call `WaitThread` (thread lifecycle is handled through `FinishTask`/`ConcludeTask` patterns)
4. **Audio stream** (`stream.c:571-576`): Uses `HibernateThread`, `TaskSwitch` — NOT affected
5. **Mixer** (`mixer.c:110-112`): Uses recursive mutexes — NOT affected
6. **Callbacks** (`callback.c:43-73`): Uses plain mutex — current plan does not modify plain-mutex behavior; the spec still carries an unresolved plain-mutex recursion blocker and downstream consumers must continue to treat plain `Mutex` as non-recursive
7. **Logging** (`uqmlog.c:56-162`): Uses plain mutex — same plain-mutex blocker note applies; no behavior change is claimed here
8. **Main entry** (`sc2/src/uqm.c:454`): Only actual `StartThread(...)` caller in the codebase — `StartThread(Starcon2Main, NULL, 1024, "Starcon2Main")`
9. **SleepThreadUntil callers**: ~40+ call sites across gameplay, UI, and engine modules (credits, encounters, starbase, comm, shipyard, setup menu, planets, battle, lander, FMV, melee, intro, restart). G2 fix restores async pumping for all of these.

## Success Criteria
- [ ] All gaps mapped to exact file/line changes
- [ ] Data flow diagrams show complete path for each fix
- [ ] Integration points verified as safe or identified as requiring attention
- [ ] Detached-thread failure contract explicitly identified as unresolved by current ABI, not falsely marked solved
- [ ] Plain-mutex recursion blocker is acknowledged where relevant and not accidentally collapsed into recursive-mutex implementation detail
- [ ] "Old code to replace" list is complete and specific

## Phase Completion Marker
Create: `project-plans/20260311/threading/.completed/P01.md` with phase ID, timestamp, files changed, verification commands run, outputs summary, semantic findings, and follow-up notes.
