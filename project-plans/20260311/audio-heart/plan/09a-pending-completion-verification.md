# Phase 09a: Pending-Completion State Machine — Verification

## Phase ID
`PLAN-20260314-AUDIO-HEART.P09a`

## Prerequisites
- Required: Phase P09 completed

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings 2>&1 | head -50
cargo test --workspace --all-features

# Verify integration-proof references / final ABI declarations
grep -n 'CheckSubtitles\|PollPendingTrackCompletion\|CommitTrackAdvancement' project-plans/20260311/audio-heart/plan/09-pending-completion-state-machine.md rust/src/sound/heart_ffi.rs sc2/src/libs/sound/audio_heart_rust.h 2>/dev/null
```

## Structural Verification Checklist
- [ ] Integration proof recorded exact comm-side call path
- [ ] New provider functions exist in Rust with final consistent signatures
- [ ] FFI exports and header declarations, if introduced, match the proven ABI design
- [ ] stop_track clears pending state

## Semantic Verification Checklist
- [ ] State machine follows spec §8.3.1 as interpreted through the proven consumer contract
- [ ] Claim-and-clear is atomic (provider-side)
- [ ] No callback invocation on decoder thread
- [ ] Deferred recording when slot occupied
- [ ] StopTrack discards cleanly
- [ ] Seek behavior is covered when pending state already exists

## Success Criteria
- [ ] All tests pass
- [ ] Integration proof and provider state machine are complete

## Phase Completion Marker
Create: `project-plans/20260311/audio-heart/.completed/P09a.md`
