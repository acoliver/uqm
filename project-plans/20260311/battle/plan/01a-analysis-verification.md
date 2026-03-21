# P01a Analysis Verification — Battle Engine Rust Port

Date: 2026-03-20
Verifier: LLxprt Code
Source under verification: `project-plans/20260311/battle/plan/01-analysis.md`

## Verdict

**FAIL**

The analysis is strong overall, but it does **not** satisfy the requested requirements-completeness check as written:

- `requirements.md` contains **274** requirement bullets.
- `01-analysis.md` assigns **269** `REQ-BAT-nnn` IDs.
- Therefore **5 requirement bullets are not individually assigned IDs** in the analysis.

The other verification areas were largely successful:

- Requirement spot-checks: **20/20 matched** for the sampled set.
- C function inventory spot-checks: **15/15 matched** for names, parameter counts, return types, and line numbers.
- Existing Rust code survey: **mostly accurate**, with one notable inaccuracy in the `COMPUTER_CONTROL` comparison.
- Open design decisions (§18): **all seven resolved with concrete, reasonable decisions**.
- Phase 1 leaf function inventory: **reasonable and correctly leaf-oriented**.

## 1. Requirements completeness verification

### Count check

- Requirement bullets in `requirements.md`: **274**
- Unique `REQ-BAT-nnn` IDs in `01-analysis.md`: **269**
- ID range present: **REQ-BAT-001 .. REQ-BAT-269**
- Missing IDs within that range: **none**

### Result

**FAIL** — not every requirement bullet has its own assigned `REQ-BAT-nnn` ID.

### Cause

The analysis appears to have **collapsed several tail-end requirement bullets into broader summary IDs**, rather than preserving one ID per bullet all the way through the source requirements document.

Most visible example:

- `requirements.md` final bullets include several distinct statements at lines **597–601**:
  - robustness against element pool exhaustion
  - robustness against display primitive exhaustion
  - deterministic display-list processing order
  - double-buffer consistency invariant
  - teardown robustness edge cases
- In `01-analysis.md`, the tail of the index ends at **REQ-BAT-269**, with aggregate rows such as:
  - `REQ-BAT-266 | Pool exhaustion robust, no corruption, deterministic order`
  - `REQ-BAT-267 | Double-buffer invariant (current/next consistency)`
  - `REQ-BAT-268 | Cooperative scheduling (DoInput pattern, frame timing, batching)`
  - `REQ-BAT-269 | Frame rate 24 fps; max-speed suppression`

This indicates the final section was not kept strictly one-bullet-to-one-ID.

## 2. Requirement spot-checks (20 distributed samples)

All sampled mappings were consistent between `requirements.md` and `01-analysis.md`.

