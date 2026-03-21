# Battle Engine Initial State

## Scope and boundary

This document covers the **battle engine runtime** — the core simulation loop, element/entity system, collision detection, velocity physics, weapon mechanics, display list management, tactical transitions, AI dispatch, and ship lifecycle management that together form the real-time combat system.

In scope:

- the top-level battle loop and frame stepping in `/Users/acoliver/projects/uqm/sc2/src/uqm/battle.c` (517 lines)
- per-frame element processing, physics stepping, camera/zoom, and rendering pipeline in `/Users/acoliver/projects/uqm/sc2/src/uqm/process.c` (1108 lines)
- collision detection and elastic response in `/Users/acoliver/projects/uqm/sc2/src/uqm/collide.c` (184 lines) and `/Users/acoliver/projects/uqm/sc2/src/uqm/collide.h` (70 lines)
- the core ELEMENT structure and element flags in `/Users/acoliver/projects/uqm/sc2/src/uqm/element.h` (242 lines)
- velocity/facing calculations in `/Users/acoliver/projects/uqm/sc2/src/uqm/velocity.c` (154 lines) and `/Users/acoliver/projects/uqm/sc2/src/uqm/velocity.h`
- weapon spawning, projectile lifecycle, damage, and tracking in `/Users/acoliver/projects/uqm/sc2/src/uqm/weapon.c` (415 lines) and `/Users/acoliver/projects/uqm/sc2/src/uqm/weapon.h` (68 lines)
- display list (doubly-linked list) management and element allocation in `/Users/acoliver/projects/uqm/sc2/src/uqm/displist.c` (275 lines) and `/Users/acoliver/projects/uqm/sc2/src/uqm/displist.h` (132 lines)
- tactical transitions — ship death sequences, next ship selection, battle end conditions in `/Users/acoliver/projects/uqm/sc2/src/uqm/tactrans.c` (1033 lines) and `/Users/acoliver/projects/uqm/sc2/src/uqm/tactrans.h` (59 lines)
- AI intelligence dispatch and computer player control in `/Users/acoliver/projects/uqm/sc2/src/uqm/intel.c` (77 lines) and `/Users/acoliver/projects/uqm/sc2/src/uqm/intel.h` (85 lines)
- ship runtime within battle — spawn, per-frame update, crew/energy management in `/Users/acoliver/projects/uqm/sc2/src/uqm/ship.c` (592 lines) and `/Users/acoliver/projects/uqm/sc2/src/uqm/ship.h` (44 lines)
- battle initialization and teardown in `/Users/acoliver/projects/uqm/sc2/src/uqm/init.c` (362 lines)
- coordinate and angle systems in `/Users/acoliver/projects/uqm/sc2/src/uqm/units.h` (228 lines)
- the shared runtime contracts in `/Users/acoliver/projects/uqm/sc2/src/uqm/races.h` (676 lines) insofar as they define the data structures consumed by the battle engine

Explicitly out of scope:

- **Individual ship race implementations** under `sc2/src/uqm/ships/*/`. The battle engine calls into race-specific code through the `RACE_DESC` function pointer contract (`preprocess_func`, `postprocess_func`, `init_weapon_func`, `intelligence_func`), but the per-race behavior is documented separately in `project-plans/20260311/ships/initialstate.md`.
- **SuperMelee setup menus and ship selection UI** (`pickmele.c`, `melee.c`). These consume the battle engine's ship queues but own their own UI loops.
- **Netplay transport layer**. Netplay hooks into the battle loop via `#ifdef NETPLAY` blocks for input buffering, frame synchronization, and checksum verification, but the actual network transport protocol is out of scope. This document covers only the battle-side integration points.
- **Resource loading details** (`loadship.c`, `master.c`, `build.c`, `dummy.c`). These are covered in the ships initialstate document.

The boundary is visible in the code:

- `Battle()` in `/Users/acoliver/projects/uqm/sc2/src/uqm/battle.c:396-517` is the single entry point that initializes, runs, and tears down a battle
- `InitShips()` / `UninitShips()` in `/Users/acoliver/projects/uqm/sc2/src/uqm/init.c:181-361` bridge between the battle engine and ship loading
- `RedrawQueue()` in `/Users/acoliver/projects/uqm/sc2/src/uqm/process.c:1012-1061` is the per-frame rendering/simulation entry called from `DoBattle()`
- race-specific code plugs in through `RACE_DESC` callbacks set during `spawn_ship()` at `/Users/acoliver/projects/uqm/sc2/src/uqm/ship.c:393-515`

## Verified port status

The battle engine is **entirely C-owned** in the active build. No Rust battle engine module exists.

### No battle-specific Rust build toggle exists

The Rust bridge flag list in `/Users/acoliver/projects/uqm/sc2/build/unix/build.config` does **not** contain any `USE_RUST_BATTLE` toggle.

Evidence:

- Search for `USE_RUST_BATTLE` across the entire repository returned **zero matches**
- The substitution-variable list in `/Users/acoliver/projects/uqm/sc2/build/unix/build.config:86-108` includes toggles for `USE_RUST_BRIDGE`, `USE_RUST_FILE`, `USE_RUST_CLOCK`, `USE_RUST_UIO`, `USE_RUST_AUDIO`, `USE_RUST_COMM`, `USE_RUST_INPUT`, `USE_RUST_VIDEO`, `USE_RUST_GFX`, `USE_RUST_RESOURCE`, `USE_RUST_THREADS`, `USE_RUST_MIXER`, `USE_RUST_MEM`, `USE_RUST_STATE`, and `USE_RUST_SHIPS`, but nothing for the battle loop, process, collide, velocity, weapon, or display list subsystems

### The Rust crate has no battle module

`/Users/acoliver/projects/uqm/rust/src/lib.rs` exports `comm`, `game_init`, `graphics`, `input`, `io`, `memory`, `resource`, `ships`, `sound`, `state`, `threading`, `time`, and `video`, but no `battle`, `process`, `collide`, `element`, `velocity`, `weapon`, or `displist` module.

### USE_RUST_SHIPS exists but covers ship lifecycle, not the battle engine

