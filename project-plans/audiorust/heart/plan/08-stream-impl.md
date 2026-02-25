# Phase 08: Stream Implementation

## Phase ID
`PLAN-20260225-AUDIO-HEART.P08`

## Prerequisites
- Required: Phase P07a (Stream TDD Verification) passed
- Expected: 29+ tests in stream.rs test module

## Requirements Implemented (Expanded)

All STREAM-* requirements (75 total), grouped by category:

### Init/Lifecycle (STREAM-INIT-01..07)
- GIVEN: Audio subsystem starting up
- WHEN: `init_stream_decoder` is called
- THEN: Sources/buffers are generated via mixer API, decoder thread is spawned with a name, StreamEngine global is initialized

### Playback Control (STREAM-PLAY-01..20)
- GIVEN: A SoundSample with a decoder
- WHEN: `play_stream`/`stop_stream`/`pause_stream`/`resume_stream`/`seek_stream` are called
- THEN: Mixer source state changes correctly, buffers are pre-filled on play, scope buffer is allocated if requested, stream_should_be_playing flag is set/cleared

### Decoder Thread (STREAM-THREAD-01..08)
- GIVEN: The decoder thread is running
- WHEN: Sources have `stream_should_be_playing == true`
- THEN: The thread decodes audio, queues buffers to mixer, processes fades, and sleeps via condvar when idle

### Buffer Processing (STREAM-PROCESS-01..16)
- GIVEN: A playing source with queued buffers
- WHEN: Mixer reports processed buffers
- THEN: Buffers are unqueued, callbacks fire via deferred execution (after releasing locks), EOF/underrun are detected correctly

### Sample Management (STREAM-SAMPLE-01..05), Tags (STREAM-TAG-01..03), Scope (STREAM-SCOPE-01..11), Fade (STREAM-FADE-01..05)
- Per-requirement behavior contracts defined in specification.md §5.1

### Pseudocode traceability
- `init_stream_decoder`: pseudocode `stream.md` lines 1-11
- `uninit_stream_decoder`: pseudocode `stream.md` lines 20-32
- `create_sound_sample`: pseudocode `stream.md` lines 40-56
- `destroy_sound_sample`: pseudocode `stream.md` lines 60-66
- `play_stream`: pseudocode `stream.md` lines 70-148
- `stop_stream`: pseudocode `stream.md` lines 160-172
- `pause_stream`: pseudocode `stream.md` lines 180-187
- `resume_stream`: pseudocode `stream.md` lines 190-198
- `seek_stream`: pseudocode `stream.md` lines 200-212
- `stream_decoder_task`: pseudocode `stream.md` lines 220-244
- `process_source_stream`: pseudocode `stream.md` lines 260-341
- `process_music_fade`: pseudocode `stream.md` lines 360-370
- `set_music_stream_fade`: pseudocode `stream.md` lines 380-389
- `graph_foreground_stream`: pseudocode `stream.md` lines 400-460
- `find_tagged_buffer/tag_buffer/clear_buffer_tag`: pseudocode `stream.md` lines 490-512
- `add_scope_data/remove_scope_data`: pseudocode `stream.md` lines 520-533

## Implementation Tasks

### Files to modify
- `rust/src/sound/stream.rs` — Replace all `todo!()` with full implementations
  - marker: `@plan PLAN-20260225-AUDIO-HEART.P08`
  - marker: `@requirement REQ-STREAM-*`

### Implementation priority order
1. Sample management (`create_sound_sample`, `destroy_sound_sample`, accessors) — simplest, foundational
2. Buffer tagging (`find_tagged_buffer`, `tag_buffer`, `clear_buffer_tag`) — pure data manipulation
3. Fade logic (`set_music_stream_fade`, `process_music_fade`) — self-contained
4. Scope buffer helpers (`add_scope_data`, `remove_scope_data`, `read_sound_sample`) — ring buffer logic
5. Stream control (`play_stream`, `stop_stream`, `pause_stream`, `resume_stream`, `seek_stream`) — core engine
6. Decoder thread (`init_stream_decoder`, `uninit_stream_decoder`, `stream_decoder_task`) — threading
7. Source processing (`process_source_stream`) — most complex, depends on all above
8. Oscilloscope (`graph_foreground_stream`) — AGC, rendering

