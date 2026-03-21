# P03a Shared Foundation Verification

Verdict: **FAIL**

## Scope checked
- `rust/src/battle/mod.rs`
- `rust/src/battle/battle_types.rs`
- `rust/src/lib.rs`
- `rust/src/ships/runtime.rs`
- `rust/src/ships/mod.rs`
- `sc2/src/uqm/element.h`
- `sc2/src/uqm/units.h`
- `git diff --name-only`
- `cd rust && cargo test --lib`

## Results

### 1. New files exist and are valid Rust
**PASS**
- `rust/src/battle/mod.rs` exists.
- `rust/src/battle/battle_types.rs` exists.
- Both files are syntactically valid enough for `cargo test --lib` to compile the crate and execute tests.

### 2. `lib.rs` updated
**PASS**
- `rust/src/lib.rs` contains:
  - `pub mod battle;`

### 3. Extraction completeness
**Mostly PASS, with one visibility issue**
`rust/src/battle/battle_types.rs` contains all requested extracted items:

- Angle/facing:
  - `FACING_SHIFT`
  - `NUM_FACINGS`
  - `CIRCLE_SHIFT`
  - `FULL_CIRCLE`
  - `HALF_CIRCLE`
  - `QUADRANT`
  - `OCTANT`
- Coordinate:
  - `VELOCITY_SHIFT`
  - `ONE_SHIFT`
- Element:
  - `NORMAL_LIFE`
  - `MAX_SHIP_MASS`
  - `GRAVITY_THRESHOLD`
  - `PLAYER_SHIP`
  - `APPEARING`
  - `DISAPPEARING`
  - `CHANGING`
  - `COLLISION_FLAG`
  - `IGNORE_SIMILAR`
  - `FINITE_LIFE`
- Trig:
  - `SINE_TABLE`
  - `sine()`
  - `cosine()`
  - `arctan()`
- Conversions:
  - `normalize_facing()`
  - `facing_to_angle()`
  - `angle_to_facing()`
  - `normalize_angle()`
  - `display_to_world()`
  - `world_to_velocity()`
  - `velocity_to_world()`
  - `gravity_mass()`
- New:
  - `wrap_x()`
  - `wrap_y()`
  - `shortest_path_delta()`

Issue found:
- `SINE_TABLE` is declared as private:
  - `const SINE_TABLE: [i32; 17] = ...`
- The verification requirement listed `SINE_TABLE` among extracted items. If external consumers are expected to use it directly, this is incomplete as an exported shared foundation API.

### 4. Re-exports transparent
**PASS, but incomplete for new helpers**
`rust/src/ships/runtime.rs` has `pub use crate::battle::battle_types::{ ... }` re-exports, covering the existing shared foundation symbols needed by current code, including:
- angle/facing constants
- coordinate constants
- element constants
- trig/conversion functions

Not re-exported from `runtime.rs`:
- `wrap_x`
- `wrap_y`
- `shortest_path_delta`

This does not break the stated transparency requirement for existing code, but it means the newly added helpers are not available through the old `ships::runtime` surface.

### 5. No race file changes
**PASS**
`git diff --name-only` reported no modified files under `rust/src/ships/` at all.
- Therefore no race files under `rust/src/ships/` other than `runtime.rs` and `mod.rs` were modified.
- In fact, even `runtime.rs` and `mod.rs` are not currently listed as modified in the working tree.

### 6. Tests pass
**FAIL**
Command run:
- `cd rust && cargo test --lib`

Observed result:
- `1925 passed; 1 failed; 6 ignored; 0 measured; 0 filtered out`
- Failure:
  - `ships::ffi::tests::test_get_cost_by_index`

Failure detail:
- Panic at `src/ships/ffi.rs:372:9`
- Assertion: `cost > 0`

Because there is 1 failure, the required verification target of 1919+ passing tests with 0 failures is not met.

### 7. Value spot-check against C headers
**Mixed**
Checked against `sc2/src/uqm/element.h` and `sc2/src/uqm/units.h`:

- `FULL_CIRCLE`
  - C: `#define FULL_CIRCLE (1 << CIRCLE_SHIFT)` with `CIRCLE_SHIFT 6` => **64**
  - Rust: `pub const FULL_CIRCLE: u16 = 1 << CIRCLE_SHIFT;` => **64**
  - **PASS**
- `VELOCITY_SHIFT`
  - Rust: **5**
  - I did not find this constant in the two requested C files (`element.h`, `units.h`).
  - So the Rust value is present, but this spot-check cannot be fully verified against those two specific files alone.
- `NUM_FACINGS`
  - C: implied by `FACING_SHIFT 4` => `1 << 4` => **16**
  - Rust: `pub const NUM_FACINGS: u16 = 1 << FACING_SHIFT;` => **16**
  - **PASS**
- `NORMAL_LIFE`
  - C (`element.h`): `#define NORMAL_LIFE 1`
  - Rust: `pub const NORMAL_LIFE: u16 = 1;`
  - **PASS**
- `MAX_SHIP_MASS`
  - C (`element.h`): `#define MAX_SHIP_MASS 10`
  - Rust: `pub const MAX_SHIP_MASS: u8 = 10;`
  - **PASS**
- `GRAVITY_THRESHOLD`
  - C (`element.h`): `#define GRAVITY_THRESHOLD (COUNT)255`
  - Rust: `pub const GRAVITY_THRESHOLD: u8 = 255;`
  - **PASS**

## Specific issues causing FAIL
1. `cargo test --lib` does **not** pass cleanly.
   - Actual result: `1925 passed, 1 failed, 6 ignored`
   - Failing test: `ships::ffi::tests::test_get_cost_by_index`
2. `SINE_TABLE` exists in `battle_types.rs` but is **not public**.
   - If the extraction requirement expects the trig table itself to be part of the exported shared foundation API, this is incomplete.
3. The requested value verification for `VELOCITY_SHIFT` could not be confirmed from the two specified C headers alone.
   - It may live in another C header such as `velocity.h`, but that was outside the explicitly requested comparison set.

## Final verdict
**FAIL**

The shared foundation extraction is largely present and wired into `lib.rs`, and the compatibility re-export approach in `ships/runtime.rs` is in place. However, the verification fails because the Rust library test suite does not pass with 0 failures, and there is also an API visibility gap for `SINE_TABLE` if it is intended to be part of the extracted public foundation.