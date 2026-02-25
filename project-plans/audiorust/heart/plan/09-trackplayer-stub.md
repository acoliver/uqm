# Phase 09: Track Player Stub

## Phase ID
`PLAN-20260225-AUDIO-HEART.P09`

## Prerequisites
- Required: Phase P08a (Stream Implementation Verification) passed
- Expected files: `stream.rs` fully implemented

## Requirements Implemented (Expanded)

### REQ-TRACK-ASSEMBLE-01 through REQ-TRACK-ASSEMBLE-19: Track Assembly Stubs
**Requirement text**: Linked-list chunk assembly with subtitle page splitting.

Behavior contract:
- GIVEN: stream.rs provides SoundSample, play_stream, etc.
- WHEN: trackplayer.rs is created with all function signatures
- THEN: All track player public functions compile with `todo!()` bodies

### REQ-TRACK-PLAY-01 through REQ-TRACK-PLAY-10: Playback Control Stubs
Behavior contract:
- GIVEN: stream.rs provides `play_stream`, `stop_stream`, `pause_stream`, `resume_stream`
- WHEN: `play_track`, `stop_track`, `jump_track`, `pause_track`, `resume_track`, `playing_track` stubs are defined
- THEN: They accept correct parameter types and return correct Result types

### REQ-TRACK-SEEK-01 through REQ-TRACK-SEEK-13: Seeking Stubs
Behavior contract:
- GIVEN: stream.rs provides `seek_stream`
- WHEN: `fast_reverse_smooth`, `fast_forward_smooth`, `fast_reverse_page`, `fast_forward_page`, `get_track_position` stubs are defined
- THEN: They compile and accept correct parameter types (offset as i32, in_units as u32)

### REQ-TRACK-CALLBACK-01 through REQ-TRACK-CALLBACK-09: Callback Stubs
Behavior contract:
- GIVEN: `StreamCallbacks` trait exists in types.rs
- WHEN: `TrackCallbacks` struct is defined implementing `StreamCallbacks`
- THEN: It compiles and can be passed as `Box<dyn StreamCallbacks>`

### REQ-TRACK-SUBTITLE-01 through REQ-TRACK-SUBTITLE-04: Subtitle Query Stubs
Behavior contract:
- GIVEN: `SoundChunk` and `SubtitleRef` types exist
- WHEN: `get_track_subtitle`, `get_first_track_subtitle`, `get_next_track_subtitle`, `get_track_subtitle_text` stubs are defined
- THEN: They return `Option<&str>` or `Option<SubtitleRef>` as appropriate

### REQ-TRACK-POSITION-01, REQ-TRACK-POSITION-02: Position Tracking Stubs
Behavior contract:
- GIVEN: `AtomicU32` used for `tracks_length`
- WHEN: `get_track_position` stub is defined
- THEN: It accepts `in_units: u32` and returns `u32`

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
5. `unsafe impl Send for TrackPlayerState {}` — with detailed safety documentation (see below)
6. `TrackCallbacks` struct implementing `StreamCallbacks`
7. `lazy_static! { static ref TRACK_STATE: parking_lot::Mutex<TrackPlayerState> }` (or `OnceLock`)
8. Constants: `TEXT_SPEED`, `ACCEL_SCROLL_SPEED`, `MAX_MULTI_TRACKS`

### Lifetime Safety Documentation for TrackPlayerState (Technical Review Issue #8)

`TrackPlayerState` contains raw pointers (`chunks_tail: *mut SoundChunk`, `cur_chunk: Option<NonNull<SoundChunk>>`, `cur_sub_chunk: Option<NonNull<SoundChunk>>`). The `unsafe impl Send` requires the following safety invariants to be documented as `// SAFETY:` comments in the code:

1. **Ownership invariant**: `chunks_head: Option<Box<SoundChunk>>` owns the linked list. All raw pointers (`chunks_tail`, `cur_chunk`, `cur_sub_chunk`) point into this list and are NEVER dereferenced after `chunks_head` is set to `None`.
2. **Single-writer invariant**: `TrackPlayerState` is always accessed under the `TRACK_STATE` `parking_lot::Mutex`, ensuring only one thread reads or writes at a time. The raw pointers are never shared between threads without the mutex held.
3. **Lifetime invariant**: `cur_chunk` and `cur_sub_chunk` are invalidated (set to `None`) in `stop_track()` before `chunks_head` is dropped. `chunks_tail` is set to `null_mut()` whenever the list is emptied.
4. **Callback invariant**: `TrackCallbacks` holds a raw pointer to `TRACK_STATE`'s `SoundSample`. Callbacks are only invoked while the sample is alive (guaranteed by the stream engine holding an `Arc` reference).
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
