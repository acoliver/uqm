# Phase 07: Stream TDD

## Phase ID
`PLAN-20260225-AUDIO-HEART.P07`

## Prerequisites
- Required: Phase P06a (Stream Stub Verification) passed
- Expected files: `rust/src/sound/stream.rs` with all stubs compiling

## Requirements Implemented (Expanded)

### REQ-STREAM-SAMPLE-01 through REQ-STREAM-SAMPLE-05: Sample CRUD
Behavior contract:
- GIVEN: A valid decoder and buffer count
- WHEN: create_sound_sample is called
- THEN: Sample has correct number of buffers, tags are all None, callbacks stored

### REQ-STREAM-TAG-01, REQ-STREAM-TAG-02, REQ-STREAM-TAG-03: Buffer Tagging
Behavior contract:
- GIVEN: A sample with buffer_tags
- WHEN: tag_buffer is called
- THEN: Tag is stored in first available slot; find_tagged_buffer finds it

### REQ-STREAM-PLAY-01 through REQ-STREAM-PLAY-20: Playback State Transitions
Behavior contract:
- GIVEN: A source in inactive state
- WHEN: play_stream is called
- THEN: Source becomes playing, mixer source starts, buffers pre-filled

### REQ-STREAM-FADE-01 through REQ-STREAM-FADE-05: Fade Logic
Behavior contract:
- GIVEN: Music is playing at volume V
- WHEN: set_music_stream_fade is called
- THEN: Fade interpolates linearly from V to target over interval

### REQ-STREAM-SCOPE-01 through REQ-STREAM-SCOPE-11: Scope Buffer
Behavior contract:
- GIVEN: A stream with scope enabled
- WHEN: Audio is decoded and queued
- THEN: Scope ring buffer contains decoded bytes for oscilloscope rendering

## Implementation Tasks

### Files to modify
- `rust/src/sound/stream.rs` — Add test module
  - marker: `@plan PLAN-20260225-AUDIO-HEART.P07`

### Tests to write

**Sample Management (REQ-STREAM-SAMPLE-*)**
1. `test_create_sound_sample_basic` — Creates sample, verifies buffer count
2. `test_create_sound_sample_with_callbacks` — Verifies callbacks stored
3. `test_create_sound_sample_no_decoder` — None decoder accepted
4. `test_destroy_sound_sample_clears_buffers` — Buffers freed
5. `test_set_get_sound_sample_data` — User data round-trip
6. `test_set_sound_sample_callbacks_replace` — Replace existing callbacks

**Buffer Tagging (REQ-STREAM-TAG-*)**
7. `test_find_tagged_buffer_empty` — Returns None on empty tags
8. `test_tag_and_find_buffer` — Tag then find succeeds
9. `test_tag_buffer_full` — Returns false when all slots used
10. `test_clear_buffer_tag` — Tag cleared successfully

**Fade Logic (REQ-STREAM-FADE-*)**
11. `test_set_fade_zero_duration_rejected` — Returns false
12. `test_set_fade_stores_params` — FadeState updated correctly
13. `test_process_fade_interpolation` — Volume interpolates correctly
14. `test_process_fade_completion` — Interval set to 0 when done
15. `test_process_fade_inactive_noop` — No-op when interval=0

**Scope Buffer (REQ-STREAM-SCOPE-*)**
16. `test_add_scope_data_writes` — Data written to ring buffer
17. `test_add_scope_data_wraps` — Wraps around ring buffer
18. `test_remove_scope_data_advances_head` — Head advances correctly
19. `test_read_sound_sample_8bit` — 8-bit conversion correct
20. `test_read_sound_sample_16bit` — 16-bit read correct
21. `test_graph_foreground_stream_no_source` — Returns 0

**Playback State (REQ-STREAM-PLAY-*)**
22. `test_playing_stream_initial_false` — Not playing initially
23. `test_stop_stream_clears_state` — All source fields cleared
24. `test_pause_records_time` — pause_time set on first pause
25. `test_resume_adjusts_start_time` — start_time adjusted for pause duration
26. `test_seek_no_sample_error` — Returns InvalidSample

**Thread (REQ-STREAM-THREAD-*)**
27. `test_init_decoder_spawns_thread` — Thread handle populated
28. `test_uninit_decoder_joins_thread` — Thread joins cleanly
29. `test_uninit_no_thread_ok` — No error when thread not spawned

Note: Full integration tests for play_stream with actual decoder interaction will require mock mixer, handled in later test refinement.

## Verification Commands

```bash
cd /Users/acoliver/projects/uqm/rust && cargo test --lib --all-features -- sound::stream::tests
cd /Users/acoliver/projects/uqm/rust && cargo fmt --all --check
cd /Users/acoliver/projects/uqm/rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
```

## Structural Verification Checklist
- [ ] Test module added to stream.rs
- [ ] 29+ test functions
- [ ] Tests reference requirements
- [ ] All tests compile

## Semantic Verification Checklist (Mandatory)
- [ ] Tests verify behavior (inputs → outputs), not internals
- [ ] Error path tests verify correct AudioError variant
- [ ] State transition tests verify field changes
- [ ] Fade tests verify numerical correctness
- [ ] Scope tests verify byte-level ring buffer behavior

## Deferred Implementation Detection (Mandatory)

```bash
grep -n "todo!()" rust/src/sound/stream.rs | grep -v "#\[cfg(test)\]" | grep -v "mod tests"
# Stubs still have todo!() — that's expected at TDD phase
```

## Success Criteria
- [ ] 29+ tests written and compiling
- [ ] Tests are meaningful (will fail with wrong implementation)

## Failure Recovery
- rollback: `git checkout -- rust/src/sound/stream.rs`
- blocking issues: If mock mixer needed, create minimal mock in test module

## Phase Completion Marker
Create: `project-plans/audiorust/heart/.completed/P07.md`
