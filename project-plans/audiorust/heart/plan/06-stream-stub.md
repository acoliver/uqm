# Phase 06: Stream Stub

## Phase ID
`PLAN-20260225-AUDIO-HEART.P06`

## Prerequisites
- Required: Phase P05a (Types Implementation Verification) passed
- Expected files: `rust/src/sound/types.rs` fully implemented and tested

## Requirements Implemented (Expanded)

### REQ-STREAM-INIT-01, REQ-STREAM-INIT-02, REQ-STREAM-INIT-03, REQ-STREAM-INIT-04, REQ-STREAM-INIT-05, REQ-STREAM-INIT-06, REQ-STREAM-INIT-07: Init/Uninit Stubs
**Requirement text**: Streaming system initialization spawns decoder thread; uninit joins it.

Behavior contract:
- GIVEN: types.rs provides all shared types
- WHEN: stream.rs is created with function signatures
- THEN: All public functions compile with `todo!()` bodies

**CRITICAL: Initialization Ordering Constraint**
`init_stream_decoder()` MUST be called after `mixer_init()`. The `StreamEngine` lazy_static depends on the mixer being initialized first (it calls `mixer_gen_sources`/`mixer_gen_buffers`). Document this ordering constraint in the `init_stream_decoder` function's doc comment.

### REQ-STREAM-PLAY-01 through REQ-STREAM-PLAY-20: Play/Stop/Pause/Resume/Seek Stubs
**Requirement text**: Stream playback control functions.

Behavior contract:
- GIVEN: SoundSample, SoundSource, StreamCallbacks exist
- WHEN: Function signatures are defined
- THEN: They accept correct parameter types and return correct Result types

### REQ-STREAM-SAMPLE-01 through REQ-STREAM-SAMPLE-05: Sample Management Stubs
Behavior contract:
- GIVEN: `SoundSample` struct defined in types.rs
- WHEN: `create_sound_sample`, `destroy_sound_sample`, `set_sound_sample_data`, `get_sound_sample_data`, `get_sound_sample_decoder` stubs are defined
- THEN: They compile with correct parameter types (decoder as `Box<dyn SoundDecoder>`, sample as `Arc<parking_lot::Mutex<SoundSample>>`)

### REQ-STREAM-TAG-01, REQ-STREAM-TAG-02, REQ-STREAM-TAG-03: Buffer Tagging Stubs
Behavior contract:
- GIVEN: `SoundTag` struct defined in types.rs
- WHEN: `find_tagged_buffer`, `tag_buffer`, `clear_buffer_tag` stubs are defined
- THEN: They accept buffer handle (u32) and data parameters and compile

### REQ-STREAM-SCOPE-01 through REQ-STREAM-SCOPE-11: Scope Buffer Stubs
Behavior contract:
- GIVEN: Scope ring buffer fields exist on `SoundSource`
- WHEN: `graph_foreground_stream`, `add_scope_data`, `remove_scope_data` stubs are defined
- THEN: They compile with correct return types (u32 for graph_foreground_stream)

### REQ-STREAM-FADE-01 through REQ-STREAM-FADE-05: Fade Stubs
Behavior contract:
- GIVEN: `FadeState` struct defined in types.rs
- WHEN: `set_music_stream_fade`, `process_music_fade` stubs are defined
- THEN: They accept volume/time parameters and compile

### REQ-STREAM-THREAD-01 through REQ-STREAM-THREAD-08: Decoder Thread Stubs
Behavior contract:
- GIVEN: `std::thread`, `parking_lot::Condvar`, `AtomicBool` available
- WHEN: `stream_decoder_task` stub is defined
- THEN: It compiles as a thread entry point (no parameters, no return value)

### REQ-STREAM-PROCESS-01 through REQ-STREAM-PROCESS-16: Source Processing Stubs
Behavior contract:
- GIVEN: Mixer API functions accessible from stream.rs
- WHEN: `process_source_stream` stub is defined
- THEN: It accepts a source index and compiles

