# P02 Pseudocode — Battle Engine Phase 1 Leaf Functions

Plan ID: PLAN-20260320-BATTLE
Phase: P02
Generated: 2026-03-20

---

## Table of Contents

1. [Elastic Collision (`collide.c`)](#1-elastic-collision)
2. [Weapon Collision (`weapon.c`)](#2-weapon-collision)
3. [Velocity Operations (`velocity.c`)](#3-velocity-operations)
4. [CRC Serialization (`checksum.c` / `crc.c`)](#4-crc-serialization)
5. [Display List Pool (`displist.c`)](#5-display-list-pool)

---

## 1. Elastic Collision

### C function signature

```c
void collide(ELEMENT *ElementPtr0, ELEMENT *ElementPtr1);
```

Source: `sc2/src/uqm/collide.c` (183 lines)

### Pseudocode

```
FUNCTION collide(elem0, elem1):
    // --- Step 1: Compute impact angle from position delta ---
    dx_rel = elem0.next.location.x - elem1.next.location.x
    dy_rel = elem0.next.location.y - elem1.next.location.y
    ImpactAngle0 = ARCTAN(dx_rel, dy_rel)
    ImpactAngle1 = NORMALIZE_ANGLE(ImpactAngle0 + HALF_CIRCLE)
        // HALF_CIRCLE = 32 (half of 64-step circle)

    // --- Step 2: Get current velocities and compute relative velocity ---
    (dx0, dy0) = GetCurrentVelocityComponents(elem0.velocity)
    TravelAngle0 = elem0.velocity.TravelAngle
    (dx1, dy1) = GetCurrentVelocityComponents(elem1.velocity)
    TravelAngle1 = elem1.velocity.TravelAngle

    dx_rel = dx0 - dx1
    dy_rel = dy0 - dy1
    RelTravelAngle = ARCTAN(dx_rel, dy_rel)
    speed = square_root(dx_rel * dx_rel + dy_rel * dy_rel)
        // integer square root, all i32 arithmetic with i64 intermediates

    // --- Step 3: Compute directness; fudge if scraping ---
    Directness = NORMALIZE_ANGLE(RelTravelAngle - ImpactAngle0)
    IF Directness <= QUADRANT OR Directness >= (HALF_CIRCLE + QUADRANT):
        // Shapes just scraped — they will collide again unless we fudge
        Directness = HALF_CIRCLE
        ImpactAngle0 = TravelAngle0 + HALF_CIRCLE
        ImpactAngle1 = TravelAngle1 + HALF_CIRCLE

    // --- Step 4: Stationary overlap → DEFY_PHYSICS ---
    IF elem0.next.location == elem0.current.location
       AND elem1.next.location == elem1.current.location:
        // Both elements are stationary (no position change this frame)

        IF (elem0.state_flags & DEFY_PHYSICS) AND (elem1.state_flags & DEFY_PHYSICS):
            // Already defying physics — nudge impact angles and zero velocities
            ImpactAngle0 = TravelAngle0 + (HALF_CIRCLE - OCTANT)  // offset by 28
            ImpactAngle1 = TravelAngle1 + (HALF_CIRCLE - OCTANT)
            ZeroVelocityComponents(elem0.velocity)
            ZeroVelocityComponents(elem1.velocity)

        elem0.state_flags |= (DEFY_PHYSICS | COLLISION)
        elem1.state_flags |= (DEFY_PHYSICS | COLLISION)

    // --- Step 5: Momentum transfer computation ---
    mass0 = elem0.mass_points
    mass1 = elem1.mass_points
    scalar = SINE(Directness, speed << 1) * (mass0 * mass1)
        // SINE(a, m) = (SINVAL(a) * m) >> 14
        // scalar is i64 (long in C)

    // --- Step 5a: Apply to elem0 (if not gravity mass) ---
    IF NOT GRAVITY_MASS(elem0.mass_points + 1):
        // GRAVITY_MASS(m) = (m > MAX_SHIP_MASS * 10)  i.e. m > 100
        // Note: test is (mass_points + 1) > 100, so mass_points >= 100 is exempt

        // Player ship penalty
        IF elem0.state_flags & PLAYER_SHIP:
            StarShipPtr = GetElementStarShip(elem0)
            StarShipPtr.cur_status_flags &= ~(SHIP_AT_MAX_SPEED | SHIP_BEYOND_MAX_SPEED)

            IF NOT (elem0.state_flags & DEFY_PHYSICS):
                IF elem0.turn_wait < COLLISION_TURN_WAIT:       // COLLISION_TURN_WAIT = 1
                    elem0.turn_wait += COLLISION_TURN_WAIT
                IF elem0.thrust_wait < COLLISION_THRUST_WAIT:   // COLLISION_THRUST_WAIT = 3
                    elem0.thrust_wait += COLLISION_THRUST_WAIT

        // Velocity change inversely proportional to own mass
        speed0 = scalar / (mass0 * (mass0 + mass1))
            // integer division (truncation toward zero in C)
        DeltaVelocityComponents(elem0.velocity,
            COSINE(ImpactAngle0, speed0),
            SINE(ImpactAngle0, speed0))

        // Minimum velocity enforcement
        (dx0, dy0) = GetCurrentVelocityComponents(elem0.velocity)
        IF dx0 < 0: dx0 = -dx0
        IF dy0 < 0: dy0 = -dy0
        IF VELOCITY_TO_WORLD(dx0 + dy0) < SCALED_ONE:
            // Result too slow — set minimum velocity along impact angle
            // VELOCITY_TO_WORLD(v) = v >> 5
            // SCALED_ONE = 4 (1 << ONE_SHIFT where ONE_SHIFT = 2)
            // So threshold is: (|dx0| + |dy0|) >> 5 < 4, i.e. |dx0|+|dy0| < 128
            min_vel = WORLD_TO_VELOCITY(SCALED_ONE) - 1  // = (4 << 5) - 1 = 127
            SetVelocityComponents(elem0.velocity,
                COSINE(ImpactAngle0, min_vel),
                SINE(ImpactAngle0, min_vel))

    // --- Step 5b: Apply to elem1 (if not gravity mass) — symmetric ---
    IF NOT GRAVITY_MASS(elem1.mass_points + 1):

        // Player ship penalty (identical logic)
        IF elem1.state_flags & PLAYER_SHIP:
            StarShipPtr = GetElementStarShip(elem1)
            StarShipPtr.cur_status_flags &= ~(SHIP_AT_MAX_SPEED | SHIP_BEYOND_MAX_SPEED)

            IF NOT (elem1.state_flags & DEFY_PHYSICS):
                IF elem1.turn_wait < COLLISION_TURN_WAIT:
                    elem1.turn_wait += COLLISION_TURN_WAIT
                IF elem1.thrust_wait < COLLISION_THRUST_WAIT:
                    elem1.thrust_wait += COLLISION_THRUST_WAIT

        speed1 = scalar / (mass1 * (mass0 + mass1))
        DeltaVelocityComponents(elem1.velocity,
            COSINE(ImpactAngle1, speed1),
            SINE(ImpactAngle1, speed1))

        // Minimum velocity enforcement (identical logic)
        (dx1, dy1) = GetCurrentVelocityComponents(elem1.velocity)
        IF dx1 < 0: dx1 = -dx1
        IF dy1 < 0: dy1 = -dy1
        IF VELOCITY_TO_WORLD(dx1 + dy1) < SCALED_ONE:
            min_vel = WORLD_TO_VELOCITY(SCALED_ONE) - 1
            SetVelocityComponents(elem1.velocity,
                COSINE(ImpactAngle1, min_vel),
                SINE(ImpactAngle1, min_vel))
```

### Key invariants and edge cases

1. **`scalar` computation uses `(long)` / `i64` arithmetic.** The expression `SINE(Directness, speed << 1) * (mass0 * mass1)` must not overflow. `SINE()` returns `i16`; `speed << 1` can be up to ~65534; mass product ≤ 100. The multiplication fits comfortably in i64.
2. **Division is integer truncation toward zero** (C `SIZE` is `i16`; `long` is i32/i64 depending on platform; the cast to `SIZE` truncates). Rust must use integer division, not floating-point.
3. **`GRAVITY_MASS(mass_points + 1)`** — the `+1` means elements with `mass_points >= 100` are immovable. The function's test is `(m > MAX_SHIP_MASS * 10)`, i.e. `(mass_points + 1 > 100)`, so `mass_points == 100` IS gravity-mass (exempt from velocity change).
4. **DEFY_PHYSICS handling is order-dependent.** Step 4 (stationary check) runs before Step 5 (momentum transfer). If both are stationary and both already DEFY_PHYSICS, velocities are zeroed, then momentum transfer applies new velocity via DeltaVelocityComponents (which reads the just-zeroed velocity and adds the delta).
5. **Scraping fudge (Step 3):** When `Directness ∈ [0..QUADRANT] ∪ [HALF_CIRCLE+QUADRANT..FULL_CIRCLE-1]`, the collision is nearly tangential. Fudging `Directness = HALF_CIRCLE` makes `SINE(Directness, ...)` return 0, which zeros out the scalar — effectively no momentum transfer. The fudged impact angles ensure the minimum velocity enforcement kicks in, giving both elements a small push away from each other along their travel directions reversed.
6. **Player ship penalty only applies if NOT DEFY_PHYSICS.** If already defying physics, the wait counters are not modified. The max-speed flags are always cleared regardless.
7. **Minimum velocity uses Manhattan distance** (`|dx| + |dy|`) in velocity coordinates, not Euclidean distance. The threshold is `VELOCITY_TO_WORLD(|dx|+|dy|) < SCALED_ONE` ⇔ `(|dx|+|dy|) >> 5 < 4` ⇔ `|dx|+|dy| < 128`.

### What Rust must preserve for C parity

- All arithmetic in fixed-point integer. No `f32`/`f64` anywhere.
- `ARCTAN()`, `SINE()`, `COSINE()`, `square_root()` must use the same lookup tables as C.
- `NORMALIZE_ANGLE(a)` = `a & (FULL_CIRCLE - 1)` = `a & 63`. Rust must use bitwise AND on unsigned.
- `scalar` must be computed as `i64` (C uses `long`). The division `scalar / (mass * (mass0 + mass1))` truncates toward zero.
- The `GRAVITY_MASS` test uses `mass_points + 1`, not `mass_points`.
- `DeltaVelocityComponents` and `SetVelocityComponents` must produce bit-identical Bresenham parameters.
- Element fields (`state_flags`, `turn_wait`, `thrust_wait`, `velocity`) are mutated in-place through `#[repr(C)]` struct pointers.
- The StarShip pointer is accessed via `elem.pParent` (cast); Rust uses unsafe FFI to modify `cur_status_flags`.

---

## 2. Weapon Collision

### 2a. `weapon_collision()`

#### C function signature

```c
HELEMENT weapon_collision(ELEMENT *WeaponElementPtr, POINT *pWPt,
                          ELEMENT *HitElementPtr, POINT *pHPt);
```

Source: `sc2/src/uqm/weapon.c` (lines 138–253)

Note: A wrapper `weapon_collision_cb` (lines 35–40) discards the return value to match `ElementCollisionFunc` signature. The actual `weapon_collision` returns `HELEMENT` (the blast element, or 0).

#### Pseudocode

```
FUNCTION weapon_collision(weapon, wPt, target, hPt) -> HELEMENT:
    // --- Step 1: Double-hit guard ---
    IF weapon.state_flags & COLLISION:
        RETURN 0   // Already processed this frame

    // --- Step 2: Damage application ---
    damage = weapon.mass_points   // mass_points stores damage value for weapons
    IF damage != 0
       AND (target.state_flags & FINITE_LIFE  OR  target.life_span == NORMAL_LIFE):
        // NORMAL_LIFE = 1; persistent elements (ships) have life_span=1
        do_damage(target, damage)
        IF target.hit_points != 0:
            // Target survived — prevent weapon from being destroyed
            weapon.state_flags |= COLLISION

    // --- Step 3: Weapon destruction check ---
    IF NOT (target.state_flags & FINITE_LIFE)
       OR (NOT (target.state_flags & COLLISION)
           AND weapon.hit_points <= target.mass_points):
        // Weapon is destroyed when:
        //   - target is NOT finite-life (ships, asteroids), OR
        //   - target doesn't have COLLISION set AND weapon is weaker than target

        // Step 3a: Play damage sound
        IF damage != 0:
            sound_idx = TARGET_DAMAGED_FOR_1_PT + (damage >> 1)
            IF sound_idx > TARGET_DAMAGED_FOR_6_PLUS_PT:
                sound_idx = TARGET_DAMAGED_FOR_6_PLUS_PT
            // Sound indices: 1pt=2, 2-3pt=3, 4-5pt=4, 6+pt=5
            ProcessSound(SetAbsSoundIndex(GameSounds, sound_idx), target)

        // Step 3b: Mark weapon as destroyed
        IF GetPrimType(DisplayArray[weapon.PrimIndex]) != LINE_PRIM:
            weapon.state_flags |= DISAPPEARING
            // Lasers (LINE_PRIM) never get DISAPPEARING — they persist for their 1-frame life
        weapon.hit_points = 0
        weapon.life_span = 0
        weapon.state_flags |= (COLLISION | NONSOLID)

        // Step 3c: Create blast effect ---
        hBlast = AllocElement()
        IF hBlast != 0:
            PutElement(hBlast)   // Insert into display list
            blast = LockElement(hBlast)

            blast.playerNr = weapon.playerNr
            blast.state_flags = APPEARING | FINITE_LIFE | NONSOLID
            SetPrimType(DisplayArray[blast.PrimIndex], STAMP_PRIM)

            // Position blast at weapon collision point (display→world)
            blast.current.location.x = DISPLAY_TO_WORLD(wPt.x)
            blast.current.location.y = DISPLAY_TO_WORLD(wPt.y)

            // Offset blast by blast_offset along weapon velocity angle
            angle = GetVelocityTravelAngle(weapon.velocity)
            blast_offs = weapon.blast_offset
            IF blast_offs > 0:
                blast.current.location.x += COSINE(angle, DISPLAY_TO_WORLD(blast_offs))
                blast.current.location.y += SINE(angle, DISPLAY_TO_WORLD(blast_offs))

            // --- Step 3d: Compute blast direction (8 bins) ---
            blast_index = NORMALIZE_FACING(ANGLE_TO_FACING(angle + HALF_CIRCLE))
                // ANGLE_TO_FACING(a) = (a + 2) >> 2  (round angle to 16 facings)
                // NORMALIZE_FACING(f) = f & 15
            blast_index = ((blast_index >> 2) << 1) + (IF blast_index & 0x3 != 0 THEN 1 ELSE 0)
                // This maps 16 facings to 8 bins:
                //   facing 0     → bin 0 (even)
                //   facing 1-3   → bin 1 (odd)
                //   facing 4     → bin 2 (even)
                //   facing 5-7   → bin 3 (odd)
                //   ... etc.

            // --- Step 3e: Standard vs custom blast ---
            num_blast_frames = GetFrameCount(weapon.next.image.frame)
            IF num_blast_frames <= ANGLE_TO_FACING(FULL_CIRCLE):
                // Standard blast: ≤ 16 frames → 2-frame explosion
                blast.life_span = 2
                blast.current.image.farray = &blast_resource
                blast.current.image.frame = SetAbsFrameIndex(blast_resource[0], blast_index)
            ELSE:
                // Custom blast: weapon provides extra frames beyond the 16 facing frames
                blast.life_span = num_blast_frames - ANGLE_TO_FACING(FULL_CIRCLE)
                blast.turn_wait = 0
                blast.next_turn = 0
                blast.preprocess_func = animation_preprocess
                blast.current.image.farray = weapon.next.image.farray
                blast.current.image.frame = SetAbsFrameIndex(
                    blast.current.image.farray[0],
                    ANGLE_TO_FACING(FULL_CIRCLE))
                // Custom blast starts at frame index 16 (past the 16 facing frames)

            UnlockElement(hBlast)
            RETURN hBlast

    RETURN 0
```

### 2b. `do_damage()`

#### C function signature

```c
void do_damage(ELEMENT *ElementPtr, SIZE damage);
```

Source: `sc2/src/uqm/misc.c` (lines 195–220)

#### Pseudocode

```
FUNCTION do_damage(element, damage):
    IF element.state_flags & PLAYER_SHIP:
        // Ship takes crew damage
        IF NOT DeltaCrew(element, -damage):
            // DeltaCrew returns 0 (false) when crew hits 0
            element.life_span = 0
            element.state_flags |= NONSOLID
    ELSE IF NOT GRAVITY_MASS(element.mass_points):
        // Non-ship, non-gravity-mass element (weapon, asteroid, etc.)
        IF (BYTE)damage < element.hit_points:
            element.hit_points -= (BYTE)damage
        ELSE:
            element.hit_points = 0
            element.life_span = 0
            element.state_flags |= NONSOLID
```

### 2c. `TrackShip()`

#### C function signature

```c
SIZE TrackShip(ELEMENT *Tracker, COUNT *pfacing);
```

Source: `sc2/src/uqm/weapon.c` (lines 300–414)

Returns: actual facing delta to target, or -1 if no target found. `*pfacing` is updated to turn one step toward target.

#### Pseudocode

```
FUNCTION TrackShip(tracker, pfacing) -> SIZE:
    best_delta = 0
    best_delta_facing = -1

    // --- Step 1: Fast path — check stored hTarget ---
    hShip = tracker.hTarget
    IF hShip != 0:
        trackee = LockElement(hShip)
        tracker.hTarget = 0
        hNextShip = 0
        GOTO CheckTracking   // Skip eligibility filters, go straight to distance check

    // --- Step 2: Display list scan fallback ---
    FOR hShip = GetHeadElement(); hShip != 0; hShip = hNextShip:
        trackee = LockElement(hShip)
        hNextShip = GetSuccElement(trackee)

        // Filter: must be enemy PLAYER_SHIP, must not be cloaked
        //         (unless tracker is PLAYER_SHIP with APPEARING)
        IF (trackee.state_flags & PLAYER_SHIP)
           AND NOT elementsOfSamePlayer(trackee, tracker)
           AND (NOT OBJECT_CLOAKED(trackee)
                OR ((tracker.state_flags & PLAYER_SHIP)
                    AND (tracker.state_flags & APPEARING))):

        CheckTracking:
            StarShipPtr = GetElementStarShip(trackee)
            IF trackee.life_span != 0
               AND StarShipPtr.RaceDescPtr.ship_info.crew_level != 0:
                // Target is alive

                // --- Step 3: Compute toroidal shortest-path delta ---
                IF tracker.state_flags & PRE_PROCESS:
                    delta_x = trackee.next.location.x - tracker.next.location.x
                    delta_y = trackee.next.location.y - tracker.next.location.y
                ELSE:
                    delta_x = trackee.current.location.x - tracker.current.location.x
                    delta_y = trackee.current.location.y - tracker.current.location.y

                delta_x = WRAP_DELTA_X(delta_x)
                delta_y = WRAP_DELTA_Y(delta_y)
                    // WRAP_DELTA_X: if |dx| > LOG_SPACE_WIDTH/2, adjust by ±LOG_SPACE_WIDTH
                    // WRAP_DELTA_Y: same for Y axis

                // --- Step 4: Compute facing delta ---
                delta_facing = NORMALIZE_FACING(
                    ANGLE_TO_FACING(ARCTAN(delta_x, delta_y)) - *pfacing)

                // --- Step 5: Manhattan distance ---
                IF delta_x < 0: delta_x = -delta_x
                IF delta_y < 0: delta_y = -delta_y
                delta_x += delta_y
                    // Manhattan distance approximation of Euclidean

                // --- Step 6: Track closest target ---
                IF best_delta == 0 OR delta_x < best_delta:
                    best_delta = delta_x
                    best_delta_facing = delta_facing
                    tracker.hTarget = hShip

        UnlockElement(hShip)

    // --- Step 7: Apply one-step turn toward target ---
    IF best_delta_facing > 0:
        facing = *pfacing
        IF best_delta_facing == ANGLE_TO_FACING(HALF_CIRCLE):
            // Target is exactly behind — random turn direction
            facing += ((TFB_Random() & 1) << 1) - 1
                // Produces +1 or -1 randomly
        ELSE IF best_delta_facing < ANGLE_TO_FACING(HALF_CIRCLE):
            // Target is to the left (shorter clockwise arc)
            facing += 1
        ELSE:
            // Target is to the right (shorter counter-clockwise arc)
            facing -= 1
        *pfacing = NORMALIZE_FACING(facing)

    RETURN best_delta_facing
```

### Key invariants and edge cases

1. **`weapon_collision` returns `HELEMENT`** but the wrapper `weapon_collision_cb` discards it to match `ElementCollisionFunc`. The Rust FFI must handle both signatures.
2. **Double-hit prevention:** If `COLLISION` is already set, the function exits immediately. This prevents a weapon from damaging multiple targets in a single frame.
3. **Laser persistence:** `LINE_PRIM` weapons never get `DISAPPEARING`. They have `life_span = 1` (LASER_LIFE) and expire naturally. Non-LINE weapons get `DISAPPEARING` on destruction.
4. **Blast direction mapping:** The 8 bins are computed from the 16-facing system via `((facing >> 2) << 1) + (facing & 3 ? 1 : 0)`. This produces pairs: `{0,1}, {2,3}, {4,5}, {6,7}, {8,9}, {10,11}, {12,13}, {14,15}` → `{0,1}, {2,3}, {4,5}, {6,7}` mapped to 8 indices.
5. **Custom blast threshold:** `num_blast_frames > 16` means the weapon's farray has extra frames beyond the 16 directional frames. These extra frames are the explosion animation.
6. **Sound index calculation:** `TARGET_DAMAGED_FOR_1_PT` = 2 (enum value). `damage >> 1` adds 0 for 1hp, 1 for 2-3hp, 2 for 4-5hp, capped at `TARGET_DAMAGED_FOR_6_PLUS_PT` = 5.
7. **TrackShip fast path:** When `hTarget` is set, it skips the eligibility filters (PLAYER_SHIP check, same-player check, cloak check) and goes directly to distance computation. The stored target is unconditionally checked.
8. **TrackShip cloaking:** `OBJECT_CLOAKED` checks display primitive type ≥ `NUM_PRIMS` or `STAMPFILL_PRIM` with `BLACK_COLOR`. Cloaked ships are invisible to tracking unless the tracker itself is a `PLAYER_SHIP` with `APPEARING` (newly spawned ship can see through cloak).
9. **TrackShip 180° tie-break:** When `best_delta_facing == ANGLE_TO_FACING(HALF_CIRCLE)` (target is exactly behind), a random bit selects +1 or −1 turn direction to avoid deterministic bias.
10. **TrackShip location selection:** Uses `.next` positions if `PRE_PROCESS` is set (preprocessing phase), `.current` otherwise. This ensures the tracker homes toward the target's most up-to-date position.

### What Rust must preserve for C parity

- `weapon_collision` must return `HELEMENT` (a pool handle, not null-pointer). The wrapper discards it.
- `do_damage` for `PLAYER_SHIP` calls `DeltaCrew()` which is a C function — Rust must call via FFI.
- Blast allocation via `AllocElement()` / `PutElement()` modifies the C display list — Rust calls via FFI.
- Sound dispatch via `ProcessSound()` is a C function — Rust calls via FFI.
- `TrackShip` uses `TFB_Random()` for the 180° tie-break. Rust must call C's RNG to maintain netplay determinism.
- `OBJECT_CLOAKED` reads `DisplayArray[PrimIndex]` — Rust must access the global C display array.
- The blast direction index formula `((f >> 2) << 1) + (f & 3 ? 1 : 0)` must be bit-identical. Note the ternary: any non-zero lower 2 bits → 1, zero → 0.

---

## 3. Velocity Operations

### Data structure

```c
typedef struct velocity_desc {
    COUNT TravelAngle;       // 0–63 angle (u16)
    EXTENT vector;           // integer part: {width: i16, height: i16}
    EXTENT fract;            // fractional part: {width: i16, height: i16}
    EXTENT error;            // Bresenham error accumulator: {width: i16, height: i16}
    EXTENT incr;             // increment encoding: {width: u16, height: u16}
} VELOCITY_DESC;
```

Constants:
- `VELOCITY_SHIFT = 5`, `VELOCITY_SCALE = 32`
- `VELOCITY_TO_WORLD(v) = v >> 5`
- `WORLD_TO_VELOCITY(l) = l << 5`
- `VELOCITY_REMAINDER(v) = v & 31`
- `MAKE_WORD(lo, hi) = ((hi as u8) << 8) | (lo as u8)`
- `LOBYTE(w) = w & 0xFF`, `HIBYTE(w) = (w >> 8) & 0xFF`

### 3a. `GetCurrentVelocityComponents`

#### C function signature

```c
void GetCurrentVelocityComponents(VELOCITY_DESC *velocityptr, SIZE *pdx, SIZE *pdy);
```

#### Pseudocode

```
FUNCTION GetCurrentVelocityComponents(vel) -> (dx, dy):
    // Reconstruct full velocity from integer part + fractional part - sign correction
    dx = WORLD_TO_VELOCITY(vel.vector.width)
         + (vel.fract.width - (SIZE)HIBYTE(vel.incr.width))
        // For positive: HIBYTE(MAKE_WORD(1, 0)) = 0, so dx = (vector.width << 5) + fract.width
        // For negative: HIBYTE(MAKE_WORD(0xFF, doubled_remainder)) = doubled_remainder
        //   so dx = (vector.width << 5) + fract.width - doubled_remainder

    dy = WORLD_TO_VELOCITY(vel.vector.height)
         + (vel.fract.height - (SIZE)HIBYTE(vel.incr.height))

    RETURN (dx, dy)
```

### 3b. `GetNextVelocityComponents`

#### C function signature

```c
void GetNextVelocityComponents(VELOCITY_DESC *velocityptr, SIZE *pdx, SIZE *pdy, COUNT num_frames);
```

**Side effect:** Mutates `velocityptr->error` (Bresenham accumulator).

#### Pseudocode

```
FUNCTION GetNextVelocityComponents(vel, num_frames) -> (dx, dy):
    // --- X axis ---
    e = (COUNT)vel.error.width + ((COUNT)vel.fract.width * num_frames)
        // Cast to unsigned 16-bit to prevent sign extension issues
    dx = (vel.vector.width * num_frames)
         + ((SBYTE)LOBYTE(vel.incr.width)) * (e >> VELOCITY_SHIFT)
        // LOBYTE(incr.width): +1 for positive velocity, 0xFF (-1 as signed byte) for negative
        // e >> 5 = number of "extra steps" accumulated
    vel.error.width = VELOCITY_REMAINDER(e)   // e & 31

    // --- Y axis (identical) ---
    e = (COUNT)vel.error.height + ((COUNT)vel.fract.height * num_frames)
    dy = (vel.vector.height * num_frames)
         + ((SBYTE)LOBYTE(vel.incr.height)) * (e >> VELOCITY_SHIFT)
    vel.error.height = VELOCITY_REMAINDER(e)

    RETURN (dx, dy)
```

### 3c. `SetVelocityVector`

#### C function signature

```c
void SetVelocityVector(VELOCITY_DESC *velocityptr, SIZE magnitude, COUNT facing);
```

#### Pseudocode

```
FUNCTION SetVelocityVector(vel, magnitude, facing):
    // --- Step 1: Convert facing to angle, compute trig ---
    angle = FACING_TO_ANGLE(NORMALIZE_FACING(facing))
        // FACING_TO_ANGLE(f) = f << 2  (multiply by 4)
    vel.TravelAngle = angle
    magnitude = WORLD_TO_VELOCITY(magnitude)   // magnitude << 5
    dx = COSINE(angle, magnitude)
    dy = SINE(angle, magnitude)

    // --- Step 2: Decompose X component ---
    IF dx >= 0:
        vel.vector.width = VELOCITY_TO_WORLD(dx)           // dx >> 5
        vel.incr.width = MAKE_WORD(1, 0)                   // 0x0001
    ELSE:
        dx = -dx
        vel.vector.width = -(VELOCITY_TO_WORLD(dx))        // -(dx >> 5), note: negate after shift
        vel.incr.width = MAKE_WORD(0xFF, VELOCITY_REMAINDER(dx) << 1)
            // LOBYTE = 0xFF (-1 as i8); HIBYTE = (dx & 31) << 1

    // --- Step 3: Decompose Y component (identical pattern) ---
    IF dy >= 0:
        vel.vector.height = VELOCITY_TO_WORLD(dy)
        vel.incr.height = MAKE_WORD(1, 0)
    ELSE:
        dy = -dy
        vel.vector.height = -(VELOCITY_TO_WORLD(dy))
        vel.incr.height = MAKE_WORD(0xFF, VELOCITY_REMAINDER(dy) << 1)

    // --- Step 4: Set fractional and clear error ---
    vel.fract.width = VELOCITY_REMAINDER(dx)    // dx & 31 (dx is now positive)
    vel.fract.height = VELOCITY_REMAINDER(dy)
    vel.error.width = 0
    vel.error.height = 0
```

### 3d. `SetVelocityComponents`

#### C function signature

```c
void SetVelocityComponents(VELOCITY_DESC *velocityptr, SIZE dx, SIZE dy);
```

#### Pseudocode

```
FUNCTION SetVelocityComponents(vel, dx, dy):
    // --- Step 1: Compute travel angle ---
    angle = ARCTAN(dx, dy)
    IF angle == FULL_CIRCLE:       // ARCTAN returns 64 for zero vector
        ZeroVelocityComponents(vel)   // memset to all zeros
    ELSE:
        // --- Step 2: Decompose X (same as SetVelocityVector) ---
        IF dx >= 0:
            vel.vector.width = VELOCITY_TO_WORLD(dx)
            vel.incr.width = MAKE_WORD(1, 0)
        ELSE:
            dx = -dx
            vel.vector.width = -(VELOCITY_TO_WORLD(dx))
            vel.incr.width = MAKE_WORD(0xFF, VELOCITY_REMAINDER(dx) << 1)

        // --- Step 3: Decompose Y (same pattern) ---
        IF dy >= 0:
            vel.vector.height = VELOCITY_TO_WORLD(dy)
            vel.incr.height = MAKE_WORD(1, 0)
        ELSE:
            dy = -dy
            vel.vector.height = -(VELOCITY_TO_WORLD(dy))
            vel.incr.height = MAKE_WORD(0xFF, VELOCITY_REMAINDER(dy) << 1)

        // --- Step 4: Set fractional and clear error ---
        vel.fract.width = VELOCITY_REMAINDER(dx)
        vel.fract.height = VELOCITY_REMAINDER(dy)
        vel.error.width = 0
        vel.error.height = 0

    vel.TravelAngle = angle
```

### 3e. `DeltaVelocityComponents`

#### C function signature

```c
void DeltaVelocityComponents(VELOCITY_DESC *velocityptr, SIZE dx, SIZE dy);
```

#### Pseudocode

```
FUNCTION DeltaVelocityComponents(vel, dx, dy):
    // --- Step 1: Read current velocity and add delta ---
    dx += WORLD_TO_VELOCITY(vel.vector.width)
          + (vel.fract.width - (SIZE)HIBYTE(vel.incr.width))
        // This is GetCurrentVelocityComponents inline + addition

    dy += WORLD_TO_VELOCITY(vel.vector.height)
          + (vel.fract.height - (SIZE)HIBYTE(vel.incr.height))

    // --- Step 2: Recompute from new components ---
    SetVelocityComponents(vel, dx, dy)
```

### 3f. `ZeroVelocityComponents` and `IsVelocityZero`

```
FUNCTION ZeroVelocityComponents(vel):
    memset(vel, 0, sizeof(VELOCITY_DESC))
        // All fields including TravelAngle become 0

FUNCTION IsVelocityZero(vel) -> bool:
    RETURN vel.vector.width == 0 AND vel.vector.height == 0
       AND vel.incr.width == 0 AND vel.incr.height == 0
       AND vel.fract.width == 0 AND vel.fract.height == 0
```

### Key invariants and edge cases

1. **Bresenham encoding:** The `incr` field encodes sign and remainder. For positive components: `incr = MAKE_WORD(1, 0)` = `0x0001`. For negative: `incr = MAKE_WORD(0xFF, remainder<<1)` where `0xFF` = `-1` as `SBYTE`, and `HIBYTE` = doubled remainder. This is the "correction term" in `GetCurrentVelocityComponents`.
2. **`MAKE_WORD(lo, hi)` byte order:** `MAKE_WORD(lo, hi) = (hi << 8) | lo`. So `LOBYTE(MAKE_WORD(lo,hi)) = lo` and `HIBYTE(MAKE_WORD(lo,hi)) = hi`. The `lo` byte is the sign indicator; the `hi` byte is the doubled fractional remainder.
3. **`GetNextVelocityComponents` mutates error:** This is a side effect! The error accumulator advances N frames. Callers that want "peek" semantics must save/restore error, or this must be documented.
4. **`ARCTAN` returns `FULL_CIRCLE` (64) for zero vector.** This is a special sentinel meaning "no direction." `SetVelocityComponents` checks for this and zeros everything.
5. **Negative decomposition:** `-VELOCITY_TO_WORLD(dx)` is computed AFTER making `dx` positive. So `vel.vector.width = -(abs(dx) >> 5)`, not `(-dx) >> 5` (which would differ for values not divisible by 32 due to sign extension in arithmetic right shift).
6. **`DeltaVelocityComponents` is read+add+rewrite.** It reconstructs the current velocity, adds the delta, then calls `SetVelocityComponents` to re-encode. This means the travel angle is recomputed via `ARCTAN`.
7. **`ZeroVelocityComponents` sets `TravelAngle = 0`**, not `FULL_CIRCLE`. A zeroed velocity has a travel angle of 0 (pointing right), but `IsVelocityZero` does not check TravelAngle.
8. **The `(COUNT)` casts in `GetNextVelocityComponents`** force unsigned 16-bit arithmetic. `(COUNT)vel.error.width + (COUNT)vel.fract.width * num_frames` wraps at 65536. This is critical for bit-identical behavior.

### What Rust must preserve for C parity

- All arithmetic must use the exact same integer widths: `SIZE` = `i16`, `COUNT` = `u16`, `BYTE` = `u8`, `SBYTE` = `i8`.
- `VELOCITY_REMAINDER` mask = `v & 31` (not modulo — identical for non-negative, but the inputs are always non-negative by this point).
- `MAKE_WORD` byte order must be `(hi << 8) | lo`. The existing VelocityState bug fix in `ships/runtime.rs` confirms this.
- `GetNextVelocityComponents` must mutate error in place (side effect). The `#[repr(C)]` layout must match C's `VELOCITY_DESC` exactly.
- `ARCTAN(0, 0) == FULL_CIRCLE` (64), triggering the zero path.
- The `(SBYTE)LOBYTE(incr)` cast chain: extract low byte, then sign-extend to `i16`. In Rust: `(incr & 0xFF) as i8 as i16`.

---

## 4. CRC Serialization

### C function signatures

```c
// crc.h / crc.c
void crc_init(crc_State *state);
void crc_processUint8(crc_State *state, uint8 val);
void crc_processUint16(crc_State *state, uint16 val);
void crc_processUint32(crc_State *state, uint32 val);
uint32 crc_finish(const crc_State *state);

// checksum.c
void crc_processELEMENT(crc_State *state, const ELEMENT *val);
void crc_processDispQueue(crc_State *state);
void crc_processState(crc_State *state);
```

Source: `sc2/src/uqm/supermelee/netplay/crc.c` and `checksum.c`

### 4a. CRC-32 Streaming Algorithm

#### Pseudocode

```
CONST CRC_TABLE: [u32; 256] = [
    // Polynomial 0x04c11db7 reflected (0xedb88320)
    // Standard CRC-32 table — 256 entries
    0x00000000, 0x77073096, 0xee0e612c, ... // (full table in crc.c)
]

STRUCT CrcState:
    crc: u32

FUNCTION crc_init(state):
    state.crc = 0xFFFFFFFF

FUNCTION crc_process_uint8(state, val: u8):
    state.crc = (state.crc >> 8) XOR CRC_TABLE[(state.crc XOR val) & 0xFF]

FUNCTION crc_process_uint16(state, val: u16):
    // Little-endian: low byte first, then high byte
    state.crc = (state.crc >> 8) XOR CRC_TABLE[(state.crc XOR (val & 0xFF)) & 0xFF]
    state.crc = (state.crc >> 8) XOR CRC_TABLE[(state.crc XOR (val >> 8)) & 0xFF]

FUNCTION crc_process_uint32(state, val: u32):
    // Little-endian: byte 0, byte 1, byte 2, byte 3
    state.crc = (state.crc >> 8) XOR CRC_TABLE[(state.crc XOR (val & 0xFF)) & 0xFF]
    state.crc = (state.crc >> 8) XOR CRC_TABLE[(state.crc XOR ((val >> 8) & 0xFF)) & 0xFF]
    state.crc = (state.crc >> 8) XOR CRC_TABLE[(state.crc XOR ((val >> 16) & 0xFF)) & 0xFF]
    state.crc = (state.crc >> 8) XOR CRC_TABLE[(state.crc XOR (val >> 24)) & 0xFF]

FUNCTION crc_finish(state) -> u32:
    RETURN NOT state.crc    // bitwise complement (~)
```

### 4b. Per-Element Serialization (`crc_processELEMENT`)

#### Pseudocode

```
FUNCTION crc_process_element(state, element):
    // --- Guard: exclude background objects ---
    IF element.state_flags & BACKGROUND_OBJECT:
        RETURN   // Skip entirely — not included in checksum

    // --- Serialize 35 bytes in exact field order ---
    // Field 1: state_flags (ELEMENT_FLAGS = u16) → 2 bytes
    crc_process_uint16(state, element.state_flags)

    // Field 2: life_span (COUNT = u16) → 2 bytes
    crc_process_uint16(state, element.life_span)

    // Field 3: crew_level / hit_points union (COUNT = u16) → 2 bytes
    crc_process_uint16(state, element.crew_level)

    // Field 4: mass_points (BYTE = u8) → 1 byte
    crc_process_uint8(state, element.mass_points)

    // Field 5: turn_wait (BYTE = u8) → 1 byte
    crc_process_uint8(state, element.turn_wait)

    // Field 6: thrust_wait / blast_offset union (BYTE = u8) → 1 byte
    crc_process_uint8(state, element.thrust_wait)

    // Field 7: velocity (VELOCITY_DESC) → 18 bytes total
    //   TravelAngle: COUNT (u16) → 2 bytes
    crc_process_uint16(state, element.velocity.TravelAngle)
    //   vector: EXTENT {width: i16, height: i16} → 4 bytes
    crc_process_uint16(state, element.velocity.vector.width)
    crc_process_uint16(state, element.velocity.vector.height)
    //   fract: EXTENT → 4 bytes
    crc_process_uint16(state, element.velocity.fract.width)
    crc_process_uint16(state, element.velocity.fract.height)
    //   error: EXTENT → 4 bytes
    crc_process_uint16(state, element.velocity.error.width)
    crc_process_uint16(state, element.velocity.error.height)
    //   incr: EXTENT → 4 bytes
    crc_process_uint16(state, element.velocity.incr.width)
    crc_process_uint16(state, element.velocity.incr.height)

    // Field 8: current state — location only (STATE.location = POINT) → 4 bytes
    //   (STATE includes image but image is NOT checksummed)
    crc_process_uint16(state, element.current.location.x)
    crc_process_uint16(state, element.current.location.y)

    // Field 9: next state — location only (STATE.location = POINT) → 4 bytes
    crc_process_uint16(state, element.next.location.x)
    crc_process_uint16(state, element.next.location.y)

    // Total: 2 + 2 + 2 + 1 + 1 + 1 + 18 + 4 + 4 = 35 bytes per element
```

### 4c. Display Queue Iteration (`crc_processDispQueue`)

```
FUNCTION crc_process_disp_queue(state):
    element = GetHeadElement()
    WHILE element != 0:
        elementPtr = LockElement(element)
        crc_process_element(state, elementPtr)
        nextElement = GetSuccElement(elementPtr)
        UnlockElement(element)
        element = nextElement
```

### 4d. Full State Checksum (`crc_processState`)

```
FUNCTION crc_process_state(state):
    // Step 1: Include RNG state
    seed = TFB_SeedRandom(0)        // Read current seed (destructive)
    crc_process_uint32(state, seed)
    TFB_SeedRandom(seed)             // Restore seed

    // Step 2: Walk display queue
    crc_process_disp_queue(state)
```

### Key invariants and edge cases

1. **Byte order is little-endian.** `crc_processUint16` processes the low byte first (`val & 0xFF`), then the high byte (`val >> 8`). This is independent of the host CPU's endianness — the code explicitly extracts bytes.
2. **`BACKGROUND_OBJECT` exclusion** is checked per-element. If the flag is set, zero bytes are contributed to the CRC for that element.
3. **`crc_processSTATE` only includes `location`**, not `image`. The `STATE` struct contains `{location: POINT, image: {frame, farray}}` but `crc_processSTATE` only calls `crc_processPOINT(&val->location)`. Image frames are deliberately excluded from the checksum — they may differ between graphics mods without affecting gameplay.
4. **35-byte field order must be exact.** Any reordering produces a different CRC. The order is: `state_flags`, `life_span`, `crew_level`, `mass_points`, `turn_wait`, `thrust_wait`, velocity (TravelAngle, vector, fract, error, incr), current.location, next.location.
5. **Union fields:** `crew_level` and `hit_points` share storage. The checksum reads whichever value is in the union — the bits are the same regardless of semantic interpretation. Same for `thrust_wait` / `blast_offset`.
6. **`COORD` is `i16`**, cast to `u16` for CRC processing. Signed values are interpreted as their unsigned bit pattern.
7. **RNG inclusion:** `TFB_SeedRandom(0)` reads-and-resets the RNG state, then it's restored. This destructive read is safe only in single-threaded context (battle frame processing).
8. **Polynomial:** CRC-32 with polynomial 0x04c11db7 (reflected form 0xedb88320). This is the standard Ethernet/ZIP CRC-32.

### What Rust must preserve for C parity

- The CRC-32 table must be the exact same 256 entries (standard polynomial).
- `crc_init` sets `0xFFFFFFFF`; `crc_finish` returns `!crc`.
- Byte extraction for uint16/uint32 must be little-endian (low byte first) regardless of platform.
- The 35-byte field serialization order must be followed exactly. Any deviation produces a different checksum and a netplay desync.
- `BACKGROUND_OBJECT` elements contribute zero bytes (not zero-padded — literally skipped).
- `COORD` (signed `i16`) is cast to `u16` without value conversion — just a reinterpretation of the bit pattern. In Rust: `val as u16`.
- The RNG state read/restore via `TFB_SeedRandom()` must call C's function via FFI to access the actual game RNG.
- Display queue traversal order must match C's linked-list order (head to tail via GetSuccElement).

---

## 5. Display List Pool

### Data structures

```c
// displist.h
typedef void* QUEUE_HANDLE;
typedef QUEUE_HANDLE HLINK;        // Handle to a link (pointer into pool)

typedef struct link {
    HLINK pred;                     // Previous element
    HLINK succ;                     // Next element
} LINK;

typedef struct {
    HLINK head;                     // First element in active list
    HLINK tail;                     // Last element in active list
    BYTE *pq_tab;                   // Preallocated pool buffer
    HLINK free_list;                // Head of free chain (singly-linked via succ)
    COUNT object_size;              // Size in bytes of each element
    BYTE num_objects;               // Capacity (max 255)
} QUEUE;
```

The `QUEUE_TABLE` variant is always active (mandatory for gameplay correctness — see comment in `displist.h`).

### 5a. `InitQueue`

#### C function signature

```c
BOOLEAN InitQueue(QUEUE *pq, COUNT num_elements, OBJ_SIZE size);
```

#### Pseudocode

```
FUNCTION InitQueue(pq, num_elements, size) -> bool:
    // --- Step 1: Initialize list pointers ---
    pq.head = NULL
    pq.tail = NULL
    pq.object_size = size
    pq.free_list = NULL

    // --- Step 2: Allocate pool buffer ---
    pq.num_objects = (BYTE)num_elements      // Truncated to u8! Max 255
    pq.pq_tab = HMalloc(object_size * num_objects)
    IF pq.pq_tab == NULL:
        RETURN FALSE

    // --- Step 3: Build free chain (push all slots onto free list) ---
    // Iterates from num_elements down to 1 (1-based indexing)
    FOR i = num_elements DOWNTO 1:
        FreeLink(pq, GetLinkAddr(pq, i))
            // GetLinkAddr(pq, i) = pq.pq_tab + (object_size * (i - 1))
            // FreeLink pushes onto free_list (singly-linked stack)
    // After loop: free_list → slot[0] → slot[1] → ... → slot[N-1] → NULL

    RETURN TRUE
```

### 5b. `ReinitQueue`

```
FUNCTION ReinitQueue(pq):
    // Reset to empty active list, rebuild free chain (no reallocation)
    pq.head = NULL
    pq.tail = NULL
    pq.free_list = NULL

    num_elements = pq.num_objects      // SizeQueueTab
    IF num_elements > 0:
        FOR i = num_elements DOWNTO 1:
            FreeLink(pq, GetLinkAddr(pq, i))
```

### 5c. `AllocLink`

#### C function signature

```c
HLINK AllocLink(QUEUE *pq);
```

#### Pseudocode

```
FUNCTION AllocLink(pq) -> HLINK:
    hLink = pq.free_list
    IF hLink != NULL:
        LinkPtr = LockLink(pq, hLink)      // Cast HLINK → LINK*
        pq.free_list = LinkPtr.succ         // Pop from free stack
        UnlockLink(pq, hLink)              // No-op in QUEUE_TABLE
    ELSE:
        log("AllocLink(): No more elements")

    RETURN hLink    // NULL if pool exhausted
```

### 5d. `FreeLink`

#### C function signature

```c
void FreeLink(QUEUE *pq, HLINK hLink);
```

#### Pseudocode

```
FUNCTION FreeLink(pq, hLink):
    LinkPtr = LockLink(pq, hLink)
    LinkPtr.succ = pq.free_list         // Push onto free stack
    UnlockLink(pq, hLink)
    pq.free_list = hLink
```

### 5e. `PutQueue` (append to tail)

#### C function signature

```c
void PutQueue(QUEUE *pq, HLINK hLink);
```

#### Pseudocode

```
FUNCTION PutQueue(pq, hLink):
    // --- Step 1: Link to existing tail ---
    IF pq.head == NULL:
        // List is empty — new element becomes head
        pq.head = hLink
    ELSE:
        // List non-empty — update old tail's succ pointer
        hTail = pq.tail
        lpTail = LockLink(pq, hTail)
        lpTail.succ = hLink
        UnlockLink(pq, hTail)

    // --- Step 2: Initialize new element's links ---
    LinkPtr = LockLink(pq, hLink)
    LinkPtr.pred = pq.tail      // Previous tail (or NULL if list was empty)
    LinkPtr.succ = NULL         // New tail has no successor
    UnlockLink(pq, hLink)

    // --- Step 3: Update tail ---
    pq.tail = hLink
```

### 5f. `InsertQueue` (insert before reference)

#### C function signature

```c
void InsertQueue(QUEUE *pq, HLINK hLink, HLINK hRefLink);
```

#### Pseudocode

```
FUNCTION InsertQueue(pq, hLink, hRefLink):
    IF hRefLink == NULL:
        // No reference — append to tail
        PutQueue(pq, hLink)
    ELSE:
        LinkPtr = LockLink(pq, hLink)
        RefLinkPtr = LockLink(pq, hRefLink)

        // Step 1: Wire new element into chain
        LinkPtr.pred = RefLinkPtr.pred      // New pred = ref's old pred
        RefLinkPtr.pred = hLink             // Ref's pred = new element
        LinkPtr.succ = hRefLink             // New succ = ref element

        // Step 2: Update head or predecessor's succ
        IF pq.head == hRefLink:
            // Inserting before head — new element becomes head
            pq.head = hLink
        ELSE:
            // Wire predecessor's succ to new element
            hPredLink = LinkPtr.pred
            PredLinkPtr = LockLink(pq, hPredLink)
            PredLinkPtr.succ = hLink
            UnlockLink(pq, hPredLink)

        UnlockLink(pq, hRefLink)
        UnlockLink(pq, hLink)
```

### 5g. `RemoveQueue`

#### C function signature

```c
void RemoveQueue(QUEUE *pq, HLINK hLink);
```

#### Pseudocode

```
FUNCTION RemoveQueue(pq, hLink):
    LinkPtr = LockLink(pq, hLink)

    // --- Step 1: Update head or predecessor ---
    IF pq.head == hLink:
        pq.head = LinkPtr.succ
    ELSE:
        hPredLink = LinkPtr.pred
        PredLinkPtr = LockLink(pq, hPredLink)
        PredLinkPtr.succ = LinkPtr.succ
        UnlockLink(pq, hPredLink)

    // --- Step 2: Update tail or successor ---
    IF pq.tail == hLink:
        pq.tail = LinkPtr.pred
    ELSE:
        hSuccLink = LinkPtr.succ
        SuccLinkPtr = LockLink(pq, hSuccLink)
        SuccLinkPtr.pred = LinkPtr.pred
        UnlockLink(pq, hSuccLink)

    UnlockLink(pq, hLink)
```

### 5h. Traversal helpers

```
FUNCTION GetHeadLink(pq) -> HLINK:  RETURN pq.head
FUNCTION GetTailLink(pq) -> HLINK:  RETURN pq.tail
FUNCTION GetSuccLink(link) -> HLINK: RETURN link.succ
FUNCTION GetPredLink(link) -> HLINK: RETURN link.pred

FUNCTION CountLinks(pq) -> COUNT:
    count = 0
    hLink = pq.head
    WHILE hLink != NULL:
        count += 1
        LinkPtr = LockLink(pq, hLink)
        hLink = LinkPtr.succ
        UnlockLink(pq, hLink)
    RETURN count

FUNCTION ForAllLinks(pq, callback, arg):
    hLink = pq.head
    WHILE hLink != NULL:
        LinkPtr = LockLink(pq, hLink)
        hNextLink = LinkPtr.succ
        callback(LinkPtr, arg)
        UnlockLink(pq, hLink)
        hLink = hNextLink
```

### 5i. Element-level allocation (from `process.c`)

The battle engine wraps `AllocLink`/`FreeLink` with display-primitive management:

```
FUNCTION AllocElement() -> HELEMENT:
    hElement = AllocLink(&disp_q)
    IF hElement != NULL:
        ElementPtr = LockElement(hElement)
        memset(ElementPtr, 0, sizeof(ELEMENT))

        // Allocate a display primitive from the separate free list
        ElementPtr.PrimIndex = AllocDisplayPrim()
            // AllocDisplayPrim() pops from DisplayFreeList
            // DisplayFreeList = GetSuccLink(DisplayArray[DisplayFreeList].links)
        IF ElementPtr.PrimIndex == END_OF_LIST:
            ERROR("Out of display prims!")
            explode()
        SetPrimType(DisplayArray[ElementPtr.PrimIndex], NO_PRIM)

        UnlockElement(hElement)
    RETURN hElement

FUNCTION FreeElement(hElement):
    IF hElement != NULL:
        ElementPtr = LockElement(hElement)
        FreeDisplayPrim(ElementPtr.PrimIndex)
            // FreeDisplayPrim(p): push DisplayArray[p] onto DisplayFreeList
        UnlockElement(hElement)
        FreeLink(&disp_q, hElement)

FUNCTION InitDisplayList():
    // Reset zoom
    zoom_out = <initial value>
    opt_max_zoom_out = <max zoom>

    // Reset element pool (rebuild free chain, empty active list)
    ReinitQueue(&disp_q)

    // Reset display primitive free list (all 330 prims available)
    FOR i = 0 TO MAX_DISPLAY_PRIMS - 1:
        SetPrimLinks(DisplayArray[i], END_OF_LIST, i + 1)
    SetPrimLinks(DisplayArray[MAX_DISPLAY_PRIMS - 1], END_OF_LIST, END_OF_LIST)
    DisplayFreeList = 0
    DisplayLinks = MakeLinks(END_OF_LIST, END_OF_LIST)
```

### Key invariants and edge cases

1. **`QUEUE_TABLE` is mandatory.** The code uses preallocated pools (`pq_tab`), not heap allocation per element. Handles are raw pointers into the pool buffer.
2. **`LockLink` / `UnlockLink` are essentially no-ops** in the `QUEUE_TABLE` variant — they just cast `HLINK` (void pointer) to `LINK*` and assert the pointer is within the pool's bounds. There is no actual locking.
3. **`num_objects` is `BYTE` (u8).** Maximum pool capacity is 255 elements. The battle engine uses 150 (`MAX_DISPLAY_ELEMENTS`).
4. **1-based indexing in `InitQueue`:** `GetLinkAddr(pq, i)` uses `(i - 1)` offset. The loop `i = num_elements DOWNTO 1` ensures slot 0 ends up at the head of the free list (last pushed = first to be allocated).
5. **Free list is singly-linked** via `succ` only. Only the `succ` pointer is used in `AllocLink`/`FreeLink`. The `pred` pointer is only meaningful for elements in the active (doubly-linked) list.
6. **Pool exhaustion:** `AllocLink` returns NULL (0) when `free_list` is empty. `AllocElement` checks for NULL. `AllocDisplayPrim` returns `END_OF_LIST` on exhaustion and triggers a fatal error.
7. **Two separate pools:** The element pool (`disp_q`, 150 slots) and the display primitive array (`DisplayArray`, 330 entries) are independently managed. Each element allocation also allocates a display primitive. The element stores the primitive index in `PrimIndex`.
8. **Display primitive free list** uses the `PrimLinks` within each `PRIMITIVE` entry to form a singly-linked chain (succ link = next free index). `END_OF_LIST` terminates the chain.
9. **`RemoveQueue` does NOT return the element to the free list.** It only unlinks from the active list. Callers must separately call `FreeLink` (or `FreeElement` which calls both `FreeDisplayPrim` and `FreeLink`).
10. **`InsertQueue` with `hRefLink == NULL`** falls through to `PutQueue` (append to tail). This is used when inserting with no specific position requirement.

### What Rust must preserve for C parity

- **Pool layout must be `#[repr(C)]` compatible.** Elements are accessed as raw pointers into a contiguous byte buffer. The `LINK` struct (pred/succ) must be the first two fields of `ELEMENT`.
- **Handle identity:** `HLINK` is a raw pointer (`void*`). The Rust representation must maintain pointer identity so that `hLink == pq.head` comparisons work correctly. In Phase 1, Rust types define the structure but C owns the pool; handles are passed through FFI as opaque pointers.
- **Free list ordering:** `InitQueue` and `ReinitQueue` push slots in reverse order (N down to 1), so slot 0 is at the free list head. Allocation order is thus: slot 0, slot 1, ..., slot N-1. This deterministic order matters for netplay reproducibility.
- **`memset` in `AllocElement`:** The entire element struct is zeroed before use. Rust must ensure the same (e.g., `MaybeUninit::zeroed()` or manual zero initialization of all fields).
- **Display primitive coupling:** Every `AllocElement` must pair with an `AllocDisplayPrim`. Every `FreeElement` must pair with `FreeDisplayPrim`. Leaking either causes pool exhaustion.
- **Assertion in `LockLink`:** The debug assertion verifies `(BYTE*)h >= pq.pq_tab && (BYTE*)h < pq.pq_tab + object_size * num_objects`. Rust should include equivalent bounds checking in debug builds.