| Sample | Requirement line | Expected meaning | Analysis ID / text | Result |
|---|---:|---|---|---|
| 1 | 21 | Unified element model for all physical battle objects | `REQ-BAT-001` Every physical object in battle represented as an element within a unified entity model | PASS |
| 2 | 25 | Per-element display primitive index, independently allocated | `REQ-BAT-005` Each element carries display primitive index linking to one entry in display primitive array; display prim allocation managed independently | PASS |
| 3 | 42 | `IGNORE_VELOCITY` skips velocity application during preprocess | `REQ-BAT-019` IGNORE_VELOCITY: prevents velocity from being applied to element position during preprocessing | PASS |
| 4 | 55 | crew field undefined during ship→explosion transition | `REQ-BAT-026` Ship→explosion transition: crew_level field value undefined during explosion | PASS |
| 5 | 69 | Removing an element clears all target references to it | `REQ-BAT-034` Element removed → iterate all elements and clear tracking target references pointing to removed element | PASS |
| 6 | 83 | Gravity threshold is 255 display units | `REQ-BAT-042` GRAVITY_THRESHOLD = 255 (display-coordinate distance for gravity pull) | PASS |
| 7 | 111 | Separate rendering-order primitive list | `REQ-BAT-056` Separate rendering-order linked list of display primitives for visual layering | PASS |
| 8 | 131 | Toroidal wrapping occurs in postprocess, not velocity stepping | `REQ-BAT-065` Toroidal wrapping applied during postprocess pass, not during velocity stepping | PASS |
| 9 | 157 | Velocity descriptor fields | `REQ-BAT-077` Velocity descriptor: travel angle (0–63), integer vector, fractional remainder, error accumulator, increment encoding | PASS |
| 10 | 171 | N-frame Bresenham stepping mutates error | `REQ-BAT-085` get_next: N-frame Bresenham accumulation, error mutated as side effect | PASS |
| 11 | 191 | Collision dispatch sets COLLISION on both | `REQ-BAT-094` Collision dispatched → COLLISION flag set on both elements | PASS |
| 12 | 211 | Player-ship collision clears speed flags and adds waits | `REQ-BAT-105` Player ship collision penalty: clear max-speed/beyond-max-speed, add wait counters (turn_wait, thrust_wait) | PASS |
| 13 | 247 | Homing first checks stored target, then scans ships | `REQ-BAT-118` Homing: first check stored h_target (fast path), then iterate all elements for enemy PLAYER_SHIP | PASS |
| 14 | 270 | Preprocess applies velocity when not ignored | `REQ-BAT-130` No IGNORE_VELOCITY → apply velocity for next position via Bresenham | PASS |
| 15 | 293 | DISAPPEARING elements are removed and deallocated | `REQ-BAT-141` DISAPPEARING → remove + deallocate | PASS |
| 16 | 320 | Battle start seeds RNG, loads music, inits ships/space | `REQ-BAT-154` Battle begins: seed RNG, load music, init ships/space, determine sides | PASS |
| 17 | 352 | Encounter writes final crew back to fleet | `REQ-BAT-171` Encounter: persist crew to fleet via ship-fragment writeback | PASS |
| 18 | 387 | Non-gravity/non-finite-life ship collision uses elastic response only | `REQ-BAT-190` Non-gravity, non-finite-life: elastic collision only, no direct damage | PASS |
| 19 | 449 | Reincarnating mass=11 zero-crew ship treated as alive | `REQ-BAT-219` mass == MAX_SHIP_MASS + 1 (=11) and zero crew → treated as alive (reincarnating) | PASS |
| 20 | 601 | Teardown robustness against partial/absent state | No distinct terminal per-bullet mapping; end of analysis collapses terminal requirements | **FAIL** |

### Requirements-specific issues found

1. **Completeness failure:** 274 bullets vs 269 IDs.
2. **Terminal aggregation:** the final requirement bullets were not preserved as one bullet = one `REQ-BAT-nnn` row.
3. **Strict traceability risk:** downstream planning and verification will be weaker for the last few requirements because some are grouped instead of individually addressable.

## 3. C function inventory spot-check (15 functions)

Spot-checked against actual source files under `sc2/src/uqm/`.

