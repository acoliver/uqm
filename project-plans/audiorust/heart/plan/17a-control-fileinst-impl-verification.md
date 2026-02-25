# Phase 17a: Control + FileInst Implementation Verification

## Phase ID
`PLAN-20260225-AUDIO-HEART.P17a`

## Prerequisites
- Required: Phase P17 completed
- Expected: control.rs and fileinst.rs fully implemented

## Verification Commands

```bash
cd /Users/acoliver/projects/uqm/rust && cargo test --lib --all-features -- sound::control::tests
cd /Users/acoliver/projects/uqm/rust && cargo test --lib --all-features -- sound::fileinst::tests
cd /Users/acoliver/projects/uqm/rust && cargo test --lib --all-features
cd /Users/acoliver/projects/uqm/rust && cargo fmt --all --check
cd /Users/acoliver/projects/uqm/rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
grep -RIn "TODO\|FIXME\|HACK\|todo!()" rust/src/sound/control.rs rust/src/sound/fileinst.rs
```

## Checks
- [ ] All control tests pass (9+)
- [ ] All fileinst tests pass (6+)
- [ ] All workspace tests pass
- [ ] Zero deferred markers

## Gate Decision
- [ ] PASS: proceed to P18
- [ ] FAIL: fix implementation
