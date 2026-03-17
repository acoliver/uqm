# Phase 02a: Pseudocode Verification

## Phase ID
`PLAN-20260314-MEMORY.P02a`

## Purpose
Verify that pseudocode covers all identified gaps and that every requirement with a gap has a corresponding pseudocode component.

## Structural Verification Checklist
- [ ] Every gap from Phase 01 has a corresponding pseudocode component
- [ ] Pseudocode is numbered and algorithmic
- [ ] Validation points are present (null checks)
- [ ] Error handling is present (OOM abort paths)
- [ ] Integration boundaries are identified (extern "C" surface, CString ownership, residual mixed-language seam handoff)
- [ ] Side effects are documented (log output, pointer ownership transfers, verification artifacts)

## Gap-to-Pseudocode Traceability

| Gap | Pseudocode Component | Lines |
|-----|---------------------|-------|
| Gap 1: Zero-size OOM | Component 1 | 01-40 |
| Gap 2: copy_argv_to_c deallocator | Component 2 | 41-57 |
| Gap 3: Missing explicit unit tests | Component 3 | 58-66 |
| Gap 4: Rust-side ABI integration tests / partial seam coverage | Component 4 | 67-108 |
| Gap 5: Traceability markers | Component 5 | 109-115 |

## Semantic Verification Checklist
- [ ] Zero-size OOM pseudocode matches spec section 4.1 / 6.3
- [ ] copy_argv_to_c pseudocode matches spec Appendix A.3
- [ ] Unit-test pseudocode explicitly covers the specification §14.1 cases for `HFree(NULL)` and `HRealloc(NULL, size)`
- [ ] Rust-side ABI integration pseudocode covers the local ABI-surface checks this plan intends to add:
  - [ ] Allocation/free via exported symbols
  - [ ] Zero-size normalization at ABI seam
  - [ ] `HRealloc(ptr, 0)` behavior via exported symbols
  - [ ] Lifecycle sequencing smoke coverage
- [ ] The pseudocode does not overclaim true C↔Rust seam coverage that requires a dedicated mixed-language harness
- [ ] The pseudocode leaves a concrete downstream handoff requirement for residual REQ-MEM-INT-009 work
- [ ] All test pseudocode verifies behavior, not implementation internals

## Gate Decision
- [ ] PASS: proceed to Phase 03
- [ ] FAIL: revise pseudocode (list issues below)
