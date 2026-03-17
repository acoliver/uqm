# Phase 10: Crew Writeback & Ship Death

## Phase ID
`PLAN-20260314-SHIPS.P10`

## Prerequisites
- Required: Phase 09a (Spawn Verification) completed and PASS
- Expected files: `lifecycle.rs` with spawn/init/uninit, queue/fragment bridge helpers, `ffi_contract.rs` with writeback-matching rules

## Requirements Implemented (Expanded)

### Death Sequence
**Requirement text**: When a ship is destroyed during battle, the subsystem shall execute death-specific behavior, free the dead ship's descriptor instance (invoking teardown hook), record crew state back to persistent fragment, and mark entry inactive.

Behavior contract:
- GIVEN: A ship whose crew reaches 0
- WHEN: Death callback fires
- THEN: Explosion/crew scatter, descriptor freed with teardown, fragment updated, entry marked dead

### Crew Writeback
**Requirement text**: When a ship is destroyed or battle ends, the subsystem shall record surviving crew counts back to persistent fragments. Matching by queue ordering and species identity, not pointer identity.

Behavior contract:
- GIVEN: A surviving ship with 3 crew remaining
- WHEN: Battle teardown occurs
- THEN: The matching ShipFragment has crew_level = 3

### Replacement
**Requirement text**: When a replacement ship is needed after death, the subsystem shall follow the standard spawn sequence. When no replacement is available, signal the battle engine.

Behavior contract:
- GIVEN: A ship dies and another is available in the queue
- WHEN: Transition occurs
- THEN: Next ship is spawned via standard spawn_ship()

### Floating Crew
**Requirement text**: When floating crew elements exist at battle teardown, the subsystem shall account for them in final crew counts.

Behavior contract:
- GIVEN: Floating crew elements in the display list at battle end
- WHEN: Final crew accounting is done
- THEN: Floating crew are counted per established game rules

### Audio Reset
**Requirement text**: Audio state shall be stopped or reset during death-to-replacement transitions.

Behavior contract:
- GIVEN: A ship dies
- WHEN: Transition to next ship begins
- THEN: Ship-specific sounds/music are stopped before replacement spawns

### Teardown Robustness
**Requirement text**: The teardown sequence shall be robust against ships that were never fully spawned, absent teardown hooks, already-freed descriptors, and queue entries with no associated descriptor.

Behavior contract:
- GIVEN: Various edge-case ship states at teardown time
- WHEN: uninit_ships() runs
- THEN: No panics, no double-frees, graceful handling

## Implementation Tasks

### Files to create

- `rust/src/ships/writeback.rs` — Crew writeback and death handling
  - marker: `@plan PLAN-20260314-SHIPS.P10`
  - marker: `@requirement REQ-WRITEBACK, REQ-WRITEBACK-MATCHING, REQ-WRITEBACK-CAMPAIGN, REQ-WRITEBACK-MELEE, REQ-FLOATING-CREW, REQ-DEATH-SEQUENCE, REQ-REPLACEMENT-SPAWN, REQ-NO-REPLACEMENT-SIGNAL, REQ-AUDIO-RESET, REQ-TEARDOWN-ROBUSTNESS, REQ-PRIVATE-STATE-LEAK`
  - Contents:
    - `ship_death(ship: &mut Starship, element: &ElementState)`:
      - Spawn explosion elements via c_bridge
      - Scatter crew elements based on remaining crew
      - Play death sound
    - `new_ship_transition(dead_ship: *mut STARSHIP, side: u8) -> Result<bool, ShipError>` or typed equivalent:
      - Stop ship-specific audio
      - If dead ship has descriptor: invoke teardown, free descriptor
      - Write back crew (0) to matching fragment
      - Mark entry inactive
      - Find next available ship in side's canonical queue
      - If found: spawn via lifecycle::spawn_ship() → return Ok(true)
      - If none: signal no replacement → return Ok(false)
    - `update_ship_frag_crew(...)`:
      - Match by queue ordering and species_id
      - Uses an explicit **transient bookkeeping structure** (for example, a writeback cursor/map keyed by queue position) rather than assuming a persistent `processed` field exists on `ShipFragment`
      - Bookkeeping lifecycle/reset semantics are documented and cleared between transitions/teardown passes
    - `battle_teardown_writeback(...)`:
      - Count floating crew elements via c_bridge
      - For each ship in the canonical race queue with active descriptor:
        - Record final crew to matching fragment
        - Invoke teardown hook
        - Free descriptor
      - Handle edge cases: no descriptor (skip gracefully), no matching fragment (log warning)
    - `count_floating_crew_elements() -> u16`:
      - Query display list via c_bridge for crew-type elements
    - Error types for teardown failures (non-fatal, logged)

### Files to modify

- `rust/src/ships/mod.rs`
  - Add `pub mod writeback;`

- `rust/src/ships/lifecycle.rs`
  - Wire `uninit_ships()` to call `writeback::battle_teardown_writeback()`
  - Wire ship death callback to call `writeback::ship_death()`

### Pseudocode traceability
- Uses pseudocode component 7, lines 310-358, with writeback bookkeeping clarified so it is consistent with the planned fragment model

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] `writeback.rs` created with death, transition, writeback functions
- [ ] `lifecycle.rs` wired to writeback for teardown
- [ ] Writeback bookkeeping is represented explicitly outside persistent fragment contract, or the type contract is updated consistently
- [ ] Plan/requirement traceability markers present

## Semantic Verification Checklist (Mandatory)
- [ ] `ship_death()` spawns explosion and crew scatter elements
- [ ] `new_ship_transition()` frees descriptor, writes back crew=0, marks inactive
- [ ] `new_ship_transition()` stops audio before replacement
- [ ] `new_ship_transition()` spawns replacement if available, signals if not
- [ ] `update_ship_frag_crew()` matches by queue order + species (not pointer)
- [ ] Double-update prevention uses explicit transient bookkeeping with documented reset semantics
- [ ] `battle_teardown_writeback()` handles surviving ships with correct crew
- [ ] `battle_teardown_writeback()` handles ships with no descriptor (gracefully)
- [ ] `battle_teardown_writeback()` accounts for floating crew elements
- [ ] Teardown with absent teardown hook: no panic
- [ ] Teardown with already-freed descriptor: no double-free
- [ ] Mixed C/Rust integration test covers teardown/writeback against real queue/fragment structures
- [ ] No placeholder/deferred implementation patterns remain

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/ships/writeback.rs
```

## Success Criteria
- [ ] Death/transition/writeback all compile and pass tests
- [ ] Edge cases verified (robustness)
- [ ] Fragment matching by queue order verified
- [ ] lifecycle integration verified
- [ ] Verification commands pass

## Failure Recovery
- rollback: `git checkout -- rust/src/ships/writeback.rs rust/src/ships/lifecycle.rs`

## Phase Completion Marker
Create: `project-plans/20260311/ships/.completed/P10.md`
