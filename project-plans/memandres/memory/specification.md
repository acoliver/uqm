# Specification: Swap C Memory Allocator to Rust

## Purpose

Replace the C memory allocation subsystem (`w_memlib.c`) with the existing Rust implementation (`rust/src/memory.rs`) using compile-time macro redirection in `memlib.h`. This eliminates the C implementation from the build when `USE_RUST_MEM` is defined, routing all 300+ call sites through Rust without modifying any caller.

## Problem Statement

The game's memory allocation functions (`HMalloc`, `HFree`, `HCalloc`, `HRealloc`, `mem_init`, `mem_uninit`) are currently implemented in C (`w_memlib.c`). A complete Rust implementation already exists in `rust/src/memory.rs` exporting `rust_hmalloc`, `rust_hfree`, `rust_hcalloc`, `rust_hrealloc`, `rust_mem_init`, `rust_mem_uninit`. The swap has not been wired up — no `USE_RUST_MEM` flag exists, no header redirect exists, and the Makeinfo always compiles `w_memlib.c`.

## Architectural Boundaries

- **API surface**: `sc2/src/libs/memlib.h` — the only header any C file includes for memory allocation.
- **C implementation**: `sc2/src/libs/memory/w_memlib.c` — replaced by Rust, guarded with `#error`.
- **Rust implementation**: `rust/src/memory.rs` — already exports the required `extern "C"` functions.
- **Build config**: `sc2/config_unix.h` — where `USE_RUST_MEM` is defined (pattern: other `USE_RUST_*` flags).
- **Build system**: `sc2/src/libs/memory/Makeinfo` — conditionally excludes `w_memlib.c`.
- **Callers**: ~300 call sites across ~55 files. **Zero caller modifications required.**

## Data Contracts and Invariants

- **Input**: `size_t size` for allocation, `void *p` for free/realloc.
- **Output**: `void *` pointer from same heap (`libc::malloc`/`free`/`realloc`).
- **Invariant**: Memory allocated by C `malloc` can be freed by Rust `libc::free` and vice versa — both use the same system allocator.
- **Zero-size**: Rust guarantees non-null return for zero-size allocations (allocates 1 byte). C behavior is implementation-defined. Rust is strictly safer.
- **OOM**: Both abort the process. C calls `explode()` (abort in debug, exit in release). Rust calls `std::process::abort()` unconditionally.
- **NULL free**: Both handle `HFree(NULL)` safely.

## Integration Points

| Integration Point | Mechanism |
|---|---|
| 322+ C call sites | Macro redirect in `memlib.h`: `#define HMalloc(s) rust_hmalloc(s)` |
| `uqm.c` lifecycle | `mem_init()` → `rust_mem_init()`, `mem_uninit()` → `rust_mem_uninit()` |
| Rust internal callers | `copy_argv_to_c()` already calls `rust_hmalloc` directly — unaffected |
| Linker | Rust static library (`libuqm_rust.a`) already linked; exports are already present |
| Build system | `Makeinfo` conditional excludes `w_memlib.c` when flag is set |

## Functional Requirements

### REQ-MEM-001: Header Macro Redirect
When `USE_RUST_MEM` is defined, `memlib.h` must redirect `HMalloc`, `HFree`, `HCalloc`, `HRealloc`, `mem_init`, and `mem_uninit` to their `rust_*` equivalents via preprocessor macros. When `USE_RUST_MEM` is not defined, the original `extern` declarations must remain unchanged.

### REQ-MEM-002: C Source Guard
`w_memlib.c` must contain `#ifdef USE_RUST_MEM` / `#error` at the top to prevent accidental compilation when the Rust path is active. Pattern: identical to `files.c`, `io.c`, `clock.c`, `vcontrol.c`.

### REQ-MEM-003: Build System Conditional
`sc2/src/libs/memory/Makeinfo` must conditionally set `uqm_CFILES=""` when `USE_RUST_MEM` is enabled, excluding `w_memlib.c` from compilation. When disabled, `uqm_CFILES="w_memlib.c"` as before.

