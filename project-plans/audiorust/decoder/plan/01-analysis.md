# Phase 01: Analysis

## Phase ID
`PLAN-20260225-AIFF-DECODER.P01`

## Prerequisites
- Required: Preflight verification (P00a) completed and PASS
- Expected artifacts: None (first substantive phase)

## Requirements Implemented (Expanded)

### All Requirements — Domain Analysis
**Requirement text**: Produce domain model, entity/state analysis, edge case map, and integration touchpoints for the AIFF decoder.

Behavior contract:
- GIVEN: The AIFF decoder Rust spec (`rust-decoder.md`) and existing decoder patterns (`wav.rs`, `dukaud.rs`)
- WHEN: Analysis phase is executed
- THEN: A `domain-model.md` is produced containing entity definitions, state transitions, error handling map, integration touchpoints, and data flow diagrams

Why it matters:
- Ensures all requirements are mapped before code is written
- Identifies edge cases and integration risks early
- Provides a reference for all subsequent implementation phases

## Implementation Tasks

### Files to create
- `project-plans/audiorust/decoder/analysis/domain-model.md` — entity/state analysis
  - marker: `@plan PLAN-20260225-AIFF-DECODER.P01`
  - Contents: AiffDecoder lifecycle states, CompressionType/CommonChunk/SoundDataHeader value objects, error handling map, integration touchpoints (Rust-side and C-side), data flow diagram

### Files to modify
- None (analysis-only phase)

## Verification Commands

```bash
# Verify analysis document exists and is non-empty
test -s project-plans/audiorust/decoder/analysis/domain-model.md && echo "OK" || echo "MISSING"

# Verify all requirement categories are referenced
grep -c "REQ-FP\|REQ-SV\|REQ-CH\|REQ-DP\|REQ-DS\|REQ-SK\|REQ-EH\|REQ-LF\|REQ-FF" project-plans/audiorust/decoder/analysis/domain-model.md
```

## Structural Verification Checklist
- [ ] `domain-model.md` created in `analysis/` directory
- [ ] Entity definitions for AiffDecoder, CompressionType, CommonChunk, SoundDataHeader, ChunkHeader
- [ ] State transition diagram present
- [ ] Error handling map covers all DecodeError variants used
- [ ] Integration touchpoints listed for both Rust and C sides

## Semantic Verification Checklist (Mandatory)
- [ ] All 9 requirement categories (FP, SV, CH, DP, DS, SK, EH, LF, FF) represented in analysis
- [ ] State transitions cover full lifecycle: new → init_module → init → open → decode/seek → close → term
- [ ] Error map includes all validation failures from `open_from_bytes()`
- [ ] Integration touchpoints include `mod.rs`, `decoder.c`, `config_unix.h.in`, `build.vars.in`
- [ ] Data flow diagram shows C→FFI→Rust→FFI→C path

## Deferred Implementation Detection (Mandatory)

```bash
# N/A for analysis phase — no implementation code
echo "Analysis phase: no deferred implementation detection needed"
```

## Success Criteria
- [ ] domain-model.md exists with substantive content
- [ ] All requirement categories represented
- [ ] State diagram complete
- [ ] Integration points explicit

## Failure Recovery
- rollback steps: `rm project-plans/audiorust/decoder/analysis/domain-model.md`
- blocking issues: If SoundDecoder trait or FFI types don't match spec, revise spec first

## Phase Completion Marker
Create: `project-plans/audiorust/decoder/.completed/P01.md`

Contents:
- phase ID: PLAN-20260225-AIFF-DECODER.P01
- timestamp
- files changed: `analysis/domain-model.md` (created)
- tests added/updated: None (analysis phase)
- verification outputs
- semantic verification summary
