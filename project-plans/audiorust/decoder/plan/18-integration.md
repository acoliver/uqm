# Phase 18: C Integration

## Phase ID
`PLAN-20260225-AIFF-DECODER.P18`

## Prerequisites
- Required: Phase 17 completed (both Rust files feature-complete)
- Expected files: `rust/src/sound/aiff.rs`, `rust/src/sound/aiff_ffi.rs` — both complete

## Requirements Implemented (Expanded)

### REQ-FF-2: Vtable Registration in C
**Requirement text**: The `rust_aifa_DecoderVtbl` must be registered in `decoder.c`'s `sd_decoders[]` table with `USE_RUST_AIFF` conditional, matching the pattern used by `USE_RUST_DUKAUD`, `USE_RUST_WAV`, etc.

Behavior contract:
- GIVEN: `USE_RUST_AIFF` is defined in `config_unix.h`
- WHEN: `decoder.c` is compiled
- THEN: The `"aif"` extension entry in `sd_decoders[]` uses `rust_aifa_DecoderVtbl` instead of `aifa_DecoderVtbl`
- GIVEN: `USE_RUST_AIFF` is NOT defined
- WHEN: `decoder.c` is compiled
- THEN: The `"aif"` extension entry uses the original C `aifa_DecoderVtbl`

### REQ-FF-7: End-to-End Integration
**Requirement text**: The complete path from C audio mixer → vtable → FFI → Rust decoder must work.

Behavior contract:
- GIVEN: A game running with `USE_RUST_AIFF` enabled
- WHEN: An `.aif` audio file is loaded
- THEN: The Rust AIFF decoder opens, decodes, and plays the audio correctly

Why it matters:
- This is the final integration step that makes the Rust decoder reachable
- Without this, all prior work is unreachable from the application

## Implementation Tasks

### Files to create
- `sc2/src/libs/sound/decoders/rust_aiff.h`
  - marker: `@plan PLAN-20260225-AIFF-DECODER.P18`
  - marker: `@requirement REQ-FF-2`
  - Content (following `rust_dukaud.h` pattern):
    ```c
    /*
     *  Rust AIFF decoder header
     *
     *  Provides extern declaration for the Rust-implemented AIFF decoder
     *  vtable. When USE_RUST_AIFF is defined, this decoder is used instead of
     *  the C aiffaud implementation.
     */

    #ifndef LIBS_SOUND_DECODERS_RUST_AIFF_H_
    #define LIBS_SOUND_DECODERS_RUST_AIFF_H_

    #include "decoder.h"

    #ifdef USE_RUST_AIFF

    /*
     * Rust AIFF decoder vtable
     * Defined in rust/src/sound/aiff_ffi.rs and exported via staticlib
     */
    extern TFB_SoundDecoderFuncs rust_aifa_DecoderVtbl;

    #endif /* USE_RUST_AIFF */

    #endif /* LIBS_SOUND_DECODERS_RUST_AIFF_H_ */
    ```

### Files to modify
- `sc2/src/libs/sound/decoders/decoder.c`
  - marker: `@plan PLAN-20260225-AIFF-DECODER.P18`
  - marker: `@requirement REQ-FF-2`
  - Add include (in the USE_RUST_* include block):
    ```c
    #ifdef USE_RUST_AIFF
    #	include "rust_aiff.h"
    #endif  /* USE_RUST_AIFF */
    ```
  - Modify `sd_decoders[]` entry for `"aif"`:
    ```c
    #ifdef USE_RUST_AIFF
    	/* Use Rust AIFF decoder (pure Rust port of aiffaud.c) */
    	{true,  true,  "aif", &rust_aifa_DecoderVtbl},
    #else
    	{true,  true,  "aif", &aifa_DecoderVtbl},
    #endif  /* USE_RUST_AIFF */
    ```

- `sc2/src/config_unix.h.in`
  - marker: `@plan PLAN-20260225-AIFF-DECODER.P18`
  - Add after the `USE_RUST_WAV` entry:
    ```
    /* Defined if using Rust AIFF decoder */
    @SYMBOL_USE_RUST_AIFF_DEF@
    ```

- `sc2/build.vars.in`
  - marker: `@plan PLAN-20260225-AIFF-DECODER.P18`
  - Add `USE_RUST_AIFF` and `SYMBOL_USE_RUST_AIFF_DEF` variables following the existing pattern:
    - In the `uqm_USE_RUST_*` block: `uqm_USE_RUST_AIFF='@USE_RUST_AIFF@'`
    - In the `USE_RUST_*` block: `USE_RUST_AIFF='@USE_RUST_AIFF@'`
    - In the export line: add `uqm_USE_RUST_AIFF` and `USE_RUST_AIFF`
    - In the `SYMBOL_*` blocks: `uqm_SYMBOL_USE_RUST_AIFF_DEF='@SYMBOL_USE_RUST_AIFF_DEF@'` and `SYMBOL_USE_RUST_AIFF_DEF='@SYMBOL_USE_RUST_AIFF_DEF@'`
    - In the symbol export line: add both