### REQ-MEM-004: Config Flag
`sc2/config_unix.h` must define `USE_RUST_MEM` following the established pattern for other `USE_RUST_*` flags.

### REQ-MEM-005: OOM Log Level Correctness
The Rust `memory.rs` OOM log calls must use a log level semantically equivalent to C's `log_Fatal`. Since `log_Fatal == log_User == 1` in the C enum, and Rust's `LogLevel::User == 1`, the numeric value is already correct. A `Fatal` alias should be added to the Rust `LogLevel` enum for semantic clarity and to document the equivalence.

### REQ-MEM-006: Behavioral Equivalence
The Rust memory functions must be behaviorally equivalent to the C functions for all non-zero-size allocations. For zero-size allocations, the Rust behavior (guaranteed non-null) is an intentional improvement over C's implementation-defined behavior.

### REQ-MEM-007: Build Both Paths
Both `USE_RUST_MEM=1` (Rust path) and `USE_RUST_MEM` undefined (C path) must compile and link successfully. This ensures reversibility.

## Error/Edge Case Expectations

| Case | Expected Behavior |
|---|---|
| OOM on `HMalloc(n)` where n > 0 | Log fatal, abort process |
| `HMalloc(0)` | Return non-null pointer (Rust improvement) |
| `HFree(NULL)` | No-op |
| `HCalloc(0)` | Return non-null pointer to zeroed byte |
| `HRealloc(p, 0)` | Free p, return non-null pointer |
| Cross-allocation free | Safe — same heap allocator |
| `w_memlib.c` compiled with `USE_RUST_MEM` | `#error` fires |

## Non-Functional Requirements

- **Performance**: Zero overhead — macro redirect is compile-time, Rust functions call same `libc::malloc`.
- **Reliability**: OOM is always fatal (no silent failure).
- **Reversibility**: One-line config change (`#undef USE_RUST_MEM` or comment out) + rebuild restores C path.
- **Thread safety**: Same as C — `malloc`/`free` are thread-safe, no additional synchronization needed.

## Testability Requirements

- Existing Rust unit tests (`test_hmalloc_hfree`, `test_hcalloc`, `test_hrealloc`, `test_zero_size_allocations`, `test_copy_argv_to_c`) must pass.
- Build verification: clean build succeeds with `USE_RUST_MEM` defined.
- Build verification: clean build succeeds without `USE_RUST_MEM` (C fallback).
- Runtime verification: game launches, loads content, enters menus.
- `cargo test --workspace` passes.
- `cargo clippy` passes.
- `cargo fmt --check` passes.


## Plan Review

**Reviewer:** LLxprt Code (claude-opus-4-6)
**Date:** 2026-02-24
**Plan ID:** PLAN-20260224-MEM-SWAP
**Reference Documents:** `dev-docs/PLAN.md`, `dev-docs/PLAN-TEMPLATE.md`, `dev-docs/RULES.md`

---

### 1. Structure Compliance (vs. PLAN.md)

