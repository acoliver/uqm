# Plan: Memory Subsystem Gap Closure

Plan ID: PLAN-20260314-MEMORY
Generated: 2026-03-14
Total Phases: 12 artifacts (P00.5, P00.5a, P01, P01a, P02, P02a, P03, P03a, P04, P04a, P05, P05a)
Requirements: REQ-MEM-ALLOC-001 through REQ-MEM-ALLOC-010, REQ-MEM-ZERO-001 through REQ-MEM-ZERO-004, REQ-MEM-OOM-001 through REQ-MEM-OOM-005, REQ-MEM-LIFE-001 through REQ-MEM-LIFE-007, REQ-MEM-OWN-001 through REQ-MEM-OWN-007, REQ-MEM-INT-001 through REQ-MEM-INT-009
Reference documents: `project-plans/20260311/memory/specification.md`, `project-plans/20260311/memory/requirements.md`

## Critical Reminders

Before implementing any phase:
1. Preflight verification is complete (Phase 0.5)
2. Integration points are explicitly listed
3. TDD cycle is defined per slice
4. Lint/test/coverage gates are declared

## Overview

The memory subsystem is **already ported and wired**. `USE_RUST_MEM` is active, C code is guarded out, and unit tests pass. This plan is a **gap closure** effort — not a reimplementation.

The gap analysis identified the following concrete deficiencies between the current code and the specification/requirements:

### Gap 1: Zero-size OOM paths are unchecked (CRITICAL)
**Affected functions**: `rust_hmalloc(0)`, `rust_hcalloc(0)`, `rust_hrealloc(_, 0)`
**Problem**: When `size == 0`, the code calls `libc::malloc(1)` but never checks if that returned null. If the 1-byte fallback allocation fails, the function returns null — violating the non-null return contract and the fatal-OOM policy.
**Requirements**: REQ-MEM-OOM-001, REQ-MEM-ZERO-001, REQ-MEM-OOM-003

### Gap 2: `copy_argv_to_c` uses wrong deallocator for CString pointers (BUG)
**Affected code**: `copy_argv_to_c` error path (line 126) and test cleanup (line 276)
**Problem**: `CString::into_raw()` pointers are freed via `libc::free()`. The specification (Appendix A.3) explicitly states these must be reclaimed via `CString::from_raw()`. Using `libc::free` is technically undefined behavior under Rust's allocator-family rules.
**Requirements**: REQ-MEM-OWN-006

### Gap 3: Missing explicit unit-test coverage required by specification §14.1
**Problem**: The current unit-test surface does not explicitly identify or verify two required cases: `HFree(NULL)` safety and `HRealloc(NULL, size)` equivalence to `HMalloc(size)`. The specification calls both out as unit-test obligations, so they must be recognized as current test-surface gaps, not only added later opportunistically.
**Requirements**: Specification §14.1 test-surface obligation supporting REQ-MEM-OWN-003 and REQ-MEM-ALLOC-010

### Gap 4: No project-level mixed-language seam coverage plan
**Problem**: The specification's §14.2 mixed-language tests require actual C↔Rust boundary coverage in both directions. Rust integration tests that call exported symbols directly are useful ABI-surface checks, but they do not fully satisfy the project-level seam-testing intent or prove C header/macro mapping and ownership transfer across the real language boundary.
**Requirements**: REQ-MEM-INT-009 (program-level integration obligation)

### Gap 5: Missing requirement traceability markers
**Problem**: Only one function (`rust_hmalloc`) has a `@requirement` marker. The remaining five exported functions and the utility function lack traceability.
**Requirements**: Plan guide traceability requirement

### Gap 6: Preflight does not verify integration-test harness assumptions
**Problem**: The plan intends to add `rust/tests/memory_integration.rs`, but the feasibility checks must also confirm the actual crate import path, public module visibility, and exact cargo invocation needed for this repository layout.
**Requirements**: Plan accuracy requirement for concrete integration points

## What Is Already at Parity

The vast majority of subsystem requirements are already satisfied:

- **REQ-MEM-ALLOC-001 through REQ-MEM-ALLOC-010**: All allocation API entry points exist with correct signatures, return types, and behavioral contracts, subject to the missing explicit test coverage called out in Gap 3. [OK]
- **REQ-MEM-ZERO-002, REQ-MEM-ZERO-003, REQ-MEM-ZERO-004**: Zero-size policy is implemented and consistent. [OK]
- **REQ-MEM-OOM-002**: OOM diagnostic messages are emitted. [OK]
- **REQ-MEM-OOM-004, REQ-MEM-OOM-005**: No successful-null contract; no recursive allocation in OOM. [OK]
- **REQ-MEM-LIFE-001 through REQ-MEM-LIFE-006, REQ-MEM-LIFE-007**: Lifecycle hooks exist and work; duplicate init logging is treated as optional cleanup rather than a compliance gap. [OK]
- **REQ-MEM-OWN-001 through REQ-MEM-OWN-005, REQ-MEM-OWN-007**: Ownership semantics are correct. [OK]
- **REQ-MEM-INT-001 through REQ-MEM-INT-007**: Module-level integration obligations are substantially met. [OK]
- **REQ-MEM-INT-008**: This is a program-level integration obligation; this plan can improve local allocator-family documentation where relevant but does not claim project-wide closure. [PARTIAL / PROGRAM-LEVEL]
- **REQ-MEM-INT-009**: This is a program-level integration obligation; this plan adds Rust-side ABI integration coverage and records the remaining true mixed-language seam work explicitly. [PARTIAL / PROGRAM-LEVEL]
- **REQ-MEM-INT-005**: Allocator abstraction freedom preserved. [OK]
- **REQ-MEM-INT-006**: Satisfied by existing libc delegation; treat this as pre-existing design evidence rather than a newly proven result of this plan. [OK / PRE-EXISTING DESIGN EVIDENCE]

