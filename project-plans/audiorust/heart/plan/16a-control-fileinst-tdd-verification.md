# Phase 16a: Control + FileInst TDD Verification

## Phase ID
`PLAN-20260225-AUDIO-HEART.P16a`

## Prerequisites
- Required: Phase P16 completed
- Expected: test modules in control.rs and fileinst.rs

## Verification Commands

```bash
cd /Users/acoliver/projects/uqm/rust && cargo test --lib --all-features -- sound::control::tests 2>&1
cd /Users/acoliver/projects/uqm/rust && cargo test --lib --all-features -- sound::fileinst::tests 2>&1
cd /Users/acoliver/projects/uqm/rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
```

## Checks
- [ ] All control tests compile (9+)
- [ ] All fileinst tests compile (6+)
- [ ] Tests cover all requirement categories

## Gate Decision
- [ ] PASS: proceed to P17
- [ ] FAIL: fix tests
