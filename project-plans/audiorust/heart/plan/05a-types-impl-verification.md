# Phase 05a: Types Implementation Verification

## Phase ID
`PLAN-20260225-AUDIO-HEART.P05a`

## Prerequisites
- Required: Phase P05 completed
- Expected: All types fully implemented, all tests passing

## Verification Commands

```bash
cd /Users/acoliver/projects/uqm/rust && cargo test --lib --all-features -- sound::types::tests
cd /Users/acoliver/projects/uqm/rust && cargo fmt --all --check
cd /Users/acoliver/projects/uqm/rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
# Deferred impl detection
grep -RIn "TODO\|FIXME\|HACK\|todo!()" rust/src/sound/types.rs
```

## Checks
- [ ] All tests pass (13+)
- [ ] Zero `todo!()` in types.rs
- [ ] Zero `TODO`/`FIXME`/`HACK` in types.rs
- [ ] fmt passes
- [ ] clippy passes
- [ ] `decode_all` returns non-empty Vec for a decoder with data
- [ ] `get_decoder_time` returns reasonable f32 value

## Gate Decision
- [ ] PASS: proceed to P06
- [ ] FAIL: fix implementation
