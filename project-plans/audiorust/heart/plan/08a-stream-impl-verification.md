# Phase 08a: Stream Implementation Verification

## Phase ID
`PLAN-20260225-AUDIO-HEART.P08a`

## Prerequisites
- Required: Phase P08 completed
- Expected: stream.rs fully implemented, all tests passing

## Verification Commands

```bash
cd /Users/acoliver/projects/uqm/rust && cargo test --lib --all-features -- sound::stream::tests
cd /Users/acoliver/projects/uqm/rust && cargo test --lib --all-features
cd /Users/acoliver/projects/uqm/rust && cargo fmt --all --check
cd /Users/acoliver/projects/uqm/rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
cd /Users/acoliver/projects/uqm/sc2 && ./build.sh uqm
# Deferred impl detection
grep -RIn "TODO\|FIXME\|HACK\|todo!()" rust/src/sound/stream.rs
```

## Structural Verification Checklist
- [ ] All `todo!()` removed from stream.rs (non-test code)
- [ ] All tests pass
- [ ] `cargo fmt` passes
- [ ] `cargo clippy` passes
- [ ] `build.sh uqm` succeeds
- [ ] No new warnings introduced

## Semantic Verification Checklist

### Deterministic checks
- [ ] All stream tests pass (29+): `cargo test --lib --all-features -- sound::stream::tests` shows 0 failures
- [ ] All workspace tests pass: `cargo test --lib --all-features` shows 0 failures (no regressions)
- [ ] Zero deferred markers: `grep -c "TODO\|FIXME\|HACK\|todo!()" rust/src/sound/stream.rs` returns 0 (excluding test module)
- [ ] Lock ordering documented in code: `grep -c "lock ordering\|Lock ordering\|LOCK ORDER" rust/src/sound/stream.rs` >= 1

### Subjective checks
- [ ] `create_sound_sample` allocates mixer buffers via `mixer_gen_buffers` — does it handle the case where buffer allocation fails?
- [ ] `play_stream` pre-fills buffers and starts mixer playback — does it correctly call `on_start_stream` callback and abort if it returns false (REQ-STREAM-PLAY-03)?
- [ ] `stop_stream` stops mixer and clears ALL source state — does it clear sample, scope, flags, timing fields?
- [ ] Decoder thread spawns and shuts down cleanly — does it use `std::thread::Builder` with a name? Does `uninit_stream_decoder` set the shutdown flag, notify the condvar, and join?
- [ ] Streaming thread wakes up when a new stream starts — is the condvar notified in `play_stream`?
- [ ] Buffer processing correctly detects EOF and triggers `on_end_chunk` callback — does `process_source_stream` handle all the EOF cases from REQ-STREAM-PROCESS-02 through REQ-STREAM-PROCESS-09?
- [ ] Fade actually changes volume over time — does `process_music_fade` compute linear interpolation and apply it via `mixer_source_f(Gain)`?
- [ ] Scope ring buffer wraps correctly at boundary — does `add_scope_data` handle the case where write position exceeds buffer length?
- [ ] `graph_foreground_stream` produces plausible oscilloscope data — does it implement AGC (REQ-STREAM-SCOPE-09) and VAD (REQ-STREAM-SCOPE-10)?
- [ ] All error paths return correct AudioError variants — no silent failures or swallowed errors
- [ ] No `unwrap()` or `expect()` in production code paths

## Deferred Implementation Detection

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented\|todo!()" rust/src/sound/stream.rs
# Must return 0 results
```

## Success Criteria
- [ ] All 29+ tests pass
- [ ] Zero deferred implementations
- [ ] Full streaming engine operational (unit-level)
- [ ] Lock ordering enforced and documented
- [ ] Init ordering constraint documented in code

## Failure Recovery
- rollback: `git stash` or `git checkout -- rust/src/sound/stream.rs`
- blocking issues: If mixer API signatures differ from spec, adapt and document the differences

## Phase Completion Marker
Create: `project-plans/audiorust/heart/.completed/P08a.md`
