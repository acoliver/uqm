# Phase 07a: Ship Runtime Pipeline Verification

## Phase ID
`PLAN-20260320-BATTLEPT2.P07a`

## Prerequisites
- Required: Phase 07 (Ship Runtime Pipeline) completed
- Expected artifacts: `ship_runtime.rs` with 5 functions, `mod.rs` updated

## Structural Verification Checklist
- [ ] `ship_runtime_types.rs` no longer exists (renamed)
- [ ] `ship_runtime.rs` contains Phase 1 types (ShipPipelineStage, constants) + new functions
- [ ] `mod.rs` declares `pub mod ship_runtime;`
- [ ] 5 functions present: ship_preprocess, ship_postprocess, inertial_thrust, animation_preprocess, ship_collision
- [ ] `animation_preprocess` is `pub` (cross-module dependency for P09)
- [ ] Git history: commit 1 is rename-only

## Semantic Verification Checklist (Mandatory — Most Important)

### ship_preprocess 7-stage pipeline (ship.c:120-370)
- [ ] **Stage 1 — Input**: cur_status_flags read from starship (LEFT/RIGHT/THRUST/WEAPON/SPECIAL)
- [ ] **Stage 2 — APPEARING**: if APPEARING in actual state_flags → suppress ALL inputs (clear LEFT/RIGHT/THRUST/WEAPON/SPECIAL in local copy) → init crew from descriptor → init crew display + ship status → race preprocess callback invoked (despite input suppression) → ship_transition warp-in → RETURN EARLY (skip stages 3-7)
- [ ] **Stage 3 — Energy**: energy_counter countdown; on zero → DeltaEnergy(energy_regeneration); counter reset to energy_wait
- [ ] **Stage 4 — Race preprocess**: dispatch race-specific preprocess callback
- [ ] **Stage 5 — Turn**: if turn input active AND turn_wait==0 → facing ±= 1 → NORMALIZE_FACING → update image frame → set CHANGING → turn_wait = characteristics.turn_wait
- [ ] **Stage 6 — Thrust**: if thrust input active AND thrust_wait==0 → inertial_thrust() → spawn_ion_trail() (unless cloaked) → thrust_wait = characteristics.thrust_wait
- [ ] **Stage 7 — Status**: update status display

### ship_postprocess (ship.c:375-470)
- [ ] Exit early if crew_level == 0
- [ ] Weapon firing: weapon input active + weapon_counter==0 + energy >= weapon_energy_cost → deduct energy → init_weapon_func (fills up to 6 handles) → bind each to parent ship → play weapon sound → weapon_wait = characteristics.weapon_wait
- [ ] Special counter: decrement if > 0
- [ ] Race postprocess: dispatch race-specific postprocess callback
- [ ] Status: update status display

### inertial_thrust (ship.c:64-118)
- [ ] **Inertialess** (thrust_increment == max_thrust): set_vector(facing, max_thrust) → return AT_MAX_SPEED
- [ ] **Normal sub-max**: compute v_squared from current components; if v_squared < max_thrust_squared → delta_components(facing, thrust_increment) → check if now at/beyond max → return flags
- [ ] **Gravity well** (distance < GRAVITY_THRESHOLD=255): allow speed up to MAX_ALLOWED_SPEED (2304) regardless of ship max
- [ ] **At max speed**: compute delta_components(facing, half_thrust) − delta_components(old_facing, thrust_increment) → gradual direction change
- [ ] Return value: StatusFlags with AT_MAX_SPEED / BEYOND_MAX_SPEED / IN_GRAVITY_WELL

### animation_preprocess (ship.c:46-62)
- [ ] Decrement turn_wait if > 0
- [ ] If turn_wait == 0: advance frame by 1 (wrapping); set CHANGING flag
- [ ] Frame advance matches C (increment frame index, wrap to 0 at frame_count)

### ship_collision (ship.c:475-520)
- [ ] Gravity mass (GRAVITY_MASS macro): damage = max(crew_level / 4, 1) applied via DoDamage equivalent
- [ ] Non-gravity, non-finite-life: no direct damage (elastic collision handles velocity changes)
- [ ] Collision with finite-life object: no special handling (weapon collision handles it)

## Branch-Parity Verification
- [ ] `IN_ENCOUNTER` / `IN_LAST_BATTLE`: APPEARING first-frame init checks encounter-specific vs SuperMelee paths
- [ ] Cloaking: ion trail skipped when ship is cloaked (prim type check)

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/battle/ship_runtime.rs
```

## Pass/Fail Gate Criteria
- **PASS:** All 5 functions match C behavioral equivalence. 7-stage pipeline order verified. Inertial physics correct (inertialess, normal, gravity, at-max-speed). Weapon firing sequence correct. animation_preprocess public. No TODO/FIXME/HACK.
- **FAIL:** Pipeline stages out of order. APPEARING doesn't suppress inputs or doesn't invoke race preprocess. Inertial physics incorrect for any of 4 modes. Weapon firing skips binding or sound. animation_preprocess not public.
