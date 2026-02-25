# Phase 10: Track Player TDD

## Phase ID
`PLAN-20260225-AUDIO-HEART.P10`

## Prerequisites
- Required: Phase P09a (Track Player Stub Verification) passed
- Expected: `trackplayer.rs` compiling with stubs

## Requirements Implemented (Expanded)

### REQ-TRACK-ASSEMBLE-01 through REQ-TRACK-ASSEMBLE-19: Assembly Tests
- GIVEN: A track name and subtitle text
- WHEN: `splice_track` is called
- THEN: A SoundChunk is created with correct decoder, start_time, text, tag_me fields, and appended to the linked list

### REQ-TRACK-PLAY-01 through REQ-TRACK-PLAY-10: Playback Tests
- GIVEN: A spliced track with chunks
- WHEN: `play_track`/`stop_track`/`pause_track`/`resume_track`/`jump_track` are called
- THEN: The stream engine is started/stopped/paused/resumed correctly, and track state (cur_chunk, track_count, dec_offset) is updated

### REQ-TRACK-SEEK-01 through REQ-TRACK-SEEK-13: Seeking Tests
- GIVEN: A playing track with multiple chunks
- WHEN: `seek_track`/`fast_reverse_smooth`/`fast_forward_smooth`/`fast_reverse_page`/`fast_forward_page` are called
- THEN: The position is adjusted correctly, clamped to `0..=tracks_length+1`, and the current chunk/sub-chunk pointers advance to match

### REQ-TRACK-CALLBACK-01 through REQ-TRACK-CALLBACK-09: Callback Tests
- GIVEN: A playing track with TrackCallbacks installed
- WHEN: The streaming engine fires `on_start_stream`/`on_end_chunk`/`on_end_stream`/`on_tagged_buffer`
- THEN: The callbacks correctly set the decoder, advance chunks, tag buffers, and handle end-of-stream cleanup

### REQ-TRACK-SUBTITLE-01 through REQ-TRACK-SUBTITLE-04: Subtitle Tests
- GIVEN: A track with subtitle text in chunks
- WHEN: `get_track_subtitle`/`get_first_track_subtitle`/`get_next_track_subtitle` are called
- THEN: The correct subtitle text is returned, and iteration follows the linked list order

### REQ-TRACK-POSITION-01, REQ-TRACK-POSITION-02: Position Tests
- GIVEN: A playing track
- WHEN: `get_track_position` is called with different unit values
- THEN: The position is returned in the requested units (game ticks or milliseconds)

## Implementation Tasks

### Files to modify
- `rust/src/sound/trackplayer.rs` — Add test module
  - marker: `@plan PLAN-20260225-AUDIO-HEART.P10`

### Tests to write

**Subtitle Splitting (REQ-TRACK-ASSEMBLE-01, REQ-TRACK-ASSEMBLE-02, REQ-TRACK-ASSEMBLE-03)**
1. `test_split_sub_pages_single` — Single page, no split
2. `test_split_sub_pages_multiple` — Split at \r\n
3. `test_split_sub_pages_continuation_marks` — `..` prefix and `...` suffix
4. `test_split_sub_pages_timing` — TEXT_SPEED * chars, min 1000ms

**Timestamp Parsing (REQ-TRACK-ASSEMBLE-14)**
5. `test_get_time_stamps_basic` — Parse comma-separated values
6. `test_get_time_stamps_skip_zeros` — Zero values skipped
7. `test_get_time_stamps_mixed_separators` — Comma, CR, LF separators

**Track Assembly (REQ-TRACK-ASSEMBLE-04 through REQ-TRACK-ASSEMBLE-13)**
8. `test_splice_track_no_text_returns_ok` — Early return on None text
9. `test_splice_track_no_name_no_tracks_warns` — Warn when track_count=0
10. `test_splice_track_creates_first_sample` — Sound sample created on first call
11. `test_splice_track_appends_chunk` — Chunk linked into list
12. `test_splice_track_last_page_negative` — Last page timestamp negated

**Multi-Track (REQ-TRACK-ASSEMBLE-15, REQ-TRACK-ASSEMBLE-16, REQ-TRACK-ASSEMBLE-17)**
13. `test_splice_multi_track_precondition` — Error when no tracks exist
14. `test_splice_multi_track_appends` — Chunks appended correctly

**Playback Control (REQ-TRACK-PLAY-01 through REQ-TRACK-PLAY-10)**
15. `test_play_track_no_sample_ok` — Returns Ok when no sample
16. `test_stop_track_clears_all` — track_count=0, chunks dropped, pointers null
17. `test_playing_track_zero_when_empty` — Returns 0 when nothing playing

**Seeking (REQ-TRACK-SEEK-01 through REQ-TRACK-SEEK-06)**
18. `test_seek_clamps_offset` — Offset clamped to valid range
19. `test_get_current_track_pos_clamped` — Position clamped to 0..tracks_length

**Position (REQ-TRACK-POSITION-01, REQ-TRACK-POSITION-02)**
20. `test_get_track_position_no_sample` — Returns 0
21. `test_get_track_position_scaled` — Correctly scales by in_units

**Subtitles (REQ-TRACK-SUBTITLE-01 through REQ-TRACK-SUBTITLE-04)**
22. `test_get_track_subtitle_none_when_empty` — Returns None
23. `test_get_first_track_subtitle_none` — Returns None when no chunks

**Navigation (REQ-TRACK-SEEK-11, REQ-TRACK-SEEK-12)**
24. `test_find_next_page_none` — Returns None at end
25. `test_find_prev_page_defaults_to_head` — Returns head when no previous tagged

## Verification Commands

```bash
cd /Users/acoliver/projects/uqm/rust && cargo test --lib --all-features -- sound::trackplayer::tests
cd /Users/acoliver/projects/uqm/rust && cargo fmt --all --check
cd /Users/acoliver/projects/uqm/rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
```

## Structural Verification Checklist
- [ ] Test module added
- [ ] 25+ test functions
- [ ] Tests reference requirements
- [ ] All tests compile

## Semantic Verification Checklist (Mandatory)
- [ ] split_sub_pages tests verify text content and timing
- [ ] Timestamp parsing tests verify edge cases
- [ ] Assembly tests verify linked list structure
- [ ] Playback tests verify state transitions
- [ ] Seeking tests verify clamping and position math

## Deferred Implementation Detection (Mandatory)
N/A — TDD phase, stubs still have `todo!()`

## Success Criteria
- [ ] 25+ tests written and compiling
- [ ] Tests are meaningful behavioral assertions

## Failure Recovery
- rollback: `git checkout -- rust/src/sound/trackplayer.rs`

## Phase Completion Marker
Create: `project-plans/audiorust/heart/.completed/P10.md`
