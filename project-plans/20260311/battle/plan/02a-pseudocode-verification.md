# P02a Pseudocode Verification — Battle Engine Phase 1 Leaf Functions

Plan ID: PLAN-20260320-BATTLE
Verified against C source on: 2026-03-20

## Verdict

**FAIL**

Most pseudocode is accurate, but the CRC serialization section contains a significant discrepancy: it names the per-element routine as `crc_process_element`/`crc_processELEMENT` in the expected location, while the actual implementation is `crc_processELEMENT` in `sc2/src/uqm/supermelee/netplay/checksum.c`, not `process.c`, and the documented 35-byte claim is only conditionally true for 16-bit `COUNT`/`COORD`/`ELEMENT_FLAGS`. That section is otherwise structurally correct, but the path/location assumption is wrong and the byte-count claim is platform-type-dependent rather than directly enforced in the function body. The other four algorithm groups are correct or only partially incomplete in edge-case documentation.

---

## 1. Elastic Collision

**Status:** Correct

**Verified source:** `sc2/src/uqm/collide.c`

### Matches confirmed

- **Impact angle formula:**
  - C computes:
    - `dx_rel = elem0.next.x - elem1.next.x`
    - `dy_rel = elem0.next.y - elem1.next.y`
    - `ImpactAngle0 = ARCTAN(dx_rel, dy_rel)`
    - `ImpactAngle1 = NORMALIZE_ANGLE(ImpactAngle0 + HALF_CIRCLE)`
  - Pseudocode matches exactly.

- **Momentum transfer formula:**
  - C computes:
    - `scalar = (long)SINE(Directness, speed << 1) * (mass0 * mass1)`
    - `speed0 = scalar / ((long)mass0 * (mass0 + mass1))`
    - `speed1 = scalar / ((long)mass1 * (mass0 + mass1))`
  - Pseudocode matches exactly.

- **DEFY_PHYSICS behavior:**
  - If both elements are stationary this frame, both get `DEFY_PHYSICS | COLLISION`.
  - If both already had `DEFY_PHYSICS`, C also:
    - adjusts both impact angles by `HALF_CIRCLE - OCTANT`
    - zeroes both velocities before later delta application
  - Pseudocode matches.

- **Gravity mass threshold:**
  - C checks `!GRAVITY_MASS(ElementPtr->mass_points + 1)`.
  - Pseudocode correctly notes this means `mass_points >= 100` is treated as gravity mass / exempt.

- **Minimum velocity enforcement:**
  - C uses Manhattan magnitude in velocity units:
    - `if (VELOCITY_TO_WORLD(dx_abs + dy_abs) < SCALED_ONE)`
    - then sets velocity to `WORLD_TO_VELOCITY(SCALED_ONE) - 1` along impact angle.
  - Pseudocode matches.

### Discrepancies

- None found in the requested spot-check areas.

### Missing edge cases / notes

- The pseudocode does not explicitly say that the stationary check requires **all four coordinate equalities** (`x` and `y` for both current vs next positions). It implies this correctly, but the exact condition is stricter than a generic “no movement” summary.
- In the scraping/fudge branch, C assigns `ImpactAngle0 = TravelAngle0 + HALF_CIRCLE` and `ImpactAngle1 = TravelAngle1 + HALF_CIRCLE` without an explicit `NORMALIZE_ANGLE`; this relies on downstream trig helpers tolerating wrapped angles. The pseudocode reflects the arithmetic but does not call out the lack of explicit normalization.

---

## 2. Weapon Collision

**Status:** Correct

**Verified sources:**
- `sc2/src/uqm/weapon.c`
- `sc2/src/uqm/misc.c`

### 2a. `weapon_collision`

### Matches confirmed

- **Guard condition:**
  - C returns immediately if `WeaponElementPtr->state_flags & COLLISION`.
  - Pseudocode matches.

- **Damage application gate:**
  - C applies damage only if:
    - `damage != 0`
    - and target has `FINITE_LIFE` or `life_span == NORMAL_LIFE`
  - Pseudocode matches.

- **Post-damage collision preservation:**
  - If target still has nonzero `hit_points`, C sets `weapon.state_flags |= COLLISION`.
  - Pseudocode matches.

- **Blast direction 8-bin formula:**
  - C computes:
    - `blast_index = NORMALIZE_FACING(ANGLE_TO_FACING(angle + HALF_CIRCLE))`
    - `blast_index = ((blast_index >> 2) << 1) + (blast_index & 0x3 ? 1 : 0)`
  - Pseudocode matches exactly.

