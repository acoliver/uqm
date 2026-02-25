# Phase 03a: Parser Stub Verification

## Phase ID
`PLAN-20260225-AIFF-DECODER.P03a`

## Prerequisites
- Required: Phase 03 completed
- Expected files: `rust/src/sound/aiff.rs`, updated `rust/src/sound/mod.rs`

## Verification Commands

```bash
cd /Users/acoliver/projects/uqm/rust

# Compile check
cargo check --all-features

# Format check
cargo fmt --all --check

# Lint check
cargo clippy --workspace --all-targets --all-features -- -D warnings

# Verify module registration
grep "pub mod aiff;" src/sound/mod.rs

# Verify trait impl
grep "impl SoundDecoder for AiffDecoder" src/sound/aiff.rs

# Verify constants
grep "FORM_ID" src/sound/aiff.rs
grep "SDX2_COMPRESSION" src/sound/aiff.rs

# Check for forbidden patterns in non-stub code
grep -n "todo!()" src/sound/aiff.rs | head -20

# Deferred implementation detection
grep -RIn "FIXME\|HACK\|placeholder\|for now\|will be implemented" src/sound/aiff.rs || echo "CLEAN"
```

## Structural Verification Checklist
- [ ] `rust/src/sound/aiff.rs` exists
- [ ] `pub mod aiff;` present in `rust/src/sound/mod.rs`
- [ ] `cargo check --all-features` succeeds
- [ ] `cargo fmt --all --check` passes
- [ ] Constants: FORM_ID, FORM_TYPE_AIFF, FORM_TYPE_AIFC, COMMON_ID, SOUND_DATA_ID, SDX2_COMPRESSION defined
- [ ] Types: CompressionType, CommonChunk, SoundDataHeader, ChunkHeader defined
- [ ] AiffDecoder struct has all fields from spec

## Semantic Verification Checklist (Mandatory)

### Deterministic Checks
- [ ] `AiffDecoder::new()` compiles and initializes all fields
- [ ] `impl SoundDecoder for AiffDecoder` compiles
- [ ] Trivial accessors (name, frequency, format, length, is_null, needs_swap, get_frame) are implemented, not stubbed
- [ ] `get_error()` implements get-and-clear pattern (REQ-EH-1)
- [ ] `close()` clears data/positions/predictor (REQ-EH-4)
- [ ] Complex methods (open, open_from_bytes, decode, seek) use `todo!()` stubs
- [ ] No `FIXME`/`HACK`/`placeholder` markers in implemented methods

### Subjective Checks
- [ ] Does `name()` return exactly "AIFF" (not "aiff" or "Aiff")?
- [ ] Does `close()` clear ALL state: data, data_pos, cur_pcm, max_pcm, prev_val — matching the idempotency requirement (REQ-EH-4)?
- [ ] Does `init()` correctly compute `need_swap` from the stored formats' `want_big_endian` flag (REQ-LF-5)?
- [ ] Does `init_module()` store the DecoderFormats and `term_module()` clear them (REQ-LF-2, REQ-LF-4)?
- [ ] Are the constant values correct: FORM_ID=0x464F524D, AIFF=0x41494646, AIFC=0x41494643?

## Deferred Implementation Detection

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
- [ ] `todo!()` only in genuinely unimplemented methods
- [ ] No forbidden placeholder patterns

## Failure Recovery
- Return to Phase 03 and fix compilation errors or missing types
- If SoundDecoder trait methods don't match, check `rust/src/sound/decoder.rs` for correct signatures
- rollback: `git checkout -- rust/src/sound/aiff.rs rust/src/sound/mod.rs`

## Phase Completion Marker
Create: `project-plans/audiorust/decoder/.completed/P03a.md`

Contents:
- phase ID: PLAN-20260225-AIFF-DECODER.P03a
- timestamp
- verification result: PASS/FAIL
- gaps identified (if any)
- gate decision: proceed to P04 or return to P03
