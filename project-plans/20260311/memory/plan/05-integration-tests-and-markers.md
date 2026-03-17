# Phase 05: Rust-Side ABI Integration Tests + Traceability Markers

## Phase ID
`PLAN-20260314-MEMORY.P05`

## Prerequisites
- Required: Phase P04 completed and verified
- Expected artifacts: `rust/src/memory.rs` with all production fixes applied

## Requirements Implemented (Expanded)

### REQ-MEM-INT-009: Mixed-language integration test coverage (partial closure in this plan)
**Requirement text**: When the memory subsystem serves as a cross-language allocation boundary, the project test suite should include dedicated mixed-language integration tests that exercise allocation in one language and deallocation in the other, zero-size normalization at the ABI seam, and lifecycle sequencing, so that boundary-specific risks are directly verified rather than only indirectly covered by single-language unit tests.

Local plan contract:
- GIVEN: The memory subsystem exports `extern "C"` functions and the crate exposes `pub mod memory`
- WHEN: Rust integration tests call those functions through the library crate as an external test target
- THEN: ABI-surface behavior such as allocation, deallocation, zero-size normalization, realloc preservation, null-free safety, and lifecycle smoke coverage are verified

Why it matters:
- Unit tests in the module test internal Rust behavior. Integration tests verify the exported ABI surface that downstream code relies on.
- These tests are valuable but are **not** the same as true C↔Rust seam tests. They do not by themselves verify `memlib.h` macro/header mapping or ownership transfer across a compiled C fixture.
- Therefore this phase provides partial progress toward REQ-MEM-INT-009 and must not be presented as full project-level closure.

## Implementation Tasks

### Files to create
- `rust/tests/memory_integration.rs` — Rust integration tests for the exported memory ABI surface
  - marker: `@plan PLAN-20260314-MEMORY.P05`
  - marker: `@requirement REQ-MEM-INT-009` (partial / ABI-surface coverage)
  - Import path: use the library-crate import path confirmed in P00.5
  - Tests (from pseudocode lines 67-108):
    - `test_allocate_and_free_via_exported_abi`: Allocate 64 bytes, write pattern, verify, free
    - `test_calloc_zero_fill_via_exported_abi`: Allocate 128 zeroed bytes, verify all zero, free
    - `test_realloc_preserves_data_via_exported_abi`: Allocate 32, write pattern, realloc to 256, verify preserved data, free
    - `test_zero_size_normalization_via_exported_abi`: `hmalloc(0)`, `hcalloc(0)`, `hrealloc(null, 0)` all return non-null, free all
    - `test_lifecycle_smoke_via_exported_abi`: `mem_init()` then `mem_uninit()` both return true
    - `test_realloc_zero_from_live_pointer_via_exported_abi`: `hrealloc(ptr, 0)` frees old block and returns non-null replacement
- `project-plans/20260311/memory/req-mem-int-009-followup.md`
  - Create this exact downstream project-level tracking artifact during P05
  - Record owner, artifact path, expected downstream deliverable, and acceptance criteria for the remaining compiled C↔Rust seam harness work
  - Acceptance criteria must explicitly cover: real C caller through `memlib.h`, C→Rust free, Rust→C free, zero-size seam behavior, and lifecycle sequencing across the compiled boundary

### Files to modify
- `rust/src/memory.rs` — Add traceability markers to all exported functions systematically
  - `rust_hmalloc`: `@plan PLAN-20260314-MEMORY.P05`; `@requirement REQ-MEM-ALLOC-001, REQ-MEM-ALLOC-002, REQ-MEM-ALLOC-003, REQ-MEM-ZERO-001, REQ-MEM-OOM-001, REQ-MEM-OOM-002, REQ-MEM-OOM-003, REQ-MEM-ALLOC-008`
  - `rust_hfree`: `@plan PLAN-20260314-MEMORY.P05`; `@requirement REQ-MEM-ALLOC-006, REQ-MEM-ALLOC-007, REQ-MEM-OWN-003`
  - `rust_hcalloc`: `@plan PLAN-20260314-MEMORY.P05`; `@requirement REQ-MEM-ALLOC-004, REQ-MEM-ALLOC-009, REQ-MEM-ZERO-001, REQ-MEM-ZERO-002, REQ-MEM-OOM-001, REQ-MEM-OOM-002, REQ-MEM-OOM-003, REQ-MEM-ALLOC-008`
  - `rust_hrealloc`: `@plan PLAN-20260314-MEMORY.P05`; `@requirement REQ-MEM-ALLOC-005, REQ-MEM-ALLOC-007, REQ-MEM-ALLOC-010, REQ-MEM-ZERO-003, REQ-MEM-OOM-001, REQ-MEM-OOM-002, REQ-MEM-OOM-003, REQ-MEM-OWN-004, REQ-MEM-OWN-005, REQ-MEM-ALLOC-008`
  - `rust_mem_init`: `@plan PLAN-20260314-MEMORY.P05`; `@requirement REQ-MEM-LIFE-001, REQ-MEM-LIFE-003, REQ-MEM-LIFE-005, REQ-MEM-LIFE-006`
  - `rust_mem_uninit`: `@plan PLAN-20260314-MEMORY.P05`; `@requirement REQ-MEM-LIFE-002, REQ-MEM-LIFE-004, REQ-MEM-LIFE-005`