- **Standard vs custom blast threshold:**
  - C uses `if (num_blast_frames <= ANGLE_TO_FACING(FULL_CIRCLE))`
  - Since `ANGLE_TO_FACING(FULL_CIRCLE) == 16`, the pseudocode’s `<= 16` statement is correct.

### 2b. `do_damage`

### Matches confirmed

- **Crew decrement path:**
  - In `misc.c`, player ships call `DeltaCrew(ElementPtr, -damage)`.
  - If it returns false, C sets `life_span = 0` and `NONSOLID`.
  - Pseudocode matches.

### 2c. `TrackShip`

### Matches confirmed

- **Distance metric:**
  - C uses Manhattan approximation:
    - abs wrap-adjusted `delta_x`
    - abs wrap-adjusted `delta_y`
    - `delta_x += delta_y`
  - Pseudocode matches.

- **180° random turn:**
  - C checks `best_delta_facing == ANGLE_TO_FACING(HALF_CIRCLE)` and applies:
    - `(((BYTE)TFB_Random() & 1) << 1) - 1`
  - This yields `+1` or `-1` exactly as pseudocode states.

### Discrepancies

- None found in the requested spot-check areas.

### Missing edge cases / notes

- The destruction condition in C is slightly more subtle than the surrounding comments suggest:
  - `(!(HitElementPtr->state_flags & COLLISION) && WeaponElementPtr->hit_points <= HitElementPtr->mass_points)` is only part of the condition when target is finite-life.
  - The pseudocode preserves the logic correctly.
- `weapon_collision` ignores `pHPt`; pseudocode doesn’t mention this, but it is not algorithmically important.
- `TrackShip` fast-path rechecks a previously stored `hTarget` **without re-running enemy/cloak/player filters**. The pseudocode correctly notes this, which is an important edge case.

---

## 3. Velocity Operations

**Status:** Correct

**Verified source:** `sc2/src/uqm/velocity.c`

### Matches confirmed

- **Bresenham accumulation in `GetCurrentVelocityComponents`:**
  - C reconstructs current velocity as:
    - `WORLD_TO_VELOCITY(vector.{axis}) + (fract.{axis} - HIBYTE(incr.{axis}))`
  - Pseudocode matches.

- **`MAKE_WORD` byte order in `SetVelocityVector`:**
  - Positive axis:
    - `MAKE_WORD(1, 0)`
  - Negative axis:
    - `MAKE_WORD(0xFF, VELOCITY_REMAINDER(component) << 1)`
  - Pseudocode matches the actual low-byte/high-byte arrangement.

- **`ARCTAN` path in `SetVelocityComponents`:**
  - C does:
    - `angle = ARCTAN(dx, dy)`
    - if `angle == FULL_CIRCLE`, zero velocity
    - else decompose x/y exactly as in `SetVelocityVector`
    - assign `TravelAngle = angle` after the branch
  - Pseudocode is consistent with that logic.

- **Delta recomposition in `DeltaVelocityComponents`:**
  - C first reconstructs current dx/dy using the same formula as `GetCurrentVelocityComponents`, adds input deltas, then calls `SetVelocityComponents`.
  - Pseudocode matches.

### Discrepancies

- None found in the requested spot-check areas.

### Missing edge cases / notes

- The byte-count and signedness assumptions depend on typedefs like `SIZE`, `COUNT`, `BYTE`, `SBYTE`; the pseudocode generally assumes the intended sizes correctly.
- In `SetVelocityComponents`, `TravelAngle` is assigned even after the zero-vector branch, meaning it becomes `FULL_CIRCLE` for zero velocity. If the pseudocode already states this elsewhere, it is fine; if not, that behavior should be preserved explicitly.

---

## 4. CRC Serialization

**Status:** Partially correct

**Verified sources:**
- `sc2/src/uqm/supermelee/netplay/checksum.c`
- `sc2/src/uqm/supermelee/netplay/crc.c`
- `sc2/src/uqm/supermelee/netplay/crc.h`

### Matches confirmed

- **Per-element exclusion of `BACKGROUND_OBJECT`:**
  - C checks `if (val->state_flags & BACKGROUND_OBJECT)` and contributes no bytes for that element.
  - Pseudocode matches.

