# Phase 03: AIFF Parser Stub

## Phase ID
`PLAN-20260225-AIFF-DECODER.P03`

## Prerequisites
- Required: Phase 02 completed (pseudocode exists)
- Expected files from previous phase: `analysis/pseudocode/aiff.md`, `analysis/pseudocode/aiff_ffi.md`

## Requirements Implemented (Expanded)

### REQ-FP-1 through REQ-FP-15: File Parsing (Stubs)
**Requirement text**: Create compile-safe skeletons for AIFF file header parsing, chunk iteration, COMM parsing, SSND parsing, and IEEE 754 80-bit float conversion.

Behavior contract:
- GIVEN: A new `aiff.rs` file is created with the `AiffDecoder` struct and all types
- WHEN: `cargo check` is run
- THEN: The code compiles with `todo!()` stubs for parsing functions

### REQ-SV-1 through REQ-SV-13: Validation (Stubs)
**Requirement text**: Declare validation logic entry points in `open_from_bytes()` stub.

### REQ-CH-1 through REQ-CH-7: Compression Handling (Stubs)
**Requirement text**: Define `CompressionType` enum and compression detection stubs.

### REQ-LF-1 through REQ-LF-10: Lifecycle (Stubs)
**Requirement text**: Implement `SoundDecoder` trait with stub methods. Simple accessors (name, get_frame, is_null, needs_swap, frequency, format, length) can be fully implemented since they are trivial.

### REQ-EH-1 through REQ-EH-4: Error Handling (Stubs)
**Requirement text**: Implement get_error (get-and-clear) and close() idempotency since these are simple.

Why it matters:
- Establishes compile-safe foundation for TDD
- Defines all types and API surface
- Registers module in `mod.rs`

## Implementation Tasks

### Files to create
- `rust/src/sound/aiff.rs` — Pure Rust AIFF decoder skeleton
  - marker: `@plan PLAN-20260225-AIFF-DECODER.P03`
  - marker: `@requirement REQ-FP-1, REQ-SV-1, REQ-CH-1, REQ-LF-1, REQ-EH-1`
  - Constants: FORM_ID, FORM_TYPE_AIFF, FORM_TYPE_AIFC, COMMON_ID, SOUND_DATA_ID, SDX2_COMPRESSION, AIFF_COMM_SIZE, AIFF_EXT_COMM_SIZE, AIFF_SSND_SIZE, MAX_CHANNELS, MIN_SAMPLE_RATE, MAX_SAMPLE_RATE
  - Types: CompressionType enum, CommonChunk struct, SoundDataHeader struct, ChunkHeader struct
  - AiffDecoder struct with all fields
  - `AiffDecoder::new()` — fully implemented (only initializes fields)
  - `impl SoundDecoder for AiffDecoder` — with:
    - `name()` → `"AIFF"` (implemented)
    - `init_module()` → stores formats (implemented)
    - `term_module()` → clears formats (implemented)
    - `get_error()` → get-and-clear (implemented)
    - `init()` → sets need_swap (implemented)
    - `term()` → calls close() (implemented)
    - `open()` → `todo!()` stub
    - `open_from_bytes()` → `todo!()` stub
    - `close()` → clears state (implemented — simple reset)
    - `decode()` → `todo!()` stub
    - `seek()` → `todo!()` stub
    - `get_frame()` → 0 (implemented)
    - `frequency()` → self.frequency (implemented)
    - `format()` → self.format (implemented)
    - `length()` → self.length (implemented)
    - `is_null()` → false (implemented)
    - `needs_swap()` → self.need_swap (implemented)
  - Byte reading helpers: `read_be_u16`, `read_be_u32`, `read_be_i16` → `todo!()` stubs
  - `read_chunk_header()` → `todo!()` stub
  - `read_common_chunk()` → `todo!()` stub
  - `read_sound_data_header()` → `todo!()` stub
  - `read_be_f80()` → `todo!()` stub

### Files to modify
- `rust/src/sound/mod.rs`
  - Add: `pub mod aiff;`
  - marker: `@plan PLAN-20260225-AIFF-DECODER.P03`

### Pseudocode traceability
- Uses pseudocode lines: 1–19 (constructor), 339–378 (trait accessors), 333–338 (close)

## Verification Commands

```bash
# Structural gate
cd /Users/acoliver/projects/uqm/rust && cargo fmt --all --check
cd /Users/acoliver/projects/uqm/rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
cd /Users/acoliver/projects/uqm/rust && cargo test --lib --all-features
```

## Structural Verification Checklist
- [ ] `rust/src/sound/aiff.rs` created
- [ ] `rust/src/sound/mod.rs` updated with `pub mod aiff;`
- [ ] All types defined: CompressionType, CommonChunk, SoundDataHeader, ChunkHeader, AiffDecoder
- [ ] All constants defined (FORM_ID, etc.)
- [ ] SoundDecoder trait implemented (even if some methods are `todo!()`)
- [ ] Plan/requirement traceability markers present
- [ ] Code compiles (`cargo check`)

## Semantic Verification Checklist (Mandatory)
- [ ] AiffDecoder::new() returns a valid instance with all fields initialized
- [ ] name() returns "AIFF"
- [ ] get_error() returns last_error and resets to 0
- [ ] close() clears data, positions, and predictor state
- [ ] init() sets need_swap based on formats
- [ ] init_module() stores formats, term_module() clears them
- [ ] No placeholder behavior in implemented methods (only `todo!()` in genuinely unimplemented functions)

## Deferred Implementation Detection (Mandatory)

```bash
# Stub phase: todo!() is allowed. Verify no other placeholder patterns:
cd /Users/acoliver/projects/uqm/rust && grep -RIn "FIXME\|HACK\|placeholder\|for now\|will be implemented" src/sound/aiff.rs || echo "CLEAN"
```

## Success Criteria
- [ ] `cargo check --all-features` succeeds
- [ ] `cargo fmt --all --check` passes
- [ ] All types and constants defined
- [ ] SoundDecoder trait implemented on AiffDecoder
- [ ] Module registered in mod.rs

## Failure Recovery
- rollback steps: `git checkout -- rust/src/sound/aiff.rs rust/src/sound/mod.rs`
- blocking issues: If SoundDecoder trait signature doesn't match spec, update spec first

## Phase Completion Marker
Create: `project-plans/audiorust/decoder/.completed/P03.md`

Contents:
- phase ID: PLAN-20260225-AIFF-DECODER.P03
- timestamp
- files changed: `rust/src/sound/aiff.rs` (created), `rust/src/sound/mod.rs` (modified)
- tests added/updated: None (stub phase)
- verification outputs
- semantic verification summary
