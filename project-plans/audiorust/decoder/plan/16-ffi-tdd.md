# Phase 16: FFI TDD

## Phase ID
`PLAN-20260225-AIFF-DECODER.P16`

## Prerequisites
- Required: Phase 15 completed (FFI stub compiles)
- Expected files: `rust/src/sound/aiff_ffi.rs` with stub functions

## Requirements Implemented (Expanded)

### REQ-FF-2: Vtable Existence
**Requirement text**: Verify vtable static exists and is accessible.

Behavior contract:
- GIVEN: The `aiff_ffi` module is loaded
- WHEN: `rust_aifa_DecoderVtbl` is referenced
- THEN: It exists as a valid `TFB_SoundDecoderFuncs` with all 12 function pointers non-null

### REQ-FF-10: Null Pointer Safety
**Requirement text**: Every FFI function handles null pointers safely.

Behavior contract:
- GIVEN: A null `*mut TFB_SoundDecoder` pointer
- WHEN: Any vtable function is called with it
- THEN: Returns safe default (0, -1, or void) without crashing

### REQ-FF-11: GetStructSize
**Requirement text**: Returns correct struct size.

Behavior contract:
- GIVEN: TFB_RustAiffDecoder struct
- WHEN: `rust_aifa_GetStructSize()` is called
- THEN: Returns `size_of::<TFB_RustAiffDecoder>()` which is >= `size_of::<TFB_SoundDecoder>()`

### REQ-FF-12: GetName
**Requirement text**: Returns valid C string "Rust AIFF".

Behavior contract:
- GIVEN: The FFI module
- WHEN: `rust_aifa_GetName()` is called
- THEN: Returns a non-null pointer to a null-terminated C string containing "Rust AIFF"

Why it matters:
- FFI tests ensure the C interface contract is correct
- Null pointer tests prevent segfaults in the C integration

## Implementation Tasks

### Files to modify
- `rust/src/sound/aiff_ffi.rs` — Add `#[cfg(test)] mod tests`
  - marker: `@plan PLAN-20260225-AIFF-DECODER.P16`
  - marker: `@requirement REQ-FF-2, REQ-FF-10, REQ-FF-11, REQ-FF-12`

### Test Cases to Write

1. `test_vtable_exists` — `rust_aifa_DecoderVtbl` is accessible and has all 12 non-null function pointers
2. `test_get_name_returns_valid_string` — GetName returns non-null pointer, C string is "Rust AIFF"
3. `test_get_struct_size_valid` — Size >= size_of TFB_SoundDecoder
4. `test_init_module_and_term_module` — InitModule stores formats, TermModule clears them
5. `test_get_error_null_decoder` — GetError with null decoder returns -1
6. `test_init_null_decoder` — Init with null decoder returns 0
7. `test_term_null_decoder` — Term with null decoder doesn't crash
8. `test_close_null_decoder` — Close with null decoder doesn't crash
9. `test_seek_null_decoder` — Seek with null decoder returns pcm_pos
10. `test_get_frame_null_decoder` — GetFrame with null decoder returns 0
11. `test_decode_null_decoder` — Decode with null decoder returns 0
12. `test_init_term_lifecycle` — Init allocates, Term deallocates without leak

### Pseudocode traceability
- Tests cover pseudocode lines: 31–172 (all FFI functions)

## Verification Commands

```bash
cd /Users/acoliver/projects/uqm/rust

# Tests compile
cargo test --lib --all-features -- aiff_ffi --no-run

# Format and lint
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

## Structural Verification Checklist
- [ ] Test module exists in `aiff_ffi.rs`
- [ ] At least 12 test functions defined
- [ ] Tests compile (`--no-run`)
- [ ] Null pointer tests for all functions taking `*mut TFB_SoundDecoder`

## Semantic Verification Checklist (Mandatory)
- [ ] Vtable test checks all 12 function pointer fields are non-null
- [ ] GetName test validates the C string content (not just non-null)
- [ ] Null pointer tests verify safe return values (0, -1, void)
- [ ] InitModule/TermModule test verifies format storage lifecycle
- [ ] No test that would pass with a crashed process

## Deferred Implementation Detection (Mandatory)

```bash
cd /Users/acoliver/projects/uqm/rust && grep -n "todo!()" src/sound/aiff_ffi.rs
# Open and Decode should still have todo!()
```

## Success Criteria
- [ ] All test functions compile
- [ ] Tests for simple functions pass (non-todo functions)
- [ ] Null safety tests pass
- [ ] All REQ-FF-* requirements have test coverage

## Failure Recovery
- rollback steps: `git checkout -- rust/src/sound/aiff_ffi.rs`
- blocking issues: If null tests crash instead of returning defaults, fix null checks

## Phase Completion Marker
Create: `project-plans/audiorust/decoder/.completed/P16.md`

Contents:
- phase ID: PLAN-20260225-AIFF-DECODER.P16
- timestamp
- files changed: `rust/src/sound/aiff_ffi.rs` (tests added)
- tests added: ~12 FFI tests
- verification outputs
- semantic verification summary
