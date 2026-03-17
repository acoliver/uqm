# Plan: Threading Subsystem Targeted Gap Remediation

Plan ID: PLAN-20260314-THREADING
Generated: 2026-03-14
Total Phases: 18 (00–00a preflight, 01–01a analysis, 02–02a pseudocode, 03–09a implementation slices)
Requirements source: `project-plans/20260311/threading/requirements.md`
Requirement IDs covered from requirements.md: none (the requirements document uses EARS statements without REQ-* identifiers)
Plan gap labels: G1–G7 (plan-internal traceability labels, not requirements.md IDs)

## Context

The threading subsystem is **already ported and wired**. All `USE_RUST_THREADS` defines are active, the original `thrcommon.c` is compiled out, and 1547+ unit tests pass. This plan does NOT reimplement any working functionality. It remediates the remaining targeted specification parity gaps identified in `initialstate.md` against `specification.md` and `requirements.md`.

This is **not** a full subsystem sign-off plan. Several normative requirement groups remain outside this implementation scope and are tracked explicitly below so the plan cannot be misread as claiming subsystem-wide closure.

## Critical Reminders

Before implementing any phase:
1. Preflight verification is complete (Phase 00a)
2. Integration points are explicitly listed
3. TDD cycle is defined per slice
4. Lint/test/coverage gates are declared
5. **Existing 1547+ tests must remain passing at every phase boundary**

## Traceability Model

This plan distinguishes between:
- **Requirements statements** in `requirements.md` and `specification.md`
- **Gap labels** (`G1`–`G7`) used only inside this plan to group work

Where earlier drafts used invented `REQ-*` labels, this revised plan now traces each gap back to exact requirement text instead of presenting plan-internal labels as requirements.md identifiers.

## Requirement Coverage Matrix

The requirements/spec documents contain both targeted parity gaps and broader normative items that were already satisfied, deferred to audits, or blocked on design decisions. This matrix makes plan scope explicit.

| Requirement group | Status before this plan | Status in this plan | End-of-plan status |
|---|---|---|---|
| Thread result propagation (`WaitThread`, join out-status semantics; spec §2.2, §10.2, §10.3) | Not satisfied | Implemented in P03–P05 and behavior-tested | **Implemented by this plan** |
| `SleepThreadUntil` async pumping (spec §6.5) | Not satisfied | Implemented in P06 and behavior-validated | **Implemented by this plan** |
| Non-joinable lifecycle cleanup path using current joinable internal handle (`FinishThread` / `ProcessThreadLifecycles`) | Already present, but documentation mismatch remained | Clarified in P07 | **Already satisfied, documentation clarified by this plan** |
| Detached-thread creation failure contract (`StartThread` failure must reclaim adapter-owned wrapper state before return; spec §2.5, §10.2) | Not satisfied under current detached ABI | Not implemented here; design mismatch documented | **Blocked / open — requires ABI or ownership redesign** |
| Plain mutex recursion policy (§3.1 unresolved blocker) | Audit-blocked | Not changed | **Open — audit required** |
| Stack-size handling audit (§2.1) | Audit-blocked | Not changed | **Open — audit required** |
| Deferred-creation audit (§2.1) | Audit-blocked | Not changed | **Open — audit required** |
| Thread naming audit (§2.1) | Audit-blocked / best-effort behavior only | Not changed | **Open — audit required** |
| Recursive mutex behavior requirements (§3.2) | Functionally satisfied, stale adapter comment remained | Comment corrected in P07 | **Already satisfied, documentation clarified by this plan** |
| Lifecycle no-op Rust stub (`process_thread_lifecycles`) | Functionally acceptable, stale TODO remained | Comment/TODO cleanup in P08 | **Already satisfied, documentation clarified by this plan** |

## Identified Gaps

