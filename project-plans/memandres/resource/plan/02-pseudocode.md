# Phase 02: Pseudocode

## Phase ID
`PLAN-20260224-RES-SWAP.P02`

## Prerequisites
- Required: Phase 01a (Analysis Verification) completed

## Requirements Implemented (Expanded)

This phase produces algorithmic pseudocode for all three core components.
No production code is written.

## Implementation Tasks

### Files to create
- `analysis/pseudocode/component-001.md` — Property file parser
  - Exact parsing algorithm matching C `PropFile_from_string`
  - Key case preservation
  - Inline `#` comment handling
  - Prefix mechanism
  - Bare-key-at-EOF detection

- `analysis/pseudocode/component-002.md` — Config API
  - `process_resource_desc` (TYPE:path splitting)
  - `res_PutString`/`res_PutInteger`/`res_PutBoolean`/`res_PutColor`
  - `SaveResourceIndex` serialization

- `analysis/pseudocode/component-003.md` — Resource dispatch
  - `InstallResTypeVectors` (C function pointer storage)
  - `res_GetResource` (lazy loading via C loadFun)
  - `res_FreeResource` (refcount + C freeFun)
  - `res_DetachResource` (ownership transfer)
  - `res_Remove` (cleanup + C freeFun)
  - `LoadResourceFromPath` (UIO file I/O + C loadFileFun)

### Pseudocode requirements
- Numbered lines for traceability from implementation phases
- Validation points explicitly marked
- Error handling at each boundary
- Ordering constraints documented
- Side effects listed

## Verification Commands

```bash
# No code changes in this phase
for f in component-001.md component-002.md component-003.md; do
  test -s "project-plans/memandres/resource/analysis/pseudocode/$f" && echo "PASS: $f"
done
```

## Structural Verification Checklist
- [ ] All three component files created
- [ ] Each has numbered pseudocode lines
- [ ] Validation points marked
- [ ] Error handling included

## Semantic Verification Checklist
- [ ] Parser algorithm matches C `PropFile_from_string` character by character
- [ ] TYPE:path split matches C `newResourceDesc` (first `:` only)
- [ ] Config Put auto-creation matches C behavior
- [ ] SaveResourceIndex output format matches C
- [ ] res_GetResource lazy loading matches C flow exactly
- [ ] res_DetachResource guard conditions match C exactly
- [ ] res_FreeResource refcount logic matches C exactly

## Success Criteria
- [ ] Pseudocode is algorithmic (not prose)
- [ ] All C behaviors represented
- [ ] Implementation phases can reference specific line ranges

## Failure Recovery
- rollback: N/A (no code changes)

## Phase Completion Marker
Create: `project-plans/memandres/resource/.completed/P02.md`
