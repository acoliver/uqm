# Phase P02b: Mixer Positional Audio Extension

**Phase ID**: PLAN-20260225-AUDIO-HEART.P02b
**Type**: Implementation (combined stub+impl — small scope)
**Prerequisites**: P02a (pseudocode verification) complete

## Purpose

Extend the Rust mixer with positional audio storage (PositionX/Y/Z). The C mixer's
`mixer_Sourcefv` is a complete no-op — positions are stored but never used for panning.
We match that behavior: store position values, expose get/set APIs, but don't alter
the mix loop. This unblocks P14 (music-sfx-impl) which calls `mixer_source_f(PositionX, ...)`.

## Requirements Implemented

- **REQ-SFX-POSITION-01**: GIVEN a source with positional audio enabled, WHEN `update_sound_position` calls `mixer_source_f(PositionX/Y/Z, val)`, THEN the mixer stores the position values without error.
- **REQ-SFX-POSITION-03**: GIVEN a source without positional audio, WHEN default position (0, 0, -1) is set, THEN the mixer stores these default values.

## Implementation Tasks

### 1. Add SourceProp variants (`rust/src/sound/mixer/types.rs`)
```
// @plan PLAN-20260225-AUDIO-HEART.P02b
// @requirement REQ-SFX-POSITION-01
PositionX = 0x2001,
PositionY = 0x2002,
PositionZ = 0x2003,
```

### 2. Add position fields to MixerSource (`rust/src/sound/mixer/source.rs`)
```
// @plan PLAN-20260225-AUDIO-HEART.P02b
pub pos_x: f32,  // positional audio X (stored, not used for panning — matches C)
pub pos_y: f32,  // positional audio Y
pub pos_z: f32,  // positional audio Z (default -1.0 for non-positional)
```

Initialize in `MixerSource::new()`: `pos_x: 0.0, pos_y: 0.0, pos_z: -1.0`

### 3. Handle in mixer_source_f and mixer_get_source_f (`rust/src/sound/mixer/source.rs`)
```
SourceProp::PositionX => { src.pos_x = value; Ok(()) }
SourceProp::PositionY => { src.pos_y = value; Ok(()) }
SourceProp::PositionZ => { src.pos_z = value; Ok(()) }
```

### 4. Add mixer_source_fv convenience function (`rust/src/sound/mixer/source.rs`)
```
/// Set 3-component float vector property (e.g., position).
/// Equivalent to 3 separate mixer_source_f calls.
pub fn mixer_source_fv(handle: usize, prop: SourceProp, values: &[f32; 3]) -> Result<(), MixerError> {
    // For Position, map to PositionX/Y/Z
    mixer_source_f(handle, SourceProp::PositionX, values[0])?;
    mixer_source_f(handle, SourceProp::PositionY, values[1])?;
    mixer_source_f(handle, SourceProp::PositionZ, values[2])?;
    Ok(())
}
```

### 5. Tests
- `test_source_position_set_get`: Set PositionX/Y/Z, verify via get
- `test_source_position_defaults`: New source has (0, 0, -1)
- `test_source_fv`: Set all 3 via mixer_source_fv, verify individually
- `test_position_invalid_handle`: Returns MixerError for invalid handle

## Verification Commands
```bash
cd rust && cargo test --lib mixer -- --nocapture
cd rust && cargo clippy --all-features -- -D warnings
cd rust && cargo fmt -- --check
cd sc2 && ./build.sh uqm
```

## Structural Verification Checklist
- [ ] `SourceProp::PositionX/Y/Z` variants exist in types.rs
- [ ] `MixerSource` has `pos_x`, `pos_y`, `pos_z` fields
- [ ] `mixer_source_f` handles PositionX/Y/Z
- [ ] `mixer_get_source_f` handles PositionX/Y/Z
- [ ] `mixer_source_fv` function exists
- [ ] 4+ tests pass

## Semantic Verification Checklist

### Deterministic
- [ ] `cargo test mixer::source::tests::test_source_position_set_get` passes
- [ ] `cargo test mixer::source::tests::test_source_position_defaults` passes
- [ ] `cargo test mixer::source::tests::test_source_fv` passes
- [ ] No clippy warnings

### Subjective
- [ ] Does setting PositionX/Y/Z actually store the values (not silently discard)?
- [ ] Does the mix loop remain unaffected (no panning logic added)?
- [ ] Are the SourceProp discriminant values (0x2001-0x2003) non-overlapping with existing values?
- [ ] Would the SFX pseudocode's `mixer_source_f(PositionX, x)` calls now succeed?

## Deferred Implementation Detection
```bash
grep -rn 'TODO\|FIXME\|HACK\|unimplemented!\|todo!' rust/src/sound/mixer/types.rs rust/src/sound/mixer/source.rs
```

## Success Criteria
- All 4 tests pass
- Existing mixer tests still pass (no regressions)
- `mixer_source_f(PositionX, val)` returns `Ok(())`
- Build succeeds

## Failure Recovery
```bash
git restore rust/src/sound/mixer/types.rs rust/src/sound/mixer/source.rs
```

## Phase Completion
When all checks pass, create `.completed/02b-mixer-extension.md` with timestamp and test results.
