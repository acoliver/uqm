# Phase 02a: Pseudocode Verification

## Phase ID
`PLAN-20260314-SHIPS.P02a`

## Prerequisites
- Required: Phase 02 (Pseudocode) completed

## Verification Checklist

### Completeness
- [ ] All 9 components have pseudocode
- [ ] Every requirement from requirements.md is addressable by at least one pseudocode component
- [ ] Every integration touchpoint from analysis has a corresponding pseudocode path

### Algorithmic Quality
- [ ] Pseudocode includes validation points (species ID validation, null checks)
- [ ] Error handling paths are explicit (Result returns, error propagation)
- [ ] Ordering constraints are documented (preprocess before energy regen, energy regen before weapon fire)
- [ ] Side effects are identified (element creation, sound playback, state mutation)

### Traceability
- [ ] Each pseudocode component has line numbers
- [ ] Each component is assigned to an implementation phase
- [ ] Implementation phases can reference specific line ranges

### Behavioral Fidelity
- [ ] Runtime pipeline ordering matches C `ship.c` (preprocess → energy → turn → thrust → weapon → special → postprocess → cooldowns)
- [ ] Two-tier loading matches C `loadship.c` (metadata-only loads icons/strings only, battle-ready loads all assets)
- [ ] Spawn sequence matches C `ship.c:spawn_ship()` (load → patch crew → alloc element → bind callbacks)
- [ ] Crew writeback matches C `init.c:UninitShips()` (enumerate active, write back, free)
- [ ] Master catalog loading matches C `master.c` (iterate species, load metadata, sort by name)

### Edge Case Coverage
- [ ] Load failure cleanup path present in Component 2
- [ ] Spawn failure handling present in Component 6
- [ ] Death/transition with no race_desc handled in Component 7
- [ ] Mode-switching mutation pattern shown in Component 9

## Gate Decision
- [ ] PASS: proceed to Phase 03
- [ ] FAIL: return to Phase 02 and address gaps
