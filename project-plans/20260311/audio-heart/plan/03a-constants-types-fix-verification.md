# Phase 03a: Constants & Types Fix — Verification

## Phase ID
`PLAN-20260314-AUDIO-HEART.P03a`

## Prerequisites
- Required: Phase P03 completed

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings 2>&1 | head -50
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] `control.rs` has no local `NORMAL_VOLUME` definition
- [ ] `types.rs` defines `NORMAL_VOLUME = 160`
- [ ] No other module redefines `NORMAL_VOLUME`
- [ ] All tests compile and pass

## Semantic Verification Checklist
- [ ] `VolumeState::new()` uses `NORMAL_VOLUME` (160), not `MAX_VOLUME` (255)
- [ ] Test verifies `NORMAL_VOLUME == 160`

## Success Criteria
- [ ] Verification commands pass
- [ ] Semantic checks pass

## Phase Completion Marker
Create: `project-plans/20260311/audio-heart/.completed/P03a.md`
