# Phase 04a: .rmp Parser Fix — TDD Verification

## Phase ID
`PLAN-20260224-RES-SWAP.P04a`

## Prerequisites
- Required: Phase 04 completed

## Verification Checklist

### Structural
- [ ] All test functions listed in P04 exist in the codebase
- [ ] Tests compile without errors
- [ ] Tests are properly attributed with plan markers

### Semantic
- [ ] Tests fail when run against stub implementations (RED confirmed)
- [ ] Test assertions verify actual behavior, not just compilation
- [ ] Multi-colon test (3DOVID, CONVERSATION) explicitly tests first-colon-only split
- [ ] Case sensitivity test verifies original case preservation

### RED Confirmation
```bash
cargo test --workspace --all-features 2>&1 | grep "FAILED"
# Expected: Tests that exercise parse_type_path and parse_propfile should fail
```

## Gate Decision
- [ ] PASS: proceed to P05
- [ ] FAIL: fix tests
