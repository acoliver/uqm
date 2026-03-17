# Phase 07a: System-Box + ReinitVideo Verification

## Phase ID
`PLAN-20260314-GRAPHICS.P07a`

## Prerequisites
- Required: Phase P07 completed

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] System-box call in C compositing sequence (under USE_RUST_GFX)
- [ ] ReinitVideo handler is real implementation (not log-only)
- [ ] Reversion + exit(1) paths present
- [ ] Tests added for ReinitVideo handler

## Semantic Verification Checklist (Mandatory)
- [ ] Compositing order: main → transition → fade → system-box → postprocess
- [ ] ReinitVideo: uninit → init → (revert on fail) → (exit on double fail)
- [ ] All existing tests pass unchanged
