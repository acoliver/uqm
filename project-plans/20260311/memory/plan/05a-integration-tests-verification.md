# Phase 05a: Integration Tests + Markers Verification

## Phase ID
`PLAN-20260314-MEMORY.P05a`

## Prerequisites
- Required: Phase P05 completed
- Expected artifacts: `rust/tests/memory_integration.rs` created, `rust/src/memory.rs` with full traceability markers, `project-plans/20260311/memory/req-mem-int-009-followup.md` created, and plan text updated to distinguish ABI integration coverage from true mixed-language seam coverage

## Verification Commands

```bash
# Full quality gate
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features

# Targeted integration tests
Use the Phase P00.5-confirmed integration-test invocation for this repository layout

# Verify all traceability markers
grep -c "@requirement REQ-MEM" rust/src/memory.rs
# Expected: >= 6 (one per exported function, some functions have multiple)

grep -c "@plan PLAN-20260314-MEMORY" rust/src/memory.rs
# Expected: >= 6

grep -c "@requirement REQ-MEM" rust/tests/memory_integration.rs
# Expected: >= 1

# Verify downstream handoff artifact and required content
ls -l project-plans/20260311/memory/req-mem-int-009-followup.md
grep -n "^## Owner\|^## Artifact Path\|^## Required Acceptance Criteria\|memlib.h\|C→Rust\|Rust→C\|zero-size\|lifecycle" project-plans/20260311/memory/req-mem-int-009-followup.md

# Final deferred implementation scan
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/memory.rs rust/tests/memory_integration.rs
# Expected: 0 matches
```

## Structural Verification Checklist
- [ ] `rust/tests/memory_integration.rs` exists and contains the planned Rust-side ABI integration tests
- [ ] The integration tests compile and pass via the P00.5-confirmed command
- [ ] The integration test file imports via the P00.5-confirmed library path
- [ ] All 6 exported functions have `@requirement` markers
- [ ] All 6 exported functions have `@plan` markers
- [ ] Integration test file has `@plan` and `@requirement` markers
- [ ] No deferred implementation patterns in either file
- [ ] `project-plans/20260311/memory/req-mem-int-009-followup.md` exists
- [ ] The downstream artifact contains owner, artifact path, expected downstream deliverable, and compiled-boundary acceptance criteria

## Semantic Verification Checklist (Mandatory)
- [ ] `test_allocate_and_free_via_exported_abi`: writes and reads 64 bytes correctly
- [ ] `test_calloc_zero_fill_via_exported_abi`: all 128 bytes verified as zero
- [ ] `test_realloc_preserves_data_via_exported_abi`: 32-byte pattern preserved after realloc to 256
- [ ] `test_zero_size_normalization_via_exported_abi`: all three zero-size calls return non-null
- [ ] `test_lifecycle_smoke_via_exported_abi`: init and uninit both return true
- [ ] `test_realloc_zero_from_live_pointer_via_exported_abi`: realloc-to-zero returns non-null replacement safe to free
- [ ] Verification notes explicitly distinguish ABI integration coverage from true mixed-language seam coverage
- [ ] `project-plans/20260311/memory/req-mem-int-009-followup.md` names owner, exact artifact path, expected downstream deliverable, and acceptance criteria for compiled C↔Rust seam tests

## Final Requirements Coverage Audit

This audit must distinguish subsystem-local closure from program-level obligations:

| Requirement | Status | Evidence |
|-------------|--------|----------|
| REQ-MEM-ALLOC-001 through 010 | Satisfied | Functions exist with correct ABI; unit tests and Rust-side ABI integration tests cover key contracts |
| REQ-MEM-ZERO-001 | Satisfied | Zero-size OOM check added (P03); tests verify non-null |
| REQ-MEM-ZERO-002 | Satisfied | memset on zero-size calloc path |
| REQ-MEM-ZERO-003 | Satisfied | realloc(ptr, 0) frees and returns non-null; ABI integration test verifies |
| REQ-MEM-ZERO-004 | Satisfied | Consistent 1-byte normalization policy |
| REQ-MEM-OOM-001 | Satisfied | All paths (including zero-size) abort on null (P03) |
| REQ-MEM-OOM-002 | Satisfied | log_add(Fatal) before abort |
| REQ-MEM-OOM-003 | Satisfied | Zero-size normalization before OOM check (P03) |
| REQ-MEM-OOM-004 | Satisfied | Dead null-check removed from copy_argv_to_c (P04) |
| REQ-MEM-OOM-005 | Satisfied | OOM handler uses log_add (no allocation) |
| REQ-MEM-LIFE-001 | Satisfied | rust_mem_init exists and returns bool |
| REQ-MEM-LIFE-002 | Satisfied | rust_mem_uninit exists and returns bool |
| REQ-MEM-LIFE-003 | Satisfied | After init, allocations work |
| REQ-MEM-LIFE-004 | Satisfied | uninit does not invalidate prior operations |
| REQ-MEM-LIFE-005 | Satisfied | Lifecycle hooks are no-ops with logging |
| REQ-MEM-LIFE-006 | Satisfied | Current behavior is already idempotent; no compliance gap claimed |
| REQ-MEM-LIFE-007 | N/A | Usage constraint, not subsystem obligation |
| REQ-MEM-OWN-001 through 005 | Satisfied | Ownership semantics correct; explicit unit tests added for NULL free and NULL realloc |
| REQ-MEM-OWN-006 | Satisfied | copy_argv_to_c uses correct deallocators (P04) |
| REQ-MEM-OWN-007 | Satisfied | heart_ffi.rs uses HMalloc/HFree correctly |
| REQ-MEM-INT-001 through 007 | Satisfied | Header compat, legacy exclusion, crate integration, and thread-safety evidence documented; treat thread safety as pre-existing libc-delegation evidence rather than a newly proven property of this plan |
| REQ-MEM-INT-008 | Partial / Program-level | `copy_argv_to_c` doc comment improves one local API, but project-wide FFI-boundary documentation is outside this plan |
| REQ-MEM-INT-009 | Partial / Program-level | Rust-side ABI integration tests added in P05; concrete downstream tracking artifact at `project-plans/20260311/memory/req-mem-int-009-followup.md` captures the remaining true C↔Rust seam tests |

## Gate Decision
- [ ] PASS: Plan complete, all subsystem-local gaps closed and residual project-level work explicitly handed off
- [ ] FAIL: identify remaining gaps and create additional phases