### Key implementation notes
- Use `parking_lot::Mutex` and `parking_lot::Condvar` (never bare `Mutex` or `std::sync::Mutex`)
- **CRITICAL: Lock ordering rule** — The full lock ordering hierarchy is: `TRACK_STATE → MUSIC_STATE → Source mutex → Sample mutex → FadeState mutex`. A thread holding a lock must NEVER acquire a lock higher in this hierarchy. The decoder thread's `process_source_stream` uses a **deferred callback pattern**: it collects callback actions while holding Source+Sample locks, then drops those locks and executes callbacks afterward — this ensures callbacks (which may acquire TRACK_STATE) never violate the ordering. Every function that acquires multiple locks must document which locks are held and in what order.
- **Initialization ordering** — `init_stream_decoder()` must be called after `mixer_init()`. Document this in the function's doc comment.
- All mixer calls must handle `Err` (log + continue, per REQ-CROSS-ERROR-01)
- No `unwrap()` in production code
- Decoder thread uses `std::thread::Builder` for named thread

### parking_lot::Condvar vs std::sync::Condvar

`parking_lot::Condvar` does **NOT** have spurious wakeups (unlike `std::sync::Condvar`). From the parking_lot documentation: "Unlike the standard library Condvar type, this does not check for spurious wakeups." This means:

- The `wait_for` timeout in `stream_decoder_task` (pseudocode line 240) does **not** need a loop condition to guard against spurious wakeups. A single `condvar.wait_for(&mut guard, Duration::from_millis(100))` call will either:
  1. Return `WaitTimeoutResult` after the timeout expires, OR
  2. Return after a genuine `notify_one`/`notify_all` signal

- This simplifies the idle sleep logic compared to a `std::sync::Condvar` approach, which would require wrapping the wait in a `while !predicate` loop. With `parking_lot`, the code can be a simple:
  ```rust
  let _timeout = condvar.wait_for(&mut guard, Duration::from_millis(100));
  // No spurious wakeup check needed — either timed out or genuinely notified
  ```

- If the implementation ever switches to `std::sync::Condvar` (not recommended), a `while !shutdown && !any_active` guard loop would be required around the wait call.

## Verification Commands

```bash
cd /Users/acoliver/projects/uqm/rust && cargo test --lib --all-features -- sound::stream::tests
cd /Users/acoliver/projects/uqm/rust && cargo test --lib --all-features
cd /Users/acoliver/projects/uqm/rust && cargo fmt --all --check
cd /Users/acoliver/projects/uqm/rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
```

## Structural Verification Checklist
- [ ] All `todo!()` removed from stream.rs
- [ ] All tests pass
- [ ] fmt and clippy pass
- [ ] No new warnings introduced

## Semantic Verification Checklist (Mandatory)
- [ ] `create_sound_sample` allocates mixer buffers
- [ ] `play_stream` pre-fills buffers and starts mixer playback
- [ ] `stop_stream` stops mixer and clears all source state
- [ ] Decoder thread spawns and shuts down cleanly
- [ ] Fade interpolation is numerically correct
- [ ] Scope ring buffer wraps correctly
- [ ] `graph_foreground_stream` produces plausible oscilloscope data
- [ ] All error paths return correct AudioError variants

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented\|todo!()" rust/src/sound/stream.rs
# Must return 0 results
```

## Success Criteria
- [ ] All 29+ tests pass
- [ ] Zero deferred implementations
- [ ] Full streaming engine operational (unit-level)

## Failure Recovery
- rollback: `git stash` or `git checkout -- rust/src/sound/stream.rs`
- blocking issues: If mixer API signatures differ from spec, adapt and document

## Phase Completion Marker
Create: `project-plans/audiorust/heart/.completed/P08.md`
