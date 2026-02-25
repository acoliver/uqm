# Phase 04a: Types TDD Verification

## Phase ID
`PLAN-20260225-AUDIO-HEART.P04a`

## Prerequisites
- Required: Phase P04 completed
- Expected: Test module in `types.rs` with 13+ tests

## Verification Commands

```bash
cd /Users/acoliver/projects/uqm/rust && cargo test --lib --all-features -- sound::types::tests 2>&1
cd /Users/acoliver/projects/uqm/rust && cargo fmt --all --check
cd /Users/acoliver/projects/uqm/rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
cd /Users/acoliver/projects/uqm/sc2 && ./build.sh uqm
```

## Structural Verification Checklist
- [ ] Test module `#[cfg(test)] mod tests` exists in `types.rs`
- [ ] At least 13 test functions present
- [ ] Tests compile without errors
- [ ] `cargo fmt` passes
- [ ] `cargo clippy` passes

## Semantic Verification Checklist

### Deterministic checks
- [ ] Test count >= 13: `cargo test --lib --all-features -- sound::types::tests 2>&1 | grep "test result"` shows >= 13 tests
- [ ] Specific test names exist: `test_constants_values`, `test_audio_error_display`, `test_audio_error_from_mixer_error`, `test_audio_error_from_decode_error`, `test_sound_position_non_positional`, `test_sound_sample_default_state`, `test_decode_all_with_null_decoder`, `test_get_decoder_time_zero`, `test_send_sync_bounds`
- [ ] Tests cover the `looping: bool` field on SoundSample: `grep -c "looping" rust/src/sound/types.rs` > 1 (field + at least one test)

### Subjective checks
- [ ] Tests verify behavior, not just compilation — do the constant tests assert specific numeric values (255, 160, 5, etc.)?
- [ ] Error conversion tests verify correct variant mapping — does `From<MixerError>` produce `AudioError::MixerError`?
- [ ] Default state tests verify initial field values — does a new SoundSample have `looping: false`, `length: 0`, etc.?
- [ ] Thread safety verified at compile time — do the Send/Sync assertions use `fn assert_send<T: Send>() {}` pattern?
- [ ] No tests that only assert `true` or trivially pass

## Deferred Implementation Detection

```bash
grep -n "TODO\|FIXME\|HACK\|placeholder" rust/src/sound/types.rs | grep -v "todo!()" | grep -v "test"
# Expected: 0 non-test, non-todo!() deferred markers
```

## Success Criteria
- [ ] All 13+ tests written and compile
- [ ] Tests cover constants, errors, conversions, defaults, SoundDecoder gaps, Send/Sync
- [ ] Tests will meaningfully FAIL with wrong implementation

## Failure Recovery
- rollback: `git checkout -- rust/src/sound/types.rs`
- blocking issues: If type signatures need adjustment, update stub first in a targeted commit

## Phase Completion Marker
Create: `project-plans/audiorust/heart/.completed/P04a.md`
