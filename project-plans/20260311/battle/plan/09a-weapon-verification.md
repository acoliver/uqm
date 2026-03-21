# P09a Weapon Verification

## Scope
Quick verification of `rust/src/battle/weapon.rs` and `cargo test --lib`.

## Test Run
Command run:

    cd rust && cargo test --lib -q

Result summary:
- Total tests: 2048
- Passed: 2042
- Failed: 0
- Ignored: 6

Status: PASS (meets expectation of 2042+ tests and 0 failures)

## Source Verification (`rust/src/battle/weapon.rs`)

### 1) LaserBlock and MissileBlock types exist with correct fields
PASS

- `LaserBlock` exists and defines: `cx`, `cy`, `ex`, `ey`, `flags`, `sender`, `pixoffs`, `face`, `color`.
- `MissileBlock` exists and defines: `cx`, `cy`, `flags`, `sender`, `pixoffs`, `speed`, `hit_points`, `damage`, `face`, `index`, `life`, `farray`, `preprocess_func`, `blast_offs`.

### 2) Blast direction uses 8-bin quantization
PASS

- `compute_blast_direction(angle: u8) -> u8` implements reverse-angle + facing quantization:
  - reverse with `HALF_CIRCLE`
  - convert angle→facing and normalize
  - map 16 facings into 8 bins using:

        ((facing >> 2) << 1) + if (facing & 0x3) != 0 { 1 } else { 0 }

### 3) do_damage decrements hit_points
PASS

- In this Rust port, hit points are stored in `Element.crew_or_hp`.
- `do_damage(target, damage)` decrements `crew_or_hp` when damage is less than current HP.
- On lethal/excess damage, sets `crew_or_hp = 0`, `life_span = 0`, and adds `NONSOLID`.

### 4) track_ship returns facing adjustment
FAIL

- `track_ship(tracker, facing)` is currently a Phase 1 stub and returns `None`.
- It does not currently compute/return an actual facing delta from live target selection.
- A helper exists (`compute_track_facing`) that performs one-step facing adjustment for a provided target position, but `track_ship` itself does not yet expose that behavior.

## Final Verdict
FAIL

Reason: checks (1), (2), and (3) pass; check (4) fails because `track_ship` currently returns `None` (stub).