- `project-plans/20260311/memory/plan/05a-integration-tests-verification.md`
  - Require verification of the exact downstream artifact at `project-plans/20260311/memory/req-mem-int-009-followup.md`

### Pseudocode traceability
- Uses pseudocode lines: 67-115

## Integration Test Design Notes

The integration tests in `rust/tests/memory_integration.rs` call the exported `extern "C"` functions directly from outside the library crate, which verifies the crate export path and ABI-facing symbol behavior.

The import path and exact cargo invocation for this repository layout must come from P00.5 rather than being re-assumed here. The package name and library crate name may differ, so P05 must reuse the verified preflight result.

The module visibility dependency is explicit: `rust/src/lib.rs` must continue to expose the memory module in the manner confirmed by P00.5 for the chosen integration-test strategy to work.

These tests are intentionally described as **Rust-side ABI integration tests**, not full mixed-language seam tests. They do not compile a C fixture and therefore do not directly verify:
- `memlib.h` macro/header mapping
- C allocation followed by Rust free
- Rust allocation followed by C free through an actual compiled C caller

This phase must also leave a concrete downstream artifact for the remaining seam work. The follow-up must identify:
- exact artifact path: `project-plans/20260311/memory/req-mem-int-009-followup.md`
- project-level owner
- expected downstream deliverable for the compiled seam harness
- acceptance criteria covering: real C caller through `memlib.h`, C→Rust free, Rust→C free, zero-size seam behavior, and lifecycle sequencing across the compiled boundary

## Verification Commands

```bash
# Structural gate
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features

# Targeted integration tests
Use the Phase P00.5-confirmed integration-test invocation for this repository layout

# Verify traceability markers are present
grep -n "@requirement REQ-MEM" rust/src/memory.rs
grep -n "@plan PLAN-20260314-MEMORY" rust/src/memory.rs
grep -n "@requirement REQ-MEM" rust/tests/memory_integration.rs

# Verify downstream handoff artifact exists
ls -l project-plans/20260311/memory/req-mem-int-009-followup.md

# Deferred implementation detection
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/memory.rs
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/tests/memory_integration.rs
```

## Structural Verification Checklist
- [ ] `rust/tests/memory_integration.rs` created with the planned Rust-side ABI integration tests
- [ ] The test file uses the library-crate import path confirmed in P00.5
- [ ] All 6 exported functions in `rust/src/memory.rs` have `@requirement` markers
- [ ] No skipped phases
- [ ] Plan/requirement traceability present in all files
- [ ] `project-plans/20260311/memory/req-mem-int-009-followup.md` is created during P05

## Semantic Verification Checklist (Mandatory)
- [ ] Integration tests exercise the exported ABI surface from outside the library crate
- [ ] Tests cover: alloc+free, calloc zero-fill, realloc preservation, zero-size normalization, lifecycle smoke coverage, realloc-to-zero from a live pointer
- [ ] Tests verify behavior (data content, non-null returns), not implementation internals
- [ ] All pre-existing tests still pass
- [ ] No placeholder/deferred implementation patterns remain
- [ ] Integration test file is discoverable by the verified cargo command
- [ ] Phase text does not overclaim full C↔Rust seam coverage
- [ ] The downstream seam-handoff artifact is specific enough to execute later without rediscovery

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/memory.rs rust/tests/memory_integration.rs
```

## Success Criteria
- [ ] Planned Rust-side ABI integration tests exist and pass
- [ ] All 6 exported functions have requirement markers
- [ ] All verification commands pass
- [ ] Semantic checks pass
- [ ] Remaining true mixed-language seam coverage is explicitly documented as separate project-level work with a concrete downstream tracking artifact at `project-plans/20260311/memory/req-mem-int-009-followup.md`

## Failure Recovery
- Rollback: `git checkout -- rust/src/memory.rs && rm -f rust/tests/memory_integration.rs`
- Blocking issues: If integration test file is not picked up by cargo, use the P00.5 findings to correct the repository-specific test invocation

## Phase Completion Marker
Create: `project-plans/20260311/memory/.completed/P05.md`
