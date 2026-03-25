# Phase 11a: AI Dispatch Verification

## Phase ID
`PLAN-20260320-BATTLEPT2.P11a`

## Prerequisites
- Required: Phase 11 (AI Dispatch) completed
- Expected artifacts: `ai.rs` with computer_intelligence function, `mod.rs` updated

## Structural Verification Checklist
- [ ] `ai_types.rs` no longer exists (renamed to `ai.rs`)
- [ ] `ai.rs` contains Phase 1 types (EvaluateDesc, MoveState, constants) + computer_intelligence
- [ ] `mod.rs` declares `pub mod ai;`
- [ ] Git history: commit 1 is rename-only

## Semantic Verification Checklist (Mandatory — Most Important)

### computer_intelligence 4-path dispatch (intel.c)
- [ ] **Path 1 — Standard combat**: evaluates distance to target via Pythagorean; evaluates angle from ship facing to target bearing; determines optimal turn direction; decides thrust (approach/retreat); decides weapon fire (range + angle threshold); decides special (race-specific conditions)
- [ ] **Path 2 — Special weapon**: delegates evaluation to race descriptor's intelligence function (race-specific callback); race function can override standard combat decisions
- [ ] **Path 3 — Flee consideration**: evaluates own health (crew_level / max_crew ratio); evaluates opponent health; if severely outmatched: may set retreat flags; flee intent influences standard combat turn/thrust decisions
- [ ] **Path 4 — Missile evasion**: scans nearby elements for incoming FINITE_LIFE projectiles targeting this ship; if threat detected: computes evasive turn direction (away from projectile trajectory); overrides standard combat thrust (thrust to dodge); may inhibit weapon firing during evasion

### Dispatch path selection logic
- [ ] Path selection based on: threat proximity, ship health, incoming projectiles, special weapon availability
- [ ] Only ONE path produces the final output per frame
- [ ] Output: StatusFlags with SHIP_LEFT/SHIP_RIGHT/THRUST/WEAPON/SPECIAL bits

### EvaluateDesc usage
- [ ] which_turn: computed from angle to target (left vs right)
- [ ] facing: ship's current facing direction
- [ ] MoveState: PURSUE/ENTICE/AVOID/EVADE based on tactical situation

### Integration with ship pipeline
- [ ] Output written to cur_status_flags
- [ ] ship_preprocess reads cur_status_flags in Stage 1
- [ ] AI-controlled ships follow same pipeline as human-controlled (difference is input source)

## Branch-Parity Verification
P11 has no compile-time branch families. Dispatch paths are runtime decisions.

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/battle/ai.rs
```

## Pass/Fail Gate Criteria
- **PASS:** All 4 dispatch paths implemented and tested. Output flags match C behavior for same inputs. EvaluateDesc populated correctly. Rename commit clean. No TODO/FIXME/HACK.
- **FAIL:** Any dispatch path missing or unreachable. Output flags wrong for a given scenario. EvaluateDesc not used. Rename commit contains logic changes.
