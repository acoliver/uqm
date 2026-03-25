# Phase 01: Analysis

## Phase ID
`PLAN-20260320-BATTLEPT2.P01`

## Prerequisites
- Required: Phase 0.5 (Preflight Verification) completed with PASS
- Verify: All Phase 1 artifacts intact, toolchain verified, C sources unmodified

## Requirements Implemented (Expanded)

This phase produces no code. It produces analysis artifacts that map every C function to its Rust target, inventory all integration points, and document the ownership transfer model.

### REQ: Function-to-module mapping completeness
**Requirement text**: Every C function in Phase 2/3 scope (75 total: 64 ported + 11 retained) must be mapped to its Rust target module or documented as a permanent C boundary.

Behavior contract:
- GIVEN: The 75-function inventory from `battlept2/specification.md` §12.1
- WHEN: The analysis artifact is produced
- THEN: Every function appears exactly once as either "Ported in Phase X to module Y" or "Retained as permanent C boundary per §3.1"

### REQ: Integration touchpoint inventory
**Requirement text**: All 44 deferred bridge operations (from spec §6.2) must be individually identified and mapped to their consuming phase and bridge wrapper.

Behavior contract:
- GIVEN: The 7 integration traits from `integration.rs`
- WHEN: The touchpoint inventory is produced
- THEN: Each of the 44 deferred operations is listed with its trait, operation name, consuming phase, and bridge wrapper function name

### REQ: Branch-parity inventory
**Requirement text**: All compile-time and runtime branches from spec §13.1 must be inventoried with their source sites and the phases that must handle them.

Behavior contract:
- GIVEN: The branch families from `battlept2/specification.md` §13.1
- WHEN: The branch inventory is produced
- THEN: Each branch family lists all source sites (file:line ranges), affected phases, and verification strategy

## Implementation Tasks

### Files to create
- `project-plans/20260311/battlept2/analysis/domain-model.md` — Complete analysis artifact
  - marker: `@plan PLAN-20260320-BATTLEPT2.P01`
  - Contents:
    1. **Dependency graph** — Rust module dependencies, C→Rust call chains, phase ordering rationale
    2. **Function-by-function mapping** — All 75 C functions → Rust target + Phase 1 types used
    3. **Integration touchpoint inventory** — All 44 deferred bridge operations with trait, operation, consumer phase
    4. **State management analysis** — Display list ownership model during dark-code vs. wired stages
    5. **Callback function pointer analysis** — How callback slots are dispatched when Rust owns process loop
    6. **Display primitive coupling analysis** — Rust process loop interaction with C DisplayArray globals
    7. **Branch-parity inventory** — All 7 branch families from spec §13.1 with source sites and per-phase impact
    8. **FFI safety matrix** — Ownership, lifetime, thread-affinity, panic containment, reentrancy per FFI boundary

### Pseudocode traceability
- N/A (analysis phase — no pseudocode yet)

## Verification Commands

```bash
# No code changes — verify Phase 1 still passes
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] `analysis/domain-model.md` created
- [ ] All 8 analysis sections present
- [ ] All 75 functions appear in the function mapping
- [ ] All 44 deferred bridge operations listed
- [ ] All 7 branch families inventoried

## Semantic Verification Checklist (Mandatory)
- [ ] Function mapping covers every row of spec §12.1
- [ ] No function is double-counted or missing
- [ ] Retained boundary functions match spec §3.1 exactly (11 functions)
- [ ] Integration touchpoints match `integration.rs` trait signatures
- [ ] Branch-parity inventory covers all families from spec §13.1
- [ ] FFI safety matrix addresses spec §10 requirements
- [ ] Callback-slot analysis addresses spec §8 requirements
- [ ] Display primitive coupling is documented per spec §2.3

## Deferred Implementation Detection (Mandatory)

```bash
# No implementation code in this phase — verify no code files were modified
git diff --name-only HEAD | grep -v 'project-plans/'
# Should produce no output
```

## Success Criteria
- [ ] Complete analysis artifact produced
- [ ] All 75 functions accounted for
- [ ] All 44 bridge operations inventoried
- [ ] Phase 1 tests still pass

## Failure Recovery
- rollback: `git checkout -- project-plans/20260311/battlept2/analysis/`
- blocking issues: Missing Phase 1 artifacts, undocumented C functions discovered

## Phase Completion Marker
Create: `project-plans/20260311/battlept2/.completed/P01.md`

Contents:
- phase ID: PLAN-20260320-BATTLEPT2.P01
- timestamp
- files created: analysis/domain-model.md
- verification outputs
- semantic verification summary
