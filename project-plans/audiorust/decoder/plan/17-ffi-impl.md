# Phase 17: FFI Implementation

## Phase ID
`PLAN-20260225-AIFF-DECODER.P17`

## Prerequisites
- Required: Phase 16 completed (FFI tests exist)
- Expected files: `rust/src/sound/aiff_ffi.rs` with tests and stubs

## Requirements Implemented (Expanded)

### REQ-FF-4: Init Allocation
**Requirement text**: Allocate `Box::new(AiffDecoder::new())`, store as raw pointer.

### REQ-FF-5: Term Deallocation
**Requirement text**: Reconstruct Box from raw pointer, drop it, null out pointer.

### REQ-FF-6: Open File Reading
**Requirement text**: Read AIFF file via UIO into `Vec<u8>`, call `open_from_bytes()`.

Behavior contract:
- GIVEN: A valid UIO directory handle and AIFF filename
- WHEN: `rust_aifa_Open()` is called
- THEN: File is read via UIO, `open_from_bytes()` is called, base struct fields are updated

### REQ-FF-7: Open Base Field Update
**Requirement text**: On success, update base struct: frequency, format, length, is_null, need_swap.

### REQ-FF-8: Open Failure Return
**Requirement text**: Log error, return 0 on failure.

### REQ-FF-9: Decode Return Mapping
**Requirement text**: `Ok(n)→n`, `EndOfFile→0`, `Err→0`.

Behavior contract:
- GIVEN: A valid decoder with data
- WHEN: `rust_aifa_Decode()` is called
- THEN: Delegates to `dec.decode()`, maps result: Ok(n)→n, EndOfFile→0, Err(_)→0

### REQ-FF-13: Seek FFI
**Requirement text**: Delegates to `dec.seek()`, returns Ok value.

### REQ-FF-14: GetFrame FFI
**Requirement text**: Delegates to `dec.get_frame()`, returns 0.

### REQ-FF-15: Close FFI
**Requirement text**: Delegates to `dec.close()`.

Why it matters:
- GREEN phase — making all FFI tests pass
- Completes the FFI bridge, enabling C integration
- After this phase, both `aiff.rs` and `aiff_ffi.rs` are feature-complete

## Implementation Tasks

### Files to modify
- `rust/src/sound/aiff_ffi.rs`
  - marker: `@plan PLAN-20260225-AIFF-DECODER.P17`
  - marker: `@requirement REQ-FF-4, REQ-FF-5, REQ-FF-6, REQ-FF-7, REQ-FF-8, REQ-FF-9, REQ-FF-13, REQ-FF-14, REQ-FF-15`
   - Implement: `rust_aifa_Open()` — remove `todo!()`:
     1. Null checks for decoder and filename
     2. Convert filename to Rust string via CStr
     3. Log open attempt
     4. Get rust decoder from wrapper struct
     5. Do NOT call init_module()/init() — those are separate vtable calls made by the C framework (matching dukaud_ffi.rs pattern)
     6. Call read_uio_file(dir, filename) for AIFF data
     7. Call dec.open_from_bytes(&data, filename_str)
     8. On success: update base struct fields (frequency, format via format mapping, length, is_null, need_swap)
     9. On failure: log error, return 0
  - Implement: `rust_aifa_Decode()` — remove `todo!()`:
    1. Null checks for decoder, buf, bufsize
    2. Get rust decoder
    3. Create mutable slice from raw pointer + bufsize
    4. Call dec.decode(slice)
    5. Map: Ok(n) → n as c_int, EndOfFile → 0, Err(_) → 0
  - Verify all other functions already implemented from Phase 15

### Pseudocode traceability
- `rust_aifa_Open`: pseudocode lines 76–128
- `rust_aifa_Decode`: pseudocode lines 137–150

## Verification Commands

```bash
cd /Users/acoliver/projects/uqm/rust

# ALL tests pass
cargo test --lib --all-features

# Quality gates
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

## Structural Verification Checklist
- [ ] **ZERO `todo!()` in `aiff_ffi.rs`**
- [ ] All 12 vtable functions implemented
- [ ] Open: UIO file reading + format mapping + base field update
- [ ] Decode: buffer slice creation + result mapping
- [ ] All FFI tests pass

## Semantic Verification Checklist (Mandatory)
- [ ] Open: does NOT call init_module()/init() — those are separate vtable calls from C framework (REQ-FF-4 pattern match with dukaud_ffi.rs)
- [ ] Open: format mapping uses locked RUST_AIFA_FORMATS to convert AudioFormat → C format code
- [ ] Open: base struct fields updated (frequency, format, length, is_null=false, need_swap)
- [ ] Open: failure returns 0 and logs error
- [ ] Decode: EndOfFile maps to 0 (not negative)
- [ ] Decode: all errors map to 0 (never negative, per AIFF spec)
- [ ] All null pointer paths return safe defaults
- [ ] read_uio_file matches existing pattern (open, fstat, read loop, close)

## Deferred Implementation Detection (Mandatory)

```bash
cd /Users/acoliver/projects/uqm/rust && grep -n "todo!()\|unimplemented!()" src/sound/aiff_ffi.rs
# Should return NO results
grep -RIn "FIXME\|HACK\|placeholder\|for now\|will be implemented" src/sound/aiff_ffi.rs || echo "CLEAN"
```

## Success Criteria
- [ ] All tests pass (aiff + aiff_ffi)
- [ ] `cargo fmt` + `cargo clippy` pass
- [ ] **ZERO `todo!()` in both `aiff.rs` and `aiff_ffi.rs`**
- [ ] Both files feature-complete

## Failure Recovery
- rollback steps: `git checkout -- rust/src/sound/aiff_ffi.rs`
- blocking issues: If format mapping fails, check RUST_AIFA_FORMATS Mutex locking

## Phase Completion Marker
Create: `project-plans/audiorust/decoder/.completed/P17.md`

Contents:
- phase ID: PLAN-20260225-AIFF-DECODER.P17
- timestamp
- files changed: `rust/src/sound/aiff_ffi.rs`
- tests added/updated: None (GREEN phase)
- verification outputs
- semantic verification summary
- **MILESTONE**: Both `aiff.rs` and `aiff_ffi.rs` feature-complete
