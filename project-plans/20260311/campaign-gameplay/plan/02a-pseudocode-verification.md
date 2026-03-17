# Phase 02a: Pseudocode Verification

## Phase ID
`PLAN-20260314-CAMPAIGN.P02a`

## Prerequisites
- Required: Phase 02 (Pseudocode) completed

## Structural Verification Checklist
- [ ] All 10 pseudocode components present
- [ ] Line numbers are unique and sequential within each component
- [ ] Every implementation phase references specific pseudocode line ranges
- [ ] Validation points identified in each component
- [ ] Error handling paths present in every component

## Semantic Verification Checklist
- [ ] Component 001 (Campaign Loop) covers all §5.1 dispatch cases: deferred, encounter/starbase, interplanetary, hyperspace
- [ ] Component 001 covers all §5.2 terminal conditions: victory, defeat, restart
- [ ] Component 002 (Start Game) covers new-game, load-game, and restart loop from §4.1
- [ ] Component 003 (Deferred Transition) preserves §5.3 observable properties (no save mutation, no fake-load)
- [ ] Component 004 (Hyperspace Transitions) covers §7.3 encounter, §7.4 interplanetary, §7.5 quasispace
- [ ] Component 005 (Encounter Handoff) covers §6.1 dispatch, §6.2 battle segue, §6.3 post-encounter
- [ ] Component 005 includes suppress-processing for abort/load/death/last-battle per §6.3
- [ ] Component 006 (Starbase) covers §6.4 special sequences: bomb-transport, pre-alliance Ilwrath
- [ ] Component 006 covers §6.5 departure via deferred transition
- [ ] Component 007 (Events) covers all 18 selectors from §8.6 catalog
- [ ] Component 008 (Save) covers §9.1 serialized fields, §9.2 summary, §9.3 semantics
- [ ] Component 009 (Load) covers §9.4 restore, §9.4.0b safe-failure, §9.4.1 rejection cases
- [ ] Component 010 (Export) covers §10.1 document structure and all required sections

## Ordering Constraint Verification
- [ ] Campaign loop initialization sequence correct: kernel init -> clock init -> event registration -> loop
- [ ] Load validation happens BEFORE session state commit (no partial application)
- [ ] Starbase forced conversation happens BEFORE normal menu access
- [ ] Deferred transition is processed at TOP of loop iteration, not inline

## Gate Decision
- [ ] PASS: proceed to Phase 03
- [ ] FAIL: revise pseudocode — document gaps