### Pseudocode traceability
- C integration: spec Part 5 (Vtable Registration)

## Verification Commands

```bash
# Rust-side verification
cd /Users/acoliver/projects/uqm/rust && cargo test --lib --all-features
cd /Users/acoliver/projects/uqm/rust && cargo fmt --all --check
cd /Users/acoliver/projects/uqm/rust && cargo clippy --workspace --all-targets --all-features -- -D warnings

# C-side verification
cd /Users/acoliver/projects/uqm/sc2 && ./build.sh uqm

# Verify C integration files
grep "USE_RUST_AIFF" sc2/src/libs/sound/decoders/decoder.c
grep "USE_RUST_AIFF" sc2/src/config_unix.h.in
grep "USE_RUST_AIFF" sc2/build.vars.in
grep "rust_aifa_DecoderVtbl" sc2/src/libs/sound/decoders/rust_aiff.h
```

## Structural Verification Checklist
- [ ] `sc2/src/libs/sound/decoders/rust_aiff.h` created
- [ ] `decoder.c` updated with `#ifdef USE_RUST_AIFF` include and sd_decoders entry
- [ ] `config_unix.h.in` updated with `@SYMBOL_USE_RUST_AIFF_DEF@`
- [ ] `build.vars.in` updated with USE_RUST_AIFF variables
- [ ] Rust side: all tests pass
- [ ] C side: `./build.sh uqm` succeeds (without USE_RUST_AIFF first, to verify no regressions)

## Semantic Verification Checklist (Mandatory)
- [ ] With `USE_RUST_AIFF` undefined: C build uses original `aifa_DecoderVtbl` — no regression
- [ ] With `USE_RUST_AIFF` defined: C build links against `rust_aifa_DecoderVtbl`
- [ ] The `"aif"` extension in `sd_decoders[]` is correctly conditionally compiled
- [ ] The boolean values in the Rust entry `{true, true, "aif", ...}` match the original C entry exactly (verified: line 173 of `decoder.c` uses `{true, true, "aif", &aifa_DecoderVtbl}`)
- [ ] `rust_aiff.h` follows exact pattern of `rust_dukaud.h`
- [ ] `decoder.c` include placement matches other USE_RUST_* includes
- [ ] `decoder.c` sd_decoders placement is in the correct position (where `"aif"` currently is)
- [ ] `build.vars.in` variables follow exact pattern of existing USE_RUST_* variables
- [ ] Integration path: C mixer → decoder.c → vtable → aiff_ffi.rs → aiff.rs → decoded audio

## Deferred Implementation Detection (Mandatory)

```bash
# Integration phase: NO todo/placeholder anywhere
cd /Users/acoliver/projects/uqm/rust && grep -RIn "todo!()\|FIXME\|HACK\|placeholder" src/sound/aiff.rs src/sound/aiff_ffi.rs || echo "CLEAN"
```

## Success Criteria
- [ ] All Rust tests pass
- [ ] C build succeeds without USE_RUST_AIFF (no regression)
- [ ] `rust_aiff.h` header created
- [ ] `decoder.c` conditionally uses Rust vtable
- [ ] `config_unix.h.in` and `build.vars.in` updated
- [ ] End-to-end integration path is complete

## Failure Recovery
- **C-side rollback** (restores all C integration changes):
  ```bash
  git checkout -- sc2/src/libs/sound/decoders/decoder.c
  git checkout -- sc2/src/config_unix.h.in
  git checkout -- sc2/build.vars.in
  rm -f sc2/src/libs/sound/decoders/rust_aiff.h
  ```
- **Verify rollback succeeded**: `cd sc2 && ./build.sh uqm` — C build must succeed identically to before this phase
- **Rust-side rollback** (if Rust files were also modified — normally not in this phase):
  ```bash
  git checkout -- rust/src/sound/mod.rs
  ```
- blocking issues:
  - If C build fails with undefined symbol `rust_aifa_DecoderVtbl`: check `#[no_mangle]` on vtable in `aiff_ffi.rs` and that the Rust staticlib is linked
  - If `build.vars.in` syntax errors: compare closely with existing `USE_RUST_DUKAUD` / `USE_RUST_WAV` entries for exact whitespace and quoting
  - If `config_unix.h.in` breaks: verify the `@SYMBOL_USE_RUST_AIFF_DEF@` placeholder is on its own line, same as other `@SYMBOL_*_DEF@` entries

## Phase Completion Marker
Create: `project-plans/audiorust/decoder/.completed/P18.md`

Contents:
- phase ID: PLAN-20260225-AIFF-DECODER.P18
- timestamp
- files changed:
  - `sc2/src/libs/sound/decoders/rust_aiff.h` (created)
  - `sc2/src/libs/sound/decoders/decoder.c` (modified)
  - `sc2/src/config_unix.h.in` (modified)
  - `sc2/build.vars.in` (modified)
- tests added/updated: None (integration phase)
- verification outputs
- semantic verification summary
- **MILESTONE**: AIFF decoder Rust port integration complete
