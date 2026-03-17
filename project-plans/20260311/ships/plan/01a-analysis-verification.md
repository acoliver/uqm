# Phase 01a: Analysis Verification

## Phase ID
`PLAN-20260314-SHIPS.P01a`

## Prerequisites
- Required: Phase 01 (Analysis) completed

## Verification Checklist

### Entity Coverage
- [ ] Every type in `races.h` has a corresponding Rust type identified
- [ ] `SPECIES_ID`, `SHIP_INFO`, `CHARACTERISTIC_STUFF`, `DATA_STUFF`, `INTEL_STUFF`, `RACE_DESC`, `STARSHIP`, `SHIP_FRAGMENT`, `FLEET_INFO` all mapped
- [ ] Ship capability flags fully enumerated (SEEKING_WEAPON through PLAYER_CAPTAIN)
- [ ] Status flags fully enumerated (LEFT through PLAY_VICTORY_DITTY)
- [ ] CAPTAIN_STUFF mapped
- [ ] MASTER_SHIP_INFO mapped
- [ ] CODERES_STRUCT lifecycle understood

### State Transition Coverage
- [ ] Ship lifecycle (Unloaded → MetadataLoaded → BattleReady → Spawned → Active → Dead → Freed) fully documented
- [ ] Master catalog lifecycle documented
- [ ] Race-specific mode state machines identified (Androsynth, Mmrnmhrm, Chmmr, Pkunk, Orz)

### Integration Point Coverage
- [ ] All C → Rust call sites identified with file:line references
- [ ] All Rust → C call needs identified
- [ ] All shared data formats across FFI boundary specified
- [ ] Element callback model (function-pointer registration on C elements) understood

### Replacement Coverage
- [ ] Every C file to be guarded is listed
- [ ] Every function to be guarded is listed
- [ ] Rust prototype code replacement strategy documented

### Requirement Traceability
- [ ] Every requirement from requirements.md appears in the coverage map
- [ ] Every requirement is assigned to at least one implementation phase

### Risk/Edge Case Coverage
- [ ] Load failure behavior specified
- [ ] Spawn failure behavior specified
- [ ] Double-free prevention specified
- [ ] Race-specific edge cases identified and assigned to batches

## Gate Decision
- [ ] PASS: proceed to Phase 02
- [ ] FAIL: return to Phase 01 and address gaps
