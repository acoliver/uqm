# Phase 11a: Track Player Implementation Verification

## Phase ID
`PLAN-20260225-AUDIO-HEART.P11a`

## Prerequisites
- Required: Phase P11 completed
- Expected: trackplayer.rs fully implemented

## Verification Commands

```bash
cd /Users/acoliver/projects/uqm/rust && cargo test --lib --all-features -- sound::trackplayer::tests
cd /Users/acoliver/projects/uqm/rust && cargo test --lib --all-features
cd /Users/acoliver/projects/uqm/rust && cargo fmt --all --check
cd /Users/acoliver/projects/uqm/rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
cd /Users/acoliver/projects/uqm/sc2 && ./build.sh uqm
# Deferred impl detection
grep -RIn "TODO\|FIXME\|HACK\|todo!()" rust/src/sound/trackplayer.rs
```

## Structural Verification Checklist
- [ ] All `todo!()` removed from trackplayer.rs (non-test code)
- [ ] All tests pass
- [ ] `cargo fmt` passes
- [ ] `cargo clippy` passes
- [ ] `build.sh uqm` succeeds

## Semantic Verification Checklist

### Deterministic checks
- [ ] All trackplayer tests pass (25+): `cargo test --lib --all-features -- sound::trackplayer::tests` shows 0 failures
- [ ] All workspace tests pass: `cargo test --lib --all-features` shows 0 failures
- [ ] Zero deferred markers: `grep -c "TODO\|FIXME\|HACK\|todo!()" rust/src/sound/trackplayer.rs` returns 0 (excluding test module)
- [ ] Iterative Drop implemented: `grep -c "impl Drop for SoundChunk" rust/src/sound/trackplayer.rs` returns 1

### Subjective checks
- [ ] `splice_track` builds linked list correctly — does it append chunks to the tail? Does it create a new SoundSample on the first call (REQ-TRACK-ASSEMBLE-07)?
- [ ] `split_sub_pages` produces correct continuation marks — are `..` and `...` marks applied per REQ-TRACK-ASSEMBLE-03?
- [ ] `stop_track` frees all chunks without leaking — does it set `chunks_head = None` (which triggers iterative Drop), null out all raw pointers, and reset track_count to 0?
- [ ] `TrackCallbacks::on_end_chunk` advances cur_chunk correctly — does it handle the case where there's no next chunk (end of list)?
- [ ] Track player correctly splices audio chunks — does splice_track create chunks with the right decoder, start_time, text, and tag_me fields?
- [ ] Seek functions clamp correctly — does `seek_track` clamp offset to `0..=tracks_length+1` per REQ-TRACK-SEEK-01?
- [ ] Subtitle queries return correct text — does `get_track_subtitle` return the cur_sub_chunk's text?
- [ ] Iterative Drop on SoundChunk avoids stack overflow — does the Drop impl use a `while let` loop instead of recursive drop?
- [ ] No `unwrap()` or `expect()` in production code paths

### Concurrency verification
- [ ] Call `stop_track` from main thread while decoder thread callback (`on_end_chunk`) is executing — verify no deadlock (stop_track no longer holds source lock before calling stop_stream)
- [ ] Call `play_track`/`stop_track` rapidly from multiple threads — verify no panics or leaked chunks
- [ ] Verify `TrackCallbacks` acquire TRACK_STATE correctly in the deferred callback context (callbacks execute after source+sample locks are released)

## Deferred Implementation Detection

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented\|todo!()" rust/src/sound/trackplayer.rs
# Must return 0 results
```

## Success Criteria
- [ ] All 25+ tests pass
- [ ] Zero deferred implementations
- [ ] Track player fully operational (unit-level)
- [ ] Iterative Drop implemented for SoundChunk
- [ ] Lifetime safety invariants documented

## Failure Recovery
- rollback: `git stash` or `git checkout -- rust/src/sound/trackplayer.rs`
- blocking issues: If stream.rs behavior differs from expected, adapt TrackCallbacks

## Phase Completion Marker
Create: `project-plans/audiorust/heart/.completed/P11a.md`
