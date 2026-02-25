# Phase 05: Types Implementation

## Phase ID
`PLAN-20260225-AUDIO-HEART.P05`

## Prerequisites
- Required: Phase P04a (Types TDD Verification) passed
- Expected files: `types.rs` with test module, stubs compiling

## Requirements Implemented (Expanded)

### Constants (REQ-CROSS-CONST-01 through REQ-CROSS-CONST-08)
- GIVEN: The audio subsystem needs shared constants
- WHEN: Any module references MAX_VOLUME, NORMAL_VOLUME, NUM_SFX_CHANNELS, etc.
- THEN: Values match specification §3.2 exactly

### Error Handling (REQ-CROSS-ERROR-01, REQ-CROSS-ERROR-02, REQ-CROSS-ERROR-03)
- GIVEN: AudioError enum with 14 variants
- WHEN: Errors are converted from MixerError or DecodeError
- THEN: The correct AudioError variant is produced via From trait

### General (REQ-CROSS-GENERAL-01, REQ-CROSS-GENERAL-04, REQ-CROSS-GENERAL-05)
- GIVEN: Core types (SoundSample, SoundTag, SoundSource, etc.)
- WHEN: Types are used in concurrent contexts
- THEN: All Mutex usage is parking_lot::Mutex, all types implement required traits (Send/Sync as needed)

### SoundDecoder Trait Gap Resolution (rust-heart.md Action Items #1-3)
In this phase, the `todo!()` stubs for `decode_all` and `get_decoder_time` are replaced with full implementations. The `SoundSample.looping` field (added in P03) is verified to work correctly. These are prerequisites for the stream impl phase (P08).

### Pseudocode traceability
- `decode_all`: pseudocode `stream.md` lines 540-585 (defined with buffer growth strategy); called from `sfx.md` line 155 (get_sound_bank_data)
- `get_decoder_time`: pseudocode `stream.md` lines 95 (called from play_stream offset calc)

## Implementation Tasks

### Files to modify
- `rust/src/sound/types.rs` — Complete all implementations
  - marker: `@plan PLAN-20260225-AUDIO-HEART.P05`
  - marker: `@requirement REQ-CROSS-CONST-01, REQ-CROSS-CONST-02, REQ-CROSS-CONST-03, REQ-CROSS-CONST-04, REQ-CROSS-CONST-05, REQ-CROSS-CONST-06, REQ-CROSS-CONST-07, REQ-CROSS-CONST-08, REQ-CROSS-ERROR-01, REQ-CROSS-ERROR-02, REQ-CROSS-ERROR-03, REQ-CROSS-GENERAL-01, REQ-CROSS-GENERAL-04, REQ-CROSS-GENERAL-05`

### Implementation details
1. **`decode_all`** (pseudocode `stream.md` lines 540-585) — Implements a two-phase buffer growth strategy to avoid O(n^2) reallocation:
   - **Pre-allocation phase**: If `decoder.length()` > 0 (known duration), compute expected byte count as `length * frequency * bytes_per_sample * channels` and pre-allocate with 10% headroom. If length is unknown, start with 64KB (`Vec::with_capacity(65536)`).
   - **Decode loop**: Use a fixed 4KB scratch buffer. Loop calling `decoder.decode(&mut scratch)`, appending results via `Vec::extend_from_slice`. This leverages Rust's standard Vec doubling growth (amortized O(1) per byte). Handle `Ok(0)` and `Err(EndOfFile)` as EOF. Log and propagate `DecoderError`. Retry on transient errors.
   - **Finalization**: Call `result.shrink_to_fit()` to release unused capacity.
   - The pre-allocation means most SFX files (where length is known) require zero reallocations. Unknown-length streams still achieve O(n log n) total allocation cost via doubling.
2. **`get_decoder_time`** — Return `decoder.get_frame() as f32 / decoder.frequency().max(1) as f32` (avoid division by zero).
3. **Verify all Display impls** produce meaningful messages.
4. **Verify From conversions** map correctly.
5. **Remove all `todo!()` stubs** — anti-placeholder rule.

## Verification Commands

```bash
cd /Users/acoliver/projects/uqm/rust && cargo test --lib --all-features -- sound::types::tests
cd /Users/acoliver/projects/uqm/rust && cargo fmt --all --check
cd /Users/acoliver/projects/uqm/rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
```

## Structural Verification Checklist
- [ ] All `todo!()` removed from types.rs
- [ ] `@plan` markers present
- [ ] All tests pass
- [ ] fmt and clippy pass

## Semantic Verification Checklist (Mandatory)
- [ ] `decode_all` actually decodes (not just returns empty)
- [ ] `get_decoder_time` divides correctly
- [ ] Error conversions produce correct variants
- [ ] All tests pass GREEN

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented\|todo!()" rust/src/sound/types.rs
# Must return 0 results
```

## Success Criteria
- [ ] All tests pass
- [ ] No deferred implementations
- [ ] Types fully usable by subsequent phases

## Failure Recovery
- rollback: `git checkout -- rust/src/sound/types.rs`
- blocking issues: If decoder trait methods are missing, add them in this phase

## Phase Completion Marker
Create: `project-plans/audiorust/heart/.completed/P05.md`