| Criterion | Status | Notes |
|---|---|---|
| Plan ID format `PLAN-YYYYMMDD-FEATURE` | ✅ | `PLAN-20260224-MEM-SWAP` |
| Sequential phases with verification | ✅ | P00a → P01/P01a → P02/P02a → P03/P03a → P04/P04a → P05/P05a → P06/P06a → P07/P07a → P08/P08a → P09/P09a |
| Required directory structure | ✅ | `specification.md`, `analysis/domain-model.md`, `analysis/pseudocode/component-001.md`, `plan/00-overview.md` through `plan/09a-*`, `.completed/` dir |
| Specification (Phase 0) | ✅ | Contains purpose, boundaries, data contracts, integration points, REQ-* IDs, error/edge cases, non-functional reqs, testability |
| Preflight Verification (Phase 0.5) | ✅ | Toolchain, dependencies, type/interface, test infrastructure, pattern verification, call site safety |
| Analysis (Phase 1) | ✅ | Entity analysis, edge/error map, integration touchpoints, old code replacement list |
| Pseudocode (Phase 2) | ✅ | Numbered algorithmic pseudocode with validation points, error handling, ordering constraints, integration boundaries |
| Implementation cycle per slice (Stub→TDD→Impl) | ✅ | Slice 1: P03→P04→P05 (Rust fixes); Slice 2: P06→P07→P08 (C header redirect) |
| Integration phase | ✅ | P09 answers all 5 integration questions (who calls, what replaced, E2E trigger, state migration, backward compat) |
| Verification layers (structural + semantic) | ✅ | Every phase has both checklists |
| Deferred implementation detection | ✅ | `grep` commands in every implementation phase |
| Phase completion markers | ✅ | `.completed/PNN.md` specified for each phase |
| Traceability markers | ✅ | `@plan` and `@requirement` markers specified for all code changes |
| Plan Evaluation Checklist | ✅ | Present in P09a with all 9 items checked |
| Lint/test/coverage gates | ✅ | `cargo fmt`, `cargo clippy`, `cargo test` in every verification phase |

**Structure Score: 10/10**

---

### 2. Phase Template Compliance (vs. PLAN-TEMPLATE.md)

| Template Element | Status | Notes |
|---|---|---|
| Phase header with Plan ID | ✅ | All phases use `PLAN-20260224-MEM-SWAP.PNN` |
| Prerequisites with previous phase reference | ✅ | Each phase lists its prerequisite phase |
| Requirements Implemented (Expanded) with GIVEN/WHEN/THEN | ✅ | Present in all implementation phases (P03-P09) |
| Implementation Tasks with files to create/modify | ✅ | Exact files and changes listed |
| Pseudocode traceability (line references) | ✅ | Line ranges referenced in P03, P04, P05, P06, P07, P08 |
| Verification Commands | ✅ | `cargo fmt`, `cargo clippy`, `cargo test`, plus C build commands |
| Structural Verification Checklist | ✅ | Present in all phases |
| Semantic Verification Checklist | ✅ | Present in all phases |
| Deferred Implementation Detection | ✅ | `grep` command in all implementation phases |
| Success Criteria | ✅ | Present in all phases |
| Failure Recovery | ✅ | Rollback commands and blocking issue lists in all phases |
| Phase Completion Marker | ✅ | Specified for all phases |
| Execution Tracker | ✅ | Present in `00-overview.md` |
| Integration Contract | ✅ | Present in P09 |

**Phase Template Score: 10/10**

---

### 3. Rules Compliance (vs. RULES.md)

| Rule | Status | Notes |
|---|---|---|
| TDD mandatory (RED→GREEN→REFACTOR) | ✅ | P04 writes test before P05 implements; P07 tests build before P08 enables |
| Rust quality baseline (fmt, clippy, test) | ✅ | Required in every verification phase |
| Type safety / explicit domain types | ✅ | `LogLevel::Fatal` alias is an explicit type-safe constant |
| Error handling with Result/Option | N/A | Plan modifies existing functions, doesn't add new Rust error handling |
| No unsafe without approval | ✅ | Existing `unsafe extern "C"` functions are unchanged |
| Module boundary preservation | ✅ | No boundary violations — changes are header-level redirect |
| Testing rules (behavior, not internals) | ✅ | `test_fatal_alias` tests observable behavior (value equivalence) |
| Anti-placeholder rule | ✅ | Explicit detection in every implementation phase; pre-existing comments acknowledged and scoped out |
| No parallel architecture (`*_v2`) | ✅ | Modifies existing files, no new parallel modules |
| LLM rules (follow patterns, no speculative abstractions) | ✅ | Closely follows established `USE_RUST_*` patterns |

**Rules Score: 10/10**

---

### 4. Requirements Traceability