- **Field order inside `crc_processELEMENT`:**
  - C serializes in this order:
    1. `state_flags`
    2. `life_span`
    3. `crew_level`
    4. `mass_points`
    5. `turn_wait`
    6. `thrust_wait`
    7. `velocity` via `TravelAngle`, `vector`, `fract`, `error`, `incr`
    8. `current.location`
    9. `next.location`
  - Pseudocode matches exactly.

- **`crc_processSTATE` excludes image data:**
  - C only serializes `val->location`.
  - Pseudocode matches.

- **CRC-32 polynomial:**
  - `crc.c` explicitly documents polynomial `0x04c11db7` with reflected form `0xedb88320` and uses the standard reflected table/update routine.
  - Pseudocode matches.

### Discrepancies

1. **Location of the implementation is wrong in the plan context.**
   - The verification target says to find `crc_process_element` likely in `process.c` or nearby.
   - Actual implementation is `crc_processELEMENT` in `sc2/src/uqm/supermelee/netplay/checksum.c`.
   - This is not an algorithm error, but it is a concrete discrepancy between the plan assumptions and the codebase.

2. **The “35-byte” statement is descriptive, not directly encoded, and depends on typedef widths.**
   - In this codebase, the routine serializes fields via type-specific CRC helpers (`crc_processCOUNT`, `crc_processBYTE`, `crc_processCOORD`, etc.).
   - The 35-byte total is correct only if those typedefs are 16/8-bit as assumed.
   - The C routine itself does not assemble a 35-byte buffer or assert that size.
   - So the pseudocode is operationally correct, but the “35-byte exact” phrasing is partly inferred from platform typedef sizes, not from a literal 35-byte implementation artifact.

### Missing edge cases / notes

- The actual function name is `crc_processELEMENT`, not `crc_process_element`.
- `crc_processDispQueue` walks the live display queue in linked-list order; checksum correctness depends on queue order stability, not just per-element serialization.
- `crc_processRNG` intentionally performs a destructive seed read via `TFB_SeedRandom(0)` and then restores it. The pseudocode captures this, but it is a critical determinism edge case worth preserving.

---

## 5. Display List Pool

**Status:** Correct

**Verified source:** `sc2/src/uqm/displist.c`

### Matches confirmed

- **Pool allocation / initialization:**
  - Under `QUEUE_TABLE`, `InitQueue`:
    - clears head/tail/free list
    - allocates queue table with `AllocQueueTab`
    - pushes every preallocated link into free list via repeated `FreeLink`
  - This matches the expected pool-allocation model.

- **Free-list management:**
  - `AllocLink` pops the head of the free list.
  - `FreeLink` pushes a link back onto the free list by wiring its successor to the prior free-list head, then updating `SetFreeList`.
  - Pseudocode intent matches the actual logic.

- **Linked-list operations:**
  - `PutQueue` appends to tail.
  - `InsertQueue` inserts before reference link, updating predecessor/successor links and queue head if needed.
  - `RemoveQueue` fixes neighbors and updates head/tail appropriately.
  - This matches standard doubly-linked-list behavior.

### Discrepancies

- None found in the requested spot-check areas.

### Missing edge cases / notes

- `InsertQueue` with `hRefLink == 0` delegates to `PutQueue`; worth preserving exactly.
- `RemoveQueue` does not free the link; unlinking and recycling are separate operations.
- Under non-`QUEUE_TABLE` builds, behavior differs because links are not drawn from a preallocated free-list pool. If the Rust port is targeting battle-engine parity with display-element handles, it likely wants the pooled semantics.

---

## Summary of discrepancies

### Must-fix / document

1. **CRC serialization source location assumption is wrong.**
   - Actual file: `sc2/src/uqm/supermelee/netplay/checksum.c`
   - Not `process.c`.

2. **CRC “35-byte exact” wording should be clarified.**
   - It is correct for this codebase’s typedef sizes.
   - It is not represented as a literal packed 35-byte buffer in C.

### No discrepancies found

- Elastic collision
- Weapon collision
- `do_damage`
- `TrackShip`
- Velocity operations
- Display list pool

---

## Recommended final status per algorithm

- **Elastic Collision:** Correct
- **Weapon Collision:** Correct
- **Velocity Operations:** Correct
- **CRC Serialization:** Partially correct
- **Display List Pool:** Correct

Because one required verification area is only partially correct, the overall result is **FAIL**.
