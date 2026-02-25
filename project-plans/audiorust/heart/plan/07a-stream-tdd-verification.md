# Phase 07a: Stream TDD Verification

## Phase ID
`PLAN-20260225-AUDIO-HEART.P07a`

## Prerequisites
- Required: Phase P07 completed
- Expected: stream.rs test module with 29+ tests

## Verification Commands

```bash
cd /Users/acoliver/projects/uqm/rust && cargo test --lib --all-features -- sound::stream::tests 2>&1
cd /Users/acoliver/projects/uqm/rust && cargo fmt --all --check
cd /Users/acoliver/projects/uqm/rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
cd /Users/acoliver/projects/uqm/sc2 && ./build.sh uqm
```

## Structural Verification Checklist
- [ ] Test module `#[cfg(test)] mod tests` exists in `stream.rs`
- [ ] At least 29 test functions present
- [ ] Tests compile without errors
- [ ] `cargo fmt` passes
- [ ] `cargo clippy` passes

## Semantic Verification Checklist

### Deterministic checks
- [ ] Test count >= 29: `cargo test --lib --all-features -- sound::stream::tests 2>&1 | grep "test result"` shows >= 29 tests
- [ ] Specific test names exist for sample CRUD: `test_create_sound_sample_basic`, `test_destroy_sound_sample_clears_buffers`
- [ ] Specific test names exist for tagging: `test_tag_and_find_buffer`, `test_clear_buffer_tag`
- [ ] Specific test names exist for fade: `test_set_fade_zero_duration_rejected`, `test_process_fade_interpolation`
- [ ] Specific test names exist for scope: `test_add_scope_data_writes`, `test_add_scope_data_wraps`
- [ ] Specific test names exist for playback: `test_playing_stream_initial_false`, `test_stop_stream_clears_state`
- [ ] Specific test names exist for thread: `test_init_decoder_spawns_thread`, `test_uninit_decoder_joins_thread`

### Subjective checks
- [ ] Tests verify behavior (inputs → outputs), not just internals — do they test what the function DOES, not how it's structured?
- [ ] Error path tests verify correct AudioError variant — does `test_seek_no_sample_error` assert `AudioError::InvalidSample`?
- [ ] Fade tests verify numerical correctness — does `test_process_fade_interpolation` check specific volume values at specific time points?
- [ ] Scope tests verify byte-level ring buffer behavior — do they test wrapping past the buffer boundary?
- [ ] Thread lifecycle tests verify clean startup and shutdown — does `test_uninit_decoder_joins_thread` actually verify the thread terminated?
- [ ] No trivially-passing tests — do any tests only assert `true` or assert on default values without meaningful setup?
- [ ] Tests use NullDecoder or mock as appropriate — are test doubles used for mixer interactions?

## Deferred Implementation Detection

```bash
grep -n "todo!()" rust/src/sound/stream.rs | grep -v "#\[cfg(test)\]" | grep -v "mod tests"
# Stubs still have todo!() — that's expected at TDD phase
# Count should match Phase 06 stubs
```

## Success Criteria
- [ ] 29+ tests written and compiling
- [ ] Tests are meaningful — will fail with wrong implementation
- [ ] All requirement areas covered: sample, tag, fade, scope, playback, thread

## Failure Recovery
- rollback: `git checkout -- rust/src/sound/stream.rs`
- blocking issues: If mock mixer needed, create minimal mock in test module before proceeding

## Phase Completion Marker
Create: `project-plans/audiorust/heart/.completed/P07a.md`
