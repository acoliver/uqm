# Phase 08a: PLRPause & Behavioral Fixes — Verification

## Phase ID
`PLAN-20260314-AUDIO-HEART.P08a`

## Prerequisites
- Required: Phase P08 completed

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings 2>&1 | head -50
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] `plr_pause_if_matching` exists in `music.rs`
- [ ] `PLRPause` in `heart_ffi.rs` uses ref-matching
- [ ] Tests for matching/non-matching exist

## Semantic Verification Checklist
- [ ] Non-matching ref leaves playback unchanged
- [ ] Matching ref pauses
- [ ] Wildcard pauses unconditionally

## Success Criteria
- [ ] All verification commands pass
- [ ] Behavior matches C parity (spec §10.4)

## Phase Completion Marker
Create: `project-plans/20260311/audio-heart/.completed/P08a.md`
