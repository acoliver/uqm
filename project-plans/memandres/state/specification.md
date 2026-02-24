# State File I/O Migration — Specification

## Purpose / Problem Statement

The C state file I/O system (`state.c`) provides 7 functions for in-memory
buffer access (`OpenStateFile`, `CloseStateFile`, `ReadStateFile`,
`WriteStateFile`, `SeekStateFile`, `LengthStateFile`, `DeleteStateFile`).
These operate on 3 named in-memory buffers (STARINFO, RANDGRPINFO,
DEFGRPINFO) used by ~103 call sites across 6 C files.

A Rust implementation exists in `rust/src/state/` with matching FFI
functions, but it has **two known blockers** that prevent activation:

1. **Seek-past-end clamping** — `StateFile::seek` clamps the cursor to
   `data.len()`, but the C version allows the cursor to exceed the buffer
   size. `grpinfo.c` depends on seek-past-end followed by write-to-extend.
2. **Copy deadlock** — `rust_copy_game_state` in `ffi.rs` tries to lock
   `GLOBAL_GAME_STATE` twice (once to read, once to write), causing a
   deadlock on the non-reentrant `Mutex`.

This plan fixes those blockers and wires the Rust implementation into the
C build via `USE_RUST_STATE` compile-time guards in `state.c`.

## Explicit Scope

### IN SCOPE

- Fix seek-past-end clamping in `StateFile::seek` (REQ-SFILE-R001)
- Fix copy deadlock in `rust_copy_game_state` (REQ-STATE-R005)
- Add `USE_RUST_STATE` flag to `config_unix.h` (initially disabled)
- Add `#ifdef USE_RUST_STATE` redirects in `state.c` for all 7 functions
- Verify `cargo test` and full C build pass in both configurations
- Verify save/load round-trip compatibility

### OUT OF SCOPE

- **Game state bits macro redirect** (GET_GAME_STATE/SET_GAME_STATE) —
  1,964 call sites across 83 files. Too large; deferred to a separate plan.
- `planet_info.rs` correctness (moon count skipping bug) — separate concern.
- `NUM_GAME_STATE_BITS` mismatch (2048 vs 1238) — affects game state bits
  serialization, not state file I/O.
- Save/load format changes — we preserve the existing format exactly.

## Architectural Boundaries

```
┌────────────────────────────────────┐
│ C callers (grpinfo.c, save.c,     │
│ load.c, load_legacy.c, state.c    │
│ data ops)                          │
│ ~103 call sites, UNCHANGED         │
└──────────────┬─────────────────────┘
               │ calls OpenStateFile, CloseStateFile, etc.
               ▼
┌────────────────────────────────────┐
│ state.c                            │
│ #ifdef USE_RUST_STATE              │
│   → rust_open_state_file(idx,mode) │
│   → rust_close_state_file(idx)     │
│   → rust_read_state_file(...)      │
│   → rust_write_state_file(...)     │
│   → rust_seek_state_file(...)      │
│   → rust_length_state_file(idx)    │
│   → rust_delete_state_file(idx)    │
│ #else                              │
│   → existing C implementation      │
│ #endif                             │
└──────────────┬─────────────────────┘
               │ FFI call
               ▼
┌────────────────────────────────────┐
│ rust/src/state/ffi.rs              │
│ rust/src/state/state_file.rs       │
│ (StateFileManager, StateFile)      │
└────────────────────────────────────┘
```

### Pointer Translation

C callers use `GAME_STATE_FILE*` (pointer to static array element). The
redirect layer in `state.c` computes `file_index = (int)(fp - state_files)`
and passes the integer to the Rust FFI. For `OpenStateFile`, the redirect
returns `&state_files[stateFile]` on success (the static array still exists
for pointer identity).

## Data Contracts and Invariants

### State File Buffer Invariants

1. Three files exist at indices 0 (STARINFO), 1 (RANDGRPINFO), 2 (DEFGRPINFO).
2. `used` ≤ `physical_size` always (physical ≥ logical).
3. `cursor` may exceed both `used` and `physical_size` (no upper clamp).
4. Write at `cursor > physical_size` grows buffer to accommodate, zero-filling gap.
5. Read at `cursor ≥ physical_size` returns 0 (EOF).
6. Buffer persists across open/close cycles. Only `DeleteStateFile` frees it.
7. `open_count` tracks reference counting (warning-only, not enforced).

