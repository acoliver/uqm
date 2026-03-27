# Phase 14: Summary Guard + Stale Marker Elimination — Implementation

## Phase ID
`PLAN-20260325-COMMPT3.P14`

## Prerequisites
- Required: Phase P13a (Summary Guard TDD Verification) completed
- Expected: tests written, expected failure on stale markers documented

## Requirements Implemented

### REQ-CS-002: Production Delegates to C Summary
`rust_ShowConversationSummary` SHALL delegate to `c_SelectConversationSummary()`
in production builds. The Rust `SummaryView` SHALL only execute under `#[cfg(test)]`.

### REQ-CS-003: No Rust SummaryView in Production
The production path SHALL NOT use the Rust SummaryView pagination loop.

### REQ-SM-001: Zero Stale Markers in Production Paths
Zero instances of `for now`, `TODO`, `FIXME`, `HACK`, `placeholder`, `stub`,
`not yet implemented`, or `not yet wired` SHALL remain in production code paths.

### REQ-SM-002: Permitted Marker Exemptions
Markers inside `#[cfg(test)]`, `///` doc comments describing design rationale, or
references to C stubs MAY remain.

## Implementation Tasks

### Files to modify
- `rust/src/comm/ffi.rs`
  - Remove "abort not yet wired" comment (line ~879)
  - Remove "input handling is not yet implemented" comment (line ~881)
  - Verify `c_SelectConversationSummary` extern declaration exists
  - Final comprehensive marker sweep across all `rust/src/comm/*.rs`
  - marker: `@plan PLAN-20260325-COMMPT3.P14`
  - marker: `@requirement REQ-CS-002, REQ-CS-003, REQ-SM-001`
  - marker: `@pseudocode 004-summary-guard-stale-markers lines 01-47`

### Files to create
- None

## Pseudocode Traceability
- `rust_ShowConversationSummary` rewrite: pseudocode `004-summary-guard-stale-markers.md` lines 01-24
  - Contract: REQ-CS-002, REQ-CS-003
- Stale marker removal (ffi.rs): pseudocode `004-summary-guard-stale-markers.md` lines 28-32
  - Contract: REQ-SM-001
- Exemption verification: pseudocode `004-summary-guard-stale-markers.md` lines 33-40
  - Contract: REQ-SM-002

## Traceability Markers (in code)
```rust
/// @plan PLAN-20260325-COMMPT3.P14
/// @requirement REQ-CS-002, REQ-CS-003, REQ-SM-001
/// @pseudocode 004-summary-guard-stale-markers lines 01-47
```

## Verification Commands

```bash
# Full quality gates
cd rust && cargo fmt --all --check
cd rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
cd rust && cargo test --workspace --all-features

# Comprehensive stale marker sweep — per-match classification
echo "=== Rust production code ==="
for f in rust/src/comm/*.rs; do
  grep -n 'TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented\|not yet wired\|not yet implemented\|todo!\|unimplemented!' "$f" 2>/dev/null | while IFS= read -r line; do
    lineno=$(echo "$line" | cut -d: -f1)
    content=$(sed -n "${lineno}p" "$f")
    if echo "$content" | grep -q 'stubs in commanim'; then echo "EXEMPT (C ref): $f:$line"
    elif echo "$content" | grep -q 'not yet disabled this encounter'; then echo "EXEMPT (design): $f:$line"
    elif echo "$content" | grep -q 'not yet initialized'; then echo "EXEMPT (sentinel): $f:$line"
    elif echo "$content" | grep -q '^ *///'; then echo "EXEMPT (doc): $f:$line"
    elif echo "$content" | grep -q 'cfg(test)'; then echo "EXEMPT (test): $f:$line"
    else echo "FAIL: production marker: $f:$line"; fi
  done
done || echo "CLEAN"

echo "=== C bridge ==="
grep -n 'TODO\|FIXME\|HACK\|placeholder\|for now\|not yet\|stub' sc2/src/uqm/rust_comm.c && echo "FAIL" || echo "CLEAN"

echo "=== comm.c USE_RUST_COMM block ==="
awk '/#ifdef USE_RUST_COMM/,/#endif/' sc2/src/uqm/comm.c | grep -n 'TODO\|FIXME\|HACK\|placeholder\|for now\|not yet' && echo "FAIL" || echo "CLEAN"
```