| # | Gap | Severity | Spec / Requirements Traceability | Files Affected |
|---|-----|----------|----------------------------------|----------------|
| G1 | Thread return value discarded — `spawn_c_thread` returns `Thread<()>` not `Thread<c_int>`, `rust_thread_join` lacks `out_status` param, C adapter writes boolean not actual status | **Critical** | Spec §2.2, §10.2, §10.3; requirements.md: “When a thread entry function returns an integer status... preserve that status...” and both `WaitThread` result-propagation requirements | `rust/src/threading/mod.rs`, `sc2/src/libs/threads/rust_thrcommon.c`, `sc2/src/libs/threads/rust_threads.h` |
| G2 | `SleepThreadUntil` missing `Async_process()` pumping loop — just computes delta and does single sleep | **Critical** | Spec §6.5; requirements.md: `SleepThreadUntil` async-pumping requirements | `sc2/src/libs/threads/rust_thrcommon.c` |
| G3 | `StartThread_Core` uses `rust_thread_spawn` instead of `rust_thread_spawn_detached` — spec §2.5 reference design suggests detached, but analysis shows `ProcessThreadLifecycles` → `WaitThread` → `rust_thread_join` NEEDS the join handle | **High** | Spec §2.5 reference-design note plus normative lifecycle cleanup requirements in spec §2.4/§2.5 and requirements.md non-joinable cleanup / no-leak statements | `sc2/src/libs/threads/rust_thrcommon.c` (documentation only) |
| G4 | `rust_thread_spawn_detached` drops `Thread<c_int>` via `let _ =` — intent should be explicit, but detached-creation failure cleanup is NOT solved by a Rust-only `match` because the ABI provides no synchronous failure reporting to reclaim C-owned wrappers | **High** | Spec §2.5 detached-thread creation failure contract; requirements.md `StartThread` failure contract and non-joinable no-leak requirement | `rust/src/threading/mod.rs`, `sc2/src/libs/threads/rust_thrcommon.c` (documentation/scope clarification only) |
| G5 | Stale TODO markers in production code: `hibernate_thread` (line 696), `process_thread_lifecycles` (line 681), task state get/set (lines 596, 611) — code actually works but TODOs trigger fraud detection | **Medium** | Plan quality requirement; spec §2.4, §6.2 | `rust/src/threading/mod.rs` |
| G6 | `process_thread_lifecycles()` is a stub with TODO — spec §2.4 says keep C-owned; should be documented no-op | **Low** | Spec §2.4; requirements.md lifecycle-processing obligations | `rust/src/threading/mod.rs` |
| G7 | Stale comment in `rust_thrcommon.c:362` claims "Rust std::sync::Mutex is not recursive; using regular mutex" — factually wrong, `RustFfiMutex` implements recursive behavior for the recursive-mutex path | **Low** | Spec §3.2; requirements.md recursive mutex behavior requirements | `sc2/src/libs/threads/rust_thrcommon.c` |

### G3/G4 Design Decision and Scope Boundary (from P02 analysis)

The spec §2.5 `[Reference design]` suggests `StartThread_Core` should use `rust_thread_spawn_detached`. However, analysis during pseudocode reveals this would BREAK the lifecycle cleanup path:

1. `StartThread_Core` sets `thread->native = rust_thread_spawn(...)` → stores `RustThread*`
2. `RustThreadHelper` calls `FinishThread(thread)` → enqueues in `pendingDeath`
3. `ProcessThreadLifecycles` calls `WaitThread(t, NULL)` → calls `rust_thread_join(t->native, ...)`

If `StartThread_Core` used `rust_thread_spawn_detached` (which returns void), `thread->native` would be NULL, and `WaitThread` would skip the join. The Rust JoinHandle would be dropped (detached) but the C-side lifecycle path would not be able to perform its current join-based cleanup flow.

At the same time, the normative detached-thread failure contract in spec §2.5 / requirements.md says failed `StartThread` creation must reclaim adapter-owned wrapper state before return. That contract is **not implementable** through the current `rust_thread_spawn_detached() -> void` ABI alone, because the C adapter gets no synchronous failure signal and therefore cannot reclaim `startInfo` / `thread` on failure.

**Resolution in this plan:**
- Keep `rust_thread_spawn` for `StartThread_Core` and document why.
- Narrow Phase 07 so it no longer claims to satisfy detached-thread creation failure semantics.
- Treat detached-failure contract reconciliation as an acknowledged open blocker requiring a future ABI/design decision rather than a solved code change in this plan.

This keeps plan scope honest: the current lifecycle path does not leak successfully started non-joinable thread handles because `ProcessThreadLifecycles` joins them, but detached-creation failure semantics are not closed by the proposed Rust helper cleanup alone.

## Deferred Design Follow-up Required for Full Spec Parity

Full spec parity for detached-thread creation failure handling requires a follow-up design/ABI phase outside this plan. Candidate remediation directions that must be evaluated explicitly:

1. Change `rust_thread_spawn_detached` from `-> void` to a success/failure-returning ABI so the C adapter can reclaim adapter-owned allocations synchronously on failure.
2. Move adapter-owned wrapper allocation until after detached spawn has crossed the last synchronous failure point.
3. Replace the current ownership split with a Rust-owned detached-start wrapper that only publishes lifecycle-visible state after successful worker start.

Any of these approaches would need concrete signature and ownership updates before the detached-thread failure requirement can be marked satisfied.

## Phase Summary