### Save/Load Compatibility

- State file buffer contents are serialized to save files by the C
  save/load system using `sread_*`/`swrite_*` helpers (native endian).
- Save files use explicit little-endian encoding (`write_32`/`read_32`).
- The Rust state file buffers must be byte-for-byte identical to C for
  any given sequence of operations, ensuring cross-implementation
  save/load compatibility.

## Integration Points

### C → Rust (via FFI)

| C Function | Rust FFI | Notes |
|---|---|---|
| `OpenStateFile(int, const char*)` | `rust_open_state_file(c_int, *const c_char)` | Returns 1/0; C wrapper returns `GAME_STATE_FILE*` or NULL |
| `CloseStateFile(GAME_STATE_FILE*)` | `rust_close_state_file(c_int)` | C extracts index from pointer |
| `ReadStateFile(void*, COUNT, COUNT, GAME_STATE_FILE*)` | `rust_read_state_file(c_int, *mut u8, usize, usize)` | Returns element count |
| `WriteStateFile(const void*, COUNT, COUNT, GAME_STATE_FILE*)` | `rust_write_state_file(c_int, *const u8, usize, usize)` | Returns element count |
| `SeekStateFile(GAME_STATE_FILE*, long, int)` | `rust_seek_state_file(c_int, i64, c_int)` | Returns 1 success, 0 clamped |
| `LengthStateFile(GAME_STATE_FILE*)` | `rust_length_state_file(c_int)` | Returns logical size |
| `DeleteStateFile(int)` | `rust_delete_state_file(c_int)` | Index-based, no pointer |

### sread_*/swrite_* Helpers

These inline functions in `state.h` call `ReadStateFile`/`WriteStateFile`.
When `USE_RUST_STATE` is active, those calls route through the `state.c`
redirects to Rust. **No changes needed in `state.h`.**

### Existing Callers (UNCHANGED)

| File | Call Sites | Usage |
|---|---|---|
| `state.c` (data ops) | 18 | InitPlanetInfo, GetPlanetInfo, PutPlanetInfo, UninitPlanetInfo |
| `grpinfo.c` | 31 | InitGroupInfo, GetGroupInfo, PutGroupInfo, FlushGroupInfo |
| `save.c` | 14 | SaveStarInfo, SaveBattleGroup, SaveGroups |
| `load.c` | 15 | LoadScanInfo, LoadGroupList, LoadBattleGroup |
| `load_legacy.c` | 9 | Legacy save format loading |

## Functional Requirements

### REQ-SF-001: Seek-Past-End Allowed
The `StateFile::seek` method shall allow the cursor to be positioned at
any non-negative value, including beyond `used` and physical buffer size.
No upper bound clamping.

### REQ-SF-002: Write-After-Seek-Past-End Extends Buffer
When writing at a cursor position beyond the physical buffer size, the
buffer shall grow to accommodate the write, zero-filling any gap.

### REQ-SF-003: Read-After-Seek-Past-End Returns EOF
When reading at a cursor position at or beyond the physical buffer size,
the read shall return 0 (no data available).

### REQ-SF-004: Copy Deadlock Prevention
`rust_copy_game_state` shall acquire the `GLOBAL_GAME_STATE` lock exactly
once and operate on the state within a single critical section.

### REQ-SF-005: Separate Used and Physical Size Tracking
`StateFile` shall track `used` (logical high-water mark) and physical
allocation size separately. Reads check against physical size per C behavior.

### REQ-SF-006: C Redirect Correctness
When `USE_RUST_STATE` is defined, all 7 state file functions in `state.c`
shall call their `rust_*` FFI equivalents. When undefined, the original
C implementation is used.

### REQ-SF-007: Save/Load Round-Trip
Save files created with the Rust backend must load correctly with either
backend, and vice versa. Buffer contents must be byte-for-byte identical.

### REQ-SF-008: FFI Panic Safety
All Rust FFI functions must catch panics at the boundary and convert them
to error return values. No panic shall propagate into C code.

