# Phase 14a: Music + SFX Implementation Verification

## Phase ID
`PLAN-20260225-AUDIO-HEART.P14a`

## Prerequisites
- Required: Phase P14 completed
- Expected: music.rs and sfx.rs fully implemented, all tests passing


## Requirements Implemented (Expanded)

N/A — Verification-only phase. Requirements are verified, not implemented.

## Implementation Tasks

N/A — Verification-only phase. No code changes.
## Verification Commands

```bash
cd /Users/acoliver/projects/uqm/rust && cargo test --lib --all-features -- sound::music::tests
cd /Users/acoliver/projects/uqm/rust && cargo test --lib --all-features -- sound::sfx::tests
cd /Users/acoliver/projects/uqm/rust && cargo test --lib --all-features
cd /Users/acoliver/projects/uqm/rust && cargo fmt --all --check
cd /Users/acoliver/projects/uqm/rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
cd /Users/acoliver/projects/uqm/sc2 && ./build.sh uqm
# Deferred impl detection
grep -RIn "TODO\|FIXME\|HACK\|todo!()" rust/src/sound/music.rs rust/src/sound/sfx.rs
```

## Structural Verification Checklist
- [ ] All `todo!()` removed from music.rs and sfx.rs (non-test code)
- [ ] All tests pass
- [ ] `cargo fmt` passes
- [ ] `cargo clippy` passes
- [ ] `build.sh uqm` succeeds

## Semantic Verification Checklist

### Deterministic checks
- [ ] All music tests pass: `cargo test --lib --all-features -- sound::music::tests` shows 0 failures
- [ ] All sfx tests pass: `cargo test --lib --all-features -- sound::sfx::tests` shows 0 failures
- [ ] All workspace tests pass: `cargo test --lib --all-features` shows 0 failures
- [ ] Zero deferred markers: `grep -c "TODO\|FIXME\|HACK\|todo!()" rust/src/sound/music.rs rust/src/sound/sfx.rs` returns 0 for both

### Subjective checks
- [ ] `plr_play_song` locks MUSIC_SOURCE, calls `play_stream`, and stores MusicRef — is the lock held for the minimum duration necessary?
- [ ] `fade_music` schedules a volume fade that produces smooth volume transitions — does it call `set_music_stream_fade` with correct parameters and return the completion time?
- [ ] `get_music_data` loads decoder, creates sample, and wraps in `Arc<Mutex<SoundSample>>` — is the MusicRef lifecycle correct (shared ownership via Arc refcounting, no double-free)?
- [ ] `release_music_data` stops active playback, drops the MusicRef's Arc, and lets the sample be freed when last reference drops — does it check if the music is currently active before stopping?
- [ ] `play_channel` implements stop-before-play (REQ-SFX-PLAY-01) — does it call stop_source first?
- [ ] `update_sound_position` uses 3 separate `mixer_source_f` calls for X, Y, Z — NOT mixer_source_fv (which doesn't exist)
- [ ] SFX bank loading pre-decodes all samples — does `get_sound_bank_data` call `decode_all` for each sample (REQ-SFX-LOAD-03)?
- [ ] Volume calculations correct — does `set_music_volume` compute gain = volume / MAX_VOLUME * music_volume_scale?
- [ ] No `unwrap()` or `expect()` in production code paths
- [ ] All error paths return correct AudioError variants

## Deferred Implementation Detection

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented\|todo!()" rust/src/sound/music.rs rust/src/sound/sfx.rs
# Must return 0 results
```

## Success Criteria
- [ ] All 27+ tests pass across both modules
- [ ] Zero deferred implementations
- [ ] Music and SFX fully operational (unit-level)
- [ ] 3D positioning uses individual mixer_source_f calls

## Failure Recovery
- rollback: `git stash` or `git checkout -- rust/src/sound/music.rs rust/src/sound/sfx.rs`
- blocking issues: If stream.rs or mixer API differs, adapt and document

## Phase Completion Marker
Create: `project-plans/audiorust/heart/.completed/P14a.md`
