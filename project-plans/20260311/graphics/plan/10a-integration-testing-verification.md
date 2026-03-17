# Phase 10a: Integration Testing Verification

## Phase ID
`PLAN-20260314-GRAPHICS.P10a`

## Prerequisites
- Required: Phase P10 completed

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo test --test graphics_integration
find rust/src/graphics/ -name "*.bak*" | wc -l
```

## Structural Verification Checklist
- [ ] Integration test file exists with 6+ tests
- [ ] All integration tests pass
- [ ] 0 backup files remain
- [ ] No deferred implementation patterns in production code

## Semantic Verification Checklist (Mandatory)
- [ ] Lifecycle: init → ops → uninit works
- [ ] DCQ: push → flush pipeline works
- [ ] Canvas: pixel roundtrip maintains coherence
- [ ] Colormap: set/get preserves data
- [ ] Batch/unbatch: correct visibility semantics
- [ ] Screen targeting: correct tagging
