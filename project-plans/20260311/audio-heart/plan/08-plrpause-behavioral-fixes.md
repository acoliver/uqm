# Phase 08: Music/Speech Control Parity

## Phase ID
`PLAN-20260314-AUDIO-HEART.P08`

## Prerequisites
- Required: Phase P07a completed
- Expected files: trackplayer.rs with working multi-track decoders

## Requirements Implemented (Expanded)

### Music pause ref-matching
**Requirement text**: When a music pause request is issued through an ABI that supplies a music reference argument, the audio-heart subsystem shall pause music only if the supplied reference matches the current music resource (by raw-handle identity) or represents the externally defined wildcard sentinel (`~0`). If a non-matching non-wildcard reference is supplied, the subsystem shall leave current music playback unchanged.

Behavior contract:
- GIVEN: Music A is playing, caller passes music reference B
- WHEN: `PLRPause(B)` is called
- THEN: Music A continues playing (no pause)

- GIVEN: Music A is playing, caller passes music reference A
- WHEN: `PLRPause(A)` is called
- THEN: Music A is paused

- GIVEN: Music A is playing, caller passes wildcard (~0)
- WHEN: `PLRPause(~0)` is called
- THEN: Music A is paused

### Music stop/query/seek parity
**Requirement text**: Music control/query APIs that accept a music reference argument shall apply the same raw-handle identity and wildcard semantics as the C contract, so that stop/query/seek operations only act on the current music when the supplied reference matches or the wildcard contract permits unconditional action.

Behavior contract:
- GIVEN: Music A is current, caller passes music reference B
- WHEN: `PLRStop(B)` or `PLRSeek(B, ...)` or `PLRPlaying(B)` is called
- THEN: The operation behaves as a non-matching no-op / false result per the ABI contract, without disturbing Music A

- GIVEN: Music A is current, caller passes wildcard (~0)
- WHEN: `PLRStop(~0)` or `PLRSeek(~0, ...)` or `PLRPlaying(~0)` is called
- THEN: The operation applies to the current music according to the wildcard contract

### Standalone speech stop behavior
**Requirement text**: Standalone speech control shall preserve ownership and stop semantics, including `snd_stop_speech` behavior, so that stopping standalone speech clears standalone-speech state without corrupting active track-owned speech playback.

Behavior contract:
- GIVEN: Standalone speech is active and track playback does not own the speech source
- WHEN: `snd_stop_speech()` is called
- THEN: Standalone speech stops and standalone speech state is cleared

- GIVEN: Track playback currently owns the speech source
- WHEN: `snd_stop_speech()` is called
- THEN: Standalone speech bookkeeping is cleared without disturbing the active track-owned speech path

Why it matters:
- C callers rely on ref-matching and wildcard semantics across the music-control surface, not just pause
- These are ABI-visible behaviors with direct gameplay implications
- Leaving stop/query/seek/speech-stop semantics until a final checklist invites late surprises

## Implementation Tasks

### TDD — Files to modify

#### `rust/src/sound/music.rs` — add tests
- `test_plr_pause_if_matching_pauses_when_matching` — matching ref pauses
- `test_plr_pause_if_matching_no_op_when_not_matching` — non-matching ref is no-op
- `test_plr_stop_if_matching_stops_when_matching` — matching ref stops
- `test_plr_stop_if_matching_no_op_when_not_matching` — non-matching ref does not stop current music
- `test_plr_playing_if_matching_false_when_not_matching` — non-matching ref reports false / ABI-negative result
- `test_plr_seek_if_matching_no_op_when_not_matching` — non-matching ref does not reposition current music
- `test_music_control_wildcard_applies_to_current_music` — wildcard sentinel covers pause/stop/query/seek path
- marker: `@plan PLAN-20260314-AUDIO-HEART.P08`

#### `rust/src/sound/heart_ffi.rs` — add focused FFI-level tests where practical
- `test_plr_pause_ffi_wildcard_still_pauses`
- `test_plr_playing_ffi_non_matching_ref_returns_false`
- marker: `@plan PLAN-20260314-AUDIO-HEART.P08`

#### `rust/src/sound/trackplayer.rs` or owning speech-control module — add tests
- `test_snd_stop_speech_clears_standalone_speech`
- `test_snd_stop_speech_does_not_break_track_owned_speech`
- marker: `@plan PLAN-20260314-AUDIO-HEART.P08`

