# Phase P02c: Mixer Extension Verification

**Phase ID**: PLAN-20260225-AUDIO-HEART.P02c
**Type**: Verification
**Prerequisites**: P02b (mixer extension) complete

## Requirements Implemented (Expanded)

N/A — Verification-only phase. Requirements are verified, not implemented.

## Implementation Tasks

N/A — Verification-only phase. No code changes.

## Verification Commands
```bash
cd rust && cargo test --lib mixer -- --nocapture
cd rust && cargo test --lib --all-features
cd rust && cargo clippy --all-features -- -D warnings
cd rust && cargo fmt -- --check
cd sc2 && rm -f uqm && ./build.sh uqm
```

## Structural Verification Checklist
- [ ] `SourceProp` enum has `PositionX = 0x2001`, `PositionY = 0x2002`, `PositionZ = 0x2003`
- [ ] `MixerSource` struct has `pos_x: f32`, `pos_y: f32`, `pos_z: f32` fields
- [ ] `MixerSource::new()` initializes pos_x=0, pos_y=0, pos_z=-1
- [ ] `mixer_source_f` handles PositionX/Y/Z set
- [ ] `mixer_get_source_f` handles PositionX/Y/Z get
- [ ] `mixer_source_fv` function exists and is public
- [ ] All existing mixer tests still pass (no regressions)
- [ ] No `todo!()`, `unimplemented!()`, `FIXME`, or `HACK` in modified files

## Semantic Verification Checklist

### Deterministic
- [ ] `test_source_position_set_get` passes
- [ ] `test_source_position_defaults` passes
- [ ] `test_source_fv` passes
- [ ] `test_position_invalid_handle` passes
- [ ] Total mixer test count >= previous count + 4
- [ ] `cargo clippy` clean

### Subjective
- [ ] Does setting pos_x to 5.0 then getting it return exactly 5.0 (no precision loss)?
- [ ] Are the position fields initialized to the documented defaults (0, 0, -1)?
- [ ] Does `mixer_source_fv` correctly delegate to 3 individual `mixer_source_f` calls?
- [ ] Is the mix loop (`process_source`) completely unmodified? (Position storage only, no panning)
- [ ] Would the SFX code's `mixer_source_f(SourceProp::PositionX, x)` calls now succeed where they previously returned `InvalidEnum`?

## Deferred Implementation Detection
```bash
grep -rn 'TODO\|FIXME\|HACK\|unimplemented!\|todo!' rust/src/sound/mixer/types.rs rust/src/sound/mixer/source.rs
```

## Success Criteria
- All mixer tests pass (old + new)
- Full build succeeds
- No clippy warnings
- Position values round-trip correctly through set/get

## Failure Recovery
```bash
git restore rust/src/sound/mixer/types.rs rust/src/sound/mixer/source.rs
```

## Phase Completion Marker
When all checks pass, create `.completed/02c-mixer-extension-verification.md` with timestamp.
