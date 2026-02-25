# Phase 09: Track Player Stub

## Phase ID
`PLAN-20260225-AUDIO-HEART.P09`

## Prerequisites
- Required: Phase P08a (Stream Implementation Verification) passed
- Expected files: `stream.rs` fully implemented

## Requirements Implemented (Expanded)

### REQ-TRACK-ASSEMBLE-01..19: Track Assembly Stubs
**Requirement text**: Linked-list chunk assembly with subtitle page splitting.

Behavior contract:
- GIVEN: stream.rs provides SoundSample, play_stream, etc.
- WHEN: trackplayer.rs is created with all function signatures
- THEN: All track player public functions compile with `todo!()` bodies

### REQ-TRACK-PLAY-01..10: Playback Control Stubs
### REQ-TRACK-SEEK-01..13: Seeking Stubs
### REQ-TRACK-CALLBACK-01..09: Callback Stubs
### REQ-TRACK-SUBTITLE-01..04: Subtitle Query Stubs
### REQ-TRACK-POSITION-01..02: Position Tracking Stubs

## Implementation Tasks

### Files to create
- `rust/src/sound/trackplayer.rs` — All public API from spec §3.2.3, internal types from §3.2.2
  - marker: `@plan PLAN-20260225-AUDIO-HEART.P09`
  - marker: `@requirement REQ-TRACK-*`

### Files to modify
- `rust/src/sound/mod.rs` — Add `pub mod trackplayer;`
  - marker: `@plan PLAN-20260225-AUDIO-HEART.P09`

### Stub contents
1. `SoundChunk` struct with all fields (decoder, start_time, tag_me, track_num, text, callback, next)
2. `SubtitleRef` struct
3. `SubPage` struct (text, timestamp)
4. `TrackPlayerState` struct with all fields
5. `unsafe impl Send for TrackPlayerState {}`
6. `TrackCallbacks` struct implementing `StreamCallbacks`
7. `lazy_static! { static ref TRACK_STATE: Mutex<TrackPlayerState> }` (or `OnceLock`)
8. Constants: `TEXT_SPEED`, `ACCEL_SCROLL_SPEED`, `MAX_MULTI_TRACKS`
9. All public functions with `todo!()`:
   - `splice_track`, `splice_multi_track`
   - `play_track`, `stop_track`, `jump_track`, `pause_track`, `resume_track`, `playing_track`
   - `fast_reverse_smooth`, `fast_forward_smooth`, `fast_reverse_page`, `fast_forward_page`
   - `get_track_position`, `get_track_subtitle`, `get_first_track_subtitle`, `get_next_track_subtitle`, `get_track_subtitle_text`
10. Internal functions with `todo!()`:
    - `split_sub_pages`, `get_time_stamps`, `seek_track`, `find_next_page`, `find_prev_page`, `do_track_tag`, `get_current_track_pos`, `tracks_end_time`

## Verification Commands

```bash
cd /Users/acoliver/projects/uqm/rust && cargo check --lib --all-features
cd /Users/acoliver/projects/uqm/rust && cargo fmt --all --check
cd /Users/acoliver/projects/uqm/rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
```

## Structural Verification Checklist
- [ ] `trackplayer.rs` created
- [ ] `mod.rs` updated
- [ ] SoundChunk linked list compiles
- [ ] TrackCallbacks implements StreamCallbacks
- [ ] All public function signatures present
- [ ] `cargo check` passes

## Semantic Verification Checklist (Mandatory)
- [ ] SoundChunk has Box<dyn SoundDecoder> field
- [ ] TrackPlayerState has raw pointer fields (chunks_tail, last_sub, cur_chunk, cur_sub_chunk)
- [ ] `unsafe impl Send` present for TrackPlayerState
- [ ] NonNull<SoundChunk> used for cur_chunk/cur_sub_chunk
- [ ] AtomicU32 used for tracks_length

## Deferred Implementation Detection (Mandatory)

```bash
grep -n "todo!()" rust/src/sound/trackplayer.rs | wc -l  # Should be > 0 (stubs)
```

## Success Criteria
- [ ] All signatures compile
- [ ] Module registered
- [ ] Linked list type compiles

## Failure Recovery
- rollback: `git checkout -- rust/src/sound/mod.rs` and `rm rust/src/sound/trackplayer.rs`

## Phase Completion Marker
Create: `project-plans/audiorust/heart/.completed/P09.md`
