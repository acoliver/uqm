# Phase 02: Pseudocode

## Phase ID
`PLAN-20260224-STATE-SWAP.P02`

## Prerequisites
- Required: Phase P01a (Analysis Verification) completed
- Expected files: `analysis/domain-model.md`

## Requirements Implemented (Expanded)

This phase produces pseudocode artifacts — no production code changes.

### REQ-SF-001: Seek-Past-End Allowed
Pseudocode for seek function without upper clamp (component-001 lines 51–60).

### REQ-SF-002: Write-After-Seek-Past-End Extends Buffer
Pseudocode for write with buffer growth (component-001 lines 75–86).

### REQ-SF-003: Read-After-Seek-Past-End Returns EOF
Pseudocode for read with physical size check (component-001 lines 63–72).

### REQ-SF-004: Copy Deadlock Prevention
Pseudocode for single-lock copy pattern (component-001 lines 119–127).

### REQ-SF-005: Separate Used and Physical Size Tracking
Pseudocode for open pre-allocation, length returning used (component-001 lines 89–107).

### REQ-SF-006: C Redirect Correctness
Pseudocode for state.c redirect functions (component-001 lines 1–48).

## Implementation Tasks

### Files to create
- `analysis/pseudocode/component-001.md` — State file I/O pseudocode
  - marker: `@plan PLAN-20260224-STATE-SWAP.P02`
  - Covers: state.c redirects, seek fix, read/write/length updates, deadlock fix

### Pseudocode requirements
- Numbered lines for traceability
- Validation points at function boundaries
- Error handling for allocation failure, invalid index, negative seek
- Integration boundaries (C → Rust FFI calls)
- Side effects (buffer growth, used update, open_count change)

## Verification Commands

```bash
# Pseudocode is documentation-only — verify file exists
ls -la project-plans/memandres/state/analysis/pseudocode/component-001.md
```

## Structural Verification Checklist
- [ ] `analysis/pseudocode/component-001.md` created
- [ ] All 7 C redirect functions have pseudocode (lines 1–48)
- [ ] Seek fix has pseudocode (lines 51–60)
- [ ] Read update has pseudocode (lines 63–72)
- [ ] Write update has pseudocode (lines 75–86)
- [ ] Length update has pseudocode (line 89–90)
- [ ] Open update has pseudocode (lines 93–107)
- [ ] Delete has pseudocode (lines 110–116)
- [ ] Copy deadlock fix has pseudocode (lines 119–127)
- [ ] All pseudocode lines are numbered

## Semantic Verification Checklist (Mandatory)
- [ ] Seek pseudocode has NO upper clamp (only negative → 0)
- [ ] Read pseudocode checks `data.len()` (physical), not `used`
- [ ] Write pseudocode uses 1.5x growth strategy matching C
- [ ] Length pseudocode returns `used`, not `data.len()`
- [ ] Copy pseudocode acquires mutex exactly once
- [ ] C redirect pseudocode computes `file_index = fp - state_files`
- [ ] OpenStateFile redirect returns `&state_files[stateFile]` (not a Rust pointer)

## Success Criteria
- [ ] All pseudocode covers all requirements
- [ ] Line numbers allow traceability from implementation phases

## Phase Completion Marker
Create: `project-plans/memandres/state/.completed/P02.md`

Contents:
- phase ID: P02
- files created: `analysis/pseudocode/component-001.md`
- pseudocode covers: 7 redirects, seek fix, read/write/length updates, copy deadlock fix