### REQ-SF-009: Poisoned Mutex Handling
If a mutex is poisoned (from a prior panic), FFI functions shall return
error values rather than panicking.

## Error/Edge Case Expectations

1. `OpenStateFile` with invalid index → returns NULL (C) / 0 (Rust)
2. `OpenStateFile` allocation failure → returns NULL / 0
3. `WriteStateFile` reallocation failure → returns 0
4. `SeekStateFile` to negative position → clamps to 0, returns 0
5. `ReadStateFile` at EOF → returns 0
6. `DeleteStateFile` while open → warning logged, deletion proceeds
7. `CloseStateFile` on already-closed file → `open_count` goes negative, warning
8. Double init → harmless (init is already guarded)

## Non-Functional Requirements

- **Reliability**: No undefined behavior at FFI boundary. No panics cross FFI.
- **Performance**: Identical to C — all operations are memcpy-speed on
  heap buffers. No additional allocation overhead beyond Rust's Vec growth.
- **Operability**: `USE_RUST_STATE` flag allows instant rollback to C path.

## Testability Requirements

- Rust unit tests must cover seek-past-end, write-after-seek-past-end,
  read-after-seek-past-end, copy without deadlock.
- C build must succeed with both `USE_RUST_STATE` defined and undefined.
- Manual integration test: save game, quit, reload, verify state intact.

## Plan Review

**Reviewed**: 2026-02-24
**Reviewer**: LLxprt Code (claude-opus-4-6)
**Plan ID**: PLAN-20260224-STATE-SWAP
**Inputs reviewed**: `PLAN.md`, `PLAN-TEMPLATE.md`, `RULES.md`, `state.md` (functional spec), `rust-state-system.md` (behavioral spec), all 29 files under `project-plans/memandres/state/`

---

### 1. Plan Structure Compliance (vs PLAN.md)

| Criterion | Status | Notes |
|-----------|--------|-------|
| Plan ID present (`PLAN-YYYYMMDD-FEATURE`) | ✅ | `PLAN-20260224-STATE-SWAP` in 00-overview.md |
| Sequential phases, no skips | ✅ | P00.5 → P01 → P01a → … → P12 → P12a, 25 phases total |
| Specification before implementation | ✅ | `specification.md` exists with purpose, boundaries, contracts, requirements |
| Preflight verification phase | ✅ | P00.5 covers toolchain, types, call-path feasibility, blockers |
| Analysis phase | ✅ | P01 produces domain-model.md; P01a verifies |
| Pseudocode phase | ✅ | P02 produces component-001.md with numbered lines; P02a verifies |
| Implementation cycle: Stub → TDD → Impl per slice | ✅ | Slice B (P03/P04/P05), Slice C (P06/P07/P08), Slice D (P09/P10/P11) |
| Integration phase | ✅ | P12 with 8 integration tests + P12a final gate |
| Traceability markers defined | ✅ | `@plan` and `@requirement` markers specified in each phase |
| Phase completion markers | ✅ | `.completed/PNN.md` specified for every phase |
| Lint/test/coverage gates | ✅ | `cargo fmt`, `cargo clippy -D warnings`, `cargo test` in every impl/verification phase |
| No placeholder reliance | ✅ | `todo!()` explicitly confined to stub phases; deferred-implementation grep in every impl phase |
| Required directory structure | ✅ | `specification.md`, `analysis/domain-model.md`, `analysis/pseudocode/component-001.md`, `plan/00-overview.md` through `plan/12a-*` |
| Execution tracker | ✅ | Present in 00-overview.md with all 25 rows |
| Integration contract | ✅ | Callers, replaced code, user access path, data migration, E2E verification all documented |

**Structure Score: 10/10**

---

### 2. Phase Template Compliance (vs PLAN-TEMPLATE.md)

