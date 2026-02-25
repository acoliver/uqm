# Phase 10a: Track Player TDD Verification

## Phase ID
`PLAN-20260225-AUDIO-HEART.P10a`

## Prerequisites
- Required: Phase P10 completed
- Expected: trackplayer.rs test module with 25+ tests

## Verification Commands

```bash
cd /Users/acoliver/projects/uqm/rust && cargo test --lib --all-features -- sound::trackplayer::tests 2>&1
cd /Users/acoliver/projects/uqm/rust && cargo fmt --all --check
cd /Users/acoliver/projects/uqm/rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
cd /Users/acoliver/projects/uqm/sc2 && ./build.sh uqm
```

## Structural Verification Checklist
- [ ] Test module `#[cfg(test)] mod tests` exists in `trackplayer.rs`
- [ ] At least 25 test functions present
- [ ] Tests compile without errors
- [ ] `cargo fmt` passes
- [ ] `cargo clippy` passes

## Semantic Verification Checklist

### Deterministic checks
- [ ] Test count >= 25: `cargo test --lib --all-features -- sound::trackplayer::tests 2>&1 | grep "test result"` shows >= 25 tests
- [ ] Specific test names exist for subtitle splitting: `test_split_sub_pages_single`, `test_split_sub_pages_multiple`, `test_split_sub_pages_continuation_marks`
- [ ] Specific test names exist for assembly: `test_splice_track_creates_first_sample`, `test_splice_track_appends_chunk`
- [ ] Specific test names exist for playback: `test_play_track_no_sample_ok`, `test_stop_track_clears_all`
- [ ] Specific test names exist for seeking: `test_seek_clamps_offset`, `test_get_current_track_pos_clamped`

### Subjective checks
- [ ] split_sub_pages tests verify text content AND timing — do they check that page durations are TEXT_SPEED * char_count with minimum 1000ms?
- [ ] Continuation marks tested — do tests verify `..` prefix on continuation pages and `...` suffix on continued pages?
- [ ] Timestamp parsing tests verify edge cases — zero values skipped, mixed separators (comma, CR, LF)?
- [ ] Assembly tests verify linked list structure — do they traverse the list and verify correct chunk ordering?
- [ ] Playback tests verify state transitions — does `test_stop_track_clears_all` verify track_count=0, chunks dropped, pointers null?
- [ ] Seeking tests verify clamping and position math — do they test boundary values (0, tracks_length, beyond)?
- [ ] No trivially-passing tests

## Deferred Implementation Detection
N/A — TDD phase, stubs still have `todo!()`.

## Success Criteria
- [ ] 25+ tests written and compiling
- [ ] Tests are meaningful behavioral assertions
- [ ] All requirement areas covered: splitting, timestamps, assembly, playback, seeking, position, subtitles, navigation

## Failure Recovery
- rollback: `git checkout -- rust/src/sound/trackplayer.rs`
- blocking issues: If stream.rs mock needed, create in trackplayer test module

## Phase Completion Marker
Create: `project-plans/audiorust/heart/.completed/P10a.md`