## Plan Scope

This is a small plan: ~30-40 LoC of production changes and ~80-100 LoC of new tests. The subsystem is at ~95% parity already. The work is:

| Phase | Description | Estimated LoC |
|-------|-------------|---------------|
| P00.5 | Preflight verification | 0 (verification only) |
| P00.5a | Preflight verification review | 0 (verification only) |
| P01   | Analysis of gaps | 0 (documentation only) |
| P01a  | Analysis verification | 0 (verification only) |
| P02   | Pseudocode for fixes | 0 (documentation only) |
| P02a  | Pseudocode verification | 0 (verification only) |
| P03   | Zero-size OOM fix + unit-test gap closure (TDD) | ~25 production, ~55 test |
| P03a  | Zero-size OOM + unit-test verification | 0 (verification only) |
| P04   | `copy_argv_to_c` deallocator fix | ~10 production, ~15 test |
| P04a  | `copy_argv_to_c` fix verification | 0 (verification only) |
| P05   | Rust-side ABI integration tests + traceability markers + project-level seam handoff artifact | ~10 production (markers), ~60 test |
| P05a  | Integration-tests verification + residual seam handoff verification | 0 (verification only) |

## Integration Contract

### Existing Callers
- All C code via `memlib.h` macros → `rust_hmalloc`/`rust_hfree`/`rust_hcalloc`/`rust_hrealloc`
- `rust/src/main.rs` → `rust_mem_init()`, `rust_mem_uninit()`
- `rust/src/sound/heart_ffi.rs` → `crate::memory::rust_hmalloc`, `crate::memory::rust_hfree`

### Existing Code Replaced/Removed
- No files deleted or replaced — only modifications to `rust/src/memory.rs`

### End-to-End Verification
- `cargo test --workspace --all-features` (all existing + new tests pass)
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo fmt --all --check`
- Use the targeted memory integration-test invocation confirmed in P00.5 for this repository layout
- Full game launch (manual smoke test)

### Integration Contract Limits
- This plan can fully close subsystem-local code and unit-test gaps inside the Rust memory module.
- This plan can add Rust integration tests against the exported ABI surface.
- This plan does **not** claim to fully close project-level mixed-language seam obligations from specification §14.2 / REQ-MEM-INT-009 without a dedicated C↔Rust harness.
- This plan does **not** claim project-wide closure of REQ-MEM-INT-008; it only improves allocator-family documentation for the local `copy_argv_to_c` utility.
- This plan must leave a concrete downstream tracking artifact for the remaining REQ-MEM-INT-009 mixed-language seam work.

## Residual Program-Level Handoff

The remaining mixed-language seam obligation is not executable within this module-local plan, but it must still produce a concrete downstream deliverable.

Required handoff artifact:
- Create `project-plans/20260311/memory/req-mem-int-009-followup.md` as the concrete downstream tracking artifact for the remaining true C↔Rust seam tests.
- Update `project-plans/20260311/memory/plan/05-integration-tests-and-markers.md` and `05a-integration-tests-verification.md` so Phase P05/P05a require creation and verification of that exact file.

Required downstream content:
- Expected artifact path for the follow-up plan or tracker entry
- Explicit ownership as project-level work rather than memory-module-local work
- Acceptance criteria covering: real C caller through `memlib.h`, allocation in C with Rust free, allocation in Rust with C free, zero-size seam behavior, lifecycle sequencing across the compiled boundary

## Execution Tracker

| Phase | Status | Verified | Semantic Verified | Notes |
|------:|--------|----------|-------------------|-------|
| P00.5 | ⬜     | ⬜       | N/A               | Preflight |
| P00.5a | ⬜    | ⬜       | ⬜                | Preflight verification review |
| P01   | ⬜     | ⬜       | ⬜                | Analysis |
| P01a  | ⬜     | ⬜       | ⬜                | Analysis verification |
| P02   | ⬜     | ⬜       | ⬜                | Pseudocode |
| P02a  | ⬜     | ⬜       | ⬜                | Pseudocode verification |
| P03   | ⬜     | ⬜       | ⬜                | Zero-size OOM + unit-test gap closure |
| P03a  | ⬜     | ⬜       | ⬜                | Zero-size OOM + unit-test verification |
| P04   | ⬜     | ⬜       | ⬜                | copy_argv_to_c fix |
| P04a  | ⬜     | ⬜       | ⬜                | copy_argv_to_c verification |
| P05   | ⬜     | ⬜       | ⬜                | Rust-side ABI integration tests + markers + seam handoff |
| P05a  | ⬜     | ⬜       | ⬜                | Integration-tests verification + seam handoff verification |
