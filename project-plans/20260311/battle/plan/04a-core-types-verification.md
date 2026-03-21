# P04a Core Types Verification

## Result
PASS

## 1. Compilation & tests
- Command run: `cd rust && cargo test --lib`
- Result: `test result: ok. 1964 passed; 0 failed; 6 ignored; 0 measured; 0 filtered out`
- Verification: passes the stated threshold of 1964+ tests and 0 failures.
- No regression in suite size: 1964 is greater than the required base threshold of 1926.

## 2. Element struct layout
Files checked:
- `rust/src/battle/element.rs`
- `sc2/src/uqm/element.h`

Rust declares `Element` with `#[repr(C)]`, which is required for C-compatible field ordering and layout.

Field order comparison against C `struct element` matches. Spot-checked fields:
1. `pred: HElement` ↔ `HELEMENT pred`
2. `succ: HElement` ↔ `HELEMENT succ`
3. `player_nr: i16` ↔ `SIZE playerNr`
4. `state_flags: ElementFlags` (u16) ↔ `ELEMENT_FLAGS state_flags` (`UWORD`)
5. `velocity: VelocityDesc` ↔ `VELOCITY_DESC velocity`
6. `intersect_control: IntersectControl` ↔ `INTERSECT_CONTROL IntersectControl`
7. `prim_index: u16` ↔ `COUNT PrimIndex`
8. `h_target: HElement` ↔ `HELEMENT hTarget`

Additional layout sanity check using native C-style offsets on this platform produced:
- `pred` @ 0
- `succ` @ 8
- `player_nr` @ 48
- `state_flags` @ 50
- `velocity` @ 60
- `intersect_control` @ 80
- `prim_index` @ 104
- `p_parent` @ 160
- `h_target` @ 168
- total size `Element = 176`

Conclusion: Rust field order is consistent with C for the verified fields, and `#[repr(C)]` is present.

## 3. ElementFlags bits
Files checked:
- `rust/src/battle/element.rs`
- `sc2/src/uqm/element.h`

Verified matching bit positions:
- `PLAYER_SHIP = 1 << 2`
- `APPEARING = 1 << 3`
- `DISAPPEARING = 1 << 4`
- `CHANGING = 1 << 5`
- `NONSOLID = 1 << 6`
- `COLLISION = 1 << 7`
- `IGNORE_SIMILAR = 1 << 8`
- `DEFY_PHYSICS = 1 << 9`
- `FINITE_LIFE = 1 << 10`
- `PRE_PROCESS = 1 << 11`
- `POST_PROCESS = 1 << 12`
- `IGNORE_VELOCITY = 1 << 13`
- `CREW_OBJECT = 1 << 14`
- `BACKGROUND_OBJECT = 1 << 15`

Conclusion: all checked flag bits match C, including the example flags requested.

## 4. VelocityDesc layout
Files checked:
- `rust/src/battle/velocity.rs`
- `sc2/src/uqm/velocity.h`

Rust declares `VelocityDesc` with `#[repr(C)]`.

Field order and types match C `VELOCITY_DESC`:
1. `travel_angle: u16` ↔ `COUNT TravelAngle`
2. `vector: Extent` ↔ `EXTENT vector`
3. `fract: Extent` ↔ `EXTENT fract`
4. `error: Extent` ↔ `EXTENT error`
5. `incr: Extent` ↔ `EXTENT incr`

Rust `Extent` is also `#[repr(C)]` with:
- `width: i16`
- `height: i16`

Native C-style offset/size sanity check on this platform:
- `travel_angle` @ 0
- `vector` @ 2
- `fract` @ 6
- `error` @ 10
- `incr` @ 14
- total size `VelocityDesc = 18`

Conclusion: `VelocityDesc` layout matches the C header structure.

## 5. Constants
Files checked:
- `rust/src/battle/element.rs`
- `rust/src/battle/velocity.rs`
- `sc2/src/uqm/element.h`
- `sc2/src/uqm/units.h`
- `sc2/src/uqm/velocity.h`

Spot-checked constants:
1. `NORMAL_LIFE = 1` ↔ `#define NORMAL_LIFE 1`
2. `MAX_CREW_SIZE = 42` ↔ `#define MAX_CREW_SIZE 42`
3. `MAX_ENERGY_SIZE = 42` ↔ `#define MAX_ENERGY_SIZE 42`
4. `MAX_SHIP_MASS = 10` ↔ `#define MAX_SHIP_MASS 10`
5. `GRAVITY_THRESHOLD = 255` ↔ `#define GRAVITY_THRESHOLD (COUNT)255`
6. `NEUTRAL_PLAYER_NUM = -1` ↔ `#define NEUTRAL_PLAYER_NUM -1`
7. `VELOCITY_SHIFT = 5` ↔ `#define VELOCITY_SHIFT 5`
8. `VELOCITY_SCALE = 32` ↔ `#define VELOCITY_SCALE (1<<VELOCITY_SHIFT)`
9. `FULL_CIRCLE = 64` is sourced from `units.h` as `#define FULL_CIRCLE (1 << CIRCLE_SHIFT)` with `CIRCLE_SHIFT = 6`

Conclusion: checked constants match C headers.

## 6. No regressions
- Verified total test count is `1964`, which is above both:
  - the stated expectation of `1964+ tests`
  - the minimum regression guard of `>= 1926`
- Failures remain `0`

## Issues found
None.

## Final verdict
PASS — core type layout, flag bits, velocity descriptor layout, constants, and test-suite regression checks all passed with no specific issues found.