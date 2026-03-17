# Phase 10: Compatibility Audit Decision Points

## Phase ID
`PLAN-20260314-SUPERMELEE.P10`

## Prerequisites
- Required: Phase 09a completed and passed
- Team persistence, setup/menu flow, combatant-selection contract, and netplay boundary are described concretely enough to audit

## Purpose

Create the explicit compatibility-audit outputs required by the requirements and specification before FFI wiring and final signoff. This phase records decisions for areas that must not be asserted unconditionally without evidence:
- exact built-in team content,
- byte-for-byte saved-team format parity,
- exact UI/navigation/timing/audio parity,
- any remaining setup-time netplay readiness details that need boundary clarification.

## Implementation Tasks

### Files to create

- `project-plans/20260311/supermelee/compatibility-audit.md`
  - marker: `@plan PLAN-20260314-SUPERMELEE.P10`
  - Contents:
    - one section per audit-sensitive area
    - evidence reviewed (C source, fixtures, docs, external compatibility expectations)
    - decision classification:
      - `ExactParityRequired`
      - `SemanticCompatibilityRequired`
      - `NeedsFollowup`
    - implementation consequences for later phases/tests
    - explicit note that valid legacy `.mle` load interoperability remains mandatory regardless of exact-save decision

### Required audit sections
- built-in team catalog exactness versus semantic-catalog obligation
- saved-team write-format exactness versus semantic reloadability obligation
- setup UI/navigation/timing/audio exactness versus semantic setup usability obligation
- any clarified SuperMelee/netplay boundary details needed for start gating or selection synchronization tests

## Verification Commands

```bash
ls -la /Users/acoliver/projects/uqm/project-plans/20260311/supermelee/compatibility-audit.md
```

## Structural Verification Checklist
- [ ] Compatibility audit artifact exists
- [ ] Each audit-sensitive area has its own section and explicit decision
- [ ] Later implementation/verification consequences are written down rather than implied

## Semantic Verification Checklist (Mandatory)
- [ ] The audit does not downgrade mandatory legacy `.mle` load interoperability
- [ ] Built-in-team exactness is treated as an evidence-based decision, not an unsupported assumption
- [ ] Save-format byte parity is treated as conditional unless proven mandatory
- [ ] UI/navigation/timing/audio parity is treated as conditional unless proven mandatory
- [ ] Any remaining netplay-boundary clarification is captured without expanding scope into transport/protocol internals

## Success Criteria
- [ ] The plan now contains the missing compatibility-audit implementation phase identified in review
- [ ] Later phases can key their assertions to explicit audit decisions instead of broad assumptions

## Phase Completion Marker
Create: `project-plans/20260311/supermelee/.completed/P10.md`
