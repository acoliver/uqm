# Phase 04: Types TDD

## Phase ID
`PLAN-20260225-AUDIO-HEART.P04`

## Prerequisites
- Required: Phase P03a (Types Stub Verification) passed
- Expected files: `rust/src/sound/types.rs` compiling

## Requirements Implemented (Expanded)

### REQ-CROSS-CONST-01, REQ-CROSS-CONST-02, REQ-CROSS-CONST-03, REQ-CROSS-CONST-04, REQ-CROSS-CONST-05, REQ-CROSS-CONST-06, REQ-CROSS-CONST-07, REQ-CROSS-CONST-08: Constants Correctness
**Requirement text**: All constants match specified values.

Behavior contract:
- GIVEN: Constants defined in types.rs
- WHEN: Tests assert their values
- THEN: All match spec exactly

### REQ-CROSS-ERROR-01, REQ-CROSS-ERROR-02, REQ-CROSS-ERROR-03: Error Handling
**Requirement text**: AudioError conversions and Display work correctly.

Behavior contract:
- GIVEN: AudioError with From<MixerError> and From<DecodeError>
- WHEN: Errors are converted
- THEN: Correct variants produced, Display messages meaningful

### REQ-CROSS-GENERAL-04: Send+Sync
**Requirement text**: Types satisfy thread-safety bounds.

Behavior contract:
- GIVEN: Types defined
- WHEN: Compile-time assertions check Send/Sync
- THEN: SoundSample is Send, AudioError is Send+Sync

### SoundDecoder Trait Gap Tests (rust-heart.md Action Items #1-3)
**Requirement text**: `decode_all` and `get_decoder_time` free functions work correctly; `SoundSample.looping` field exists.

Behavior contract:
- GIVEN: `decode_all` loops decoder.decode() until EOF
- WHEN: Called with a NullDecoder
- THEN: Returns `Ok(Vec::new())` (empty, since NullDecoder returns EOF immediately)
- GIVEN: `get_decoder_time` computes `get_frame() / frequency()`
- WHEN: Called with a fresh decoder
- THEN: Returns 0.0 (frame 0, frequency > 0)
- GIVEN: `SoundSample` has `looping: bool` field
- WHEN: Constructed with defaults
- THEN: `looping` is `false`

## Implementation Tasks

### Files to modify
- `rust/src/sound/types.rs` — Add `#[cfg(test)] mod tests` with TDD tests
  - marker: `@plan PLAN-20260225-AUDIO-HEART.P04`
  - marker: `@requirement REQ-CROSS-CONST-01, REQ-CROSS-CONST-02, REQ-CROSS-CONST-03, REQ-CROSS-CONST-04, REQ-CROSS-CONST-05, REQ-CROSS-CONST-06, REQ-CROSS-CONST-07, REQ-CROSS-CONST-08, REQ-CROSS-ERROR-01, REQ-CROSS-ERROR-02, REQ-CROSS-ERROR-03, REQ-CROSS-GENERAL-04`

### Tests to write (RED phase — these should FAIL if types are wrong)
1. `test_constants_values` — Assert all 8 constant groups match spec values
2. `test_audio_error_display` — Verify Display impl for each variant
3. `test_audio_error_from_mixer_error` — Verify From<MixerError>
4. `test_audio_error_from_decode_error` — Verify From<DecodeError>
5. `test_sound_position_non_positional` — Verify NON_POSITIONAL constant
6. `test_sound_position_repr_c` — Verify #[repr(C)] layout (size/alignment)
7. `test_sound_tag_repr_c` — Verify #[repr(C)] layout
8. `test_sound_sample_default_state` — Verify new sample has correct defaults
9. `test_sound_source_default_state` — Verify new source has correct defaults
10. `test_decode_all_with_null_decoder` — Verify decode_all with NullDecoder returns empty Vec
11. `test_get_decoder_time_zero` — Verify get_decoder_time returns 0.0 for a fresh decoder
12. `test_stream_callbacks_defaults` — Verify default trait method return values
13. `test_send_sync_bounds` — Compile-time assertions for thread safety

## Verification Commands

```bash
cd /Users/acoliver/projects/uqm/rust && cargo test --lib --all-features -- sound::types::tests
cd /Users/acoliver/projects/uqm/rust && cargo fmt --all --check
cd /Users/acoliver/projects/uqm/rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
```

## Structural Verification Checklist
- [ ] Test module added to `types.rs`
- [ ] At least 13 test functions
- [ ] Tests reference `@plan` and `@requirement`
- [ ] All tests compile

## Semantic Verification Checklist (Mandatory)
- [ ] Tests verify behavior, not just compilation
- [ ] Constant tests fail if values change
- [ ] Error conversion tests verify correct variant mapping
- [ ] Default state tests verify initial field values
- [ ] Thread safety is verified at compile time

## Deferred Implementation Detection (Mandatory)

```bash
grep -n "TODO\|FIXME\|HACK\|placeholder" rust/src/sound/types.rs | grep -v "todo!()" | grep -v "test"
```

## Success Criteria
- [ ] All 13+ tests written and compile
- [ ] Tests should PASS once implementations are correct (GREEN phase is P05)

## Failure Recovery
- rollback: `git checkout -- rust/src/sound/types.rs`
- blocking issues: If type signatures need adjustment, update stub first

## Phase Completion Marker
Create: `project-plans/audiorust/heart/.completed/P04.md`