| Requirement | Specification | Pseudocode | Stub | TDD | Impl | Integration | Traced? |
|---|---|---|---|---|---|---|---|
| REQ-MEM-001 | ✅ | Lines 01-26 | P06 | P07 | P08 | P09 | ✅ |
| REQ-MEM-002 | ✅ | Lines 30-34 | P06 | P07 | P08 | P09 | ✅ |
| REQ-MEM-003 | ✅ | Lines 40-45 | P06 | P07 | P08 | P09 | ✅ |
| REQ-MEM-004 | ✅ | Lines 50-53 | P06 | P07 | P08 | P09 | ✅ |
| REQ-MEM-005 | ✅ | Lines 60-74 | P03 | P04 | P05 | — | ✅ |
| REQ-MEM-006 | ✅ | All | — | P04 | P05 | P09 | ✅ |
| REQ-MEM-007 | ✅ | Lines 01-26, 40-45 | — | P07 | P08 | P09 | ✅ |

**Traceability Score: 10/10**

---

### 5. Integration Explicitness

| Integration Question (per PLAN.md §Integration Requirements) | Answered? | Where |
|---|---|---|
| Who calls this new behavior? (exact file/functions) | ✅ | Specification §3 (322+ call sites via `memlib.h`), P09 Integration Contract |
| What old behavior gets replaced? | ✅ | `w_memlib.c` excluded from build, guarded with `#error` |
| How can a user trigger this end-to-end? | ✅ | P09: game launch, menu navigation, content loading, clean exit |
| What state/config must migrate? | ✅ | No state migration needed — same heap allocator |
| How is backward compatibility handled? | ✅ | Comment out `#define USE_RUST_MEM` → rebuild restores C path |

**Integration Score: 10/10**

---

### 6. Proportionality Assessment

The task is a well-bounded build-system-level swap: 6 files modified, ~30 lines of meaningful change, no new Rust logic beyond a constant alias. The plan has 10 phases (plus verification phases), 23 markdown files, and detailed domain analysis.

**Assessment:** The plan is somewhat heavy for the scope of the actual code changes, but this is **appropriate** given:
- The change affects 322+ call sites across 55+ files (blast radius is high even though per-file changes are zero).
- The specification document (`memory.md`) is thorough and demonstrates deep understanding of both codebases.
- The plan follows the prescribed structure faithfully.
- Better to over-plan a cross-cutting allocator swap than under-plan it.

**Proportionality: ACCEPTABLE** — no excess phases that don't map to real work.

---

### 7. Issues Found

#### CRITICAL — Build System Files Missing from Plan

**Issue:** The parent specification (`memory.md` §2.3) correctly identifies three build system files that need modification:
1. `sc2/src/config_unix.h.in` — needs `@SYMBOL_USE_RUST_MEM_DEF@` placeholder
2. `sc2/build.vars.in` — needs `USE_RUST_MEM`, `uqm_USE_RUST_MEM`, `SYMBOL_USE_RUST_MEM_DEF` entries (matching the pattern for all other `USE_RUST_*` flags)
3. `sc2/config_unix.h` — correctly included

**However**, the plan phases (P06, P08) and the specification (`specification.md`) only reference `sc2/config_unix.h` directly. Neither `config_unix.h.in` nor `build.vars.in` appear anywhere in the plan directory.

**Why this matters:** The `Makeinfo` conditional in P06 checks `$USE_RUST_MEM` and `$uqm_USE_RUST_MEM`. These shell variables are populated from `build.vars.in`, which reads `@USE_RUST_MEM@` placeholders. Without updating `build.vars.in` and `config_unix.h.in`, the variables will never be set, and the `Makeinfo` conditional will always take the `else` branch (compiling `w_memlib.c`). The `#error` guard would then fire, causing a build failure.

Verified by examining the existing pattern: every `USE_RUST_*` flag appears in all three files (`config_unix.h`, `config_unix.h.in`, `build.vars.in`). This is not optional.

