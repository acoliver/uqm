# Phase 15a: Control + FileInst Stub Verification

## Phase ID
`PLAN-20260225-AUDIO-HEART.P15a`

## Prerequisites
- Required: Phase P15 completed
- Expected files: `control.rs`, `fileinst.rs`, updated `mod.rs`

## Verification Commands

```bash
cd /Users/acoliver/projects/uqm/rust && cargo check --lib --all-features
cd /Users/acoliver/projects/uqm/rust && cargo fmt --all --check
cd /Users/acoliver/projects/uqm/rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
```

## Checks
- [ ] Both files compile
- [ ] control.rs has 9+ function signatures
- [ ] fileinst.rs has 4+ function signatures
- [ ] SOURCES importable from other sound modules
- [ ] FileLoadGuard has Drop impl

## Gate Decision
- [ ] PASS: proceed to P16
- [ ] FAIL: fix stubs