### Implementation — Files to modify

#### `rust/src/sound/music.rs`
- Add function `pub fn plr_pause_if_matching(music_ref: &MusicRef) -> AudioResult<()>`:
  - Lock MUSIC_STATE
  - Check if `cur_music_ref` matches `music_ref` by `Arc::ptr_eq`
  - If matching: drop state lock, call `plr_pause()`
  - If not matching: return Ok(()) without pausing
- Add matching helpers for the rest of the control/query surface as needed:
  - `plr_stop_if_matching(...)`
  - `plr_playing_if_matching(...)`
  - `plr_seek_if_matching(...)`
- Preserve existing borrowed-handle identity semantics when the FFI passes a borrowed handle into these helpers
- marker: `@plan PLAN-20260314-AUDIO-HEART.P08`

#### `rust/src/sound/heart_ffi.rs`
- Modify `PLRPause`: use wildcard-aware ref-matching instead of unconditional pause
- Audit the corresponding stop/query/seek entry points and apply the same identity/wildcard rules:
  - `PLRStop`
  - `PLRPlaying`
  - `PLRSeek`
- For each function, preserve the ABI-specific return shape for non-matching references and wildcard inputs
- Remove comments that describe Rust as intentionally looser than C in this area
- marker: `@plan PLAN-20260314-AUDIO-HEART.P08`

#### Owning speech-control module (`music.rs`, `control.rs`, or `trackplayer.rs` as analysis dictates)
- Implement/adjust `snd_stop_speech` ownership handling so standalone-speech stop behavior is explicit and safe when track-owned speech is active
- Keep the structure aligned with the existing source-of-truth module for speech-source ownership
- marker: `@plan PLAN-20260314-AUDIO-HEART.P08`

### Pseudocode traceability
- Implements PC-05 lines 01-16 for `PLRPause`
- Extends the same identity/wildcard pattern to the remaining music control/query APIs covered by the requirements surface
- Pulls standalone speech-stop behavior out of the final checklist and into an explicit implementation slice

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings 2>&1 | head -50
cargo test --workspace --all-features

# Verify music control/query entry points apply ref-matching or wildcard handling
grep -n 'fn PLRPause\|fn PLRStop\|fn PLRPlaying\|fn PLRSeek\|snd_stop_speech' rust/src/sound/heart_ffi.rs rust/src/sound/music.rs rust/src/sound/trackplayer.rs rust/src/sound/control.rs
```

## Structural Verification Checklist
- [ ] `music.rs` has helper(s) for ref-matching control/query behavior
- [ ] `heart_ffi.rs` applies ref-matching or wildcard logic in `PLRPause`
- [ ] `heart_ffi.rs` applies the same contract to `PLRStop`, `PLRPlaying`, and `PLRSeek`
- [ ] Wildcard sentinel (~0) still triggers the allowed unconditional behavior where required
- [ ] `snd_stop_speech` ownership behavior is implemented in the correct owning module
- [ ] Comments describing intentionally looser Rust behavior are removed

## Semantic Verification Checklist
- [ ] Non-matching ref is a no-op / false result for pause/stop/seek/query as required
- [ ] Matching ref acts on the current music resource
- [ ] Wildcard sentinel behavior is verified for pause/stop/query/seek
- [ ] Null pointer handling remains ABI-consistent
- [ ] Borrowed-handle identity semantics are preserved in the play/control path
- [ ] `snd_stop_speech` clears standalone speech state without breaking track-owned speech playback
- [ ] Signature/ABI semantics remain consistent with the existing C surface

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|Rust just\|regardless" rust/src/sound/heart_ffi.rs rust/src/sound/music.rs rust/src/sound/trackplayer.rs rust/src/sound/control.rs
```

## Success Criteria
- [ ] Music control/query APIs match C behavior for ref-matching and wildcard semantics
- [ ] Standalone speech stop semantics are explicit and correct
- [ ] Tests pass
- [ ] Parity comments removed

## Failure Recovery
- rollback: `git restore rust/src/sound/music.rs rust/src/sound/heart_ffi.rs rust/src/sound/trackplayer.rs rust/src/sound/control.rs`

## Phase Completion Marker
Create: `project-plans/20260311/audio-heart/.completed/P08.md`