| Function | File | Analysis claim | Source check | Result |
|---|---|---|---|---|
| `GetCurrentVelocityComponents` | `velocity.c` | line 28, `void (VELOCITY_DESC*, SIZE*, SIZE*)` | Found at line 28 with exact signature | PASS |
| `GetNextVelocityComponents` | `velocity.c` | line 37, `void (VELOCITY_DESC*, SIZE*, SIZE*, COUNT)` | Found at line 37 with exact signature | PASS |
| `SetVelocityVector` | `velocity.c` | line 58, `void (VELOCITY_DESC*, SIZE, COUNT)` | Found at line 58 with exact signature | PASS |
| `SetVelocityComponents` | `velocity.c` | line 99, `void (VELOCITY_DESC*, SIZE, SIZE)` | Found at line 99 with exact signature | PASS |
| `DeltaVelocityComponents` | `velocity.c` | line 143, `void (VELOCITY_DESC*, SIZE, SIZE)` | Found at line 143 with exact signature | PASS |
| `collide` | `collide.c` | line 30, `void (ELEMENT*, ELEMENT*)` | Found at line 30 with exact signature | PASS |
| `initialize_laser` | `weapon.c` | line 45, `HELEMENT (LASER_BLOCK*)` | Found at line 45 with exact signature | PASS |
| `initialize_missile` | `weapon.c` | line 88, `HELEMENT (MISSILE_BLOCK*)` | Found at line 88 with exact signature | PASS |
| `weapon_collision` | `weapon.c` | line 135, `HELEMENT (ELEMENT*, POINT*, ELEMENT*, POINT*)` | Found at line 135 with exact signature | PASS |
| `ModifySilhouette` | `weapon.c` | line 249, `FRAME (ELEMENT*, STAMP*, BYTE)` | Found at line 249 with exact signature | PASS |
| `TrackShip` | `weapon.c` | line 319, `SIZE (ELEMENT*, COUNT*)` | Found at line 319 with exact signature | PASS |
| `AllocElement` | `process.c` | line 77, `HELEMENT (void)` | Found at line 77 with exact signature | PASS |
| `FreeElement` | `process.c` | line 102, `void (HELEMENT)` | Found at line 102 with exact signature | PASS |
| `PreProcess` | `process.c` | line 129, `void (ELEMENT*)` | Found at line 129 as `static void PreProcess (ELEMENT*)` | PASS |
| `DoBattle` | `battle.c` | line 259, `static BOOLEAN (BATTLE_STATE*)` | Found at line 259 with exact signature | PASS |

### Inventory result

**PASS** — all 15 sampled inventory entries matched the real C code for:

- function name
- parameter count
- return type
- line number

No inaccuracies were found in the sampled inventory rows.

## 4. Existing Rust code survey verification (`rust/src/ships/runtime.rs`)

### Verified correct in the analysis

The analysis correctly identifies that `runtime.rs` already contains battle-adjacent logic and constants, including:

- angle/facing constants and helpers:
  - `FACING_SHIFT`, `NUM_FACINGS`, `CIRCLE_SHIFT`, `FULL_CIRCLE`, `HALF_CIRCLE`, `QUADRANT`, `OCTANT`
  - `normalize_facing`, `facing_to_angle`, `angle_to_facing`, `normalize_angle`
- coordinate/velocity helpers:
  - `display_to_world`, `world_to_velocity`, `velocity_to_world`
  - `gravity_mass`
- fixed-point trig:
  - `sine`, `cosine`, `arctan`
- velocity type and operations:
  - `VelocityState`
  - `zero`, `get_current_components`, `set_vector`, `set_components`, `delta_components`, `velocity_squared`, `is_zero`
- ship-pipeline functions that are ships-owned rather than battle-owned:
  - `ship_preprocess`
  - `ship_postprocess`
  - `inertial_thrust`
  - `delta_energy`
  - `animation_preprocess`
  - `default_ship_collision`

### Verified analysis findings about bugs/gaps

The analysis is also directionally correct that `runtime.rs` is **not** a drop-in battle-engine ABI layer:

- `VelocityState` is not `#[repr(C)]`
- `ElementState` is only a subset / convenience model, not a C-compatible `ELEMENT`
- `get_next_components()` is missing
- several battle flags are absent from the current constants
- there are functional issues around velocity encoding and `is_zero()`

### Issue found in the analysis

One statement is inaccurate:

- `01-analysis.md` says:
  - `COMPUTER_CONTROL | 1 | C: CYBORG|PSYTRON=6 | BUG — value mismatch`
- In `runtime.rs`, `COMPUTER_CONTROL` is documented as:
  - `// Computer-control value for Starship.control (C: COMPUTER_CONTROL).`
- So the comparison against `CYBORG|PSYTRON` is not well-supported by the code being surveyed.

### Rust survey result

**PASS with issue**

The survey is mostly accurate and useful, but it contains **one notable mismatch** around `COMPUTER_CONTROL` semantics.

## 5. Design decision verification (`specification.md` §18)

Checked against specification section 18. All open decisions identified there have corresponding concrete resolutions in `01-analysis.md` section 4.

