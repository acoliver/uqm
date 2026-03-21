# P08a Collision Verification

## Scope
Quick verification against `sc2/src/uqm/collide.c` for Rust implementation in `rust/src/battle/collision.rs`.

## Test Run
Command run:

    cd rust && cargo test --lib

Observed summary:
- `test result: ok. 2024 passed; 0 failed; 6 ignored; 0 measured; 0 filtered out`

Result: **PASS** (meets expectation of 2024+ tests and 0 failures)

## Source Verification
Compared:
- Rust: `rust/src/battle/collision.rs`
- C reference: `sc2/src/uqm/collide.c`

### 1) Elastic collide uses mass-weighted momentum transfer
- C: `scalar = SINE(Directness, speed << 1) * (mass0 * mass1)` then per-object delta with divisor `massN * (mass0 + mass1)`.
- Rust matches this structure exactly:
  - `scalar = sine(directness, (speed as i32) << 1) as i64 * (mass0 * mass1)`
  - `speed0 = scalar / (mass0 * (mass0 + mass1))`
  - `speed1 = scalar / (mass1 * (mass0 + mass1))`

Status: **PASS**

### 2) Gravity mass exemption at threshold 100
- C gates updates with `!GRAVITY_MASS(mass_points + 1)` and comment-equivalent behavior means `mass_points >= 100` exempt.
- Rust defines `GRAVITY_MASS_THRESHOLD = MAX_SHIP_MASS * 10` (100) and `is_gravity_mass(mass_points) => mass_points >= 100`, and skips updates when true.

Status: **PASS**

### 3) `DEFY_PHYSICS` set on both elements after stationary overlap collision
- C: when both elements show no positional movement this frame, both get `DEFY_PHYSICS | COLLISION`.
- Rust does same stationary-overlap check and inserts `ElementFlags::DEFY_PHYSICS | ElementFlags::COLLISION` on both elements.

Status: **PASS**

### 4) Minimum velocity enforcement
- C: if `VELOCITY_TO_WORLD(abs(dx)+abs(dy)) < SCALED_ONE`, reset velocity along impact angle to `WORLD_TO_VELOCITY(SCALED_ONE) - 1`.
- Rust applies same conditional and reset along impact angle, but uses `world_to_velocity(SCALED_ONE)` (no `- 1`) intentionally (commented rationale: ensure threshold pass).

Status: **PASS (behaviorally compliant, not bit-identical constant)**

### 5) Player ship penalty handling
- C (for player ships):
  - clear `SHIP_AT_MAX_SPEED | SHIP_BEYOND_MAX_SPEED`
  - if not `DEFY_PHYSICS`, increment turn/thrust waits up to collision minima.
- Rust:
  - applies turn/thrust wait penalties under same `!DEFY_PHYSICS` condition and same minima logic.
  - max-speed-flag clear is marked TODO (FFI not yet wired), so this sub-part is not functionally implemented.

Status: **PARTIAL / FAIL (strict parity)**
- Penalty wait handling: implemented.
- Max-speed flag clear side-effect: missing (TODO).

## Overall Verdict
**FAIL** for strict P08a parity verification due to missing player-ship max-speed flag clearing side-effect.

If evaluating only collision kinematics and wait-penalty behavior, it would pass; for full `collide.c` behavior parity, it currently does not.
