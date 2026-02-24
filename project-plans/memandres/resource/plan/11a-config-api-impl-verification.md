# Phase 11a: Config API — Implementation Verification

## Phase ID
`PLAN-20260224-RES-SWAP.P11a`

## Prerequisites
- Required: Phase 11 completed

## Verification Checklist
- [ ] All P10 config tests pass
- [ ] No `todo!()`/`FIXME`/`HACK` in implementation
- [ ] Put auto-creates with correct default values
- [ ] SaveResourceIndex filters by root and strips prefix correctly
- [ ] Color serialization in save matches C format exactly

### Quality Gates
```bash
cargo fmt --all --check && echo "FMT OK"
cargo clippy --workspace --all-targets --all-features -- -D warnings && echo "CLIPPY OK"
cargo test --workspace --all-features && echo "TESTS OK"
```

## Gate Decision
- [ ] PASS: proceed to P12
- [ ] FAIL: fix implementation