| Spec §18 item | Open question in spec | Resolution in analysis | Assessment |
|---|---|---|---|
| 18.1 Union field layout verification | How to model C anonymous unions correctly | Use explicit `#[repr(C)] union` types plus layout assertions | Reasonable |
| 18.2 Callback function pointer ABI compatibility | Whether Rust callback pointer representation is ABI-compatible | Use `Option<unsafe extern "C" fn(...)>` and verify with compile-time/cross-compilation checks | Reasonable |
| 18.3 `p_parent` void pointer semantics | Safe accessor vs opaque pointer | Keep raw `*mut c_void` in Phase 1; no false safety over non-`#[repr(C)]` `Starship` | Reasonable |
| 18.4 Frame and drawable handles | How to represent `FRAME` / opaque handles | Use opaque `*mut c_void` pointer handles | Reasonable |
| 18.5 Display primitive array ownership timeline | Battle-vs-graphics ownership boundary | Keep C-owned in Phase 1, defer later boundary clarification | Reasonable |
| 18.6 `DrawablesIntersect` replacement | Reimplement vs FFI | Keep in C / call via FFI; do not reimplement | Reasonable |
| 18.7 `ships/runtime.rs` migration timing | How to migrate to shared `battle_types` | Fix velocity encoding first, then extract shared types in stages | Reasonable |

### Design decision result

**PASS** — every open design decision in spec §18 was resolved with a concrete and defensible decision.

## 6. Phase 1 leaf function inventory verification

Analysis section 5 lists these Phase 1 leaf functions:

- Velocity operations:
  - `GetCurrentVelocityComponents`
  - `GetNextVelocityComponents`
  - `SetVelocityVector`
  - `SetVelocityComponents`
  - `DeltaVelocityComponents`
- Collision physics:
  - `collide`
- Weapon system:
  - `weapon_collision`
  - `TrackShip`
- Netplay:
  - `crc_processELEMENT`
- Internal Rust helpers:
  - `is_collidable`
  - `collision_possible`

### Verification of “leaf-ness”

**Confirmed leaf/computation-oriented:**

- The five `velocity.c` functions are pure math / state-update helpers over a single velocity descriptor.
- `collide()` is computation-heavy physics over two elements, but **not** the collision orchestration loop. It does not traverse the display list or own frame processing.
- `weapon_collision()` is local collision-effect logic for a weapon/target pair. It creates a blast element and calls C services, but it is still pair-local behavior, not a frame orchestration loop.
- `TrackShip()` scans elements to choose a target and compute a turn direction; it is localized weapon AI behavior, not a top-level battle/process loop.
- `crc_processELEMENT()` is serialization/checksum leaf work.

**Correctly excluded from Phase 1 leaf set:**

- `ProcessCollisions()` in `process.c`
- `PreProcessQueue()` / `PostProcessQueue()`
- `RedrawQueue()`
- `Battle()` / `DoBattle()`
- display-list ownership functions
- tactical transition orchestration

### Leaf inventory result

**PASS** — the listed Phase 1 inventory is appropriately leaf-oriented and does not incorrectly include top-level orchestration loops.

## 7. Summary of issues found

### Must-fix issues

1. **Requirements completeness failure**
   - `requirements.md`: 274 bullets
   - `01-analysis.md`: 269 IDs
   - The analysis must be revised so **every requirement bullet has its own `REQ-BAT-nnn` row**.

### Should-fix issues

2. **Tail-end requirement aggregation**
   - Terminal requirements appear to be collapsed into broader summary rows instead of individually traceable rows.

3. **`COMPUTER_CONTROL` comparison in Rust survey is questionable**
   - The analysis compares `COMPUTER_CONTROL` to `CYBORG|PSYTRON`, but `runtime.rs` documents it as mapping to C `COMPUTER_CONTROL` rather than that composite flag value.

## Final assessment

`01-analysis.md` is a high-quality analysis overall, and the C inventory, Rust survey, design-decision resolutions, and Phase 1 leaf inventory are substantially correct.

However, the requested verification standard was explicit: **every requirement bullet must have a `REQ-BAT-nnn` ID assigned**. That condition is not met.

**Final verdict: FAIL**
