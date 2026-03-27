# Phase 15a: Integration Build Verification

## Phase ID
`PLAN-20260325-COMMPT3.P15a`

## Prerequisites
- Required: Phase P15 completed
- Expected artifacts: All build and test outputs captured

## Verification Commands

```bash
cd rust && cargo fmt --all --check
cd rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
cd rust && cargo test --workspace --all-features

cd rust && cargo test --workspace --all-features -- comm 2>&1 | grep "test result"
```

## Structural Verification Checklist
- [ ] `cargo fmt` passes
- [ ] `cargo clippy` passes with zero warnings
- [ ] `cargo test` passes with 268+ comm tests, 0 failures
- [ ] `USE_RUST_COMM=on` C build compiles and links
- [ ] `USE_RUST_COMM=off` C build compiles and links
- [ ] Completion markers exist for all 12 implementation phases (P03-P14)
- [ ] `@plan` markers confirmed in all 5 modified files
- [ ] `@requirement` markers confirmed in all 4 implementation files
- [ ] `@pseudocode` cross-references confirmed

## Semantic Verification Checklist (Mandatory)

### Cross-Module Integration
- [ ] `rust_comm.c` → `comm.c` subtitle path: no linker errors
- [ ] `talk_segue.rs` → `rust_comm.c` bridge calls: no undefined symbols
- [ ] `ffi.rs` → `talk_segue.rs` `do_communication`: return type matches
- [ ] `ffi.rs` → `c_SelectConversationSummary`: linker resolves correctly

### Pseudocode Traceability Audit
- [ ] P03/P05 code references pseudocode `001-colormap-music-bridges.md`
- [ ] P06/P08 code references pseudocode `002-subtitle-display-fix.md`
- [ ] P09/P11 code references pseudocode `003-do-communication-rewrite.md`
- [ ] P12/P14 code references pseudocode `004-summary-guard-stale-markers.md`

### Deferred Implementation Final Gate
- [ ] Per-match classification: ZERO unexempted matches
- [ ] Only three REQ-SM-002 exemptions permitted

### Deadlock Freedom (structural)
- [ ] `rust_DoCommunication`: exactly one lock acquisition
- [ ] `drop(state)` in Selected arm precedes callback
- [ ] No other FFI function holds write lock during extern "C" calls

## Semantic Negative-Proof Gate (Mandatory)
- [ ] **Confirmed**: If any implementation phase's completion marker is missing,
  the Stub→TDD→Impl audit fails
- [ ] **Confirmed**: If any `@plan` marker is missing from a modified file,
  the traceability audit fails
- [ ] **Confirmed**: If a deferred marker were added to production code,
  the sweep would catch it

## Phase Completion Marker
Create: `project-plans/20260311/commpt3/.completed/P15a.md`
