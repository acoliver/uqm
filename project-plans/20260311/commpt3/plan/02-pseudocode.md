# Phase 02: Pseudocode

## Phase ID
`PLAN-20260325-COMMPT3.P02`

## Prerequisites
- Required: Phase P01a (Analysis Verification) completed
- Expected artifacts: `analysis/domain-model.md` verified

## Requirements Implemented (Expanded)

This is a pseudocode phase — no requirements are implemented directly.
Algorithmic pseudocode is produced for all five fix categories that will
be implemented in subsequent phases.

## Implementation Tasks

### Files to create
- `analysis/pseudocode/001-colormap-music-bridges.md` — Colormap/music bridge C + Rust changes
  - marker: `@plan PLAN-20260325-COMMPT3.P02`
  - covers: REQ-CM-001..003, REQ-MU-001..003
- `analysis/pseudocode/002-subtitle-display-fix.md` — Subtitle routing fix in comm.c + rust_comm.c
  - marker: `@plan PLAN-20260325-COMMPT3.P02`
  - covers: REQ-SD-001..005
- `analysis/pseudocode/003-do-communication-rewrite.md` — DoCommunication lock discipline + state machine
  - marker: `@plan PLAN-20260325-COMMPT3.P02`
  - covers: REQ-RL-001..004, REQ-DC-001..005
- `analysis/pseudocode/004-summary-guard-stale-markers.md` — Summary cfg(test) guard + marker sweep
  - marker: `@plan PLAN-20260325-COMMPT3.P02`
  - covers: REQ-CS-002..003, REQ-SM-001..002
- `analysis/pseudocode/005-end-to-end-integration.md` — Full encounter flow verification pseudocode
  - marker: `@plan PLAN-20260325-COMMPT3.P02`
  - covers: REQ-E2E-001..007, REQ-TS-001..004

### Files to modify
- None (pseudocode phase only)

## Verification Commands

```bash
# Verify all pseudocode files exist
for i in 001 002 003 004 005; do
  test -f "project-plans/20260311/commpt3/analysis/pseudocode/${i}"*.md && echo "PASS: $i" || echo "FAIL: $i"
done

# Verify numbered format used (lines starting with digits)
for f in project-plans/20260311/commpt3/analysis/pseudocode/*.md; do
  count=$(grep -cE '^ *[0-9]+:' "$f")
  echo "$(basename $f): $count numbered lines"
  if [ "$count" -lt 5 ]; then echo "FAIL: insufficient numbered pseudocode"; fi
done
```

## Structural Verification Checklist
- [ ] All 5 pseudocode files created
- [ ] Each file uses numbered algorithmic format (not prose)
- [ ] Each file includes validation points
- [ ] Each file includes error handling
- [ ] Each file includes ordering constraints
- [ ] Each file includes integration boundaries
- [ ] Each file includes side effects

## Semantic Verification Checklist (Mandatory)
- [ ] 001 covers REQ-CM-001..003 (colormap bridge + null guard + state tracking)
- [ ] 001 covers REQ-MU-001..003 (music bridge + null guard + background vol)
- [ ] 002 covers REQ-SD-001..005 (C-side subtitle clear/check/redraw + Rust model test-only)
- [ ] 003 covers REQ-RL-001..004 (lock drop before callback, extraction pattern)
- [ ] 003 covers REQ-DC-001..005 (single-pass state machine, abort, no-responses)
- [ ] 004 covers REQ-CS-002..003 (cfg(test) guard, production delegation)
- [ ] 004 covers REQ-SM-001..002 (stale marker elimination + exemptions)
- [ ] 005 covers REQ-E2E-001..007 (full flow, summary, response, replay, build compat)
- [ ] 005 covers REQ-TS-001..004 (intro sequence, per-frame, talking anim, track completion)
- [ ] Pseudocode line numbers can be referenced by implementation phases P03..P06

## Deferred Implementation Detection (Mandatory)

```bash
# Not applicable for pseudocode phase — no implementation code
echo "N/A for pseudocode phase"
```

## Success Criteria
- [ ] All 5 pseudocode files exist with numbered algorithms
- [ ] All requirement families have traceable pseudocode coverage
- [ ] Implementation phases can reference specific line ranges

## Failure Recovery
- rollback: `rm -rf project-plans/20260311/commpt3/analysis/pseudocode/`
- blocking issues: incomplete requirement coverage in pseudocode

## Phase Completion Marker
Create: `project-plans/20260311/commpt3/.completed/P02.md`

Contents:
- phase ID: PLAN-20260325-COMMPT3.P02
- timestamp
- files created: 5 pseudocode component files
- verification outputs
- semantic verification summary