| Template Element | Present in Phases | Notes |
|-----------------|-------------------|-------|
| Phase ID header | ✅ All | `PLAN-20260224-STATE-SWAP.PNN` format |
| Prerequisites section | ✅ All | Each phase lists prior phase + expected artifacts |
| Requirements Implemented (Expanded) with GIVEN/WHEN/THEN | ✅ All impl phases | P01, P03, P04, P05, P06, P07, P08, P09, P10, P11 |
| Files to create/modify | ✅ All impl phases | With markers and pseudocode traceability |
| Pseudocode traceability | ✅ P03–P08 | Line references to component-001.md |
| Verification Commands | ✅ All | fmt/clippy/test in every phase |
| Structural Verification Checklist | ✅ All | File-level and code-level checks |
| Semantic Verification Checklist | ✅ All | Behavioral checks for each requirement |
| Deferred Implementation Detection | ✅ All impl phases | grep command for TODO/FIXME/HACK |
| Success Criteria | ✅ All | Clear pass conditions |
| Failure Recovery | ✅ All impl phases | git checkout rollback commands + blocker identification |
| Phase Completion Marker | ✅ All | `.completed/PNN.md` with contents specified |
| Preflight template elements | ✅ P00.5 | Toolchain, deps, types, test infra, call-path, blockers, gate decision |

**Template Score: 10/10**

---

### 3. Rules Compliance (vs RULES.md)

| Rule | Status | Notes |
|------|--------|-------|
| TDD mandatory (RED→GREEN→REFACTOR) | ✅ | P04 (RED: tests fail on todo), P05 (GREEN: impl passes). P07/P08 same pattern. |
| `cargo fmt/clippy/test` gates | ✅ | Every verification phase includes all three commands |
| No `unwrap`/`expect` in production | ✅ | P08 impl uses `match` on lock(), poisoned handling. Pseudocode avoids unwrap. |
| `Result`/`Option` error handling | ✅ | Spec mandates `Result<T, StateFileError>` internally, sentinel values at FFI |
| `unsafe` only if approved | ✅ | Overview notes `unsafe` approved for FFI boundary |
| No `*_v2` / `new_*` parallel modules | ✅ | Plan modifies existing `state_file.rs` and `ffi.rs` in place |
| Anti-placeholder rule | ✅ | `todo!()` only in explicit stub phases; mandatory grep in impl phases |
| Module boundaries preserved | ✅ | State system stays in `rust/src/state/`, C callers unchanged |
| No weak typing | ✅ | Strongly typed: `SeekWhence` enum, `FileMode`, `Result<T, E>` |
| Tests verify behavior, not internals | ✅ | P04 tests: seek positions, read byte counts, write buffer growth, gap content |

**Rules Score: 10/10**

---

### 4. Blocker Coverage

#### Blocker 1: Seek-Past-End Clamping (REQ-SF-001, REQ-SFILE-R001)

| Aspect | Status | Where |
|--------|--------|-------|
| Root cause identified | ✅ | domain-model.md §4, pseudocode lines 51–60 |
| Specification requirement | ✅ | REQ-SF-001, REQ-SFILE-R001, REQ-SFILE-024 |
| Stub phase | ✅ | P03 adds `used` field, removes upper clamp |
| TDD phase | ✅ | P04: 10 tests covering SEEK_SET/CUR/END past end, write-extend, read-EOF, used-vs-physical |
| Impl phase | ✅ | P05: pseudocode lines 51–60 implemented |
| Integration test | ✅ | P12 Test 4: grpinfo.c seek+write pattern verified |
| `used` vs physical separation | ✅ | REQ-SF-005, domain-model §2.4, P03 struct change, P04 test `test_length_returns_used_not_physical` |
| Read checks physical (not used) | ✅ | Pseudocode lines 63–72, P04 `test_read_checks_physical_size_not_used` |

**Blocker 1 Score: 10/10** — Comprehensively addressed at every layer.

#### Blocker 2: Copy Deadlock (REQ-SF-004, REQ-STATE-R005)

| Aspect | Status | Where |
|--------|--------|-------|
| Root cause identified | ✅ | domain-model.md §6, spec §4.4.5 |
| Specification requirement | ✅ | REQ-SF-004, REQ-STATE-R005 |
| Fix strategy | ✅ | Snapshot-then-mutate (pseudocode lines 119–127) |
| Stub phase | ✅ | P06 replaces double-lock with `todo!()` |
| TDD phase | ✅ | P07: 4 tests including timeout-based deadlock detection, self-copy, overlapping ranges |
| Impl phase | ✅ | P08: single lock, `GameState::from_bytes` snapshot, `catch_unwind` at FFI |
| Poisoned mutex handling | ✅ | P08 uses `into_inner()` |
| Integration test | ✅ | P12 Test 5: legacy load path verified |

