# Phase 05a: Types Implementation Verification

## Phase ID
`PLAN-20260225-AUDIO-HEART.P05a`

## Prerequisites
- Required: Phase P05 completed
- Expected: All types fully implemented, all tests passing

## Verification Commands

```bash
cd /Users/acoliver/projects/uqm/rust && cargo test --lib --all-features -- sound::types::tests
cd /Users/acoliver/projects/uqm/rust && cargo fmt --all --check
cd /Users/acoliver/projects/uqm/rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
cd /Users/acoliver/projects/uqm/sc2 && ./build.sh uqm
# Deferred impl detection
grep -RIn "TODO\|FIXME\|HACK\|todo!()" rust/src/sound/types.rs
```

## Structural Verification Checklist
- [ ] All `todo!()` removed from types.rs
- [ ] All tests pass
- [ ] `cargo fmt` passes
- [ ] `cargo clippy` passes
- [ ] `build.sh uqm` succeeds

## Semantic Verification Checklist

### Deterministic checks
- [ ] All 13+ tests pass: `cargo test --lib --all-features -- sound::types::tests` shows 0 failures
- [ ] Zero `todo!()` in types.rs: `grep -c "todo!()" rust/src/sound/types.rs` returns 0
- [ ] Zero deferred markers: `grep -c "TODO\|FIXME\|HACK" rust/src/sound/types.rs` returns 0

### Subjective checks
- [ ] `decode_all` actually decodes — does it loop calling `decoder.decode()` until EOF and collect bytes into a Vec? Will it correctly handle the NullDecoder (empty Vec) AND a decoder with actual data?
- [ ] `get_decoder_time` divides correctly — does it avoid division by zero (frequency().max(1))? Does it return a meaningful f32 time value?
- [ ] Error conversions produce correct variants — does `From<DecodeError::EndOfFile>` map to `AudioError::EndOfStream`? Are the mappings semantically correct?
- [ ] SoundSample.looping field is properly defaulted to false and can be set to true — ready for stream processing use
- [ ] Types are fully usable by subsequent phases — can stream.rs, trackplayer.rs, etc. import everything they need?

## Deferred Implementation Detection

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented\|todo!()" rust/src/sound/types.rs
# Must return 0 results
```

## Success Criteria
- [ ] All tests pass GREEN
- [ ] No deferred implementations
- [ ] Types fully usable by subsequent phases
- [ ] decode_all and get_decoder_time are functional, not stubs

## Failure Recovery
- rollback: `git checkout -- rust/src/sound/types.rs`
- blocking issues: If decoder trait methods are missing, add them in this phase before proceeding

## Phase Completion Marker
Create: `project-plans/audiorust/heart/.completed/P05a.md`
