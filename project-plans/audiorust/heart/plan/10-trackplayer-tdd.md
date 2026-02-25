# Phase 10: Track Player TDD

## Phase ID
`PLAN-20260225-AUDIO-HEART.P10`

## Prerequisites
- Required: Phase P09a (Track Player Stub Verification) passed
- Expected: `trackplayer.rs` compiling with stubs

## Requirements Implemented (Expanded)

### REQ-TRACK-ASSEMBLE-01..19: Assembly Tests
### REQ-TRACK-PLAY-01..10: Playback Tests
### REQ-TRACK-SEEK-01..13: Seeking Tests
### REQ-TRACK-CALLBACK-01..09: Callback Tests
### REQ-TRACK-SUBTITLE-01..04: Subtitle Tests
### REQ-TRACK-POSITION-01..02: Position Tests

## Implementation Tasks

### Files to modify
- `rust/src/sound/trackplayer.rs` — Add test module
  - marker: `@plan PLAN-20260225-AUDIO-HEART.P10`

### Tests to write

**Subtitle Splitting (REQ-TRACK-ASSEMBLE-01..03)**
1. `test_split_sub_pages_single` — Single page, no split
2. `test_split_sub_pages_multiple` — Split at \r\n
3. `test_split_sub_pages_continuation_marks` — `..` prefix and `...` suffix
4. `test_split_sub_pages_timing` — TEXT_SPEED * chars, min 1000ms

**Timestamp Parsing (REQ-TRACK-ASSEMBLE-14)**
5. `test_get_time_stamps_basic` — Parse comma-separated values
6. `test_get_time_stamps_skip_zeros` — Zero values skipped
7. `test_get_time_stamps_mixed_separators` — Comma, CR, LF separators

**Track Assembly (REQ-TRACK-ASSEMBLE-04..13)**
8. `test_splice_track_no_text_returns_ok` — Early return on None text
9. `test_splice_track_no_name_no_tracks_warns` — Warn when track_count=0
10. `test_splice_track_creates_first_sample` — Sound sample created on first call
11. `test_splice_track_appends_chunk` — Chunk linked into list
12. `test_splice_track_last_page_negative` — Last page timestamp negated

**Multi-Track (REQ-TRACK-ASSEMBLE-15..17)**
13. `test_splice_multi_track_precondition` — Error when no tracks exist
14. `test_splice_multi_track_appends` — Chunks appended correctly

**Playback Control (REQ-TRACK-PLAY-01..10)**
15. `test_play_track_no_sample_ok` — Returns Ok when no sample
16. `test_stop_track_clears_all` — track_count=0, chunks dropped, pointers null
17. `test_playing_track_zero_when_empty` — Returns 0 when nothing playing

**Seeking (REQ-TRACK-SEEK-01..06)**
18. `test_seek_clamps_offset` — Offset clamped to valid range
19. `test_get_current_track_pos_clamped` — Position clamped to 0..tracks_length

**Position (REQ-TRACK-POSITION-01..02)**
20. `test_get_track_position_no_sample` — Returns 0
21. `test_get_track_position_scaled` — Correctly scales by in_units

**Subtitles (REQ-TRACK-SUBTITLE-01..04)**
22. `test_get_track_subtitle_none_when_empty` — Returns None
23. `test_get_first_track_subtitle_none` — Returns None when no chunks

**Navigation (REQ-TRACK-SEEK-11..12)**
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
