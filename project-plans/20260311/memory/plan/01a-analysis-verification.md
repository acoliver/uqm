# Phase 01a: Analysis Verification

## Phase ID
`PLAN-20260314-MEMORY.P01a`

## Purpose
Verify that the gap analysis is complete and accurate before proceeding to pseudocode.

## Structural Verification Checklist
- [ ] All requirements from `requirements.md` are accounted for (either satisfied or gap-identified)
- [ ] Each gap references specific file paths and line numbers
- [ ] Each gap references specific REQ-* IDs
- [ ] Each gap references specific specification sections
- [ ] No gaps are assumed without code evidence

## Semantic Verification Checklist
- [ ] Every exported function has been reviewed against its spec section
- [ ] Zero-size paths have been traced for all three allocation functions
- [ ] OOM paths have been traced for all three allocation functions
- [ ] `copy_argv_to_c` ownership model matches Appendix A.3
- [ ] Lifecycle hook behavior matches spec sections 7.1 and 7.2
- [ ] Integration points (memlib.h, main.rs, heart_ffi.rs) confirmed stable
- [ ] Specification §14.1 unit-test obligations are all accounted for in the gap analysis
- [ ] Program-level obligations (REQ-MEM-INT-008, REQ-MEM-INT-009) are not overstated as module-local closure
- [ ] Planned integration-test import path / crate visibility assumptions are grounded in repo evidence

## Requirements Coverage Matrix

| Requirement | Status | Gap # |
|-------------|--------|-------|
| REQ-MEM-ALLOC-001 | Satisfied | - |
| REQ-MEM-ALLOC-002 | Satisfied | - |
| REQ-MEM-ALLOC-003 | Satisfied | - |
| REQ-MEM-ALLOC-004 | Satisfied | - |
| REQ-MEM-ALLOC-005 | Satisfied | - |
| REQ-MEM-ALLOC-006 | Satisfied | - |
| REQ-MEM-ALLOC-007 | Satisfied | - |
| REQ-MEM-ALLOC-008 | Satisfied | - |
| REQ-MEM-ALLOC-009 | Satisfied | - |
| REQ-MEM-ALLOC-010 | Gap | Gap 3 (spec-required explicit unit test for `HRealloc(NULL, size)` missing) |
| REQ-MEM-ZERO-001 | Gap | Gap 1 (null check missing on zero-size fallback) |
| REQ-MEM-ZERO-002 | Satisfied | - |
| REQ-MEM-ZERO-003 | Satisfied | - |
| REQ-MEM-ZERO-004 | Satisfied | - |
| REQ-MEM-OOM-001 | Gap | Gap 1 (zero-size fallback not covered by OOM) |
| REQ-MEM-OOM-002 | Satisfied | - |
| REQ-MEM-OOM-003 | Gap | Gap 1 (zero-size OOM would be false positive without fix) |
| REQ-MEM-OOM-004 | Satisfied | - |
| REQ-MEM-OOM-005 | Satisfied | - |
| REQ-MEM-LIFE-001 | Satisfied | - |
| REQ-MEM-LIFE-002 | Satisfied | - |
| REQ-MEM-LIFE-003 | Satisfied | - |
| REQ-MEM-LIFE-004 | Satisfied | - |
| REQ-MEM-LIFE-005 | Satisfied | - |
| REQ-MEM-LIFE-006 | Satisfied | - (current no-op/logging behavior is already idempotent; duplicate logs are quality-only) |
| REQ-MEM-LIFE-007 | Satisfied | - (usage constraint, not subsystem obligation) |
| REQ-MEM-OWN-001 | Satisfied | - |
| REQ-MEM-OWN-002 | Satisfied | - |
| REQ-MEM-OWN-003 | Gap | Gap 3 (spec-required explicit unit test for `HFree(NULL)` missing) |
| REQ-MEM-OWN-004 | Satisfied | - |
| REQ-MEM-OWN-005 | Satisfied | - |
| REQ-MEM-OWN-006 | Gap | Gap 2 (copy_argv_to_c wrong deallocator) |
| REQ-MEM-OWN-007 | Satisfied | - |
| REQ-MEM-INT-001 | Satisfied | - |
| REQ-MEM-INT-002 | Satisfied | - |
| REQ-MEM-INT-003 | Satisfied | - |
| REQ-MEM-INT-004 | Satisfied | - (usage constraint) |
| REQ-MEM-INT-005 | Satisfied | - |
| REQ-MEM-INT-006 | Satisfied | - |
| REQ-MEM-INT-007 | Satisfied | - |
| REQ-MEM-INT-008 | Partial / Program-level | Gap 2 improves local allocator-family correctness and docs, but project-wide closure is outside this module plan |
| REQ-MEM-INT-009 | Partial / Program-level | Gap 4 (no true mixed-language seam coverage; Rust-side ABI integration tests are only partial closure) |

## Gate Decision
- [ ] PASS: proceed to Phase 02
- [ ] FAIL: revise analysis (list issues below)
