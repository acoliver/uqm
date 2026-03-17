# Phase 01a: Analysis Verification

## Phase ID
`PLAN-20260314-UIO.P01a`

## Prerequisites
- Required: Phase 01 completed
- Gap analysis document exists at `project-plans/20260311/uio/plan/01-analysis.md`

## Structural Verification Checklist
- [ ] All REQ-UIO-* IDs from `requirements.md` are mapped to at least one GAP-* entry or explicitly documented as already satisfied
- [ ] All GAP-* entries reference specific file paths and line numbers in current code where practical
- [ ] Integration touchpoints table is complete — all callers from `initialstate.md` are listed
- [ ] Error handling map covers all error classes from specification §6.2.1
- [ ] "Old code to replace/remove" table maps to specific phases
- [ ] Exported-surface audit exists for stubbed/unsupported APIs and FFI-visible structs

## Semantic Verification Checklist
- [ ] Every Tier 1 gap has a consumer dependency or explicit engine-critical rationale documented
- [ ] Every gap entry explains both what is wrong and how to fix it
- [ ] No requirements are missing from the gap analysis (cross-check against requirements.md)
- [ ] Integration touchpoints cover both C→Rust and Rust→Rust-FFI directions
- [ ] The distinction between Tier 1, Tier 2, and Tier 3 matches requirements.md classifications
- [ ] Boundary requirements are recorded as constraints, not accidentally converted into startup-policy ownership by UIO

## Requirement Coverage Check

### Engine-critical requirements coverage
- [ ] REQ-UIO-INIT-001, -003 → already implemented baseline + GAP-12 / GAP-31 review
- [ ] REQ-UIO-REPO-001, -002, -003 → existing implementation + GAP-12 lifecycle audit
- [ ] REQ-UIO-MOUNT-001 through -005, -009 → GAP-01, GAP-07, GAP-12
- [ ] REQ-UIO-PATH-001 through -004 → partial existing behavior + GAP-07 / GAP-24
- [ ] REQ-UIO-DIR-001 through -004 → existing implementation + GAP-12 lifecycle audit
- [ ] REQ-UIO-LIST-001, -002, -003, -011, -012, -013, -016, -017 → GAP-16, GAP-17
- [ ] REQ-UIO-FILE-001 through -011, -014 → GAP-02, GAP-05, GAP-24
- [ ] REQ-UIO-STREAM-001 through -008, -010 through -016 → GAP-03, GAP-04, GAP-06, GAP-08, GAP-09, GAP-29, GAP-30
- [ ] REQ-UIO-ARCHIVE-001 through ARCHIVE-ACCEPT → GAP-01, GAP-02
- [ ] REQ-UIO-MEM-001 through -004 → GAP-06, GAP-31
- [ ] REQ-UIO-ERR-001 through -006, -008, -010, -011 → GAP-03, GAP-04, GAP-05, GAP-09, GAP-10
- [ ] REQ-UIO-CONC-001, -002 → GAP-11
- [ ] REQ-UIO-LIFE-001, -002 → GAP-12
- [ ] REQ-UIO-INT-001 through -005 → GAP-04, GAP-10, GAP-13, GAP-25
- [ ] REQ-UIO-FFI-001 through -004 → GAP-09, GAP-10, GAP-17
- [ ] REQ-UIO-BOUND-001 through -003 → GAP-13

### Compatibility-complete requirements coverage
- [ ] REQ-UIO-STREAM-009, -014, -017, -018, -019 → GAP-14, GAP-22, GAP-26
- [ ] REQ-UIO-LIST-004 through -010, -015 → GAP-15, GAP-16, GAP-17
- [ ] REQ-UIO-FILE-012, -013, -015, -016 → GAP-18
- [ ] REQ-UIO-MOUNT-006, -007, -008, -010 → GAP-21
- [ ] REQ-UIO-FB-001 through -007 → GAP-19
- [ ] REQ-UIO-STDIO-001 through -006 → GAP-20, GAP-23, GAP-24, GAP-31
- [ ] REQ-UIO-PATH-005, -006 → GAP-24
- [ ] REQ-UIO-ARCHIVE-009, -010, -011 → GAP-18, GAP-20, GAP-24
- [ ] REQ-UIO-ERR-007, -009, -012 → GAP-19, GAP-20, GAP-22, GAP-23, GAP-25
- [ ] REQ-UIO-CONC-003, -004 → GAP-11, GAP-12, GAP-21
- [ ] REQ-UIO-LIFE-003, -004, -005 → GAP-12, GAP-21
- [ ] REQ-UIO-INT-006, -007 → GAP-13, GAP-28
- [ ] REQ-UIO-MEM-005, -007 → GAP-17, GAP-31

### Quality / cleanup requirements coverage
- [ ] REQ-UIO-INIT-002 → Phase 11 review + lifecycle audit
- [ ] REQ-UIO-CONC-005 → GAP-11, GAP-31
- [ ] REQ-UIO-MEM-006 → GAP-31
- [ ] REQ-UIO-LOG-001, -002 → GAP-28

## Gate Decision
- [ ] PASS: all requirements mapped, proceed to Phase 02
- [ ] FAIL: missing requirements identified — update analysis before proceeding