| Phase | ID | Title | Gaps Addressed |
|-------|----|-------|---------------|
| 00a | P00a | Preflight Verification | — |
| 01 | P01 | Analysis | All gaps |
| 01a | P01a | Analysis Verification | All gaps |
| 02 | P02 | Pseudocode | G1, G2, G3, G4 |
| 02a | P02a | Pseudocode Verification | G1, G2, G3, G4 |
| 03 | P03 | Return-value-propagation stub | G1 |
| 03a | P03a | Return-value-propagation stub verification | G1 |
| 04 | P04 | Return-value-propagation TDD | G1 |
| 04a | P04a | Return-value-propagation TDD verification | G1 |
| 05 | P05 | Return-value-propagation impl | G1 |
| 05a | P05a | Return-value-propagation impl verification | G1 |
| 06 | P06 | SleepThreadUntil async-pump restoration (C-only) | G2 |
| 06a | P06a | SleepThreadUntil verification | G2 |
| 07 | P07 | Detached-thread documentation, scoped detached-helper cleanup, stale comment | G3, G4, G7 |
| 07a | P07a | Detached-thread verification | G3, G4, G7 |
| 08 | P08 | TODO/stub cleanup | G5, G6 |
| 08a | P08a | TODO/stub cleanup verification | G5, G6 |
| 09 | P09 | Final integration verification | All |
| 09a | P09a | Final sign-off | All |

## Execution Order

```text
P00a → P01 → P01a → P02 → P02a → P03 → P03a → P04 → P04a → P05 → P05a → P06 → P06a → P07 → P07a → P08 → P08a → P09 → P09a
```

## Verification Baseline

```bash
cd /Users/acoliver/projects/uqm/rust
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --lib --all-features
```

All 1560+ existing lib tests must pass at every phase boundary. (Integration tests under `tests/` have a pre-existing linker issue with `input_integration_tests` unrelated to threading — `--lib` is the correct baseline.)

## C Build Verification Command

The repo build entry point in `sc2/Makefile.build` is:

```bash
cd /Users/acoliver/projects/uqm/sc2
make -f Makefile.build
```

`USE_RUST_THREADS` is already enabled in `sc2/config_unix.h`, so this build verifies the Rust-threaded engine path compiled by the existing project build system.

## Integration Contract

### Existing Callers
- `sc2/src/libs/threads/rust_thrcommon.c` → `WaitThread()` → `rust_thread_join()` (G1)
- `sc2/src/libs/threads/rust_thrcommon.c` → `SleepThreadUntil()` (G2)
- `sc2/src/libs/threads/rust_thrcommon.c` → `StartThread_Core()` → `rust_thread_spawn()` (G3 — kept as-is)
- `sc2/src/libs/threads/rust_threads.h` → FFI declarations (G1)

### Existing Code Replaced/Modified
- `spawn_c_thread` return type changes from `Thread<()>` to `Thread<c_int>` (G1)
- `rust_thread_join` signature gains `out_status` parameter (G1)
- `WaitThread` updated to pass `&out_status` and write actual return value (G1)
- `SleepThreadUntil` body replaced with async-pumping loop (G2)
- `StartThread_Core` — comment added documenting design choice and detached-failure ABI limitation (G3, no functional change)
- `rust_thread_spawn_detached` — explicit match/error-handling docs replace `let _ =` without claiming detached-failure contract closure (G4)
- `CreateRecursiveMutex_Core` — stale comment corrected without implying a settled public contract for plain `Mutex` (G7)
- Stale TODOs removed from 4 functions (G5, G6)

### End-to-End Verification
- Build C project with `make -f Makefile.build` in `sc2/`
- All Rust tests pass (baseline + 4 Rust-generic tests from P04, plus adapter/public-API behavior coverage added in P05)
- Thread return values survive through `WaitThread`
- `SleepThreadUntil` services async callbacks

## Completion Marker Contract

Every phase that completes must create `project-plans/20260311/threading/.completed/PXX.md` containing at least:
- Phase ID and title
- Completion timestamp
- Files changed (or `none` for verification-only phases)
- Tests/verification commands run
- Verification outputs summary
- Semantic checks performed
- Any deviations or follow-up notes

## Execution Tracker

| Phase | Status | Verified | Semantic Verified | Notes |
|------:|--------|----------|-------------------|-------|
| P00a  | ⬜     | ⬜       | N/A               |       |
| P01   | ⬜     | ⬜       | ⬜                |       |
| P01a  | ⬜     | ⬜       | ⬜                |       |
| P02   | ⬜     | ⬜       | ⬜                |       |
| P02a  | ⬜     | ⬜       | ⬜                |       |
| P03   | ⬜     | ⬜       | ⬜                |       |
| P03a  | ⬜     | ⬜       | ⬜                |       |
| P04   | ⬜     | ⬜       | ⬜                |       |
| P04a  | ⬜     | ⬜       | ⬜                |       |
| P05   | ⬜     | ⬜       | ⬜                |       |
| P05a  | ⬜     | ⬜       | ⬜                |       |
| P06   | ⬜     | ⬜       | ⬜                |       |
| P06a  | ⬜     | ⬜       | ⬜                |       |
| P07   | ⬜     | ⬜       | ⬜                |       |
| P07a  | ⬜     | ⬜       | ⬜                |       |
| P08   | ⬜     | ⬜       | ⬜                |       |
| P08a  | ⬜     | ⬜       | ⬜                |       |
| P09   | ⬜     | ⬜       | ⬜                |       |
| P09a  | ⬜     | ⬜       | ⬜                |       |
