# Phase 12: Requirement Traceability Matrix

## Phase ID
`PLAN-20260314-SUPERMELEE.P12`

## Prerequisites
- Required: Phase 11a completed and passed
- All implementation and integration phases complete enough to map every requirement statement to concrete coverage

## Purpose

Create a requirement matrix keyed to each requirement statement in `requirements.md`, not broad topic labels. This phase closes the traceability gaps identified in review by assigning every requirement statement to:
1. an owning implementation phase,
2. one or more concrete verification artifacts, and
3. any compatibility-audit dependency when stronger obligations are conditional.

## Implementation Tasks

### Files to create

- `project-plans/20260311/supermelee/requirement-traceability-matrix.md`
  - marker: `@plan PLAN-20260314-SUPERMELEE.P12`
  - Contents:
    - one row per requirement statement from `requirements.md`
    - stable requirement IDs normalized for this plan, for example:
      - `SM-ENTRY-01`
      - `SM-TEAM-01`
      - `SM-MENU-01`
      - `SM-BATTLE-01`
      - `SM-BROWSE-01`
      - `SM-PERSIST-01`
      - `SM-NET-01`
      - `SM-ERR-01`
      - `SM-COMPAT-01`
    - columns:
      - requirement ID
      - exact requirement text
      - implementation phase(s)
      - automated verification item(s)
      - manual verification item(s), if needed
      - compatibility-audit dependency (`none`, `built-in exactness`, `save-format exactness`, `UI/timing exactness`)
      - status

### Required traceability rows (non-exhaustive examples; final matrix must include all statements)
- successful loading of a valid built-in team into the active side
- successful loading of a valid saved team file into the active side
- confirm in fleet-edit applies the selection
- cancel in fleet-edit leaves team unchanged
- local-only behavior when netplay unsupported/disabled
- setup-time synchronization events for ship-slot changes, team-name changes, and whole-team bootstrap
- start gating on connection/readiness/confirmation preconditions
- exposure of local battle-time selection outcomes
- acceptance/rejection semantics for remote selection updates
- battle-facing handoff preserving battle-ready combatant objects rather than bare ship IDs
- fallback initialization from built-in team offering
- setup-state persistence and transient-network sanitization

## Verification Commands

```bash
# Verify the matrix artifact exists and can be reviewed as plain text.
ls -la /Users/acoliver/projects/uqm/project-plans/20260311/supermelee/requirement-traceability-matrix.md
```

## Structural Verification Checklist
- [ ] Traceability matrix file exists
- [ ] Every requirement statement from `requirements.md` appears exactly once with a stable ID
- [ ] Every matrix row maps to at least one concrete implementation phase
- [ ] Every matrix row maps to at least one concrete verification item

## Semantic Verification Checklist (Mandatory)
- [ ] No requirement statement is represented only by a broad topic umbrella
- [ ] Netplay boundary requirements have explicit rows for setup sync, start gating, local outcome exposure, and remote selection acceptance/rejection semantics
- [ ] Built-in team loading and saved-team loading have separate rows
- [ ] Confirm/cancel fleet-edit semantics have separate rows
- [ ] Battle-facing handoff compatibility has an explicit row documenting the non-weakened contract
- [ ] Compatibility-sensitive exactness obligations are marked as audit-gated where appropriate rather than claimed unconditionally

## Success Criteria
- [ ] Full statement-level requirement coverage exists
- [ ] The matrix can drive final end-to-end verification without ambiguity
- [ ] The matrix resolves the review finding about overstated P15 traceability coverage

## Failure Recovery
- rollback: `git checkout -- /Users/acoliver/projects/uqm/project-plans/20260311/supermelee/requirement-traceability-matrix.md`

## Phase Completion Marker
Create: `project-plans/20260311/supermelee/.completed/P12.md`
