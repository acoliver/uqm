# Phase 14a: Summary Guard Implementation Verification

## Phase ID
`PLAN-20260325-COMMPT3.P14a`

## Prerequisites
- Required: Phase P14 completed
- Expected artifacts: Cleaned `ffi.rs`, all stale markers eliminated

## Verification Commands

```bash
cd rust && cargo fmt --all --check
cd rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
cd rust && cargo test --workspace --all-features

# Verify cfg(test) structure
grep -B2 -A15 "rust_ShowConversationSummary" rust/src/comm/ffi.rs

# Verify production delegates to C
grep -A5 "cfg(not(test))" rust/src/comm/ffi.rs | grep "c_SelectConversationSummary"

# Verify test retains SummaryView
grep -A10 "cfg(test)" rust/src/comm/ffi.rs | grep "SummaryView"

# Full stale marker sweep
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
    else echo "FAIL: $f:$line"; fi
  done
done
grep -n 'TODO\|FIXME\|HACK\|placeholder\|for now\|not yet\|stub' sc2/src/uqm/rust_comm.c && echo "FAIL" || echo "CLEAN"
```

## Structural Verification Checklist
- [ ] `rust_ShowConversationSummary` has cfg-bifurcated structure
- [ ] "abort not yet wired" comment gone
- [ ] "input handling is not yet implemented" comment gone
- [ ] No placeholder markers in production code

## Semantic Verification Checklist (Mandatory)

### Summary Path
- [ ] Production path: only `c_SelectConversationSummary()` call + return
- [ ] Test path: retains SummaryView model
- [ ] `c_SelectConversationSummary` extern declaration correct

### Stale Marker Sweep (REQ-SM-001)
- [ ] `ffi.rs` production paths: ZERO markers
- [ ] `talk_segue.rs` production paths: ZERO markers
- [ ] `state.rs` production paths: ZERO markers
- [ ] `rust_comm.c`: ZERO markers
- [ ] `comm.c` USE_RUST_COMM block: ZERO markers

### Marker Exemptions (REQ-SM-002)
- [ ] "stubs in commanim" — C reference, valid EXEMPT
- [ ] "not yet disabled this encounter" — design semantics, valid EXEMPT
- [ ] "not yet initialized" — sentinel description, valid EXEMPT
- [ ] No other unverified exemptions

### Integration
- [ ] All P13 TDD tests PASS
- [ ] All 268+ comm tests pass
- [ ] Both build modes compile

## Semantic Negative-Proof Gate (Mandatory)
- [ ] **Confirmed**: Re-adding a stale marker to production code causes the sweep test to fail
- [ ] **Confirmed**: Removing the C delegation call causes the delegation test to fail
- [ ] **Confirmed**: Exempted markers are correctly classified and genuinely non-deferred

## Phase Completion Marker
Create: `project-plans/20260311/commpt3/.completed/P14a.md`