`USE_RUST_SHIPS` guards exist in 5 C files (`ship.c`, `init.c`, `build.c`, `master.c`, `loadship.c`) and the toggle is defined (but `#undef`'d by default) in `/Users/acoliver/projects/uqm/sc2/build/unix/build.config:107`. When the Rust bridge is enabled, it is activated at `/Users/acoliver/projects/uqm/sc2/build/unix/build.config:566`. These guards cover ship spawning, initialization, and teardown — not the frame loop, element processing, collision detection, or rendering pipeline.

Evidence of the guards in `init.c`:

- `InitShips()` at line 184: `#ifdef USE_RUST_SHIPS` → `return rust_ships_init();`
- `UninitShips()` at line 279: `#ifdef USE_RUST_SHIPS` → `rust_ships_uninit();`

**Porting risk — COUNT vs SIZE type mismatch**: `InitShips()` is declared as returning `SIZE` (= `SWORD` = `sint16`, signed) at `init.c:181`, and it legitimately uses the sign — `Battle()` at `battle.c:515` tests `num_ships < 0` to detect hyperspace exit. However, the Rust bridge extern at `init.c:39` declares `rust_ships_init()` as returning `COUNT` (= `UWORD` = `uint16`, unsigned). This means a Rust implementation returning a negative value (e.g. `-1` / `0xFFFF` as unsigned) would be silently reinterpreted. A Rust port must either: (a) match the `SIZE` return type exactly in the FFI binding, or (b) fix the C extern to `SIZE` and update the Rust side accordingly. This is an ABI-level mismatch in the existing bridge. Whether it causes problems depends on whether the Rust path ever needs to return negative values (the C path does). This must be verified and resolved during porting.

Evidence of FFI entry points in `ship.c` at lines 38-43:

- `extern void rust_ships_preprocess(ELEMENT *element);`
- `extern void rust_ships_postprocess(ELEMENT *element);`
- `extern void rust_ships_death(ELEMENT *element);`
- `extern BOOLEAN rust_ships_spawn(STARSHIP *starship);`

These are called conditionally in `ship_preprocess` (line 158), `ship_postprocess` (line 295), and `spawn_ship` (line 396). They replace the **per-ship race-specific callbacks**, not the battle engine frame loop itself.

### Existing Rust ships code is race implementations, not battle engine

The `rust/src/ships/` module contains 13 submodules (`c_bridge`, `catalog`, `ffi_contract`, `lifecycle`, `loader`, `queue`, `races`, `registry`, `runtime`, `traits`, `types`, `writeback`, `ffi`) and all 25+ race implementations under `rust/src/ships/races/`. These implement the `ShipBehavior` trait from `rust/src/ships/traits.rs` with `preprocess()`, `postprocess()`, `init_weapon()`, and `intelligence()` methods. This is the Rust equivalent of the per-race C code, **not** the battle engine loop.

## What is and is not ported

### Not ported / still active in C — the entire battle engine

Every file in scope is C-only with no Rust replacement:

- `battle.c` — battle entry, per-frame callback (InputFunc-driven via DoInput), input processing
- `process.c` — element PreProcess/PostProcess pipeline, collision detection, camera/zoom, rendering dispatch
- `collide.c` — elastic collision physics
- `element.h` — ELEMENT struct definition and flags
- `velocity.c` — Bresenham-style velocity system
- `weapon.c` — weapon spawning, damage, projectile tracking
- `displist.c` — doubly-linked list pool allocator
- `tactrans.c` — ship death, explosion, transition, flee, winner tracking
- `intel.c` — AI dispatch
- `ship.c` — ship spawn, per-frame preprocess/postprocess, collision handler
- `init.c` — InitShips/UninitShips, space initialization

### Partially bridged — ship lifecycle via USE_RUST_SHIPS

The ship spawn/init/uninit path has `#ifdef USE_RUST_SHIPS` guards that redirect to Rust. When active, Rust handles ship spawning and race-specific callbacks but still depends on C-owned ELEMENT allocation, display list management, and the entire process.c frame loop.

## Active C-side structure

## Coordinate, angle, and precision systems (units.h)

`/Users/acoliver/projects/uqm/sc2/src/uqm/units.h` defines the fundamental numeric precision chain that pervades the entire battle engine.

### Three coordinate scales

The engine uses three precision levels connected by bit shifts:

1. **Display coordinates** — screen pixels, the coarsest
2. **World coordinates** — 4× display (`ONE_SHIFT=2`, `SCALED_ONE=4`). `DISPLAY_TO_WORLD(x)` = `x << 2`, `WORLD_TO_DISPLAY(x)` = `x >> 2`
3. **Velocity coordinates** — 32× world (`VELOCITY_SHIFT=5`, `VELOCITY_SCALE=32`). `WORLD_TO_VELOCITY(l)` = `l << 5`, `VELOCITY_TO_WORLD(v)` = `v >> 5`

So **1 display pixel = 128 velocity units**. This gives sub-pixel precision for smooth movement.

Evidence: `/Users/acoliver/projects/uqm/sc2/src/uqm/units.h:24-30` defines `ONE_SHIFT=2`, `SCALED_ONE=4`, `DISPLAY_TO_WORLD`, `WORLD_TO_DISPLAY`. `/Users/acoliver/projects/uqm/sc2/src/uqm/velocity.h:27-30` defines `VELOCITY_SHIFT=5`, `VELOCITY_SCALE=32`, `WORLD_TO_VELOCITY`, `VELOCITY_TO_WORLD`.

### Logical space dimensions

- `MAX_REDUCTION = 3`, `MAX_VIS_REDUCTION = 2`, `REDUCTION_SHIFT = 1` at `/Users/acoliver/projects/uqm/sc2/src/uqm/units.h:34-36`
- `NUM_VIEWS = 3` — three zoom levels of pre-rendered sprites at `/Users/acoliver/projects/uqm/sc2/src/uqm/units.h:37`
- `LOG_SPACE_WIDTH = DISPLAY_TO_WORLD(SPACE_WIDTH) << MAX_REDUCTION` at `/Users/acoliver/projects/uqm/sc2/src/uqm/units.h:38`
- `LOG_SPACE_HEIGHT = DISPLAY_TO_WORLD(SPACE_HEIGHT) << MAX_REDUCTION` at `/Users/acoliver/projects/uqm/sc2/src/uqm/units.h:39`
- Continuous zoom: `ZOOM_SHIFT = 8`, `MAX_ZOOM_OUT = (4 << ZOOM_SHIFT)` at `/Users/acoliver/projects/uqm/sc2/src/uqm/units.h:41-42`

### Toroidal wrapping

The battle space wraps toroidally. Macros at `/Users/acoliver/projects/uqm/sc2/src/uqm/units.h:44-66`:

- `WRAP_X(x)` and `WRAP_Y(y)` — wrap position to `[0, LOG_SPACE_WIDTH)` / `[0, LOG_SPACE_HEIGHT)` with modular arithmetic
- `WRAP_DELTA_X(dx)` and `WRAP_DELTA_Y(dy)` — compute shortest-path delta across the torus (if `|delta| > half_dimension`, subtract `dimension`)
- `DISPLAY_ALIGN(x)` — rounds to `SCALED_ONE` boundaries: `((COORD)(x) & ~(SCALED_ONE - 1))`

### Angle and facing systems

At `/Users/acoliver/projects/uqm/sc2/src/uqm/units.h:72-105`:

- **Angles**: `CIRCLE_SHIFT = 6`, `FULL_CIRCLE = 64`, `HALF_CIRCLE = 32`, `QUADRANT = 16`, `OCTANT = 8`. Angles are 0-63 with wraparound.
- **Facings**: `FACING_SHIFT = 4`, giving **16 facing directions** (one per 22.5°). `ANGLE_TO_FACING(a) = ((a) + 2) >> 2` — rounds to nearest facing by adding half a facing step (`1 << (CIRCLE_SHIFT - FACING_SHIFT - 1)` = 2) before right-shifting by `CIRCLE_SHIFT - FACING_SHIFT` = 2. `FACING_TO_ANGLE(f) = (f) << 2` — converts facing to the angle at the start of that facing's range.
- `NORMALIZE_ANGLE(a) = ((COUNT)(a) & (FULL_CIRCLE - 1))`
- `NORMALIZE_FACING(f) = ((COUNT)(f) & (FULL_CIRCLE / (1 << FACING_SHIFT) - 1))`

### Trigonometry

At `/Users/acoliver/projects/uqm/sc2/src/uqm/units.h:107-118`:

- `SIN_SHIFT = 14`, `SIN_SCALE = 16384` — 14-bit fixed-point sine table
- `SINE(angle, magnitude) = ((SIZE)((long)sinetab[NORMALIZE_ANGLE(a)] * (long)(m)) >> SIN_SHIFT)`
- `COSINE(angle, magnitude) = SINE(angle + QUADRANT, magnitude)`
- `ARCTAN(dx, dy)` — lookup-table arctangent returning angle 0-63

### Screen layout

At `/Users/acoliver/projects/uqm/sc2/src/uqm/units.h:120-135`:

- `STATUS_WIDTH = 64` — left-side status panel width
- `SPACE_WIDTH = SCREEN_WIDTH - STATUS_WIDTH` — playfield width
- `SAFE_X = 0`, `SAFE_Y = 0` — no overscan offset
- Universe coordinates max: `MAX_X_UNIVERSE = 9999`, `MAX_Y_UNIVERSE = 9999`

## Core data structure: ELEMENT (element.h)

`/Users/acoliver/projects/uqm/sc2/src/uqm/element.h:104-168` defines the central entity type for the entire battle simulation. Every physical object — ships, weapons, asteroids, crew pickups, explosions, ion trails, blast effects — is an ELEMENT.

### ELEMENT struct layout

```c
struct element {
    // Doubly-linked list pointers (must be first for LINK compatibility)
    HELEMENT pred, succ;                     // lines 107

    // Callback function pointers — the polymorphism mechanism
    ElementProcessFunc *preprocess_func;     // line 109
    ElementProcessFunc *postprocess_func;    // line 110
    ElementCollisionFunc *collision_func;    // line 111
    ElementProcessFunc *death_func;          // line 112

    // Owner
    SIZE playerNr;                           // line 118 — -1=neutral, 0=bottom/human, 1=top/NPC

    // State
    ELEMENT_FLAGS state_flags;               // line 120
    union { COUNT life_span; COUNT scan_node; };  // line 123-124
    union { COUNT crew_level; COUNT hit_points;   // line 128-133
            COUNT facing; COUNT cycle; };
    union { BYTE mass_points; };             // line 136-144
    union { BYTE turn_wait; BYTE sys_loc; }; // line 147-149
    union { BYTE thrust_wait; BYTE blast_offset; BYTE next_turn; }; // line 151-155
    BYTE colorCycleIndex;                    // line 156

    // Physics
    VELOCITY_DESC velocity;                  // line 160
    INTERSECT_CONTROL IntersectControl;      // line 161
    COUNT PrimIndex;                         // line 162 — index into DisplayArray[]

    // Visual state (double-buffered: current frame, next frame)
    STATE current, next;                     // line 163
    // STATE = { POINT location; struct { FRAME frame; FRAME *farray; } image; }

    // Ownership
    void *pParent;                           // line 165 — points to STARSHIP owner
    HELEMENT hTarget;                        // line 167 — for homing weapons
};
```

### ELEMENT_FLAGS bit definitions

At `/Users/acoliver/projects/uqm/sc2/src/uqm/element.h:38-65`:

| Bit | Name | Meaning |
|-----|------|---------|
| `1<<2` | `PLAYER_SHIP` | Element is a player-controlled ship |
| `1<<3` | `APPEARING` | Element is newly spawned, skip first preprocess |
| `1<<4` | `DISAPPEARING` | Element is dying, will be removed |
| `1<<5` | `CHANGING` | Graphical state changed this frame |
| `1<<6` | `NONSOLID` | Skip collision detection |
| `1<<7` | `COLLISION` | Collision already processed this frame |
| `1<<8` | `IGNORE_SIMILAR` | Don't collide with elements sharing same pParent |
| `1<<9` | `DEFY_PHYSICS` | Used for overlapping stationary objects |
| `1<<10` | `FINITE_LIFE` | life_span decrements each frame |
| `1<<11` | `PRE_PROCESS` | PreProcess has been called this frame |
| `1<<12` | `POST_PROCESS` | PostProcess has been called this frame |
| `1<<13` | `IGNORE_VELOCITY` | Don't apply velocity to position |
| `1<<14` | `CREW_OBJECT` | Element is a floating crew pickup |
| `1<<15` | `BACKGROUND_OBJECT` | Purely visual, excluded from netplay checksums |

### Display primitive allocation

At `/Users/acoliver/projects/uqm/sc2/src/uqm/element.h:181-191`:

- `MAX_DISPLAY_ELEMENTS = 150` — hard cap on simultaneous elements
- `MAX_DISPLAY_PRIMS = 330` — hard cap on display primitives (more than elements because some elements can have extra prims)
- `DisplayArray[MAX_DISPLAY_PRIMS]` — global array of PRIMITIVE structs
- `DisplayFreeList` — head of free list chain through DisplayArray
- `AllocDisplayPrim()` / `FreeDisplayPrim(p)` — inline free-list operations using PrimLinks

### Key macros

At `/Users/acoliver/projects/uqm/sc2/src/uqm/element.h:192-217`:

- `GetElementStarShip(e, ppsd)` — extracts STARSHIP* from `e->pParent`
- `SetElementStarShip(e, psd)` — sets `e->pParent`
- `OBJECT_CLOAKED(eptr)` — true if prim type >= NUM_PRIMS or is black STAMPFILL
- `PutElement(h)` = `PutQueue(&disp_q, h)` — append to display list
- `InsertElement(h, i)` = `InsertQueue(&disp_q, h, i)` — insert before position
- `LockElement(h, ppe)` / `UnlockElement(h)` — no-op locks (QUEUE_TABLE mode)

### Key constants

- `NORMAL_LIFE = 1` — default life_span for ships (line 32)
- `HYPERJUMP_LIFE = 15` — warp transition duration (line 69)
- `MAX_SHIP_MASS = 10` — regular ship mass cap (line 197)
- `GRAVITY_MASS(m)` — true if `m > MAX_SHIP_MASS * 10` (line 198)
- `GRAVITY_THRESHOLD = 255` (line 199)
- `MAX_CREW_SIZE = 42`, `MAX_ENERGY_SIZE = 42` (lines 195-196)

## Display list system (displist.h, displist.c)

The battle engine uses a generic doubly-linked list backed by a preallocated table pool (`QUEUE_TABLE` mode is mandatory — enabled by define).

### QUEUE and LINK structures

At `/Users/acoliver/projects/uqm/sc2/src/uqm/displist.h`:

```c
typedef struct { HLINK pred, succ; } LINK;

typedef struct {
    HLINK head, tail;
    BYTE *pq_tab;       // preallocated pool backing store
    HLINK free_list;     // head of free chain
    COUNT object_size;   // size of each element
    BYTE num_objects;    // pool capacity
} QUEUE;
```

`HLINK` = `QUEUE_HANDLE` = `void*`. Addressing is **1-based** via `GetLinkAddr(pq, i)` which returns `&pq->pq_tab[(i-1) * object_size]`.

Lock/Unlock are no-ops in QUEUE_TABLE mode (direct memory access).

### Pool operations (displist.c)

At `/Users/acoliver/projects/uqm/sc2/src/uqm/displist.c`:

- `InitQueue(pq, num_elements, size)` (lines 33-58): allocates pool, chains all slots into free list
- `UninitQueue(pq)` (lines 60-82): frees pool memory
- `ReinitQueue(pq)` (lines 86-105): empties list, rebuilds free chain (does NOT free pool)
- `AllocLink(pq)` (lines 108-126): pops from free list
- `FreeLink(pq, hLink)` (lines 128-138): pushes onto free list
- `PutQueue(pq, hLink)` (lines 141-165): append to tail
- `InsertQueue(pq, hLink, hRefLink)` (lines 167-198): insert before reference
- `RemoveQueue(pq, hLink)` (lines 200-236): unlink from list
- `CountLinks(pq)` (lines 238-257): count by traversal
- `ForAllLinks(pq, callback, arg)` (lines 259-272): iterate with callback

### Battle display queue

The battle engine uses a single global queue:

- `QUEUE disp_q` — the display list for all active ELEMENTs
- Allocated in `InitContexts()` at `/Users/acoliver/projects/uqm/sc2/src/uqm/setup.c:183` via `InitQueue(&disp_q, MAX_DISPLAY_ELEMENTS, sizeof(ELEMENT))` with `MAX_DISPLAY_ELEMENTS = 150` slots
- Reset at battle start by `InitDisplayList()` at `/Users/acoliver/projects/uqm/sc2/src/uqm/process.c:985-1008`, which calls `ReinitQueue(&disp_q)` (empties the list and rebuilds the free chain without reallocating)

## Velocity system (velocity.h, velocity.c)

### VELOCITY_DESC structure

At `/Users/acoliver/projects/uqm/sc2/src/uqm/velocity.h`:

```c
typedef struct {
    COUNT TravelAngle;    // current angle of travel (0-63)
    EXTENT vector;        // world-coordinate integer part {width, height}
    EXTENT fract;         // fractional part (Bresenham remainder)
    EXTENT error;         // Bresenham error accumulator
    EXTENT incr;          // Bresenham increment encoding
} VELOCITY_DESC;
```

This is a **Bresenham-style fixed-point accumulator**. Rather than floating-point velocity components, the system decomposes velocity into integer world units per frame (`vector`) plus a fractional part (`fract`) accumulated via `error` with direction sign encoded in `incr`.

### Velocity functions (velocity.c)

At `/Users/acoliver/projects/uqm/sc2/src/uqm/velocity.c`:

- `GetCurrentVelocityComponents(vel, &dx, &dy)` (lines 28-34): reads current velocity as velocity-scale components. Formula: `dx = WORLD_TO_VELOCITY(vector.width) + (fract.width - HIBYTE(incr.width))`
- `GetNextVelocityComponents(vel, &dx, &dy, num_frames)` (lines 37-55): computes position delta for N frames. Accumulates `fract * num_frames` into error, triggers sub-pixel step when `error >> VELOCITY_SHIFT`. Mutates `error` (side effect).
- `SetVelocityVector(vel, magnitude, facing)` (lines 58-96): sets velocity from magnitude (world coords) and facing (0-15). Converts to angle, applies COSINE/SINE at velocity scale, splits into vector (world) + fract (remainder) + incr (sign-encoded step direction).
- `SetVelocityComponents(vel, dx, dy)` (lines 99-140): sets velocity from velocity-scale dx/dy directly. Computes TravelAngle via ARCTAN.
- `DeltaVelocityComponents(vel, dx, dy)` (lines 143-152): **adds** dx/dy to current velocity. Reads current, adds, calls SetVelocityComponents.
- `ZeroVelocityComponents(vel)` = `memset(vel, 0, sizeof(*vel))` — macro in header
- `IsVelocityZero(vel)` — checks that vector + incr + fract are all zero

### Critical implementation detail: incr encoding

The `incr` field uses a packed encoding at `/Users/acoliver/projects/uqm/sc2/src/uqm/velocity.c:71-91`:

- **Positive direction**: `incr = MAKE_WORD(1, 0)` — LOBYTE=1 (step direction), HIBYTE=0
- **Negative direction**: `incr = MAKE_WORD(0xFF, remainder<<1)` — LOBYTE=0xFF (step = -1 cast to SBYTE), HIBYTE=doubled remainder

This encoding is FFI-critical and must be replicated exactly in Rust.

## Collision system (collide.h, collide.c)

### Collision eligibility macros

At `/Users/acoliver/projects/uqm/sc2/src/uqm/collide.h:28-39`:

```c
#define SKIP_COLLISION (NONSOLID | DISAPPEARING)
#define CollidingElement(e) (!((e)->state_flags & SKIP_COLLISION))
#define CollisionPossible(e0, e1) (
    CollidingElement(e0)
    && !(((e1)->state_flags & (e0)->state_flags) & COLLISION)   // not both already collided
    && (!(((e1)->state_flags & (e0)->state_flags) & IGNORE_SIMILAR)
        || (e1)->pParent != (e0)->pParent)                      // different owners for same-ship weapons
    && ((e1)->mass_points || (e0)->mass_points)                  // at least one has mass
)
```

### Intersection initialization macros

At `/Users/acoliver/projects/uqm/sc2/src/uqm/collide.h:41-62`:

- `InitIntersectStartPoint(eptr)`: sets `IntersectControl.IntersectStamp.origin` to `WORLD_TO_DISPLAY(current.location)`
- `InitIntersectEndPoint(eptr)`: sets `IntersectControl.EndPoint` to `WORLD_TO_DISPLAY(next.location)`
- `InitIntersectFrame(eptr)`: sets `IntersectControl.IntersectStamp.frame` to `SetEquFrameIndex(next.image.farray[0], next.image.frame)` — selects the equivalent frame in the base zoom-level sprite sheet

### Elastic collision response (collide.c)

At `/Users/acoliver/projects/uqm/sc2/src/uqm/collide.c:30-183`:

The `collide()` function implements mass-based elastic collision response:

1. **Impact angle** = `ARCTAN(pos0 - pos1)` (line 41)
2. **Relative velocity** = velocity0 - velocity1, `speed = sqrt(dx² + dy²)` (lines 44-51)
3. **Directness check** = `NORMALIZE_ANGLE(RelTravelAngle - ImpactAngle0)`. If ≤ QUADRANT or ≥ 3*QUADRANT → scraping collision, fudge to `HALF_CIRCLE` (lines 53-62)
4. **DEFY_PHYSICS handling** = if both objects stationary and overlapping, sets DEFY_PHYSICS|COLLISION, fudges angles by `HALF_CIRCLE - OCTANT` (lines 72-92)
5. **Momentum transfer**: `scalar = SINE(Directness, speed*2) * mass0 * mass1` (line 100)
6. **Per-object velocity change** (applied to each non-GRAVITY_MASS object):
   - `speed = scalar / (massN * (mass0 + mass1))` (lines 120, 157)
   - `DeltaVelocityComponents(velocity, COSINE(ImpactAngle, speed), SINE(ImpactAngle, speed))` (lines 121-123, 158-160)
7. **Minimum velocity enforcement**: if resulting velocity is less than `SCALED_ONE` in world coords, set to `WORLD_TO_VELOCITY(SCALED_ONE) - 1` along impact angle (lines 131-136, 168-173)
8. **Player ship penalty**: on collision, clears `SHIP_AT_MAX_SPEED | SHIP_BEYOND_MAX_SPEED`, adds `COLLISION_TURN_WAIT=1` to turn_wait and `COLLISION_THRUST_WAIT=3` to thrust_wait (lines 104-117, 141-154)
9. **Gravity mass exemption**: objects with `GRAVITY_MASS(mass+1)` (mass > 100) are immovable (lines 102, 139)

## Weapon system (weapon.h, weapon.c)

### Weapon descriptor structures

At `/Users/acoliver/projects/uqm/sc2/src/uqm/weapon.h:29-49`:

```c
typedef struct {
    COORD cx, cy, ex, ey;        // start position (cx,cy), end offset (ex,ey) for lasers
    ELEMENT_FLAGS flags;
    SIZE sender;                  // player number
    SIZE pixoffs;                 // pixel offset from ship center (along facing)
    COUNT face;                   // facing direction (0-15)
    Color color;                  // beam color
} LASER_BLOCK;

typedef struct {
    COORD cx, cy;                // spawn position (world coords)
    ELEMENT_FLAGS flags;
    SIZE sender;
    SIZE pixoffs, speed, hit_points, damage;
    COUNT face, index, life;     // facing, frame index, life span
    FRAME *farray;               // sprite array pointer
    void (*preprocess_func)(ELEMENT *);  // optional per-frame behavior
    SIZE blast_offs;              // offset for blast effect
} MISSILE_BLOCK;
```

### Laser initialization (weapon.c:44-85)

`initialize_laser(LASER_BLOCK*)`:
- Allocates ELEMENT via `AllocElement()`, sets `LINE_PRIM` type
- `life_span = LASER_LIFE = 1` (single frame)
- Position = `(cx, cy)` + offset along facing via `COSINE/SINE(FACING_TO_ANGLE(face), DISPLAY_TO_WORLD(pixoffs))`
- Velocity = endpoint - startpoint as velocity components (the laser "moves" to the endpoint in one frame, which effectively sweeps the line segment)
- `collision_func = weapon_collision_cb`

### Missile initialization (weapon.c:87-132)

`initialize_missile(MISSILE_BLOCK*)`:
- Allocates ELEMENT via `AllocElement()`, sets `STAMP_PRIM` type
- Configurable `hit_points`, `damage` (→ mass_points), `life`, `speed`, `preprocess_func`
- Position = spawn point + offset along facing
- Velocity = `COSINE/SINE(angle, WORLD_TO_VELOCITY(speed))`
- **Critical**: backs up position by one velocity step (`location -= VELOCITY_TO_WORLD(delta)`) so that the missile doesn't start one frame ahead of where it should visually appear (lines 126-127)

### Weapon collision (weapon.c:134-246)

`weapon_collision(WeaponElementPtr, pWPt, HitElementPtr, pHPt)` → returns `HELEMENT` (blast element or 0):

1. **Guard**: if `COLLISION` flag already set, return 0 (prevents double-hit, line 141)
2. **Damage application**: calls `do_damage(HitElementPtr, damage)` if target has `FINITE_LIFE` or `NORMAL_LIFE` (lines 145-158)
3. **Sound**: plays `TARGET_DAMAGED_FOR_1_PT..6_PLUS_PT` scaled by damage amount (lines 166-173)
4. **Weapon destruction**: non-LINE_PRIM weapons get `DISAPPEARING`; all get `COLLISION|NONSOLID`, `hit_points=0`, `life_span=0` (lines 175-181)
5. **Blast effect creation**: allocates blast ELEMENT at weapon collision point (lines 183-241):
   - Position = `DISPLAY_TO_WORLD(pWPt)` + offset along weapon travel angle
   - Blast direction index = 8 directional bins from weapon travel angle (lines 210-213)
   - If weapon has ≤ 16 blast frames → uses shared `blast[]` sprite, `life_span=2`
   - If weapon has > 16 blast frames → uses weapon's own sprite frames as animated blast with `animation_preprocess`, `life_span = num_frames - 16` (lines 224-236)

### Damage silhouette (weapon.c:248-309)

`ModifySilhouette(ElementPtr, modify_stamp, modify_flags)`:
- Uses `DrawablesIntersect()` in a rejection-sampling loop to find random positions within the ship's silhouette (lines 275-285)
- Renders damage indicators on the status panel

### Homing / tracking (weapon.c:318-413)

`TrackShip(Tracker, &pfacing)` → returns `SIZE` (delta_facing or -1):

1. If `Tracker->hTarget` is set, check that target first (fast path, lines 328-334)
2. Otherwise, iterate all elements looking for enemy `PLAYER_SHIP` elements (lines 337-396)
3. Cloaked ships (`OBJECT_CLOAKED`) are invisible unless tracker is a ship with `APPEARING` flag (lines 343-346)
4. Distance = Manhattan approximation `|dx| + |dy|` with `WRAP_DELTA_X/Y` for toroidal shortest path (lines 357-385)
5. Returns delta facing to closest target. If delta == HALF_CIRCLE (directly behind), random left/right. Otherwise ±1 facing adjustment (lines 398-410)

## Process loop — the heart of the battle engine (process.c)

`/Users/acoliver/projects/uqm/sc2/src/uqm/process.c` is the 1108-line file that implements the per-frame simulation. It contains element lifecycle management, collision detection, camera control, zoom level calculation, coordinate transforms, and rendering dispatch.

### Top-level: RedrawQueue (lines 1012-1061)

`RedrawQueue(BOOLEAN clear)` is called every frame from `DoBattle()`:

```
SetContext(StatusContext)
PreProcessQueue()     → returns VIEW_STATE, modifies dx/dy
PostProcessQueue()    → builds render list, removes dead elements
UpdateSoundPositions()
SetContext(SpaceContext)
if (frame skip check passes):
    ClearDrawable()
    CALC_ZOOM_STUFF(&idx, &sc) → compute zoom index and fractional scale
    SetGraphicScale(sc)
    DrawBatch(DisplayArray, DisplayLinks, 0)  → render all primitives
    SetGraphicScale(0)
FlushSounds()
```

### PreProcessQueue (lines 629-746)

First pass over all elements. For each element:

1. **Call PreProcess()** if not yet preprocessed (line 660)
2. **Run collision detection** via `ProcessCollisions()` if element is collidable (lines 665-668)
3. **Track PLAYER_SHIP positions** for camera (lines 669-703): maintains min/max X/Y across all ships to compute viewport
4. **Compute zoom** via `CalcReduction(ship_distance)` (line 711)
5. **Compute camera origin** as midpoint between ships (lines 716-723)
6. **Return VIEW_STATE** from `CalcView()` (line 744)

### PreProcess (lines 128-186)

Called for each element per frame:

1. If `life_span == 0`: call `Untarget()` to clear references, set `DISAPPEARING`, call `death_func` (lines 133-141)
2. If not DISAPPEARING and APPEARING: call `SetUpElement()` for initial intersection setup. For `PLAYER_SHIP` elements, clear APPEARING in the **local** `state_flags` copy only (line 151) — `ElementPtr->state_flags` retains APPEARING so callbacks can detect first-frame (lines 146-152)
3. If not APPEARING (in local copy): call `preprocess_func` — note that `ElementPtr->state_flags` still has APPEARING set for PLAYER_SHIP on first frame (lines 154-161)
4. If not `IGNORE_VELOCITY`: apply velocity via `GetNextVelocityComponents()` to compute next position (lines 163-175)
5. If collidable: `InitIntersectEndPoint()` (lines 177-178)
6. If `FINITE_LIFE`: decrement `life_span` (lines 180-181)
7. Set `PRE_PROCESS` flag, clear `POST_PROCESS|COLLISION` (lines 184-186)

### PostProcess (lines 188-204)

Called after collision detection:

1. Call `postprocess_func` if set (lines 191-192)
2. Copy `next` → `current` (line 193)
3. If collidable: reinit intersection start/end points for next frame (lines 195-199)
4. Set `POST_PROCESS` flag, clear `PRE_PROCESS|CHANGING|APPEARING` (lines 201-203)

### ProcessCollisions (lines 361-627)

Recursive collision detection between element pairs:

1. Iterate successor elements from current position (lines 367-374)
2. Check `CollisionPossible()` (line 382)
3. Call `DrawablesIntersect()` for pixel-accurate intersection test with time of collision (line 397)
4. **Stuck object handling** (lines 400-516): if elements are intersecting at time=1 with identical frames:
   - APPEARING elements are killed immediately (lines 427-449)
   - Non-APPEARING elements get their frames reverted to current state (lines 455-506)
5. **Recursive deeper collision check**: before dispatching collision callbacks, recursively check if either element would hit something *earlier* (lines 531-540)
6. **Collision dispatch** (lines 549-570): Collision handlers are called in pairs. The dispatch order depends on whether the `TestElementPtr` (the element found during forward iteration) has `PLAYER_SHIP`:
   - If test element is `PLAYER_SHIP` → test's `collision_func` called first, then element's
   - Otherwise → element's `collision_func` called first, then test's
   In `PreProcessQueue`, each element only checks collisions against its **successors** in the display list (`ProcessCollisions(hNextElement, ...)`  at line 667). In `PostProcessQueue`, newly-added elements check against the **entire** list (`ProcessCollisions(GetHeadElement(), ...)` at line 858). Which element is "element" vs "test" depends on display list insertion order. The `PLAYER_SHIP` check ensures the ship's collision handler always runs first regardless of which role the ship has in a given pair.

7. **Post-collision position update**: elements that got COLLISION flag have their position snapped to the collision point (lines 572-609)
8. **Physical bounce**: calls `collide()` for non-FINITE_LIFE pairs (line 601)

### PostProcessQueue (lines 798-983)

Second pass — rendering setup and cleanup:

1. Clear `DEFY_PHYSICS` and `COLLISION` flags (lines 824-825)
2. **Newly-added elements** (no `PRE_PROCESS` flag — lines 842-870): when the outer loop reaches an element that was added during PreProcessQueue (or during this loop itself), it enters an inner loop starting from that element and iterating all remaining elements to the end of the list. For each element without `PRE_PROCESS`, it calls `PreProcess()` (line 853), then runs `ProcessCollisions(GetHeadElement(), ...)` against the **entire** display list (line 858). **Critical cascading behavior**: because `PreProcess` calls `preprocess_func`, which can itself spawn new elements via `PutElement()` (appended to the tail), and the inner loop follows successor links to the end of the list (`hPostElement != 0`), elements spawned during this inner loop's PreProcess are also reached and get their own PreProcess and collision detection in the same frame. This cascade continues until no more new elements appear. After the inner loop, `scroll_x`/`scroll_y` are zeroed because newly-added elements are already in adjusted coordinates (lines 864-869). The outer loop then re-reads `state_flags` from the first new element (line 870) and continues with coordinate transform and PostProcess for it.
3. **DISAPPEARING elements**: call `RemoveElement()` then `FreeElement()` (lines 873-879)
4. **Coordinate transform**: call `CalcDisplayCoord()` to convert world → screen coordinates with zoom
5. **LINE_PRIM**: transform both endpoints, handle wrap-around (lines 890-920)
6. **STAMP_PRIM / STAMPFILL_PRIM**: select zoom-level frame from `farray[zoom_index]` via `SetEquFrameIndex()`, optionally set up trilinear mipmap (lines 922-963)
7. **POINT_PRIM**: simple coordinate transform (lines 948-956)
8. Call `PostProcess()` callback (line 969)
9. `InsertPrim()` into display-ordered linked list for rendering (line 972)

### CalcReduction — zoom level (lines 206-281)

Two zoom modes:

1. **Step mode** (`optMeleeScale == TFB_SCALE_STEP`): discrete zoom levels 0/1/2 based on ship separation via shift comparison with `TRANSITION_WIDTH/HEIGHT`, with hysteresis to prevent oscillation (lines 215-248)
2. **Continuous mode**: smooth zoom with ZOOM_SHIFT precision, linear interpolation of ship distance → zoom factor, clamped to `[1<<ZOOM_SHIFT, MAX_ZOOM_OUT]` (lines 250-274)

### CalcView — camera position (lines 283-358)

Computes viewport origin and scroll delta:

1. `dx/dy` = distance from center of logical space to new scroll point (midpoint between ships)
2. Single-ship mode: clamp scroll speed to `ORG_JUMP_X/Y` for smooth follow (lines 297-309)
3. Hyperspace: calls `MoveSIS()` for overworld movement (line 312)
4. If zoom changed: recalculates `SpaceOrg` based on new zoom level (lines 318-344)
5. If zoom is same and no scroll: `VIEW_STABLE`; scroll only: `VIEW_SCROLL`; zoom changed: `VIEW_CHANGE`

### CalcDisplayCoord — world to screen (lines 785-796)

Two formulas based on zoom mode:

- **Step**: `screen = (world - SpaceOrg) >> reduction_level`
- **Continuous**: `screen = ((world - SpaceOrg) << ZOOM_SHIFT) / zoom_out`

### InitDisplayList (lines 985-1008)

Called at battle start. Note: the actual allocation of `disp_q` happens earlier in `InitContexts()` at `setup.c:183` via `InitQueue(&disp_q, MAX_DISPLAY_ELEMENTS, sizeof(ELEMENT))`. `InitDisplayList()` only resets the existing pool:

1. Sets initial `zoom_out` and `opt_max_zoom_out`
2. `ReinitQueue(&disp_q)` — empties the element list and rebuilds the free chain (does NOT allocate — the pool already exists)
3. Chains `DisplayArray[0..MAX_DISPLAY_PRIMS-1]` into free list
4. Resets `DisplayLinks` head/tail to `END_OF_LIST`

### InsertPrim (lines 748-781)

Inserts a display primitive into the rendering order linked list (`DisplayLinks`). Primitives are ordered by their display position for proper draw ordering.

### Untarget (lines 1063-1091)

When an element dies, iterates all elements and clears any `hTarget` pointing to it:

```c
for each element in disp_q:
    if element->hTarget == dying_handle:
        element->hTarget = 0
```

### RemoveElement (lines 1093-1106)

Removes element from `disp_q` and cleans up stereo sound position tracking.

## Battle entry and frame loop (battle.c)

### Architecture: callback-driven, not self-looping

The battle frame function `DoBattle` does **not** own its own loop. It is an `InputFunc` callback driven by the engine-wide `DoInput()` polling loop defined in `gameinp.c:361-412`. `DoInput()` is a generic cooperative loop used throughout UQM — for menus, solar system, comm screens, and battle alike. It repeatedly calls `((INPUT_STATE_DESC*)pInputState)->InputFunc(pInputState)` until the callback returns FALSE (see `gameinp.c:408`).

The `BATTLE_STATE` struct's first field is the `InputFunc` pointer, which satisfies the `INPUT_STATE_DESC` layout convention that `DoInput()` relies on (it casts the `void*` to access the function pointer at offset 0). `Battle()` sets `bs.InputFunc = DoBattle` at `battle.c:467`, then enters the loop via `DoInput(&bs, FALSE)` at `battle.c:472`. Each invocation of `DoBattle` processes exactly one battle frame. `DoBattle` returns TRUE to keep the loop running, FALSE to exit the battle.

This callback-driven architecture means a Rust port must either: (a) replicate the `DoInput` loop and `InputFunc` callback contract, (b) implement `DoBattle` as a callback compatible with a C-side `DoInput`, or (c) restructure into an explicit loop. Option (c) is cleanest for Rust but requires careful handling of the frame timing, input polling, and async processing that `DoInput` coordinates.

### Battle() — the top-level entry point

At `/Users/acoliver/projects/uqm/sc2/src/uqm/battle.c:396-517`:

```
Battle(callback):
    1. Seed RNG (time-based or journal seed)
    2. BattleSong(FALSE) — load music
    3. InitShips() → num_ships
    4. Handle instantVictory shortcut
    5. If num_ships > 0:
       a. Set IN_BATTLE in CurrentActivity
       b. Count battle_counter[0] and [1] from race_q links
       c. Set graphic scale mode
       d. setupBattleInputOrder()
       e. [NETPLAY: init buffers, checksums, frame count]
       f. selectAllShips(num_ships) — pick and spawn first ships
       g. BattleSong(TRUE) — start music
       h. Set bs.InputFunc = DoBattle (line 467)
       i. DoInput(&bs, FALSE) — enters DoBattle callback loop (line 472)
    6. Cleanup: StopDitty, StopMusic, StopSound
    7. UninitShips()
    8. FreeBattleSong()
    9. Return whether battle was a hyperspace exit
```

### BATTLE_STATE structure

At `/Users/acoliver/projects/uqm/sc2/src/uqm/battle.h:37-42`:

```c
typedef struct battlestate_struct {
    BOOLEAN (*InputFunc)(struct battlestate_struct *pInputState);
    BOOLEAN first_time;
    DWORD NextTime;
    BattleFrameCallback *frame_cb;
} BATTLE_STATE;
```

The `InputFunc` field **must be the first member** — `DoInput()` (at `gameinp.c:408`) casts the opaque `void *pInputState` to `INPUT_STATE_DESC*` and calls `->InputFunc(pInputState)`. This is a pervasive convention across all UQM input state structs (`MELEE_STATE`, `SOLARSYS_STATE`, `GETMELEE_STRUCT`, `TEXTENTRY_STATE`, etc.) — they all place `InputFunc` at offset 0.

### DoBattle — the per-frame callback

At `/Users/acoliver/projects/uqm/sc2/src/uqm/battle.c:258-354`:

```
DoBattle(bs):
    1. [NETPLAY: checksum computation and verification]
    2. ProcessInput() — read controller inputs for all sides
    3. If first_time: SetTransitionSource
    4. BatchGraphics()
    5. Call frame callback if set
    6. RedrawQueue(TRUE) — THE MAIN SIMULATION + RENDER STEP
       (always called — frame-skip/render suppression happens inside
       RedrawQueue via nth_frame in process.c:1030-1050)
    7. If first_time: screen transition effect
    8. UnbatchGraphics()
    9. Check IN_BATTLE / CHECK_ABORT — return FALSE to exit
    10. Frame timing:
       - max speed: Async_process() + TaskSwitch() — skips sleep entirely;
         rendering is also fully suppressed inside RedrawQueue because
         skip_frames == 0xFF causes the DrawBatch block to be skipped
         (process.c:1033 check `skip_frames != (BYTE)~0` is false)
       - normal: SleepThreadUntil(NextTime + BATTLE_FRAME_RATE/(speed+1))

    11. Return TRUE to continue
```

`BATTLE_FRAME_RATE = ONE_SECOND / 24` — 24 fps target (line 57 of battle.h).

### ProcessInput — input handling

At `/Users/acoliver/projects/uqm/sc2/src/uqm/battle.c:144-226`:


For each side (in `battleInputOrder` order):
1. For each STARSHIP in `race_q[player]`:
2. Call `PlayerInput[player]->handlers->frameInput()` to get `BATTLE_INPUT_STATE`
3. [NETPLAY: buffer input, push/pop for delay]
4. Map `BATTLE_INPUT_STATE` bits to `STATUS_FLAGS`: `BATTLE_LEFT→LEFT`, `BATTLE_RIGHT→RIGHT`, `BATTLE_THRUST→THRUST`, `BATTLE_WEAPON→WEAPON`, `BATTLE_SPECIAL→SPECIAL`
5. If escape allowed and pressed: call `DoRunAway()`

### DoRunAway — escape sequence

At `/Users/acoliver/projects/uqm/sc2/src/uqm/battle.c:72-105`:

Sets up the flee sequence:
- Decrements `battle_counter[0]`
- Sets `preprocess_func = flee_preprocess`, `mass_points = MAX_SHIP_MASS * 10` (marks as "running away")
- Zeros velocity, sets initial color to dark red, changes prim to `STAMPFILL_PRIM`

### Key globals

At `/Users/acoliver/projects/uqm/sc2/src/uqm/battle.c:49-61`:

- `BYTE battle_counter[NUM_SIDES]` — ships remaining per side
- `BOOLEAN instantVictory` — skip battle flag
- `size_t battleInputOrder[NUM_SIDES]` — input processing order (local first for netplay)
- `BattleFrameCounter battleFrameCount` (netplay only) — global frame counter for sync

## Ship runtime within battle (ship.c)

### spawn_ship — creating a ship element

At `/Users/acoliver/projects/uqm/sc2/src/uqm/ship.c:393-515`:

1. `load_ship(SpeciesID, TRUE)` — load battle-ready RACE_DESC (line 402)
2. Patch crew from `StarShipPtr->crew_level` (lines 415-424)
3. `AllocElement()` — create ELEMENT (line 434)
4. Set `state_flags = APPEARING | PLAYER_SHIP | IGNORE_SIMILAR` (line 450)
5. Assign function pointers: `preprocess_func = ship_preprocess`, `postprocess_func = ship_postprocess`, `death_func = ship_death`, `collision_func = collision` (lines 501-504)
6. **Random position placement** (lines 491-498): picks random location avoiding:
   - Gravity wells (planet collision via `CalculateGravity`)
   - Matter conflicts via `TimeSpaceMatterConflict`
   - If Sa-Matra: forces center position instead (lines 459-470)
7. Set initial velocity=zero, image frame from ShipFacing, mass from characteristics, life_span=NORMAL_LIFE (lines 449-505)
8. Wire `pParent = StarShipPtr` via `SetElementStarShip`, `StarShipPtr->hShip = hShip` (lines 431-439, 507)

### ship_preprocess — per-frame ship update

At `/Users/acoliver/projects/uqm/sc2/src/uqm/ship.c:155-290`:

1. **APPEARING initialization** (lines 171-231): set `crew_level`, `ShipFacing`, apply `InitShipStatus`, `ship_transition`
2. **Status flags** from input state (lines 168-175)
3. **Energy regeneration** (lines 234-239): `energy_counter` ticks down, then `DeltaEnergy(+energy_regeneration)`
4. **Race-specific preprocess** callback (lines 241-245): `RaceDescPtr->preprocess_func(ElementPtr)`
5. **Turning** (lines 247-263): if `turn_wait == 0` and LEFT/RIGHT input, ±1 to ShipFacing, set image frame to new facing, apply `turn_wait` from characteristics
6. **Thrust** (lines 265-285): if `thrust_wait == 0` and THRUST input, call `inertial_thrust()`, spawn `spawn_ion_trail()`, apply `thrust_wait`
7. Call `PreProcessStatus(ElementPtr)` for status bar update (lines 287-288)

### ship_postprocess — weapon firing

At `/Users/acoliver/projects/uqm/sc2/src/uqm/ship.c:292-364`:

1. **Weapon firing** (lines 307-353): if `weapon_counter == 0` and WEAPON input and `DeltaEnergy(-weapon_energy_cost)` succeeds:
   - Call `RaceDescPtr->init_weapon_func(ElementPtr, Weapon)` → returns COUNT of spawned weapons into `HELEMENT Weapon[6]` array (line 316). The signature is `COUNT (*init_weapon_func)(ELEMENT *ElementPtr, HELEMENT Weapon[])` (defined in `races.h:208`).
   - Wire each weapon element to parent ship (line 335)
   - Play weapon sound (lines 336-339)
   - Apply `weapon_wait` cooldown (lines 351-352)
2. **Special counter** decrement (lines 355-356)
3. **Race-specific postprocess** callback (lines 358-359): `RaceDescPtr->postprocess_func(ElementPtr)`
4. Call `PostProcessStatus(ElementPtr)` (lines 361-362)

### inertial_thrust — physics model

At `/Users/acoliver/projects/uqm/sc2/src/uqm/ship.c:61-153`:

1. **Inertialess mode** (thrust_increment == max_thrust): instant velocity to max speed along current facing (lines 75-86)
2. **Normal mode**: computes velocity², compares against max_thrust² × WORLD_TO_VELOCITY² (lines 89-113)
3. **Gravity well override**: allows up to `MAX_ALLOWED_SPEED = WORLD_TO_VELOCITY(DISPLAY_TO_WORLD(18)) = 2304` velocity units (lines 95-99)
4. **At-max-speed angle change**: if at max speed, applies half-thrust in new direction minus full-thrust in old direction for gradual turning (lines 117-144)
5. Returns status flags: `SHIP_AT_MAX_SPEED`, `SHIP_BEYOND_MAX_SPEED`, `SHIP_IN_GRAVITY_WELL`

### collision — default ship collision handler

At `/Users/acoliver/projects/uqm/sc2/src/uqm/ship.c:366-391`:

For non-FINITE_LIFE collisions with ships:
- If hit by gravity well (`GRAVITY_MASS(target)`): `do_damage(ship, hit_points >> 2)` (minimum 1)
- Otherwise: no direct damage (elastic collision handles velocity changes)

### GetNextStarShip / GetInitialStarShips

At `/Users/acoliver/projects/uqm/sc2/src/uqm/ship.c:518-593`:

- `GetNextStarShip(LastStarShipPtr, which_side)` (lines 518-552): selects the next ship from `race_q[which_side]` via `GetEncounterStarShip()`. Handles ship recycling (infinite ship worlds) and element reuse (`StarShipPtr->hShip = LastStarShipPtr->hShip`).
- `GetInitialStarShips()` (lines 554-593): spawns initial ships for both sides. In SUPER_MELEE: calls `GetInitialMeleeStarShips()` then spawns in `GetPlayerOrder` sequence. In non-SUPER_MELEE (encounters): simply loops `for (i = NUM_PLAYERS; i > 0; --i)` calling `GetNextStarShip(NULL, i - 1)` for both sides — no special human-picks/NPC-first logic, both sides use the same `GetEncounterStarShip` path.

## Tactical transitions (tactrans.c)

### Ship death pipeline

The death sequence is a multi-frame state machine:

```
ship_death()
  → StopAllBattleMusic()
  → clear PLAY_VICTORY_DITTY on dying ship
  → StartShipExplosion()
  → FindAliveStarShip() → SetWinnerStarShip()
  → RecordShipDeath()
```

#### StartShipExplosion (lines 703-727)

- Zero velocity
- Drain all energy via `DeltaEnergy(-energy_level)`
- Set `life_span = NUM_EXPLOSION_FRAMES * 3 = 36` frames
- Set FINITE_LIFE | NONSOLID (cannot be hit during explosion)
- Wire `preprocess_func = explosion_preprocess`, `death_func = cleanup_dead_ship`
- Play `SHIP_EXPLODES` sound

#### explosion_preprocess (lines 543-616)

Spawns 1-3 random explosion debris particles per frame for 36 frames:

- Frame 0-2: 1 particle; frames 3-5, 18-19: 2 particles; frames 6-14, 16-17, 20+: 3 particles
- At frame 15: hides the ship (`SetPrimType(NO_PRIM)`)
- At frame 25: clears preprocess_func (explosion ends)
- Each particle: `NEUTRAL_PLAYER_NUM`, `life_span=9`, uses `explosion[]` frame array, random position within 16px of ship, random velocity within 5px/frame, `animation_preprocess` for frame cycling

#### cleanup_dead_ship (lines 288-374)

Called when explosion finishes:

1. Record final crew count (line 302)
2. Iterate all elements: clear ownership for dead ship's elements, set them up for deletion (`NONSOLID|DISAPPEARING|FINITE_LIFE`, clear all callbacks) (lines 307-337)
3. But preserve `CREW_OBJECT` elements with `crew_preprocess` — floating crew stays alive
4. If winner has `PLAY_VICTORY_DITTY`: play victory music (lines 339-346)
5. Set `death_func = new_ship`, keep element alive for `MIN_DITTY_FRAME_COUNT = (ONE_SECOND * 3) / BATTLE_FRAME_RATE` (lines 358-373)
6. Winner ship kept alive one frame longer than loser to ensure winning side picks last

#### new_ship — death_func after cleanup (lines 441-540)

1. Wait for `readyForBattleEnd()` — ditty must finish, netplay sync must complete (lines 447-461)
2. Stop all music/sound
3. Free dead ship's `RACE_DESC` via `free_ship()` (line 476)
4. `UpdateShipFragCrew()` — record final crew in fleet (line 503)
5. Deactivate dead ship (`SpeciesID = NO_ID`) (line 505)
6. `GetNextStarShip()` — spawn replacement ship (line 508)
7. If no ships left (`battle_counter[x] == 0`): clear `IN_BATTLE` to end battle (lines 526-530)

### Winner tracking

- `winnerStarShip` static variable tracks who won (lines 49, 661-680)
- `FindAliveStarShip(deadShip)` (lines 625-659): iterates the display list from head to tail looking for the **first** `PLAYER_SHIP` element that is not the `deadShip` and not fleeing (`mass_points <= MAX_SHIP_MASS + 1`, which excludes fleeing ships at `MAX_SHIP_MASS * 10`). **Breaks immediately on the first qualifying element** (line 652) — it does not search by side or iterate further. If that element's `crew_level == 0` and it is not a reincarnating Pkunk (`mass_points != MAX_SHIP_MASS + 1`), the function returns NULL (both ships are dead). This means the "winner" is determined by display list insertion order, not by side index. In practice, there are only two `PLAYER_SHIP` elements (one per side), so the first non-dead one found is the opponent — but a Rust port must preserve the display-list-order dependency, not assume side-based lookup.
- `SetWinnerStarShip()` (lines 667-680): only sets once per battle (first `ship_death()` call determines winner; if both ships die, the second call is a no-op because `winnerStarShip != NULL`)
- `RecordShipDeath()` (lines 683-700): decrements `battle_counter[playerNr]`, calls `MeleeShipDeath()` in SuperMelee

### OpponentAlive — naming trap

`OpponentAlive(TestStarShipPtr)` at `tactrans.c:54-75` has **counterintuitive semantics** that make its name misleading for porters. Despite the name suggesting "is the opponent alive?", it actually answers: "is there NO other ship element in the display list with `crew_level == 0`?"

The function iterates the **entire display list** (not just `PLAYER_SHIP` elements), and for each element with a non-NULL `StarShipPtr` that is not the `TestStarShipPtr`, it checks if `crew_level == 0`. If it finds such an element, it returns **FALSE**. If it reaches the end without finding one, it returns **TRUE**.

This means `OpponentAlive` returns TRUE in **three** cases: (a) the opponent ship is alive (`crew_level > 0`), (b) no opponent ship exists at all, or (c) all other ships in the display list are the same ship. It returns FALSE only when a different ship with zero crew is found.

Its sole usage is at `tactrans.c:474` in `new_ship()`: `RestartMusic = OpponentAlive(DeadStarShipPtr)`. Here it controls whether battle music restarts after a ship death — music restarts only if the opponent is still alive (the battle will continue with a new ship). If the opponent is also dead (simultaneous kill), no music restart.

The naming trap for Rust porters: do NOT implement this as a simple side-based check like `race_q[other_side].crew > 0`. The function scans the display list and checks element ownership, which means it could interact with elements beyond the two main ships (though in practice only two `PLAYER_SHIP` elements exist). Preserve the display-list iteration to maintain exact behavioral parity.

### Ion trail

`spawn_ion_trail()` at lines 792-849:
- Creates `POINT_PRIM` element at ship's rear (HALF_CIRCLE from facing)
- `death_func = cycle_ion_trail` — 12-color orange→red fade, each color held for `ION_LIFE=1` frame
- Inserted at **head** of display list (drawn behind everything)
- Pre-processed immediately (`PRE_PROCESS` set, life_span pre-decremented) because head-inserted elements skip normal preprocessing

### Ship warp transition

`ship_transition()` at lines 855-961:
- **Warp in**: `APPEARING` → `life_span = HYPERJUMP_LIFE = 15`, hide ship (`NO_PRIM`), set `NONSOLID|FINITE_LIFE`
- Each frame spawns a ghost image (`STAMPFILL_PRIM` with `START_ION_COLOR`) along facing vector
- At `life_span == NORMAL_LIFE`: materialize ship — show stamp, restore `ship_preprocess/ship_postprocess`, clear NONSOLID|FINITE_LIFE
- `TRANSITION_SPEED = DISPLAY_TO_WORLD(40)` — distance between ghost images

### Flee sequence

`flee_preprocess()` at lines 963-1033:
- 20-color red pulse cycle (dark→bright→dark)
- `turn_wait` decrements; when it hits 0, advance color cycle
- `thrust_wait` decrements each full color cycle — controls pulse speed (starts slow, accelerates)
- When `thrust_wait == 0` and color reaches midpoint: trigger warp-out via `ship_transition` with `crew_level=0`
- All control input suppressed during flee (lines 1028-1029)

## Battle initialization and teardown (init.c)

### InitShips (lines 181-250)

When `USE_RUST_SHIPS` is NOT defined:

1. `InitSpace()` — load shared assets (stars, explosions, blasts, asteroids)
2. Set graphics contexts (StatusContext, SpaceContext)
3. `InitDisplayList()` — reset element pool and primitive array
4. `InitGalaxy()` — set up star background
5. **Hyperspace mode** (`inHQSpace()`): reinit race queues, build SIS ship, load hyperspace, return 1
6. **Battle mode**: set up clip rect, clear background, spawn asteroids (5) and planet (1) (or `free_gravity_well()` for Sa-Matra battle), return `NUM_SIDES = 2`

### UninitShips (lines 276-361)

When `USE_RUST_SHIPS` is NOT defined:

1. `StopSound()`, `UninitSpace()` (free shared assets)
2. Count floating crew elements (`CountCrewElements()`)
3. Iterate all elements: find surviving ship, add floating crew to its crew count (lines 297-333)
4. Record final crew in `StarShipPtr->crew_level` (line 327)
5. Free each ship's `RACE_DESC` via `free_ship(..., TRUE, TRUE)` (line 329)
6. Clear `IN_BATTLE` flag (line 335)
7. For encounters: call `UpdateShipFragCrew()` to persist crew counts (lines 337-349)
8. For non-encounters: reinit race queues, free hyperspace if needed (lines 351-359)

### InitSpace (lines 118-148)

Reference-counted initialization (`space_ini_cnt`):

- Loads `stars_in_space` (star field mask)
- Loads `explosion[NUM_VIEWS]` — 3 zoom levels of explosion sprites
- Loads `blast[NUM_VIEWS]` — 3 zoom levels of blast sprites
- Loads `asteroid[NUM_VIEWS]` — 3 zoom levels of asteroid sprites

## AI dispatch (intel.c, intel.h)

### computer_intelligence (intel.c:31-74)

The AI entry point called from `ProcessInput()`:

1. In `IN_LAST_BATTLE`: returns 0 (no AI for Sa-Matra) (line 36)
2. If `StarShipPtr != NULL` (in battle, selecting action):
   - `CYBORG_CONTROL`: calls `tactical_intelligence(context, StarShipPtr)` for actual AI behavior (line 43)
   - RPG player overlay: merges `BATTLE_ESCAPE` from human input (lines 46-48)
   - Non-cyborg: direct human input via `CurrentInputToBattleInput()` (line 51)
3. If `StarShipPtr == NULL` (selecting ship):
   - `PSYTRON_CONTROL` in SUPER_MELEE: sleep half second, return `BATTLE_WEAPON` (random ship pick) (lines 56-63)

### AI constants (intel.h:31-48)

- `CLOSE_RANGE_WEAPON = DISPLAY_TO_WORLD(50)` — 200 world units
- `LONG_RANGE_WEAPON = DISPLAY_TO_WORLD(1000)` — 4000 world units
- `FAST_SHIP = 150`, `MEDIUM_SHIP = 45`, `SLOW_SHIP = 25` — maneuverability indices
- `WORLD_TO_TURN(d) = d >> 6` — convert distance to turning frames

### AI object tracking indices

`EVALUATE_DESC` objects are indexed by concern type (lines 43-49):
- `ENEMY_SHIP_INDEX = 0`
- `CREW_OBJECT_INDEX = 1`
- `ENEMY_WEAPON_INDEX = 2`
- `GRAVITY_MASS_INDEX = 3`
- `FIRST_EMPTY_INDEX = 4`

### Control flag definitions (intel.h:67-78)

- `HUMAN_CONTROL = 1<<0`
- `CYBORG_CONTROL = 1<<1` — computer fights battles
- `PSYTRON_CONTROL = 1<<2` — computer selects ships
- `NETWORK_CONTROL = 1<<3`
- `STANDARD_RATING = 1<<4`, `GOOD_RATING = 1<<5`, `AWESOME_RATING = 1<<6` — AI difficulty

### tactical_intelligence

Declared in `intel.h:53` but implemented in `sc2/src/uqm/cyborg.c` (out of scope). Takes `ComputerInputContext` and `STARSHIP*`, returns `BATTLE_INPUT_STATE`. This is where the actual AI decision-making happens — evaluating threats, choosing movement, firing weapons.

## Thread and timing interactions

### Battle runs on the game thread

The battle runs within `DoInput()` at `gameinp.c:361-412`, a generic cooperative polling loop on the main game thread. `DoInput` calls `DoBattle` as an `InputFunc` callback once per iteration — `DoBattle` does not contain its own loop:

- `DoInput(&bs, FALSE)` at `/Users/acoliver/projects/uqm/sc2/src/uqm/battle.c:472`
- `DoBattle()` is called once per frame; returns TRUE to continue, FALSE to end
- Frame timing via `SleepThreadUntil(NextTime + BATTLE_FRAME_RATE / (speed + 1))` at `/Users/acoliver/projects/uqm/sc2/src/uqm/battle.c:342-344`
- At maximum speed (`battle_speed == (BYTE)~0`): `Async_process() + TaskSwitch()` replaces the sleep timer (battle.c lines 335-338). `RedrawQueue(TRUE)` is still called — simulation (PreProcessQueue + PostProcessQueue) runs normally. However, rendering (DrawBatch) is fully suppressed inside RedrawQueue because `HIBYTE(nth_frame) == 0xFF` fails the `skip_frames != (BYTE)~0` check at process.c line 1033, skipping the entire draw block. Sounds still flush (process.c line 1052).


### TaskSwitch / SleepThread

The battle engine uses cooperative multitasking:

- `TaskSwitch()` — yields to other tasks (one occurrence in battle.c at line 338)
- `SleepThreadUntil()` — yields until a timestamp (one occurrence at line 342)
- `SleepThread(ONE_SECOND >> 1)` — used in PSYTRON_CONTROL AI ship selection (intel.c line 61)
- `BatchGraphics()` / `UnbatchGraphics()` — bracket rendering operations to batch draw commands (battle.c lines 314/327, tactrans.c lines 481/538)

### Graphics interaction

The battle engine's primary graphics interface is:

1. **DisplayArray[]** — global primitive array indexed by `ELEMENT.PrimIndex`
2. **Primitive types**: `STAMP_PRIM` (sprites), `STAMPFILL_PRIM` (colored sprites), `LINE_PRIM` (lasers), `POINT_PRIM` (particles), `NO_PRIM` (hidden)
3. **Rendering pipeline**: `RedrawQueue()` → `ClearDrawable()` → `SetGraphicScale()` → `DrawBatch(DisplayArray, DisplayLinks, 0)` → `SetGraphicScale(0)` at process.c lines 1038-1049
4. **Zoom frames**: each ELEMENT's `image.farray[]` contains 3 zoom levels (NUM_VIEWS). `PostProcessQueue()` selects the appropriate frame via `SetEquFrameIndex()` based on current zoom index (process.c lines 875-946)
5. **Contexts**: `StatusContext` for status panel, `SpaceContext` for battle viewport (process.c lines 1015-1032)

### Netplay integration

Netplay hooks are concentrated in three areas:

1. **Input buffering** (battle.c lines 183-194): `BattleInputBuffer` push/pop with configurable delay
2. **Frame checksum** (battle.c lines 268-303): CRC computation and verification at `NETPLAY_CHECKSUM_INTERVAL` frame intervals
3. **Battle end synchronization** (tactrans.c lines 108-278): multi-phase protocol — `NetState_inBattle` → `NetState_endingBattle` → `NetState_endingBattle2` → `NetState_interBattle` — to synchronize ship death timing across network

### Netplay checksum — exact serialized ELEMENT fields

The netplay checksum is computed in `sc2/src/uqm/supermelee/netplay/checksum.c`. A Rust port **must** serialize exactly the same fields in the same order to maintain netplay compatibility.

`crc_processState()` (checksum.c:182-196) checksums two things:
1. **RNG state** — current random seed via `TFB_SeedRandom(0)` round-trip (as `DWORD` / uint32)
2. **Display queue** — all elements in `disp_q` iteration order

`crc_processELEMENT()` (checksum.c:106-131) serializes per element:
- Elements with `BACKGROUND_OBJECT` flag are **entirely skipped** (no bytes contributed)
- For all other elements, the following fields are included in this exact order:

| Field | C type | Serialized as | Size |
|-------|--------|---------------|------|
| `state_flags` | `ELEMENT_FLAGS` | uint16 | 2 bytes |
| `life_span` | `COUNT` | uint16 | 2 bytes |
| `crew_level` | `COUNT` | uint16 | 2 bytes |
| `mass_points` | `BYTE` | uint8 | 1 byte |
| `turn_wait` | `BYTE` | uint8 | 1 byte |
| `thrust_wait` | `BYTE` | uint8 | 1 byte |
| `velocity.TravelAngle` | `COUNT` | uint16 | 2 bytes |
| `velocity.vector` | `EXTENT` | 2×uint16 (width, height) | 4 bytes |
| `velocity.fract` | `EXTENT` | 2×uint16 | 4 bytes |
| `velocity.error` | `EXTENT` | 2×uint16 | 4 bytes |
| `velocity.incr` | `EXTENT` | 2×uint16 | 4 bytes |
| `current.location` | `POINT` | 2×uint16 (x, y) | 4 bytes |
| `next.location` | `POINT` | 2×uint16 (x, y) | 4 bytes |

**Fields NOT included in checksum** (critical — do NOT add these):
- `playerNr`, `PrimIndex`, `colorCycleIndex`
- `IntersectControl` (commented out with `#if 0` in checksum.c:88-98)
- `current.image` / `next.image` (STATE only checksums location, not image/frame — checksum.c:102-104)
- `pParent`, `hTarget`, linked list pointers (`pred`, `succ`)
- All function pointers (`preprocess_func`, `postprocess_func`, `collision_func`, `death_func`)

Total per non-background element: **35 bytes** of CRC input. The CRC processes bytes via `crc_processUint8/16/32` (in `crc.c:96-135`) which extract bytes using explicit bit shifts (`val & 0xff`, `val >> 8`, etc.) in **little-endian order** regardless of platform endianness — a Rust port can use `to_le_bytes()` or replicate the shift-and-mask pattern.

The checksum type is `uint32` (typedef `Checksum` in `checksum.h:25`). Verification happens at `DoBattle` time: the local checksum is compared against remote checksums, and a mismatch triggers `CHECK_ABORT` and connection reset (battle.c:297-299).

## Key invariants and gotchas for Rust port

### 1. Fixed-point arithmetic must be exact

The velocity system uses Bresenham-style integer accumulation (`velocity.c`). The `incr` field's packed encoding (`MAKE_WORD(LOBYTE_sign, HIBYTE_remainder)`) and the error accumulation in `GetNextVelocityComponents()` must produce **bit-identical results** to the C code, as any drift will cause netplay desyncs and subtly different game behavior.

### 2. Element processing order matters

`PreProcessQueue()` iterates elements from head to tail. Elements added during processing (via `PutElement`) go to the tail and are processed in `PostProcessQueue()`. Elements inserted at the head (via `InsertElement`, used for ion trails) get `PRE_PROCESS` set immediately and skip normal preprocessing. This ordering affects collision detection priority and visual layering.

### 3. Collision dispatch order is asymmetric

In `ProcessCollisions()` at process.c lines 549-570, collision handlers are called in pairs with order determined by `PLAYER_SHIP`:

- If the test element is `PLAYER_SHIP`: **test's collision_func is called first**, then element's
- Otherwise: **element's collision_func is called first**, then test's

Which element is "element" (the one being checked) vs "test" (found by forward iteration) depends on display list position. In `PreProcessQueue`, each element checks only its successors (`ProcessCollisions(hNextElement, ...)` at line 667). In `PostProcessQueue`, newly-added elements check the entire list (`ProcessCollisions(GetHeadElement(), ...)` at line 858). The `PLAYER_SHIP` check guarantees the ship's collision handler always runs first regardless of which role the ship occupies in the pair. This matters for shields and special damage handling.

### 4. The COLLISION flag prevents re-processing

Once an element gets `COLLISION` flag from a collision, it won't be tested for new collisions in the same frame (checked in `CollisionPossible()`). This is reset in `PostProcessQueue()` (process.c line 825). A weapon that hits something gets `COLLISION|NONSOLID` to prevent it from hitting anything else.

### 5. Display primitive allocation is separate from element allocation

Elements are allocated from `disp_q` (150 slots). Display primitives are allocated from `DisplayArray` (330 slots) via `AllocDisplayPrim()`. One element always has one prim, but the prim free list is managed independently.

### 6. The double-buffer pattern (current/next)

Each ELEMENT has `current` and `next` STATE. During PreProcess, the `next` state is computed. After PostProcess, `next` is copied to `current`. Collision detection uses the intersection between `current` position (start point) and `next` position (end point). This means collision detection operates on the *trajectory* within a single frame, not point-in-time positions.

### 7. APPEARING elements skip their first preprocess

At process.c line 154: `if (preprocess_func && !(state_flags & APPEARING))`. When an element first appears, it gets `SetUpElement()` (intersection setup) but not its preprocess_func. Exception: `PLAYER_SHIP` elements clear `APPEARING` in the **local** `state_flags` variable before the check (line 151), so ships DO get preprocessed on their first frame. Critically, this only clears the local copy — `ElementPtr->state_flags` **still has APPEARING set** when `ship_preprocess` is called (line 156). The callback can detect first-frame status by checking `ElementPtr->state_flags & APPEARING`. The local copy (with APPEARING cleared) is written back to `ElementPtr->state_flags` at line 184, after the preprocess callback returns.

### 8. Death is a two-step process

An element's death_func is called when `life_span == 0` during PreProcess (process.c line 139). The death_func can:
- Set `DISAPPEARING` to remove the element (default)
- Extend life by setting new `life_span` and clearing `DISAPPEARING` (done by ion trail cycling, ditty waiting)
- Replace itself with a new death_func (done by `cleanup_dead_ship` → `new_ship`)

### 9. Toroidal wrapping is applied in PostProcessQueue, not during velocity stepping

Velocity stepping in `PreProcess()` (process.c lines 163-175) does **not** apply wrapping — `ElementPtr->next.location` is updated with raw velocity deltas, so positions can temporarily exceed `[0, LOG_SPACE_WIDTH)`. Wrapping is applied later in `PostProcessQueue()` (process.c lines 899-900, 915-916) where positions are wrapped via `WRAP_X()`/`WRAP_Y()` and the wrapped result is written back to `ElementPtr->next.location` at line 966. `WRAP_DELTA_X/Y` is used for camera distance calculations (process.c lines 295-296, 683-685). `velocity.c` contains no wrapping logic at all — it is purely arithmetic on velocity components. `CalcDisplayCoord()` converts world coordinates to screen coordinates relative to `SpaceOrg` but does not itself wrap.


### 10. GRAVITY_MASS objects are immovable

Objects with `mass_points > MAX_SHIP_MASS * 10 = 100` (planets) cannot be moved by collisions (collide.c lines 102, 139). Ships running away also use `mass_points = MAX_SHIP_MASS * 10 = 100` which is exactly at the threshold — `GRAVITY_MASS(100+1) = TRUE`.

### 11. Zoom-level frame selection modifies the element

In `PostProcessQueue()` (process.c lines 875-946), the element's `next.image.frame` is **mutated** to point to the zoom-appropriate frame from `farray[zoom_index]`. This is a rendering concern leaking into simulation state. A Rust port should consider separating render state from simulation state.

### 12. Pkunk reincarnation special case

`FindAliveStarShip()` (tactrans.c line 640-648) first checks that the element is a non-fleeing `PLAYER_SHIP` (`mass_points <= MAX_SHIP_MASS + 1`). Then, if `crew_level == 0`, it checks `mass_points != MAX_SHIP_MASS + 1` — a Pkunk ship that is "reincarnating" has `mass_points = MAX_SHIP_MASS + 1 = 11` and is considered alive despite `crew_level == 0`. The function breaks on the first qualifying `PLAYER_SHIP` (line 652), so display list order determines which ship is checked — it does not search both sides.

### 13. Fleet infinity check

`FleetIsInfinite(playerNr)` determines whether crew/ship tracking should be skipped. Infinite fleets (SuperMelee) don't persist crew changes.

## Dependencies on other subsystems

### Graphics subsystem

- `DisplayArray[]`, `DisplayFreeList`, `DisplayLinks` — primitive storage
- `DrawBatch()` — renders all primitives
- `SetGraphicScale()` / `SetGraphicScaleMode()` — zoom rendering
- `BatchGraphics()` / `UnbatchGraphics()` — command batching
- `ClearDrawable()` — clear frame buffer
- `SetContext()` / `SetContextFGFrame()` / `SetContextClipRect()` / `SetContextBackGroundColor()` — drawing context management
- `SetTransitionSource()` / `ScreenTransition()` — screen transition effects
- Frame operations: `GetFrameIndex()`, `SetAbsFrameIndex()`, `SetEquFrameIndex()`, `DecFrameIndex()`, `GetFrameRect()`, `GetFrameCount()` — sprite frame manipulation
- `DrawablesIntersect()` — pixel-accurate intersection test (used for collision detection)
- `TFB_DrawScreen_SetMipmap()` — trilinear filtering for smooth zoom transitions
- Primitive operations: `SetPrimType()`, `GetPrimType()`, `SetPrimColor()`, `GetPrimColor()`, `SetPrimLinks()`, `GetPrimLinks()`

### Threading subsystem

- `TaskSwitch()` — cooperative yield
- `SleepThreadUntil()` / `SleepThread()` — timed yield
- `DoInput()` — cooperative input loop framework

### Audio subsystem

- `PlaySound()` / `StopSound()` / `ProcessSound()` — sound effects
- `PlayMusic()` / `StopMusic()` — background music
- `CalcSoundPosition()` / `UpdateSoundPositions()` / `RemoveSoundObjectPosition()` — stereo positioning
- `FlushSounds()` — commit pending sounds
- `PLRPlaying()` — check if music is still playing
- `SetMenuSounds()` — disable menu sounds during battle

### Input subsystem

- `PlayerInput[]` / `PlayerControl[]` — per-player input handlers
- `CurrentInputToBattleInput()` — convert raw input to battle input state
- `frameInput()` handler — polymorphic input (human, computer, network)

### Resource subsystem

- `LoadGraphic()` / `LoadMusic()` — asset loading
- `CaptureDrawable()` / `ReleaseDrawable()` / `DestroyDrawable()` — drawable lifecycle
- `DestroyMusic()` — music cleanup

### Ship/race subsystem

- `RACE_DESC` function pointers: `preprocess_func`, `postprocess_func`, `init_weapon_func`, `uninit_func`
- `STARSHIP` queue objects in `race_q[NUM_SIDES]`
- `load_ship()` / `free_ship()` — ship resource management
- `DeltaEnergy()` — energy management
- `InitShipStatus()` / `PreProcessStatus()` / `PostProcessStatus()` — status bar

### Global state

- `GLOBAL(CurrentActivity)` — game activity flags (`IN_BATTLE`, `CHECK_ABORT`, `CHECK_LOAD`, `IN_ENCOUNTER`, `IN_LAST_BATTLE`, `SUPER_MELEE`)
- `GET_GAME_STATE()` — game state variables
- `TFB_Random()` / `TFB_SeedRandom()` — pseudo-random number generator
- `inHQSpace()` / `inHyperSpace()` / `inQuasiSpace()` — space type detection

## Integration points — exact function signatures

### Entry into battle engine from outside

```c
// battle.c:396 — single entry point for all combat
BOOLEAN Battle(BattleFrameCallback *callback);

// battle.c:234 — load/play battle music
void BattleSong(BOOLEAN DoPlay);

// battle.c:251 — free battle music
void FreeBattleSong(void);

// init.c:181 — initialize space, display list, spawn environment
SIZE InitShips(void);

// init.c:276 — tear down battle, record results
void UninitShips(void);
```

### Battle engine calling into ship subsystem

```c
// ship.c:518 — select and spawn next ship for a side
BOOLEAN GetNextStarShip(STARSHIP *LastStarShipPtr, COUNT which_side);

// ship.c:554 — spawn initial ships for both sides
BOOLEAN GetInitialStarShips(void);

// ship.c:393 — create ship ELEMENT from STARSHIP (static, not directly callable)
static BOOLEAN spawn_ship(STARSHIP *StarShipPtr);
```

### Battle engine calling into race-specific code (via RACE_DESC)

```c
// Called from ship_preprocess (ship.c:241-243)
void (*preprocess_func)(ELEMENT *ElementPtr);

// Called from ship_postprocess (ship.c:358-359)
void (*postprocess_func)(ELEMENT *ElementPtr);

// Called from ship_postprocess (ship.c:316) — fills Weapon[] array, returns count
COUNT (*init_weapon_func)(ELEMENT *ElementPtr, HELEMENT Weapon[]);

// Called from free_ship (loadship.c)
void (*uninit_func)(RACE_DESC *RaceDescPtr);

// Called from computer_intelligence (intel.c:43)
BATTLE_INPUT_STATE (*intelligence_func)(ComputerInputContext *, STARSHIP *);
```

### Battle engine callbacks wired per-element

```c
// Common ship callbacks (set in spawn_ship, ship.c:501-504)
void ship_preprocess(ELEMENT *ElementPtr);   // ship.c:155
void ship_postprocess(ELEMENT *ElementPtr);  // ship.c:292
void ship_death(ELEMENT *ShipPtr);           // tactrans.c:730
void collision(ELEMENT *ElementPtr0, POINT *pPt0,  // ship.c:366
               ELEMENT *ElementPtr1, POINT *pPt1);

// Weapon callbacks (set in initialize_laser/initialize_missile, weapon.c)
static void weapon_collision_cb(ELEMENT *, POINT *, ELEMENT *, POINT *);

// Transition callbacks (set in ship_transition/flee_preprocess, tactrans.c)
void ship_transition(ELEMENT *ElementPtr);     // tactrans.c:855
void flee_preprocess(ELEMENT *ElementPtr);     // tactrans.c:963

// Death/lifecycle callbacks (set in tactrans.c)
void cleanup_dead_ship(ELEMENT *DeadShipPtr);  // tactrans.c:288
void new_ship(ELEMENT *DeadShipPtr);           // tactrans.c:441
void explosion_preprocess(ELEMENT *ShipPtr);   // tactrans.c:543
void cycle_ion_trail(ELEMENT *ElementPtr);     // tactrans.c:756

// Shared utility callbacks
void animation_preprocess(ELEMENT *ElementPtr);  // ship.c:46
void crew_preprocess(ELEMENT *ElementPtr);        // element.h declaration
void crew_collision(ELEMENT *E0, POINT *P0, ELEMENT *E1, POINT *P1); // element.h
```

### Battle engine calling into graphics

```c
// process.c:1048 — render all display primitives
void DrawBatch(PRIMITIVE *DisplayArray, PRIM_LINKS DisplayLinks, int flags);

// process.c:1045 — set zoom rendering scale
void SetGraphicScale(COUNT scale);

// battle.c:435 — set zoom mode
void SetGraphicScaleMode(int mode);

// process.c:1038 — clear frame buffer
void ClearDrawable(void);

// process.c:397 — pixel-accurate intersection test
TIME_VALUE DrawablesIntersect(INTERSECT_CONTROL *, INTERSECT_CONTROL *, TIME_VALUE max_time);
```

### Battle engine calling into audio

```c
// process.c:1029 — update stereo positions for all sound objects
void UpdateSoundPositions(void);

// process.c:1052 — commit pending sounds
void FlushSounds(void);

// tactrans.c:724 — play positioned sound effect
void PlaySound(SOUND sound, SDWORD position, ELEMENT *element, BYTE priority);

// weapon.c:171 — play positioned sound from specific element
void ProcessSound(SOUND sound, ELEMENT *element);
```

### USE_RUST_SHIPS FFI bridge (when enabled)

```c
// init.c:39-41 — ship initialization/teardown
// WARNING: rust_ships_init() is declared COUNT (unsigned) but InitShips() returns SIZE (signed).
// See "COUNT vs SIZE type mismatch" note in the USE_RUST_SHIPS section above.
extern COUNT rust_ships_init(void);
extern void rust_ships_uninit(void);
```

These FFI entry points replace the race-specific behavior within the ship lifecycle, but the battle engine loop (`process.c`, `battle.c`, `collide.c`, `weapon.c`, `tactrans.c`, `velocity.c`, `displist.c`) remains entirely in C with no corresponding Rust toggle or bridge.
