# Phase 02: Pseudocode

## Phase ID
`PLAN-20260225-AIFF-DECODER.P02`

## Prerequisites
- Required: Phase 01 completed (analysis artifacts exist)
- Expected files from previous phase: `analysis/domain-model.md`

## Requirements Implemented (Expanded)

### All Requirements — Algorithmic Design
**Requirement text**: Produce numbered algorithmic pseudocode for both `aiff.rs` and `aiff_ffi.rs` covering all logic paths.

Behavior contract:
- GIVEN: The domain model and AIFF spec requirements
- WHEN: Pseudocode phase is executed
- THEN: Two pseudocode files are produced with numbered algorithmic steps covering every function, every validation, every error path, and every integration boundary

Why it matters:
- Provides a line-addressable reference for implementation phases
- Forces explicit handling of all edge cases before coding
- Enables implementation-phase traceability (`@pseudocode lines X-Y`)

## Implementation Tasks

### Files to create
- `analysis/pseudocode/aiff.md` — Algorithmic pseudocode for `aiff.rs`
  - marker: `@plan PLAN-20260225-AIFF-DECODER.P02`
  - Contents: Numbered steps for constructor, byte readers, f80 parsing, chunk parsing, open_from_bytes, decode_pcm, decode_sdx2, seek, close, all trait methods
  - Must include: validation points, error handling, ordering constraints, integration boundaries, side effects

- `analysis/pseudocode/aiff_ffi.md` — Algorithmic pseudocode for `aiff_ffi.rs`
  - marker: `@plan PLAN-20260225-AIFF-DECODER.P02`
  - Contents: Numbered steps for all 12 vtable functions, UIO file reading, module-level state, vtable export
  - Must include: null safety checks, Box lifecycle, format mapping, error-to-return-value conversion

### Files to modify
- None (design-only phase)

## Verification Commands

```bash
# Verify pseudocode files exist and are non-empty
test -s project-plans/audiorust/decoder/analysis/pseudocode/aiff.md && echo "OK aiff" || echo "MISSING aiff"
test -s project-plans/audiorust/decoder/analysis/pseudocode/aiff_ffi.md && echo "OK ffi" || echo "MISSING ffi"

# Verify numbered lines are present
grep -cE "^[0-9 ]+:" project-plans/audiorust/decoder/analysis/pseudocode/aiff.md
grep -cE "^[0-9 ]+:" project-plans/audiorust/decoder/analysis/pseudocode/aiff_ffi.md
```

## Structural Verification Checklist
- [ ] `analysis/pseudocode/aiff.md` created with numbered algorithmic steps
- [ ] `analysis/pseudocode/aiff_ffi.md` created with numbered algorithmic steps
- [ ] `aiff.md` covers: new(), byte readers, read_be_f80(), chunk parsing, open_from_bytes(), decode_pcm(), decode_sdx2(), seek(), close(), all trait methods
- [ ] `aiff_ffi.md` covers: all 12 vtable functions, read_uio_file(), vtable static definition

## Semantic Verification Checklist (Mandatory)
- [ ] Every REQ-FP-* requirement has corresponding pseudocode lines in `aiff.md`
- [ ] Every REQ-SV-* requirement has corresponding pseudocode lines in `aiff.md`
- [ ] Every REQ-CH-* requirement has corresponding pseudocode lines in `aiff.md`
- [ ] Every REQ-DP-* requirement has corresponding pseudocode lines in `aiff.md`
- [ ] Every REQ-DS-* requirement has corresponding pseudocode lines in `aiff.md`
- [ ] Every REQ-SK-* requirement has corresponding pseudocode lines in `aiff.md`
- [ ] Every REQ-EH-* requirement has corresponding pseudocode lines in `aiff.md`
- [ ] Every REQ-LF-* requirement has corresponding pseudocode lines in `aiff.md`
- [ ] Every REQ-FF-* requirement has corresponding pseudocode lines in `aiff_ffi.md`
- [ ] Error handling paths are explicitly shown (not "handle error" but specific Err variants)
- [ ] SDX2 algorithm matches spec: v = (sample * abs(sample)) << 1, odd-bit delta, clamp, predictor store

## Deferred Implementation Detection (Mandatory)

```bash
# N/A for pseudocode phase — no implementation code
echo "Pseudocode phase: no deferred implementation detection needed"
```

## Success Criteria
- [ ] Both pseudocode files exist with numbered algorithmic steps
- [ ] All 9 requirement categories have traceable pseudocode
- [ ] Validation points, error handling, side effects are explicit

## Failure Recovery
- rollback steps: `rm analysis/pseudocode/aiff.md analysis/pseudocode/aiff_ffi.md`
- blocking issues: If spec requirements are ambiguous, clarify in specification.md first

## Phase Completion Marker
Create: `project-plans/audiorust/decoder/.completed/P02.md`

Contents:
- phase ID: PLAN-20260225-AIFF-DECODER.P02
- timestamp
- files changed: `analysis/pseudocode/aiff.md`, `analysis/pseudocode/aiff_ffi.md` (created)
- tests added/updated: None (design phase)
- verification outputs
- semantic verification summary
