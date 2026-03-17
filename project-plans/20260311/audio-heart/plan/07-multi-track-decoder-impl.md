# Phase 07: Multi-Track Decoder — TDD + Implementation

## Phase ID
`PLAN-20260314-AUDIO-HEART.P07`

## Prerequisites
- Required: Phase P06a completed (canonical loaders available)
- Expected files: shared loader helpers with working decoder acquisition support

## Requirements Implemented (Expanded)

### Multi-Track Decoder Loading
**Requirement text**: When a multi-track splice is accepted, the audio-heart subsystem shall load or associate a real decoder for each appended audio track rather than creating placeholder chunks with no playable audio.

Behavior contract:
- GIVEN: A base track program exists (track_count > 0)
- WHEN: `splice_multi_track` is called with track names
- THEN: Each non-null track gets a chunk with a real decoder attached

### Timeline Advancement
**Requirement text**: When a multi-track splice contributes playable audio, the audio-heart subsystem shall advance the logical program timeline to include the duration of each appended track.

Behavior contract:
- GIVEN: A multi-track splice with tracks totaling 5.0 seconds
- WHEN: splice_multi_track completes
- THEN: `dec_offset` has advanced by 5000.0 ms

### No Base Track Guard
**Requirement text**: If a multi-track splice is requested without an existing base track context, then the audio-heart subsystem shall silently ignore the request without constructing an invalid partial program.

Why it matters:
- Without real decoders, multi-track speech segments produce silence
- The comm subsystem depends on multi-track audio for certain dialogue sequences

## Implementation Tasks

### TDD — Files to modify

#### `rust/src/sound/trackplayer.rs` — add tests
- `test_splice_multi_track_no_base_track_ignored` — track_count == 0 leaves state unchanged
- `test_splice_multi_track_with_decoders_creates_chunks` — chunks have decoders
- `test_splice_multi_track_advances_dec_offset` — dec_offset increases by decoder lengths
- `test_splice_multi_track_appends_subtitle_to_last_sub` — text appended correctly
- `test_splice_multi_track_sets_no_page_break` — no_page_break flag set after call
- marker: `@plan PLAN-20260314-AUDIO-HEART.P07`

### Implementation — Files to modify

#### `rust/src/sound/trackplayer.rs`
- Modify `splice_multi_track()` signature to accept pre-loaded decoders:
  `pub fn splice_multi_track(decoders: Vec<Option<Box<dyn SoundDecoder>>>, texts: &[Option<&str>]) -> AudioResult<()>`
  OR keep existing signature and add a new function:
  `pub fn splice_multi_track_with_decoders(decoders: Vec<Option<Box<dyn SoundDecoder>>>, texts: &[Option<&str>]) -> AudioResult<()>`
- In the chunk creation loop:
  - Take the decoder from the decoders vec (`decoder_opt.take()`)
  - Query `decoder.length()` to get duration in seconds
  - Convert to ms: `length_ms = decoder.length() * 1000.0`
  - Set `chunk.decoder = Some(decoder)`
  - Set `chunk.run_time = ms_to_ticks(length_ms as u32)`
  - Advance `state.dec_offset += length_ms as f64`
- Preserve silent-ignore behavior when there is no base track context
- Remove the comment "decoder loading deferred to FFI"
- marker: `@plan PLAN-20260314-AUDIO-HEART.P07`

#### `rust/src/sound/heart_ffi.rs`
- In `SpliceMultiTrack` FFI shim (around lines 658-672):
  - Before calling the internal function, load decoders for each track name:
    - Use the shared loader read/decoder helpers (same pattern as `SpliceTrack` FFI)
  - Pass loaded decoders to `trackplayer::splice_multi_track_with_decoders()`
- marker: `@plan PLAN-20260314-AUDIO-HEART.P07`

### Pseudocode traceability
- Implements PC-04 lines 01-26

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings 2>&1 | head -50
cargo test --workspace --all-features

# Specific test
cargo test --workspace --all-features splice_multi_track
```

## Structural Verification Checklist
- [ ] `splice_multi_track` (or variant) accepts decoders
- [ ] FFI shim loads decoders before calling internal function
- [ ] No `decoder: None` in multi-track chunk creation except when a specific track load fails
- [ ] `dec_offset` advances by decoder length

## Semantic Verification Checklist
- [ ] Tests verify chunks have decoders attached
- [ ] Tests verify dec_offset advancement
- [ ] Tests verify no-base-track request is silently ignored
- [ ] No placeholder/deferred implementation patterns remain
- [ ] The `#[ignore = "P11: play_track stub"]` test annotation is evaluated for removal

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented\|deferred" rust/src/sound/trackplayer.rs | grep -i multi
```

## Success Criteria
- [ ] Multi-track chunks have real decoders
- [ ] dec_offset reflects actual audio duration
- [ ] No-base-track behavior is silent-ignore, not error
- [ ] All tests pass

## Failure Recovery
- rollback: `git restore rust/src/sound/trackplayer.rs rust/src/sound/heart_ffi.rs`
- blocking issues: Decoder constructors may need adjustment for byte-buffer input

## Phase Completion Marker
Create: `project-plans/20260311/audio-heart/.completed/P07.md`
