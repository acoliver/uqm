# Phase 11: AI Dispatch

## Phase ID
`PLAN-20260320-BATTLEPT2.P11`

## Prerequisites
- Required: Phase 10a (Flee + Warp + Winner Verification) completed with PASS
- Expected files: `tactical.rs` with P09+P10 functions (25 tactical transition functions)
- Expected artifacts: Death chain, flee/warp/winner all verified

## Requirements Implemented (Expanded)

### REQ: AI dispatch (battle/requirements.md §AI dispatch)
**Requirement text**: computer_intelligence selects and executes one of four dispatch paths based on ship state: (1) standard combat evaluation, (2) special weapon handling, (3) flee consideration, (4) missile/torpedo evasion.

Behavior contract:
- GIVEN: A computer-controlled ship with an active EvaluateDesc
- WHEN: computer_intelligence is called as the ship's intelligence function
- THEN: Exactly one of four dispatch paths executes based on ship state, producing input flags (LEFT/RIGHT/THRUST/WEAPON/SPECIAL)

### REQ: AI input generation (battle/requirements.md §AI input generation)
**Requirement text**: The AI dispatch produces cur_status_flags (ship input state) that feeds into ship_preprocess.

Behavior contract:
- GIVEN: computer_intelligence has evaluated the ship's situation
- WHEN: The dispatch produces input flags
- THEN: cur_status_flags is updated with LEFT/RIGHT/THRUST/WEAPON/SPECIAL as appropriate

### REQ: AI evaluation descriptors (battle/requirements.md §AI evaluation descriptors)
**Requirement text**: EvaluateDesc contains targeting information (which_turn, facing, MoveState) used by all 4 dispatch paths.

Behavior contract:
- GIVEN: An active ship element and its nearest threat
- WHEN: The AI evaluation descriptor is populated
- THEN: which_turn, facing, and MoveState fields reflect the geometric relationship to the target

## Implementation Tasks

### Commit 1 (rename-only)
- Rename `rust/src/battle/ai_types.rs` → `rust/src/battle/ai.rs`
- Update `rust/src/battle/mod.rs` to reference `ai` instead of `ai_types`
- marker: `@plan PLAN-20260320-BATTLEPT2.P11`
- **NO logic changes** — only file rename and import path updates

### Commit 2+: Files to modify

- `rust/src/battle/ai.rs` — Add AI dispatch logic
  - marker: `@plan PLAN-20260320-BATTLEPT2.P11`
  - marker: `@requirement REQ-AI-DISPATCH, REQ-AI-INPUT`
  - Contents to add:
    - `pub fn computer_intelligence(ship_element: &mut Element, evaluate: &EvaluateDesc) -> StatusFlags` — Full dispatch matching intel.c computer_intelligence. Four paths:
      1. **Standard combat**: evaluate target distance/angle → compute turn direction → decide thrust/weapon/special
      2. **Special weapon**: race-specific special weapon evaluation (delegated to race descriptor intelligence function)
      3. **Flee consideration**: check health, opponent strength → may set flee intent
      4. **Missile evasion**: detect incoming missiles → evasive maneuvers (turn away, thrust)
    - Private helpers as needed for each dispatch path
    - Input mapping: results written to ship's cur_status_flags for ship_preprocess consumption

### C reference functions ported

| C Function | C File | C Lines | Rust Function | Rust Module |
|-----------|--------|---------|---------------|-------------|
| `computer_intelligence()` | intel.c | full file | `computer_intelligence()` | `ai.rs` |

### C branches to handle

| Branch | Source Sites | Handling |
|--------|-------------|----------|
| None directly | — | computer_intelligence has no compile-time branch families; dispatch paths are runtime |

### Integration points
- P07 `ship_runtime.rs`: ship_preprocess consumes cur_status_flags produced by AI
- Phase 1 `ai.rs` (types): EvaluateDesc, MoveState, AI constants, control flags
- Phase 1 `battle_types.rs`: SINE/COSINE, NORMALIZE_FACING, angle calculations
- Phase 1 `velocity.rs`: get_current_components for speed checks
- Phase 1 `element.rs`: Element struct for position/facing queries
- P06 `c_bridge.rs`: get_element_starship, TFB_Random

### Pseudocode traceability (if impl phase)
- Uses pseudocode lines from `analysis/pseudocode/ai-dispatch.md`: computer_intelligence section with all 4 paths

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] `ai_types.rs` renamed to `ai.rs` (commit 1 rename-only)
- [ ] `mod.rs` updated to reference `ai`
- [ ] `computer_intelligence()` implemented in `ai.rs`
- [ ] Phase 1 type definitions (EvaluateDesc, etc.) preserved
- [ ] Plan/requirement traceability markers present
- [ ] Tests compile and run

## Semantic Verification Checklist (Mandatory)
- [ ] All 4 dispatch paths implemented and reachable
- [ ] Path 1 (standard combat): evaluates distance, angle to target; produces turn/thrust/weapon flags
- [ ] Path 2 (special weapon): delegates to race-specific intelligence callback
- [ ] Path 3 (flee consideration): checks health vs opponent; may set flee-related flags
- [ ] Path 4 (missile evasion): detects incoming projectiles; produces evasive maneuver flags
- [ ] Output: cur_status_flags contains correct input flags for ship_preprocess
- [ ] EvaluateDesc populated correctly before dispatch
- [ ] Integration: AI output feeds correctly into ship_preprocess pipeline
- [ ] No placeholder/deferred implementation patterns

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/battle/ai.rs
```

## Success Criteria
- [ ] computer_intelligence implemented with all 4 dispatch paths
- [ ] Input generation matches C behavior
- [ ] All Phase 1 tests pass
- [ ] Verification commands pass

## Failure Recovery
- rollback: `git checkout -- rust/src/battle/ai.rs rust/src/battle/mod.rs`
- blocking issues: Race-specific intelligence callback integration

## Phase Completion Marker
Create: `project-plans/20260311/battlept2/.completed/P11.md`

Contents:
- phase ID: PLAN-20260320-BATTLEPT2.P11
- timestamp
- files changed: ai.rs (renamed + logic), mod.rs
- tests added/updated
- verification outputs
- semantic verification summary
