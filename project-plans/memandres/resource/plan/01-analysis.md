# Phase 01: Analysis

## Phase ID
`PLAN-20260224-RES-SWAP.P01`

## Prerequisites
- Required: Phase 00.5 (Preflight) completed
- All blocking issues resolved

## Requirements Implemented (Expanded)

This phase produces analysis artifacts, not code. It covers understanding of
all requirements (REQ-RES-001 through REQ-RES-R015).

## Implementation Tasks

### Files to create
- `project-plans/memandres/resource/analysis/domain-model.md`
  - Entity/state transition model
  - Resource lifecycle diagram
  - Two-phase loading explanation
  - Config vs game resource distinction
  - Path resolution through UIO
  - Error/edge case map
  - Integration touchpoints

### Analysis outputs required
1. **Entity Model**: ResourceIndexDesc, ResourceDesc, ResourceHandlers, ResourceData
2. **State Transitions**: Uninitialized → Initialized → Populated → Loaded → Freed/Detached
3. **Edge/Error Handling Map**: All warning/error conditions from C code
4. **Integration Touchpoints**: All 200+ C call sites categorized
5. **Old Code to Replace**: List of C functions to guard with USE_RUST_RESOURCE
6. **Existing Rust Code Assessment**: What to refactor, preserve, or deprecate

## Verification Commands

```bash
# No code changes in this phase
# Verify analysis file exists and is non-empty
test -s project-plans/memandres/resource/analysis/domain-model.md && echo "PASS"
```

## Structural Verification Checklist
- [ ] domain-model.md created with all required sections
- [ ] Entity model covers all C data structures
- [ ] State transitions match C lifecycle
- [ ] Error map covers all warning/error paths in C code
- [ ] Integration touchpoints list is complete

## Semantic Verification Checklist
- [ ] Domain model accurately reflects C source code behavior
- [ ] No C behaviors omitted from analysis
- [ ] Edge cases documented (NULL key, undefined key, double free, etc.)
- [ ] UIO integration boundary clearly defined

## Success Criteria
- [ ] All analysis sections present and accurate
- [ ] Analysis matches C source code (ground truth)

## Failure Recovery
- rollback: N/A (no code changes)
- blocking: If C source analysis reveals undocumented behavior, update spec

## Phase Completion Marker
Create: `project-plans/memandres/resource/.completed/P01.md`
