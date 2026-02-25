# Phase 11: Track Player Implementation

## Phase ID
`PLAN-20260225-AUDIO-HEART.P11`

## Prerequisites
- Required: Phase P10a (Track Player TDD Verification) passed
- Expected: 25+ tests in trackplayer.rs

## Requirements Implemented (Expanded)

All TRACK-* requirements (57 total): TRACK-ASSEMBLE-01..19, TRACK-PLAY-01..10, TRACK-SEEK-01..13, TRACK-CALLBACK-01..09, TRACK-SUBTITLE-01..04, TRACK-POSITION-01..02.

### Pseudocode traceability
- `splice_track`: pseudocode `trackplayer.md` lines 1-89
- `splice_multi_track`: pseudocode `trackplayer.md` lines 100-135
- `split_sub_pages`: pseudocode `trackplayer.md` lines 140-164
- `get_time_stamps`: pseudocode `trackplayer.md` lines 170-180
- `play_track`: pseudocode `trackplayer.md` lines 190-210
- `stop_track`: pseudocode `trackplayer.md` lines 220-242
- `jump_track`: pseudocode `trackplayer.md` lines 250-258
- `seek_track`: pseudocode `trackplayer.md` lines 300-332
- `TrackCallbacks`: pseudocode `trackplayer.md` lines 410-460
- `do_track_tag`: pseudocode `trackplayer.md` lines 470-476

## Implementation Tasks

### Files to modify
- `rust/src/sound/trackplayer.rs` — Replace all `todo!()` with implementations
  - marker: `@plan PLAN-20260225-AUDIO-HEART.P11`
  - marker: `@requirement REQ-TRACK-*`

### Implementation priority order
1. `split_sub_pages` and `get_time_stamps` — pure functions, no state
2. `SoundChunk::new` and linked list helpers
3. `splice_track` — core assembly logic
4. `splice_multi_track` — multi-decoder assembly
5. `play_track`, `stop_track`, `jump_track` — playback control
6. `pause_track`, `resume_track`, `playing_track` — state queries
7. `seek_track` with `find_next_page`, `find_prev_page` — navigation
8. `TrackCallbacks` implementation — callback wiring
9. `do_track_tag` — subtitle tag firing
10. Subtitle and position queries

### Key implementation notes
- SoundChunk linked list uses `Box<SoundChunk>` for ownership, `*mut SoundChunk` for non-owning back-pointers
- `unsafe impl Send` justified by single-thread mutation pattern
- `cur_chunk`/`cur_sub_chunk` as `Option<NonNull<SoundChunk>>` for null-safety with raw access
- Recursive Drop on linked list (acceptable for UQM track lengths)

## Verification Commands

```bash
cd /Users/acoliver/projects/uqm/rust && cargo test --lib --all-features -- sound::trackplayer::tests
cd /Users/acoliver/projects/uqm/rust && cargo test --lib --all-features
cd /Users/acoliver/projects/uqm/rust && cargo fmt --all --check
cd /Users/acoliver/projects/uqm/rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
```

## Structural Verification Checklist
- [ ] All `todo!()` removed from trackplayer.rs
- [ ] All tests pass
- [ ] fmt and clippy pass

## Semantic Verification Checklist (Mandatory)
- [ ] `splice_track` builds linked list correctly
- [ ] `split_sub_pages` produces correct continuation marks
- [ ] `stop_track` frees all chunks (no leaks)
- [ ] `TrackCallbacks::on_end_chunk` advances cur_chunk
- [ ] Seek functions clamp correctly
- [ ] Subtitle queries return correct text

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented\|todo!()" rust/src/sound/trackplayer.rs
# Must return 0 results
```

## Success Criteria
- [ ] All 25+ tests pass
- [ ] Zero deferred implementations
- [ ] Track player fully operational (unit-level)

## Failure Recovery
- rollback: `git stash` or `git checkout -- rust/src/sound/trackplayer.rs`

## Phase Completion Marker
Create: `project-plans/audiorust/heart/.completed/P11.md`