**Blocker 2 Score: 10/10** — Deadlock eliminated with safe snapshot pattern.

---

### 5. Requirements Traceability

| Requirement | Spec'd | Pseudocode | Tested | Implemented | Integration |
|-------------|--------|------------|--------|-------------|-------------|
| REQ-SF-001 (seek past end) | ✅ | lines 51–60 | P04 (4 tests) | P05 | P12 Test 4 |
| REQ-SF-002 (write extends) | ✅ | lines 75–86 | P04 (1 test) | P05 | P12 Test 4 |
| REQ-SF-003 (read EOF) | ✅ | lines 63–72 | P04 (1 test) | P05 | P12 Test 3 |
| REQ-SF-004 (no deadlock) | ✅ | lines 119–127 | P07 (4 tests) | P08 | P12 Test 5 |
| REQ-SF-005 (used/physical) | ✅ | lines 89–107 | P04 (2 tests) | P05 | P12 Test 3 |
| REQ-SF-006 (C redirect) | ✅ | lines 1–48 | P10 (build test) | P09/P11 | P12 Tests 1–4 |
| REQ-SF-007 (save/load) | ✅ | — | — | P09 | P12 Tests 1–2 |
| REQ-SF-008 (panic safety) | ✅ | — | — | P08 | P12 Test 6 |
| REQ-SF-009 (mutex poison) | ✅ | — | — | P08 | P12 Test 6 |

**Traceability Score: 9/10** — REQ-SF-007 (save/load round-trip) and REQ-SF-008/009 (panic/poison) lack dedicated Rust unit tests; they rely on integration testing and P08's implementation pattern. Acceptable given the FFI-boundary nature, but explicit unit tests for panic-catching and poisoned-mutex recovery would strengthen confidence.

---

### 6. Save/Load Compatibility Protection

| Protection Mechanism | Status | Notes |
|---------------------|--------|-------|
| Byte-for-byte buffer identity | ✅ | Specification §3.2 + §7, domain-model §5, P12 Test 1 |
| `LengthStateFile` returns `used` | ✅ | REQ-SF-005, pseudocode line 89–90 |
| Endianness: in-memory native, save LE | ✅ | Spec §3.3, state.md §5.5: `sread_*/swrite_*` remain C inline |
| `sread_*/swrite_*` unchanged | ✅ | Spec §3.3: "These helpers remain as C inline functions in state.h" |
| Sync points defined | ✅ | `rust_get_game_state_bytes()` / `rust_restore_game_state_from_bytes()` |
| Legacy save compatibility | ✅ | Spec §4.4.5: `load_legacy.c` uses C functions on local arrays; unaffected |
| Feature flag rollback | ✅ | Comment out `USE_RUST_STATE` → instant C path restore |
| Both-config build test | ✅ | P10/P12 Test 8: build with flag on and off |

**Save/Load Score: 10/10**

---

### 7. Game State Bits Scope Exclusion

| Aspect | Status | Notes |
|--------|--------|-------|
| Explicitly out of scope | ✅ | Specification "OUT OF SCOPE" lists macro redirect, planet_info moon bug, NUM_GAME_STATE_BITS |
| 00-overview.md reaffirms | ✅ | "OUT OF SCOPE" section with rationale ("1,964 call sites across 83 files. Too large; deferred") |
| No accidental scope creep | ✅ | No phase touches `globdata.h`, `globdata.c`, or `game_state.rs` bit constant |
| Copy deadlock fix is in scope | ✅ | Despite being game-state-related, it's a blocker for the state file FFI layer |

**Scope Score: 10/10**

---

### 8. Integration Explicitness