## Implementation Tasks

### Files to create
- `rust/src/sound/stream.rs` — All public function signatures from spec §3.1.3, internal types from §3.1.2
  - marker: `@plan PLAN-20260225-AUDIO-HEART.P06`
  - marker: `@requirement REQ-STREAM-INIT-01, REQ-STREAM-INIT-02, REQ-STREAM-INIT-03, REQ-STREAM-INIT-04, REQ-STREAM-INIT-05, REQ-STREAM-INIT-06, REQ-STREAM-INIT-07, REQ-STREAM-PLAY-01 through REQ-STREAM-PLAY-20, REQ-STREAM-THREAD-01 through REQ-STREAM-THREAD-08, REQ-STREAM-PROCESS-01 through REQ-STREAM-PROCESS-16, REQ-STREAM-SAMPLE-01 through REQ-STREAM-SAMPLE-05, REQ-STREAM-TAG-01, REQ-STREAM-TAG-02, REQ-STREAM-TAG-03, REQ-STREAM-SCOPE-01 through REQ-STREAM-SCOPE-11, REQ-STREAM-FADE-01 through REQ-STREAM-FADE-05`

### Files to modify
- `rust/src/sound/mod.rs` — Add `pub mod stream;`
  - marker: `@plan PLAN-20260225-AUDIO-HEART.P06`

### Stub contents
1. `StreamEngine` struct with `sources`, `fade`, `decoder_thread`, `shutdown`, `wake` fields
2. `lazy_static! { static ref ENGINE: ... }` (or `OnceLock`)
3. All public functions from spec §3.1.3 with `todo!()` bodies:
   - `create_sound_sample`, `destroy_sound_sample`, `set_sound_sample_data`, `get_sound_sample_data`, `set_sound_sample_callbacks`, `get_sound_sample_decoder`
   - `play_stream`, `stop_stream`, `pause_stream`, `resume_stream`, `seek_stream`, `playing_stream`
   - `find_tagged_buffer`, `tag_buffer`, `clear_buffer_tag`
   - `graph_foreground_stream`
   - `set_music_stream_fade`
   - `init_stream_decoder`, `uninit_stream_decoder`
4. Internal functions with `todo!()`:
   - `stream_decoder_task`, `process_source_stream`, `process_music_fade`
   - `add_scope_data`, `remove_scope_data`, `read_sound_sample`

## Verification Commands

```bash
cd /Users/acoliver/projects/uqm/rust && cargo check --lib --all-features
cd /Users/acoliver/projects/uqm/rust && cargo fmt --all --check
cd /Users/acoliver/projects/uqm/rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
```

## Structural Verification Checklist
- [ ] `rust/src/sound/stream.rs` created
- [ ] `mod.rs` updated with `pub mod stream;`
- [ ] All public function signatures match spec §3.1.3
- [ ] `cargo check` passes
- [ ] fmt and clippy pass

## Semantic Verification Checklist (Mandatory)
- [ ] StreamEngine has all 5 fields from spec §3.1.2
- [ ] All parameter types match spec (`Arc<parking_lot::Mutex<SoundSample>>`, usize for source_index, etc.)
- [ ] Return types match spec (AudioResult<()>, bool, usize, etc.)
- [ ] StreamCallbacks trait referenced correctly from types.rs
- [ ] Import paths to mixer and decoder modules are correct

## Deferred Implementation Detection (Mandatory)

```bash
# Only todo!() allowed in stub phase
grep -n "todo!()" rust/src/sound/stream.rs | wc -l
# Should be > 0 (stubs exist) but controlled
```

## Success Criteria
- [ ] All signatures compile
- [ ] Module registered
- [ ] Other modules can import from stream.rs

## Failure Recovery
- rollback: `git checkout -- rust/src/sound/mod.rs` and `rm rust/src/sound/stream.rs`
- blocking issues: If type signatures in types.rs need adjustment, fix in types.rs first

## Phase Completion Marker
Create: `project-plans/audiorust/heart/.completed/P06.md`
