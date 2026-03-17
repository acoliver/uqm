# Phase 05: Loader Consolidation — TDD

## Phase ID
`PLAN-20260314-AUDIO-HEART.P05`

## Prerequisites
- Required: Phase P04a completed
- Expected files from previous phase: shared loader module/file with stubs and explicit seams

## Requirements Implemented (Expanded)

### Music loading behavior
**Requirement text**: Canonical music loading validates filename, opens via UIO, creates decoder, creates SoundSample, returns MusicRef.

Behavior contract:
- GIVEN: Empty filename
- WHEN: `load_music_canonical("")` is called
- THEN: Returns `Err(NullPointer)`

- GIVEN: Valid filename with known extension
- WHEN: `load_music_canonical("test.ogg")` is called through test seams
- THEN: Returns `Ok(MusicRef)` with decoder attached and length > 0

- GIVEN: Filename with unknown extension
- WHEN: `load_music_canonical("test.xyz")` is called
- THEN: Returns `Err(ResourceNotFound)`

### Sound bank loading behavior
**Requirement text**: Canonical bank loading parses listing, loads each sound, decodes, uploads to mixer.

Behavior contract:
- GIVEN: Empty filename
- WHEN: `load_sound_bank_canonical("")` is called
- THEN: Returns `Err(NullPointer)`

- GIVEN: Valid bank file with 3 sound entries
- WHEN: `load_sound_bank_canonical("effects.snd")` is called through test seams
- THEN: Returns `Ok(SoundBank)` with 3 samples, each with mixer buffers

### Canonical routing and verification realism
**Requirement text**: All entry points share the same canonical file-backed loading behavior, but that behavior is integration-heavy and must be verified with explicit seams or a later dedicated integration phase.

Why it matters:
- Validation-only tests are not enough to de-risk canonical-loader work
- The hard parts are UIO reads, decoder creation, relative-path resolution, and mixer upload behavior
- This phase must establish realistic test seams before P06 claims full loader behavior

## Implementation Tasks

### Files to create/modify

#### Shared loader module/file — add seam-oriented tests
Add tests that exercise these seams explicitly:
- `test_load_music_empty_filename_error`
- `test_load_music_no_extension_error`
- `test_load_music_unknown_extension_error`
- `test_load_music_uses_uio_read_seam`
- `test_load_music_uses_decoder_factory_seam`
- `test_load_bank_empty_filename_error`
- `test_load_bank_parses_listing_and_resolves_relative_paths`
- `test_load_bank_skips_or_fails_entries_per_contract` (depending on established behavior)
- `test_load_bank_uses_mixer_upload_seam`
- `test_create_decoder_for_known_extensions`
- `test_create_decoder_for_unknown_extension`
- marker: `@plan PLAN-20260314-AUDIO-HEART.P05`

### Mandatory seam definition
Before P06 implementation, the shared loader layer must expose testable seams/interfaces for:
1. **UIO file reads** — so tests can control bank-file text and binary payloads without live filesystem/engine integration
2. **Decoder creation** — so tests can validate extension dispatch, length reporting, and decode failures deterministically
3. **Mixer upload / sample materialization** — so tests can validate PCM upload intent without requiring the full runtime mixer path

These seams may be traits, helper function parameters, or cfg(test) injectable hooks, but they must be explicit and reusable.

### Integration-style coverage requirement
This phase must choose one of two paths and record it explicitly:
- **Path A:** add at least one integration-style test fixture per loader path (music + bank) using controlled fixture resources; or
- **Path B:** explicitly defer end-to-end loader verification to P06a/P12a with named fixture scenarios and success criteria.

If Path B is chosen, P06/P06a must be updated to state that implementation confidence comes from seams + later integration verification, not from unit tests alone.

### Pseudocode traceability
- Tests cover PC-01 lines 02-12 (validation, extension, file-read seam, decoder selection/creation)
- Tests cover PC-02 lines 02-24 (validation, bank parsing, relative paths, decoder selection/creation, mixer upload seam)
- Tests cover PC-03 lines 01-09 conceptually by proving shared seams are loader-entry-point friendly

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings 2>&1 | head -50
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] Test functions exist for validation, UIO seam, decoder seam, and mixer upload seam
- [ ] Seam mechanism is explicit and reusable by implementation code
- [ ] Plan/requirement traceability present in test comments or phase notes
- [ ] Integration-style verification path (A or B) is recorded

## Semantic Verification Checklist
- [ ] Tests cover: empty filename, missing extension, unknown extension, known extension
- [ ] Tests verify error types, not just success/failure
- [ ] Tests verify loader behavior through seams, not just internal helpers
- [ ] Bank-path tests cover relative-path resolution from the bank file location
- [ ] The phase does not overclaim end-to-end confidence beyond what the test seams actually prove

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/sound/
```

## Success Criteria
- [ ] All seam-level tests compile
- [ ] Validation/seam tests pass or are intentionally staged for red/green TDD work
- [ ] Loader seams are in place for P06
- [ ] Integration verification path is explicitly defined

## Failure Recovery
- rollback: restore only the shared loader module/file and directly related tests

## Phase Completion Marker
Create: `project-plans/20260311/audio-heart/.completed/P05.md`