| Integration Question (from PLAN.md) | Answered | Where |
|--------------------------------------|----------|-------|
| Who calls this new behavior? | ✅ | Specification §Integration Points table; 00-overview §Integration Contract |
| What old behavior gets replaced? | ✅ | 00-overview: "state.c lines 53–226: 7 function bodies replaced by #ifdef blocks" |
| How can a user trigger E2E? | ✅ | 00-overview: "Start game → enter solar system → scan planet"; P12 Tests 1–4 |
| What state/config must migrate? | ✅ | 00-overview: "No migration needed. State file buffers are volatile" |
| How is backward compatibility handled? | ✅ | Feature flag, both-config build, save file byte identity |

**Integration Score: 10/10**

---

### 9. Issues Found

| # | Severity | Issue | Recommendation |
|---|----------|-------|----------------|
| 1 | **Low** | REQ-SF-008 (panic safety) and REQ-SF-009 (poisoned mutex) have no dedicated unit tests — verified only through P08's implementation pattern and P12 integration. | Add 2 focused unit tests in P07 or P08: one that forces a panic inside `catch_unwind` and verifies recovery, one that poisons the mutex and verifies graceful return. |
| 2 | **Low** | P04 lists 10 tests but the test names in the body total 10 including `test_open_count_can_go_negative`. The `open_count` test is not tied to a pseudocode line range. | Minor: add pseudocode traceability note for open_count (it's covered in domain-model §7 but not in pseudocode). |
| 3 | **Low** | `rust_state_ffi.h` is created in P09 but there is no mention of whether an existing FFI header (e.g., `rust_ffi.h`) already exists that should be extended instead. Preflight P00.5 checks `config_unix.h` for `USE_RUST_*` flags, implying other Rust FFI headers may exist. | P00.5 should verify whether a shared FFI header exists and whether `rust_state_ffi.h` should be a new file or additions to an existing one. |
| 4 | **Info** | The specification's REQ-SF-007 ("Save/Load Round-Trip") is actually a system-level property, not a unit-testable requirement. It has no dedicated Rust unit test — relying entirely on P12 integration. | Acceptable — save/load round-trip is inherently an integration concern. No action needed. |
| 5 | **Info** | P10 (TDD for C redirect) is labeled "TDD" but is actually a build-system verification phase. The plan acknowledges this ("This phase is a build-system verification phase, not a unit test phase") which is honest, but it slightly bends the TDD cycle convention. | Acceptable — build verification is the correct "test" for a compile-time feature flag. No action needed. |
| 6 | **Low** | The `open_count` type change (u32 → i32) is addressed in P03/P05 but there is no explicit REQ-SF-* requirement for it. It's a correctness fix discovered in the domain model. | Consider adding `REQ-SF-010: open_count type` or documenting it as a sub-requirement of REQ-SF-006 (C redirect correctness requires matching C's `int open_count` semantics). |

---

### 10. Compliance Scores Summary

| Category | Score | Max |
|----------|-------|-----|
| Plan structure (vs PLAN.md) | 10 | 10 |
| Phase template (vs PLAN-TEMPLATE.md) | 10 | 10 |
| Rules compliance (vs RULES.md) | 10 | 10 |
| Blocker 1: Seek-past-end | 10 | 10 |
| Blocker 2: Copy deadlock | 10 | 10 |
| Requirements traceability | 9 | 10 |
| Save/load compatibility | 10 | 10 |
| Game state bits scope exclusion | 10 | 10 |
| Integration explicitness | 10 | 10 |
| **Total** | **89** | **90** |

---

### Verdict

**✅ APPROVED — Ready for execution.**

The plan is exceptionally thorough. It correctly identifies both blockers, traces them from specification through pseudocode to test to implementation to integration, and provides rollback at every phase. The scope is well-contained (state file I/O only, game state bits deferred), save/load compatibility is protected at multiple levels, and the `USE_RUST_STATE` feature flag provides a safe fallback.

The single point deduction is for missing dedicated unit tests on panic-catching and poisoned-mutex recovery at the FFI boundary (Issues #1). These are low-severity because the implementation pattern in P08 handles them correctly and integration testing covers the runtime path — but explicit unit tests would prevent regressions if the FFI code is later refactored.

No blocking issues. Proceed to P00.5 (Preflight Verification).

