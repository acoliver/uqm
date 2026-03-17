# Phase 02a: Pseudocode Verification

## Phase ID
`PLAN-20260314-AUDIO-HEART.P02a`

## Prerequisites
- Required: Phase P02 completed

## Structural Verification Checklist
- [ ] Every gap (G1-G13) has corresponding pseudocode (PC-01 through PC-11)
- [ ] Pseudocode is numbered and algorithmic
- [ ] Validation points are explicit (lines checking preconditions)
- [ ] Error handling paths are present (every RETURN Err case)
- [ ] Integration boundaries are marked (calls to UIO, mixer, decoder)
- [ ] Side effects documented (state mutations, locks acquired/released)

## Semantic Verification Checklist
- [ ] PC-01 matches spec §14.1 music-loading behavior exactly
- [ ] PC-02 matches spec §14.2 sound-bank-loading behavior exactly
- [ ] PC-03 ensures single canonical path (spec §14.4)
- [ ] PC-04 produces chunks with real decoders (spec §8.1 splice_multi_track)
- [ ] PC-05 matches spec §10.4 PLRPause semantics (ref-matching + wildcard)
- [ ] PC-06 resolves NORMAL_VOLUME to spec §6 canonical value (160)
- [ ] PC-07 implements spec §8.3.1 state machine (record, claim-and-clear, commit, stop interaction)
- [ ] PC-08 implements spec §13.3 wait-for-sound-end (paused=active, sentinel, default-branch)
- [ ] PC-09 implements spec §13.1 / §19.3 pre-init guard
- [ ] PC-10 satisfies spec §23.2 / §24 diagnostic cleanup
- [ ] PC-11 satisfies spec §23.2 warning suppression removal
- [ ] All pseudocode is implementable with existing project dependencies (parking_lot, log, cpal)

## Coverage Check
- [ ] Every spec §23.1 end-state requirement gap is covered by pseudocode
- [ ] Every spec §23.2 maintainability requirement is covered by pseudocode

## Success Criteria
- [ ] Pseudocode is complete, correct, and implementable
- [ ] Implementation phases can reference specific pseudocode line ranges

## Phase Completion Marker
Create: `project-plans/20260311/audio-heart/.completed/P02a.md`