## Structural Verification Checklist
- [ ] "abort not yet wired" comment removed
- [ ] "input handling is not yet implemented" comment removed
- [ ] `rust_ShowConversationSummary` production path: only `c_SelectConversationSummary()` + return
- [ ] `c_SelectConversationSummary` extern declaration present
- [ ] No `todo!()`, `unimplemented!()`, or placeholder markers in production code

## Semantic Verification Checklist (Mandatory)
- [ ] Production `rust_ShowConversationSummary` delegates to C — no Rust rendering
- [ ] Test `rust_ShowConversationSummary` retains SummaryView for unit tests
- [ ] Zero stale markers in `rust/src/comm/*.rs` production paths (sweep verified)
- [ ] Zero stale markers in `sc2/src/uqm/rust_comm.c` (sweep verified)
- [ ] Zero stale markers in `comm.c` USE_RUST_COMM block (sweep verified)
- [ ] Permitted exemptions verified:
  - [ ] `ffi.rs` "USE_RUST_COMM stubs in commanim.c" — C reference, KEEP
  - [ ] `phrase_state.rs` "not yet disabled this encounter" — design semantics, KEEP
  - [ ] `state.rs` "not yet initialized" — sentinel description, KEEP
- [ ] All P13 TDD tests now PASS (stale marker test no longer fails)
- [ ] All 268+ comm tests pass
- [ ] Both build modes compile

## Semantic Negative-Proof Gate (Mandatory)
- [ ] **Negative proof — marker**: Re-add "abort not yet wired" comment → stale marker
  sweep test (test 3 from P13) fails. Revert.
- [ ] **Negative proof — delegation**: Remove `c_SelectConversationSummary()` call from
  production path → delegation test (test 1 from P13) fails. Revert.
- [ ] **Negative proof — exemption**: Change exempted "stubs in commanim" to "TODO stubs
  in commanim" → classifier still marks it EXEMPT (C ref pattern match). Confirm correct.

## Deferred Implementation Detection (Mandatory)

```bash
fail=0
for f in rust/src/comm/*.rs; do
  grep -n 'TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented\|not yet wired\|not yet implemented\|todo!\|unimplemented!' "$f" 2>/dev/null | while IFS= read -r line; do
    lineno=$(echo "$line" | cut -d: -f1)
    content=$(sed -n "${lineno}p" "$f")
    if echo "$content" | grep -q 'stubs in commanim'; then echo "EXEMPT (C ref): $f:$line"
    elif echo "$content" | grep -q 'not yet disabled this encounter'; then echo "EXEMPT (design): $f:$line"
    elif echo "$content" | grep -q 'not yet initialized'; then echo "EXEMPT (sentinel): $f:$line"
    elif echo "$content" | grep -q '^ *///'; then echo "EXEMPT (doc): $f:$line"
    elif echo "$content" | grep -q 'cfg(test)'; then echo "EXEMPT (test): $f:$line"
    else echo "FAIL: $f:$line"; fail=1; fi
  done
done
grep -n 'TODO\|FIXME\|HACK\|placeholder\|for now\|not yet\|stub' sc2/src/uqm/rust_comm.c && fail=1 || true
[ "$fail" -eq 0 ] && echo "PASS: zero production markers" || echo "FAIL"
```

## Success Criteria
- [ ] All P13 TDD tests now pass
- [ ] Production summary path delegates to C
- [ ] All stale markers eliminated
- [ ] Exemptions documented and justified

## Failure Recovery
- rollback: `git restore rust/src/comm/ffi.rs`

## Phase Completion Marker
Create: `project-plans/20260311/commpt3/.completed/P14.md`

Contents:
- phase ID: PLAN-20260325-COMMPT3.P14
- files changed: `ffi.rs`
- tests that now pass (stale marker sweep)
- negative-proof results
- full sweep output with per-match classification