**Impact:** P07 (TDD — build both paths) would likely catch this as a build failure on the Rust path, but the plan should anticipate and specify the changes rather than discover them during testing.

**Recommendation:** Add `build.vars.in` and `config_unix.h.in` modifications to P06 (Stub phase) with explicit entries for:
- `build.vars.in`: `uqm_USE_RUST_MEM`, `USE_RUST_MEM`, `SYMBOL_USE_RUST_MEM_DEF` (plus export lines)
- `config_unix.h.in`: `@SYMBOL_USE_RUST_MEM_DEF@`

Also update the `specification.md` "Architectural Boundaries" and "Integration Points" sections to list these files.

#### MINOR — `config_unix.h` Direct Edit vs. Template Pattern

The plan (P06) adds a commented-out `#define USE_RUST_MEM` directly to `config_unix.h`. But `config_unix.h` appears to be a generated file (generated from `config_unix.h.in` via the build system's `@SYMBOL_*@` substitution). Directly editing `config_unix.h` works for local development but won't survive a `./configure`-equivalent regeneration. The proper approach is to edit `config_unix.h.in` with `@SYMBOL_USE_RUST_MEM_DEF@` and configure the build system to substitute it. The plan should clarify whether `config_unix.h` is hand-edited (as it appears to be in this project, given that other `USE_RUST_*` flags are hardcoded there) or generated.

#### MINOR — REQ-MEM-005 Log Level Scope

The plan correctly notes that `LogLevel::User == 1 == log_Fatal` numerically, making the `Fatal` alias a no-op at runtime. The Stub→TDD→Impl cycle (P03→P04→P05) for this single constant alias is thorough but the TDD phase (P04) tests a compile-time constant equivalence, which is more of a documentation/assertion test than behavioral TDD. This is acknowledged in the plan and is acceptable for this scope.

#### MINOR — Pre-existing Deferred Markers

P05 correctly notes pre-existing "later phases" comments in `memory.rs` for `rust_mem_init`/`rust_mem_uninit`. The plan scopes these out explicitly. Good practice.

---

### 8. Compliance Scores

| Category | Score | Max |
|---|---|---|
| Plan Structure (PLAN.md) | 10 | 10 |
| Phase Template (PLAN-TEMPLATE.md) | 10 | 10 |
| Rules (RULES.md) | 10 | 10 |
| Requirements Traceability | 10 | 10 |
| Integration Explicitness | 10 | 10 |
| Proportionality | 9 | 10 |
| **Subtotal (Structure & Process)** | **59** | **60** |
| Build System Completeness | 5 | 10 |
| **Total** | **64** | **70** |

---

### 9. Verdict

**NEEDS REVISION**

The plan is exemplary in structure, traceability, TDD discipline, integration planning, and adherence to all three reference documents. The single critical issue — **missing `build.vars.in` and `config_unix.h.in` modifications** — is a functional gap that would cause the Rust-path build to fail. The `Makeinfo` conditional depends on shell variables that are only populated through `build.vars.in`, and the `config_unix.h.in` template is the canonical source for the `#define`. Without these, the plan as written will not achieve a successful Rust-path build in P07/P08.

**Required changes before APPROVED:**
1. Add `sc2/build.vars.in` to P06 implementation tasks (add `USE_RUST_MEM`/`uqm_USE_RUST_MEM`/`SYMBOL_USE_RUST_MEM_DEF` entries + export lines, following the exact pattern of `USE_RUST_FILE`).
2. Add `sc2/src/config_unix.h.in` to P06 implementation tasks (add `@SYMBOL_USE_RUST_MEM_DEF@` line after the existing `USE_RUST_*` block).
3. Update `specification.md` "Architectural Boundaries" to include `config_unix.h.in` and `build.vars.in`.
4. Update pseudocode `component-001.md` Component 4 (Config Flag) to cover all three config files.
5. Update P06a, P07, P08, P09a verification checklists to include these files.

Once these additions are made, the plan will be **APPROVED** — the rest of the plan is well above the quality bar.
