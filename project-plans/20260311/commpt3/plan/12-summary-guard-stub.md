# Phase 12: Summary Guard + Stale Marker Elimination — Stub

## Phase ID
`PLAN-20260325-COMMPT3.P12`

## Prerequisites
- Required: Phase P11a (DoCommunication Impl Verification) completed
- Expected: DoCommunication rewrite fully implemented and verified

## Requirements Addressed
- REQ-CS-002, REQ-CS-003, REQ-SM-001, REQ-SM-002 (stubs only — behavior in P14)

## Purpose
Create compile-safe bifurcation of `rust_ShowConversationSummary` into
`#[cfg(not(test))]` and `#[cfg(test)]` paths. Wire the production path to call
`c_SelectConversationSummary()`. No stale marker removal yet.

## Stub Tasks

### Rust bifurcation (ffi.rs)
- Add `#[cfg(not(test))]` block to `rust_ShowConversationSummary()` with
  `c_SelectConversationSummary()` call and return
- Move existing Rust SummaryView logic into `#[cfg(test)]` block
- Verify `c_SelectConversationSummary` extern declaration exists (add if needed)

### Allowed
- Both paths present (cfg(test) and cfg(not(test)))
- Stale markers still present (cleaned in P14)

### Not Allowed
- Removing test-path SummaryView code
- Leaving both paths active in production

## Pseudocode Traceability
- Bifurcation structure: pseudocode `004-summary-guard-stale-markers.md` lines 01-24 (structure only)

## Traceability Markers (in code)
```rust
/// @plan PLAN-20260325-COMMPT3.P12
/// @requirement REQ-CS-002
/// @pseudocode 004-summary-guard-stale-markers lines 01-24
```

## Implementation Tasks

### Files to modify
- `rust/src/comm/ffi.rs` — bifurcate `rust_ShowConversationSummary` with cfg attributes

### Files to create
- None

## Verification Commands

```bash
# Build gate
cd rust && cargo check --workspace --all-features
cd rust && cargo fmt --all --check
cd rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
cd rust && cargo test --workspace --all-features

# Verify cfg(test) structure
grep -B2 -A15 "rust_ShowConversationSummary" rust/src/comm/ffi.rs

# Verify production path
grep -A5 "cfg(not(test))" rust/src/comm/ffi.rs | grep "c_SelectConversationSummary"

# Verify test path retains SummaryView
grep -A10 "cfg(test)" rust/src/comm/ffi.rs | grep "SummaryView"
```

## Structural Verification Checklist
- [ ] `rust_ShowConversationSummary` has `#[cfg(not(test))]` block
- [ ] `rust_ShowConversationSummary` has `#[cfg(test)]` block with SummaryView
- [ ] `c_SelectConversationSummary` extern declaration present
- [ ] Project compiles in both build modes

## Success Criteria
- [ ] Both build modes compile
- [ ] All existing tests pass
- [ ] Production path wired to C delegation

## Failure Recovery
- rollback: `git restore rust/src/comm/ffi.rs`

## Phase Completion Marker
Create: `project-plans/20260311/commpt3/.completed/P12.md`
