# Phase 02: Pseudocode

## Phase ID
`PLAN-20260223-GFX-VTABLE-FIX.P02`

## Prerequisites
- Required: Phase P01a (Analysis Verification) completed
- Expected artifacts: `analysis/domain-model.md` verified

## Requirements Implemented (Expanded)

This is a pseudocode phase — no requirements are implemented directly.
Algorithmic pseudocode is produced for all components that will be
implemented in subsequent phases.

## Implementation Tasks

### Files to create
- `analysis/pseudocode/component-001-init.md` — Init & teardown algorithms
  - marker: `@plan PLAN-20260223-GFX-VTABLE-FIX.P02`
- `analysis/pseudocode/component-002-preprocess.md` — Preprocess algorithm
  - marker: `@plan PLAN-20260223-GFX-VTABLE-FIX.P02`
- `analysis/pseudocode/component-003-screen-layer.md` — ScreenLayer unscaled
  - marker: `@plan PLAN-20260223-GFX-VTABLE-FIX.P02`
- `analysis/pseudocode/component-004-screen-layer-scaled.md` — ScreenLayer scaled
  - marker: `@plan PLAN-20260223-GFX-VTABLE-FIX.P02`
- `analysis/pseudocode/component-005-color-layer.md` — ColorLayer algorithm
  - marker: `@plan PLAN-20260223-GFX-VTABLE-FIX.P02`
- `analysis/pseudocode/component-006-postprocess.md` — Postprocess + UploadTransitionScreen
  - marker: `@plan PLAN-20260223-GFX-VTABLE-FIX.P02`
- `analysis/pseudocode/component-007-surface-access.md` — Surface access & aux
  - marker: `@plan PLAN-20260223-GFX-VTABLE-FIX.P02`

### Files to modify
- None (pseudocode phase only)

## Verification Commands

```bash
# Verify all pseudocode files exist
for i in 001 002 003 004 005 006 007; do
  test -f "project-plans/gfx/analysis/pseudocode/component-${i}"*.md && echo "PASS: $i" || echo "FAIL: $i"
done
```

## Structural Verification Checklist
- [ ] All 7 pseudocode files created
- [ ] Each file uses numbered algorithmic format (not prose)
- [ ] Each file includes validation points
- [ ] Each file includes error handling
- [ ] Each file includes ordering constraints
- [ ] Each file includes integration boundaries
- [ ] Each file includes side effects

## Semantic Verification Checklist (Mandatory)
- [ ] Component-003 (ScreenLayer) covers all REQ-SCR-* requirements
- [ ] Component-004 (Scaling) covers all REQ-SCALE-* requirements
- [ ] Component-005 (ColorLayer) covers all REQ-CLR-* requirements
- [ ] Component-006 (Postprocess) satisfies REQ-INV-010 (no upload)
- [ ] Pseudocode line numbers can be referenced by implementation phases
- [ ] All `// SAFETY:` documentation points are identified
- [ ] convert_rect helper is defined and reusable

## Deferred Implementation Detection (Mandatory)

```bash
# Not applicable for pseudocode phase
echo "N/A for pseudocode phase"
```

## Success Criteria
- [ ] All 7 pseudocode components are complete
- [ ] Numbered algorithmic format throughout
- [ ] Line ranges are referenceable

## Failure Recovery
- rollback: `rm -rf project-plans/gfx/analysis/pseudocode/`
- blocking issues: incomplete requirement coverage in pseudocode

## Phase Completion Marker
Create: `project-plans/gfx/.completed/P02.md`

Contents:
- phase ID: P02
- timestamp
- files created: 7 pseudocode component files
- verification outputs
- semantic verification summary
