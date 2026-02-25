# Phase 11: Track Player Implementation

## Phase ID
`PLAN-20260225-AUDIO-HEART.P11`

## Prerequisites
- Required: Phase P10a (Track Player TDD Verification) passed
- Expected: 25+ tests in trackplayer.rs

## Requirements Implemented (Expanded)

All TRACK-* requirements (57 total), grouped by category:

### Assembly (TRACK-ASSEMBLE-01..19)
- GIVEN: A track name, subtitle text, and optional timestamps
- WHEN: `splice_track`/`splice_multi_track` are called
- THEN: SoundChunks are created with correct decoder, start_time, text, tag_me fields, subtitle pages are split at CRLF with continuation marks (`..`/`...`), timestamps are parsed, and chunks are appended to the linked list in order

### Playback Control (TRACK-PLAY-01..10)
- GIVEN: A spliced track with one or more chunks
- WHEN: `play_track`/`stop_track`/`pause_track`/`resume_track`/`jump_track` are called
- THEN: The stream engine is started/stopped/paused/resumed correctly, track state (cur_chunk, track_count, dec_offset) is updated, and `stop_track` drops all chunks via iterative Drop

### Seeking (TRACK-SEEK-01..13)
- GIVEN: A playing track with multiple chunks
- WHEN: `seek_track`/`fast_reverse_smooth`/`fast_forward_smooth`/`fast_reverse_page`/`fast_forward_page` are called
- THEN: The position is adjusted correctly, clamped to `0..=tracks_length+1`, the current chunk/sub-chunk pointers advance to match, and page navigation finds the correct tagged chunk boundary

### Callbacks (TRACK-CALLBACK-01..09)
- GIVEN: A playing track with TrackCallbacks installed
- WHEN: The streaming engine fires `on_start_stream`/`on_end_chunk`/`on_end_stream`/`on_tagged_buffer`
- THEN: Callbacks correctly set the decoder, advance chunks, tag buffers with subtitle data, and handle end-of-stream cleanup (resetting track state)

### Subtitles (TRACK-SUBTITLE-01..04)
- GIVEN: A track with subtitle text in chunks
- WHEN: `get_track_subtitle`/`get_first_track_subtitle`/`get_next_track_subtitle` are called
- THEN: The correct subtitle text is returned, and iteration follows the linked list order via SubtitleRef

### Position (TRACK-POSITION-01..02)
- GIVEN: A playing track with known run_time per chunk
- WHEN: `get_track_position` is called with different unit values
- THEN: The position is returned in the requested units (game ticks or milliseconds), computed from dec_offset and sample timing

### Pseudocode traceability
- `splice_track`: pseudocode `trackplayer.md` lines 1-89
- `splice_multi_track`: pseudocode `trackplayer.md` lines 100-274 (expanded with detailed chunk boundary calculation)
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
- `unsafe impl Send` justified by single-thread mutation pattern (all access under `TRACK_STATE` `parking_lot::Mutex`)
- `cur_chunk`/`cur_sub_chunk` as `Option<NonNull<SoundChunk>>` for null-safety with raw access
- **Iterative Drop for SoundChunk** (Technical Review Issue #7): Implement `Drop` for `SoundChunk` using an iterative loop instead of recursive drop to avoid stack overflow on very long chains. The default recursive `Drop` would consume one stack frame per list node. While UQM track lengths are typically short (<50 chunks), implement iteratively for safety:
  ```rust
  impl Drop for SoundChunk {
      fn drop(&mut self) {
          let mut next = self.next.take();
          while let Some(mut chunk) = next {
              next = chunk.next.take();
              // chunk drops here without recursion
          }
      }
  }
  ```
- All `parking_lot::Mutex` references (never bare `Mutex` or `std::sync::Mutex`)

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
