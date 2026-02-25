# Phase 06: PCM Decode Stub

## Phase ID
`PLAN-20260225-AIFF-DECODER.P06`

## Prerequisites
- Required: Phase 05 completed (parser fully implemented)
- Expected files: `rust/src/sound/aiff.rs` with working parser

## Requirements Implemented (Expanded)

### REQ-DP-1 (dispatch): PCM Decode Dispatch
**Requirement text**: Replace `decode()` `todo!()` with a match dispatch to `decode_pcm()` and `decode_sdx2()`.

Behavior contract:
- GIVEN: The `decode()` method currently uses `todo!()`
- WHEN: This phase completes
- THEN: `decode()` dispatches to `decode_pcm()` for `CompressionType::None`, and to `decode_sdx2()` for `CompressionType::Sdx2`

### REQ-EH-6: Unknown Compression Dispatch
**Requirement text**: `decode()` with unknown comp_type returns `Err(DecoderError)`.

Behavior contract:
- GIVEN: The dispatch match covers all `CompressionType` variants
- WHEN: A PCM file is opened and `decode()` is called
- THEN: The match dispatches to `decode_pcm()`; the match is exhaustive so unknown types are handled at compile time

### Stub Functions
Behavior contract:
- GIVEN: `decode_pcm()` and `decode_sdx2()` are created as `todo!()` stubs
- WHEN: `cargo check` is run
- THEN: The code compiles; the stubs will panic at runtime if called (expected in RED phase)

Why it matters:
- Separates the decode dispatch mechanism from the actual decoding logic
- Allows TDD to focus on PCM decoding behavior specifically

## Implementation Tasks

### Files to modify
- `rust/src/sound/aiff.rs`
  - marker: `@plan PLAN-20260225-AIFF-DECODER.P06`
  - marker: `@requirement REQ-DP-1`
  - Replace `decode()` `todo!()` with dispatch:
    ```
    match self.comp_type {
        CompressionType::None => self.decode_pcm(buf),
        CompressionType::Sdx2 => self.decode_sdx2(buf),
    }
    ```
  - Add `fn decode_pcm(&mut self, buf: &mut [u8]) -> DecodeResult<usize>` — `todo!()` stub
  - Add `fn decode_sdx2(&mut self, buf: &mut [u8]) -> DecodeResult<usize>` — `todo!()` stub

### Pseudocode traceability
- Dispatch: pseudocode lines 316–319
- Stubs for: lines 226–249 (decode_pcm), lines 250–295 (decode_sdx2)

## Verification Commands

```bash
cd /Users/acoliver/projects/uqm/rust

# Must still compile
cargo check --all-features

# Existing parser tests must still pass
cargo test --lib --all-features -- aiff::tests::test_parse
cargo test --lib --all-features -- aiff::tests::test_f80
cargo test --lib --all-features -- aiff::tests::test_reject
cargo test --lib --all-features -- aiff::tests::test_chunk

# Format and lint
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

## Structural Verification Checklist
- [ ] `decode()` no longer contains `todo!()` — dispatches to decode_pcm/decode_sdx2
- [ ] `decode_pcm()` method exists with `todo!()` body
- [ ] `decode_sdx2()` method exists with `todo!()` body
- [ ] All previous tests still pass

## Semantic Verification Checklist (Mandatory)
- [ ] Decode dispatch is a real match on `self.comp_type`
- [ ] No fake decode behavior (only `todo!()` in decode methods)
- [ ] Parser tests unaffected by this change

## Deferred Implementation Detection (Mandatory)

```bash
# Stub phase: todo!() allowed in decode_pcm and decode_sdx2
cd /Users/acoliver/projects/uqm/rust && grep -n "todo!()" src/sound/aiff.rs
# Should show: decode_pcm, decode_sdx2, seek
```

## Success Criteria
- [ ] `cargo check --all-features` succeeds
- [ ] All parser tests pass
- [ ] `decode()` dispatches to `decode_pcm`/`decode_sdx2`
- [ ] `decode_pcm()` and `decode_sdx2()` exist as `todo!()` stubs

## Failure Recovery
- rollback steps: `git checkout -- rust/src/sound/aiff.rs`
- blocking issues: None expected (simple refactor of existing todo)

## Phase Completion Marker
Create: `project-plans/audiorust/decoder/.completed/P06.md`

Contents:
- phase ID: PLAN-20260225-AIFF-DECODER.P06
- timestamp
- files changed: `rust/src/sound/aiff.rs`
- tests added/updated: None
- verification outputs
- semantic verification summary
