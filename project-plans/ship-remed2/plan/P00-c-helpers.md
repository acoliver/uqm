# P00 — Add C Helper Functions to rust_bridge_ships.c

## Goal

Add three C helper functions that encapsulate the C-side battle engine operations needed by the Rust FFI lifecycle entry points. Add a corresponding header with prototypes so these symbols are visible across translation units. Verify all symbols compile, link, and export correctly before any Rust wiring.

## Prerequisite

None. This is the first phase.

## Static/Internal Symbol Inventory (C1)

Before copying function bodies from `init.c` and `ship.c`, every symbol used by those bodies must be classified. If a symbol is `static` or TU-local in its origin file, it must be made accessible to `rust_bridge_ships.c` (via header, forward declaration, or re-implementation).

### Symbols used by `rust_bridge_spawn_element()` (from ship.c spawn_ship lines 431-514)

| Symbol | Origin | Visibility | Action Required |
|--------|--------|-----------|-----------------|
| `AllocElement()` | element.c | Global (element.h) | Include `element.h` |
| `InsertElement()` | element.c | Global (element.h) | Include `element.h` |
| `GetHeadElement()` | element.c | Global (element.h) | Include `element.h` |
| `LockElement()` | element.c | Global (element.h) | Include `element.h` |
| `UnlockElement()` | element.c | Global (element.h) | Include `element.h` |
| `SetPrimType()` | macro in gfxlib.h | Macro | Include `libs/gfxlib.h` (via element.h) |
| `DisplayArray` | element.c | Global (element.h) | Include `element.h` |
| `SetAbsFrameIndex()` | gfxlib | Global | Include `libs/gfxlib.h` |
| `NORMALIZE_FACING()` | macro in units.h | Macro | Include `units.h` |
| `TFB_Random()` | mathlib | Global | Include `libs/mathlib.h` |
| `inHQSpace()` | hyper.h | Macro/Global | Include `hyper.h` |
| `GLOBAL()` | globdata.h | Macro | Include `globdata.h` |
| `WRAP_X()`, `WRAP_Y()` | macro in units.h | Macro | Include `units.h` |
| `DISPLAY_ALIGN_X()`, `DISPLAY_ALIGN_Y()` | macro in units.h | Macro | Include `units.h` |
| `CalculateGravity()` | process.c | Global (process.h) | Include `process.h` |
| `TimeSpaceMatterConflict()` | process.c | Global (process.h) | Include `process.h` |
| `ZeroVelocityComponents()` | velocity macro | Macro | Include via element.h or velocity.h |
| `SetElementStarShip()` | macro in element.h | Macro | Include `element.h` |
| `ship_preprocess` | ship.c | **Global** (ship.h:32) | Include `ship.h` |
| `ship_postprocess` | ship.c | **Global** (ship.h:33) | Include `ship.h` |
| `collision` | ship.c | **Global** (ship.h:34) | Include `ship.h` |
| `ship_death` | tactrans.c | **Global** (tactrans.h:40) | Include `tactrans.h` |
| `NPC_PLAYER_NUM` | macro in races.h | Macro | Include `races.h` |
| `IN_LAST_BATTLE` | globdata.h | Macro | Include `globdata.h` |
| `APPEARING`, `PLAYER_SHIP`, `IGNORE_SIMILAR` | element.h | Macros | Include `element.h` |
| `NORMAL_LIFE` | element.h:32 | Macro | Include `element.h` |
| `STAMP_PRIM` | gfxlib | Macro | Include via element.h |
| `LOG_SPACE_WIDTH`, `LOG_SPACE_HEIGHT` | units.h | Macros | Include `units.h` |

**Result: No static/TU-local symbols needed.** All symbols are globally visible via headers or are macros available through standard includes. Direct copy is safe.

### Symbols used by `rust_bridge_init_battle_arena()` (from init.c InitShips lines 187-249)

| Symbol | Origin | Visibility | Action Required |
|--------|--------|-----------|-----------------|
| `InitSpace()` | init.c | **Global** (init.h:32) | Include `init.h` |
| `SetContext()` | graphics | Global | Include `setup.h` / gfx headers |
| `StatusContext` | setup.c | Global (setup.h) | Include `setup.h` |
| `SpaceContext` | setup.c | Global (setup.h) | Include `setup.h` |
| `ScreenContext` | setup.c | Global (setup.h) | Include `setup.h` |
| `Screen` | setup.c | Global (setup.h) | Include `setup.h` |
| `InitDisplayList()` | element.c | Global | Include `element.h` or process.h |
| `InitGalaxy()` | galaxy.c | Global | Already included via existing path |
| `inHQSpace()` | hyper.h | Macro/Global | Include `hyper.h` |
| `ReinitQueue()` | queue.c | Global | Include `build.h` |
| `BuildSIS()` | init.c | **STATIC** | **Must re-implement or make accessible** |
| `LoadHyperspace()` | hyper.c | Global (hyper.h) | Include `hyper.h` |
| `SetContextFGFrame()` | graphics | Global | Include via gfxlib.h |
| `SetContextClipRect()` | graphics | Global | Include via gfxlib.h |
| `SetContextBackGroundColor()` | graphics | Global | Include via gfxlib.h |
| `ClearDrawable()` | graphics | Global | Include via gfxlib.h |
| `BLACK_COLOR` | colors.h | Macro | Include `colors.h` |
| `SAFE_X`, `SAFE_Y` | units.h | Macros | Include `units.h` |
| `SPACE_WIDTH`, `SPACE_HEIGHT` | units.h | Macros | Include `units.h` |
| `free_gravity_well()` | cons_res.c | Global (cons_res.h:28) | Include `cons_res.h` |
| `spawn_asteroid()` | process.c | Global (element.h:221) | Include `element.h` |
| `spawn_planet()` | process.c | Global (element.h:220) | Include `element.h` |
| `IN_LAST_BATTLE` | globdata.h | Macro | Include `globdata.h` |
| `LOBYTE()` | compiler.h | Macro | Include `libs/compiler.h` |
| `NUM_SIDES` | init.h:28 | Macro | Include `init.h` |
| `race_q` | build.c | Global (build.h) | Include `build.h` |
| `Build()` | build.c | Global (build.h) | Include `build.h` |
| `SIS_SHIP_ID` | races.h | Macro | Include `races.h` |
| `LockStarShip()`, `UnlockStarShip()` | build.h | Macros | Include `build.h` |
| `RPG_PLAYER_NUM` | races.h:287 | Macro | Include `races.h` |

**Critical: `BuildSIS()` is `static` in init.c (line 164).** This function is only 13 lines and builds a flagship entry in the race queue for hyperspace. Options:

1. **Copy the body inline** into `rust_bridge_init_battle_arena()` — simplest, self-contained.
2. **Make `BuildSIS()` non-static** in init.c and add to init.h — changes existing code.
3. **Re-implement as a local static** in rust_bridge_ships.c — clean separation.

**Chosen approach: Option 1 — inline the BuildSIS body.** It's only 6 statements. This avoids modifying init.c and keeps the helper self-contained. The inlined code is:

```c
/* Inlined from init.c BuildSIS() — static in its origin file */
{
    HSTARSHIP hStarShip;
    STARSHIP *StarShipPtr;

    hStarShip = Build (&race_q[0], SIS_SHIP_ID);
    if (hStarShip)
    {
        StarShipPtr = LockStarShip (&race_q[0], hStarShip);
        StarShipPtr->playerNr = RPG_PLAYER_NUM;
        StarShipPtr->captains_name_index = 0;
        UnlockStarShip (&race_q[0], hStarShip);
    }
}
```

### Symbols used by `rust_bridge_uninit_ships()` (from init.c UninitShips lines 282-360)

| Symbol | Origin | Visibility | Action Required |
|--------|--------|-----------|-----------------|
| `StopSound()` | sndlib | Global (libs/sndlib.h) | Include `libs/sndlib.h` |
| `UninitSpace()` | init.c | **Global** (init.h:33) | Include `init.h` |
| `CountCrewElements()` | init.c | **STATIC** (line 252) | **Must re-implement or make accessible** |
| `GetHeadElement()` | element.c | Global (element.h) | Include `element.h` |
| `GetSuccElement()` | element.c | Global (element.h) | Include `element.h` |
| `LockElement()`, `UnlockElement()` | element.c | Global (element.h) | Include `element.h` |
| `GetElementStarShip()` | macro in element.h | Macro | Include `element.h` |
| `free_ship()` | loadship.c | Global | Declared via existing path |
| `new_ship` | tactrans.c | **Global** (tactrans.h:39) | Include `tactrans.h` |
| `FleetIsInfinite()` | supermelee.c | Global | Include via pickship.h or similar |
| `UpdateShipFragCrew()` | encount.c | Global (encount.h) | Include `encount.h` |
| `ReinitQueue()` | queue.c | Global | Include `build.h` |
| `FreeHyperspace()` | hyper.c | Global (hyper.h) | Include `hyper.h` |
| `PLAYER_SHIP` | element.h | Macro | Include `element.h` |
| `CREW_OBJECT` | element.h | Macro | Include `element.h` |
| `IN_BATTLE`, `IN_ENCOUNTER`, `CHECK_ABORT` | globdata.h | Macros | Include `globdata.h` |
| `NUM_PLAYERS` | init.h:27 | Macro | Include `init.h` |
| `race_q` | build.c | Global (build.h) | Include `build.h` |

**Critical: `CountCrewElements()` is `static` in init.c (line 252).** This function is 19 lines — it iterates the display list counting CREW_OBJECT elements.

**Chosen approach: Copy the body as a local static function** in `rust_bridge_ships.c`. It has no dependencies beyond `GetHeadElement`, `GetSuccElement`, `LockElement`, `UnlockElement`, and `CREW_OBJECT` — all globally available.

```c
/* Copied from init.c:CountCrewElements() which is static in its origin TU.
 * STALENESS WARNING: If init.c's CountCrewElements() is modified, this copy
 * must be updated manually. Grep for "rust_bridge_CountCrewElements" to find
 * this copy. Last synced with init.c line 252. */
static COUNT
rust_bridge_CountCrewElements(void)
{
    COUNT result;
    HELEMENT hElement, hNextElement;

    result = 0;
    for (hElement = GetHeadElement();
            hElement != 0; hElement = hNextElement)
    {
        ELEMENT *ElementPtr;

        LockElement(hElement, &ElementPtr);
        hNextElement = GetSuccElement(ElementPtr);
        if (ElementPtr->state_flags & CREW_OBJECT)
            ++result;

        UnlockElement(hElement);
    }

    return result;
}
```

### P00 Completion Criteria (C1 addition)

P00 is NOT complete until:
1. Every symbol used by each copied function body has been verified accessible (via header include or local copy).
2. No unresolved `static` or TU-local references exist — compile with `-Wall -Werror` must produce zero warnings.
3. The symbol inventory table above is confirmed accurate by successful compilation.

### Semantic Parity Verification (C2)

Compiling successfully does NOT prove that the copied function bodies behave identically in `rust_bridge_ships.c` vs their original translation units. Macros, globals, and static state may behave differently when relocated to a new TU. The following parity checks are **mandatory** before P00 is considered complete:

#### C2-A: `rust_bridge_spawn_element()` parity with `ship.c:spawn_ship()` lines 431-514

| Check | What to verify | How |
|-------|---------------|-----|
| C2-A1 | `SetElementStarShip` macro expansion is identical | Inspect preprocessor output: `cc -E rust_bridge_ships.c \| grep SetElementStarShip` — verify it sets `pParent` field. Compare with `cc -E ship.c` output. |
| C2-A2 | `GLOBAL(ShipFacing)` accesses the same global | Both files include `globdata.h`. Verify `GLOBAL` macro resolves to the same `GlobData` struct access in both TUs. |
| C2-A3 | `inHQSpace()` macro/function resolves identically | Both files include `hyper.h`. Verify same expansion. |
| C2-A4 | `CalculateGravity` / `TimeSpaceMatterConflict` have no hidden TU-local state | These are in `process.c` and declared in `process.h`. Verify they only access global display list state (no `static` variables in process.c that affect their behavior). |
| C2-A5 | Random position loop behavior is identical | Verify `TFB_Random()` is the same RNG in both TUs (it is — global in mathlib). |

#### C2-B: `rust_bridge_init_battle_arena()` parity with `init.c:InitShips()` lines 187-249

| Check | What to verify | How |
|-------|---------------|-----|
| C2-B1 | `InitSpace()` ref-counting behavior | `InitSpace()` uses a `static COUNT InitialSpace` counter in `init.c`. When called from `rust_bridge_ships.c`, it still calls the same function in `init.c` (not a copy). Verify via link: `nm -u rust_bridge_ships.o \| grep InitSpace` shows undefined (resolved at link time to init.o). |
| C2-B2 | Inlined `BuildSIS()` matches `init.c` static version | Line-by-line comparison of inlined body vs `init.c:BuildSIS()`. Must be identical. Any future changes to `init.c:BuildSIS()` will NOT automatically propagate — document this in a comment above the inlined code. |
| C2-B3 | `SetContext` side effects (active context stack) | Verify `SetContext(StatusContext)` then `SetContext(SpaceContext)` leaves `SpaceContext` active, same as in original. No TU-local state affects this. |
| C2-B4 | `ReinitQueue(&race_q[])` operates on the same global queues | `race_q` is global in `build.c`, declared in `build.h`. Same in both TUs. |
| C2-B5 | Asteroid/planet spawn state | `spawn_asteroid(NULL)` and `spawn_planet()` are global functions. Verify no `static` counters in `process.c` affect spawn behavior differently when called from a different TU. |

#### C2-C: `rust_bridge_uninit_ships()` parity with `init.c:UninitShips()` lines 282-360

| Check | What to verify | How |
|-------|---------------|-----|
| C2-C1 | `UninitSpace()` ref-counting is symmetric with `InitSpace()` | Same function in `init.c`, called via link. The `static COUNT InitialSpace` counter is decremented correctly regardless of calling TU. |
| C2-C2 | `free_ship()` dispatch to `rust_ships_free()` | Under `USE_RUST_SHIPS`, `free_ship()` in `loadship.c` dispatches to `rust_ships_free()`. Verify this dispatch happens correctly when called from `rust_bridge_ships.c` (it does — `free_ship` is a global function, dispatch is compile-time via `#ifdef`). |
| C2-C3 | `GLOBAL(CurrentActivity) &= ~IN_BATTLE` — same global mutation | `GLOBAL` macro accesses `GlobData` struct. Same in both TUs. |
| C2-C4 | `UpdateShipFragCrew()` crew writeback outcome | Verify `UpdateShipFragCrew()` accesses `StarShipPtr->crew_level` which was just written by the uninit loop. The field write and the subsequent read happen in the same function body — no TU boundary between them. |
| C2-C5 | `FleetIsInfinite()` behavior | Global function in `supermelee.c`. No TU-local state. |
| C2-C6 | Copied `rust_bridge_CountCrewElements()` matches `init.c:CountCrewElements()` | Line-by-line comparison. Must be identical. Document staleness risk in a comment. |

**Implementation:** These checks are performed during P00 implementation by the implementor. Each check is marked PASS/FAIL in the PR description. Any FAIL blocks P00 completion until resolved.

## Spawn Branch Parity Checklist (C2)

The C `spawn_ship()` in `ship.c` has two distinct branches depending on whether `StarShipPtr->hShip` is zero (fresh allocation) or non-zero (element reuse from a previous ship, e.g. `GetNextStarShip` recycling). The C helper **must** handle both branches with identical side effects to the original. The following checklist is a **mandatory verification** — every field and callback must be confirmed set in BOTH branches.

### Branch A: `hShip == 0` (fresh allocation path)

This is the normal first-spawn path. `AllocElement()` is called, and the element is inserted at the head of the display list.

### Branch B: `hShip != 0` (element reuse path)

This occurs when `GetNextStarShip()` copies `LastStarShipPtr->hShip` to `StarShipPtr->hShip` before calling `spawn_ship()` (see `ship.c` line 536). The existing element handle is reused — no `AllocElement()`/`InsertElement()` is called, but **all element fields are still overwritten** by the `LockElement` block below.

### Parity assertion matrix — fields set in BOTH branches

Both branches converge at `LockElement(hShip, &ShipElementPtr)`. The following fields are set **unconditionally** regardless of which branch was taken. This is verified by inspection of `ship.c` lines 445-510: the field-setting block is outside the `if (hShip == 0)` block.

| Field / Side Effect | Set in Branch A | Set in Branch B | C reference line |
|---------------------|----------------|----------------|-----------------|
| `ShipElementPtr->playerNr = StarShipPtr->playerNr` | YES | YES | 447 |
| `ShipElementPtr->crew_level = 0` | YES | YES | 448 |
| `ShipElementPtr->mass_points = ship_mass` | YES | YES | 449 |
| `ShipElementPtr->state_flags = APPEARING \| PLAYER_SHIP \| IGNORE_SIMILAR` | YES | YES | 450 |
| `ShipElementPtr->turn_wait = 0` | YES | YES | 451 |
| `ShipElementPtr->thrust_wait = 0` | YES | YES | 452 |
| `ShipElementPtr->life_span = NORMAL_LIFE` | YES | YES | 453 |
| `ShipElementPtr->colorCycleIndex = 0` | YES | YES | 454 |
| `SetPrimType(&DisplayArray[...], STAMP_PRIM)` | YES | YES | 456 |
| `current.image.farray = RDPtr->ship_data.ship` | YES | YES | 457 |
| `current.image.frame = SetAbsFrameIndex(...)` | YES | YES | 464/488 |
| `current.location.x` (positioned) | YES | YES | 467/493 |
| `current.location.y` (positioned) | YES | YES | 468/496 |
| `ShipFacing` (set on StarShipPtr) | YES | YES | 463/473 |
| `preprocess_func = ship_preprocess` | YES | YES | 501 |
| `postprocess_func = ship_postprocess` | YES | YES | 502 |
| `death_func = ship_death` | YES | YES | 503 |
| `collision_func = collision` | YES | YES | 504 |
| `ZeroVelocityComponents(&velocity)` | YES | YES | 505 |
| `SetElementStarShip(ShipElementPtr, StarShipPtr)` | YES | YES | 507 |
| `hTarget = 0` | YES | YES | 508 |
| `UnlockElement(hShip)` | YES | YES | 510 |
| `life_span++` (Sa-Matra only) | YES (if Sa-Matra) | YES (if Sa-Matra) | 469 |

**Key insight:** The only difference between branches A and B is whether `AllocElement()`/`InsertElement()` are called. ALL element field initialization and callback assignment happens unconditionally in both branches. The C helper preserves this by having the `if (hShip == 0)` block ONLY control allocation/insertion, with all field writes in the common path below.

### Verification procedure

During P00 implementation, the implementor MUST verify BOTH branches produce identical side effects (per REQ-REMED-SPAWN-PARITY):

1. **Structural check:** Confirm the C helper's `if (hShip == 0)` block contains ONLY `AllocElement()`/`InsertElement()`. All field writes (items 1-22 in the matrix above) must be in the common path OUTSIDE that block. No field writes inside the `if` branch.
2. **Field completeness:** Confirm all 22 fields/side effects in the matrix above are present in the helper's common path after `LockElement`.
3. **hShip writeback order:** Confirm that `StarShipPtr->hShip = hShip` is written AFTER the conditional alloc but BEFORE the field-setting block (line 439 in C).
4. **Branch B test scenario:** Test the element reuse path by verifying `GetNextStarShip()` with `LastStarShipPtr != NULL` works correctly (replacement ship spawn in melee). Confirm the reused element has no stale state from the previous ship — all fields overwritten.
5. **Sa-Matra conditional:** Confirm `life_span++` (item 23) is inside a conditional (`if (NPC_PLAYER_NUM && IN_LAST_BATTLE)`) that fires in BOTH branches (it's in the common path, so it does).

## Functions to Add

### 1. `rust_bridge_spawn_element()`

**Signature:**
```c
BOOLEAN rust_bridge_spawn_element(
    STARSHIP *StarShipPtr,
    RACE_DESC *RDPtr,
    BYTE ship_mass,
    BYTE activity
);
```

**Body:** Extract from `ship.c` spawn_ship() lines 431-514 (everything after descriptor loading and crew patching). Both hShip==0 (allocation) and hShip!=0 (reuse) branches are handled — see Spawn Branch Parity Checklist above.

**Implementation:**
```c
BOOLEAN
rust_bridge_spawn_element(STARSHIP *StarShipPtr, RACE_DESC *RDPtr,
        BYTE ship_mass, BYTE activity)
{
    HELEMENT hShip;

#ifndef NDEBUG
    /* L1: Verify Rust-provided activity matches C global.
     * Catches timing drift between Rust's read and C's use. */
    assert(activity == LOBYTE(GLOBAL(CurrentActivity))
        && "activity parameter does not match GLOBAL(CurrentActivity)");
#endif

    /* --- Branch A vs B: hShip==0 means fresh alloc, hShip!=0 means reuse --- */
    hShip = StarShipPtr->hShip;
    if (hShip == 0)
    {
        /* Branch A: fresh allocation */
        hShip = AllocElement();
        if (hShip != 0)
            InsertElement(hShip, GetHeadElement());
    }
    /* Branch B: hShip != 0 — reuse existing element handle.
     * No AllocElement/InsertElement needed; element is already in display list.
     * All fields below are still overwritten unconditionally (C2 parity). */

    StarShipPtr->hShip = hShip;
    if (StarShipPtr->hShip != 0)
    {
        /* Common path for BOTH branches — all fields set unconditionally */
        ELEMENT *ShipElementPtr;

        LockElement(hShip, &ShipElementPtr);

        ShipElementPtr->playerNr = StarShipPtr->playerNr;
        ShipElementPtr->crew_level = 0;
        ShipElementPtr->mass_points = ship_mass;
        ShipElementPtr->state_flags = APPEARING | PLAYER_SHIP | IGNORE_SIMILAR;
        ShipElementPtr->turn_wait = 0;
        ShipElementPtr->thrust_wait = 0;
        ShipElementPtr->life_span = NORMAL_LIFE;
        ShipElementPtr->colorCycleIndex = 0;

        SetPrimType(&DisplayArray[ShipElementPtr->PrimIndex], STAMP_PRIM);
        ShipElementPtr->current.image.farray = RDPtr->ship_data.ship;

        if (ShipElementPtr->playerNr == NPC_PLAYER_NUM
                && activity == IN_LAST_BATTLE)
        {
            /* Sa-Matra special case */
            StarShipPtr->ShipFacing = 0;
            ShipElementPtr->current.image.frame =
                    SetAbsFrameIndex(RDPtr->ship_data.ship[0],
                    StarShipPtr->ShipFacing);
            ShipElementPtr->current.location.x = LOG_SPACE_WIDTH >> 1;
            ShipElementPtr->current.location.y = LOG_SPACE_HEIGHT >> 1;
            ++ShipElementPtr->life_span;
        }
        else
        {
            StarShipPtr->ShipFacing = NORMALIZE_FACING(TFB_Random());
            if (inHQSpace())
            {
                COUNT facing = GLOBAL(ShipFacing);
                if (facing > 0)
                    --facing;
                StarShipPtr->ShipFacing = facing;
            }
            ShipElementPtr->current.image.frame =
                    SetAbsFrameIndex(RDPtr->ship_data.ship[0],
                    StarShipPtr->ShipFacing);
            do
            {
                ShipElementPtr->current.location.x =
                        WRAP_X(DISPLAY_ALIGN_X(TFB_Random()));
                ShipElementPtr->current.location.y =
                        WRAP_Y(DISPLAY_ALIGN_Y(TFB_Random()));
            } while (CalculateGravity(ShipElementPtr)
                    || TimeSpaceMatterConflict(ShipElementPtr));
        }

        /* Callbacks — set in BOTH branches (C2 parity) */
        ShipElementPtr->preprocess_func = ship_preprocess;
        ShipElementPtr->postprocess_func = ship_postprocess;
        ShipElementPtr->death_func = ship_death;
        ShipElementPtr->collision_func = collision;
        ZeroVelocityComponents(&ShipElementPtr->velocity);

        SetElementStarShip(ShipElementPtr, StarShipPtr);
        ShipElementPtr->hTarget = 0;

        UnlockElement(hShip);
    }

    return (hShip != 0);
}
```

### 2. `rust_bridge_init_battle_arena()`

**Signature:**
```c
SIZE rust_bridge_init_battle_arena(void);
```

**Body:** The complete body of `InitShips()` from init.c lines 187-249 (everything after the `#ifdef USE_RUST_SHIPS` guard), with `BuildSIS()` inlined since it is `static` in init.c.

**Implementation:**
```c
SIZE
rust_bridge_init_battle_arena(void)
{
    SIZE num_ships;

    InitSpace();

    SetContext(StatusContext);
    SetContext(SpaceContext);

    InitDisplayList();
    InitGalaxy();

    if (inHQSpace())
    {
        ReinitQueue(&race_q[0]);
        ReinitQueue(&race_q[1]);

        /* Inlined from init.c:BuildSIS() — static in origin file.
         * STALENESS WARNING: If init.c's BuildSIS() is modified, this
         * inlined copy must be updated manually. Grep for "BuildSIS"
         * in rust_bridge_ships.c to find this copy.
         * Last synced with init.c line 164. */
        {
            HSTARSHIP hStarShip;
            STARSHIP *StarShipPtr;

            hStarShip = Build(&race_q[0], SIS_SHIP_ID);
            if (hStarShip)
            {
                StarShipPtr = LockStarShip(&race_q[0], hStarShip);
                StarShipPtr->playerNr = RPG_PLAYER_NUM;
                StarShipPtr->captains_name_index = 0;
                UnlockStarShip(&race_q[0], hStarShip);
            }
        }

        LoadHyperspace();

        num_ships = 1;
    }
    else
    {
        COUNT i;
        RECT r;

        SetContextFGFrame(Screen);
        r.corner.x = SAFE_X;
        r.corner.y = SAFE_Y;
        r.extent.width = SPACE_WIDTH;
        r.extent.height = SPACE_HEIGHT;
        SetContextClipRect(&r);

        SetContextBackGroundColor(BLACK_COLOR);
        {
            CONTEXT OldContext;

            OldContext = SetContext(ScreenContext);

            SetContextBackGroundColor(BLACK_COLOR);
            ClearDrawable();

            SetContext(OldContext);
        }

        if (LOBYTE(GLOBAL(CurrentActivity)) == IN_LAST_BATTLE)
            free_gravity_well();
        else
        {
#define NUM_ASTEROIDS 5
            for (i = 0; i < NUM_ASTEROIDS; ++i)
                spawn_asteroid(NULL);
#define NUM_PLANETS 1
            for (i = 0; i < NUM_PLANETS; ++i)
                spawn_planet();
        }

        num_ships = NUM_SIDES;
    }

    return (num_ships);
}
```

### 3. `rust_bridge_uninit_ships()`

**Signature:**
```c
void rust_bridge_uninit_ships(void);
```

**Body:** The complete body of `UninitShips()` from init.c lines 282-360 (everything after the `#ifdef USE_RUST_SHIPS` guard), with `CountCrewElements()` replaced by the local `rust_bridge_CountCrewElements()` copy.

**C-side sanity guard (C3):** The C helper must guard against being called in a partially-initialized or non-initialized state. If no PLAYER_SHIP elements exist in the display list (possible after init failure or external C call ordering issue), the function must still complete safely — it will simply skip the crew writeback loop and proceed to `GLOBAL(CurrentActivity) &= ~IN_BATTLE` cleanup.

Additionally, assertion logging is added around state transitions to make debugging desync issues tractable.

### C3: Mandatory Null-Guard Ordering During Display List Iteration (Code-Level, Not Diagnostic)

The display list iteration in `rust_bridge_uninit_ships()` dereferences element→starship→RaceDesc in sequence. Under partial init, panic recovery, or external call ordering, **any** of these pointers may be null or stale.

**The null guards below are UNCONDITIONAL CODE-LEVEL REQUIREMENTS — they are production control-flow statements (`if (...) { UnlockElement; continue; }`), NOT debug-only diagnostics.** They must be present in ALL builds (debug AND release). Only the `log_add` messages inside the guard blocks are `#ifndef NDEBUG`. This distinction is critical: the guards prevent crashes; the logs assist debugging.

Required order before any field dereference:

1. **Guard 1 — ElementPtr validity:** `LockElement(hElement, &ElementPtr)` must succeed. If `ElementPtr` is null, skip (should not happen in practice, but guard defensively).
2. **Guard 2 — Extract StarShipPtr safely:** Call `GetElementStarShip(ElementPtr, &StarShipPtr)` to extract the starship pointer from the element. This uses the macro (not manual field access) to ensure correctness.
3. **Guard 3 — StarShipPtr != NULL:** `if (StarShipPtr == NULL) { /* log in debug */ UnlockElement(hElement); continue; }`. **UNCONDITIONAL.** No dereference of StarShipPtr may occur before this check passes.
4. **Guard 4 — StarShipPtr->RaceDescPtr != NULL:** `if (StarShipPtr->RaceDescPtr == NULL) { /* log in debug */ UnlockElement(hElement); continue; }`. **UNCONDITIONAL.** No dereference of RaceDescPtr may occur before this check passes.
5. **Only then:** Read `StarShipPtr->RaceDescPtr->ship_info.crew_level` and other fields.

This ordering ensures no dereference happens before its pointer is validated. **Verification:** Code review must confirm that the `if (StarShipPtr == NULL)` and `if (StarShipPtr->RaceDescPtr == NULL)` checks are NOT wrapped in `#ifndef NDEBUG`. The pattern must be:

```c
/* Guard — unconditional, fires in all builds */
if (StarShipPtr == NULL)
{
#ifndef NDEBUG
    log_add(log_Debug, "...");  /* Diagnostic — debug-only */
#endif
    UnlockElement(hElement);  /* Guard action — unconditional */
    continue;                 /* Guard action — unconditional */
}
```

**Implementation:**
```c
void
rust_bridge_uninit_ships(void)
{
    COUNT crew_retrieved;
    int i;
    HELEMENT hElement, hNextElement;
    STARSHIP *SPtr[NUM_PLAYERS];

    StopSound();

    UninitSpace();

    for (i = 0; i < NUM_PLAYERS; ++i)
        SPtr[i] = 0;

    crew_retrieved = rust_bridge_CountCrewElements();

#ifndef NDEBUG
    /* C3: Log state at entry for debugging desync between Rust and C */
    log_add(log_Debug, "rust_bridge_uninit_ships: crew_retrieved=%u",
            (unsigned)crew_retrieved);
#endif

    for (hElement = GetHeadElement();
            hElement != 0; hElement = hNextElement)
    {
        ELEMENT *ElementPtr;

        /* C3 Guard 1: Lock element and validate pointer */
        LockElement(hElement, &ElementPtr);
        hNextElement = GetSuccElement(ElementPtr);
        if ((ElementPtr->state_flags & PLAYER_SHIP)
                || ElementPtr->death_func == new_ship)
        {
            STARSHIP *StarShipPtr;

            /* C3 Guard 2: Extract starship pointer from element */
            GetElementStarShip(ElementPtr, &StarShipPtr);

            /* C3 Guard 3: Validate StarShipPtr before ANY dereference.
             * MANDATORY — not optional diagnostics. Fires in all builds.
             * This handles: partial spawn failure, stale element refs,
             * panic-path desync where element exists but starship is gone. */
            if (StarShipPtr == NULL)
            {
#ifndef NDEBUG
                log_add(log_Debug,
                        "rust_bridge_uninit_ships: null StarShipPtr, "
                        "skipping element");
#endif
                UnlockElement(hElement);
                continue;
            }

            /* C3 Guard 4: Validate RaceDescPtr before field access.
             * MANDATORY — not optional diagnostics. Fires in all builds.
             * This handles: descriptor already freed, init failure before
             * descriptor was set, double-uninit where first pass freed it. */
            if (StarShipPtr->RaceDescPtr == NULL)
            {
#ifndef NDEBUG
                log_add(log_Debug,
                        "rust_bridge_uninit_ships: null RaceDescPtr on "
                        "StarShipPtr=%p, skipping", (void *)StarShipPtr);
#endif
                UnlockElement(hElement);
                continue;
            }

            /* C3: All guards passed — safe to dereference RaceDescPtr fields */
            if (StarShipPtr->RaceDescPtr->ship_info.crew_level)
            {
                if (crew_retrieved >=
                        StarShipPtr->RaceDescPtr->ship_info.max_crew -
                        StarShipPtr->RaceDescPtr->ship_info.crew_level)
                    StarShipPtr->RaceDescPtr->ship_info.crew_level =
                            StarShipPtr->RaceDescPtr->ship_info.max_crew;
                else
                    StarShipPtr->RaceDescPtr->ship_info.crew_level +=
                            crew_retrieved;
            }

            StarShipPtr->crew_level =
                    StarShipPtr->RaceDescPtr->ship_info.crew_level;
            SPtr[StarShipPtr->playerNr] = StarShipPtr;
            free_ship(StarShipPtr->RaceDescPtr, TRUE, TRUE);
            /* Post-free nulling — prevents double-free on same element */
            StarShipPtr->RaceDescPtr = 0;
        }
        UnlockElement(hElement);
    }

    GLOBAL(CurrentActivity) &= ~IN_BATTLE;

    if (LOBYTE(GLOBAL(CurrentActivity)) == IN_ENCOUNTER
            && !(GLOBAL(CurrentActivity) & CHECK_ABORT))
    {
        for (i = NUM_PLAYERS - 1; i >= 0; --i)
        {
            if (SPtr[i] && !FleetIsInfinite(i))
                UpdateShipFragCrew(SPtr[i]);
        }
    }

    if (LOBYTE(GLOBAL(CurrentActivity)) != IN_ENCOUNTER)
    {
        for (i = 0; i < NUM_PLAYERS; i++)
            ReinitQueue(&race_q[i]);

        if (inHQSpace())
            FreeHyperspace();
    }

#ifndef NDEBUG
    log_add(log_Debug, "rust_bridge_uninit_ships: teardown complete");
#endif
}
```

## Callback Symbol Visibility — Verified (C2)

All four callback functions assigned in `rust_bridge_spawn_element` are **globally visible** with extern prototypes in public headers:

| Callback | Defined in | Header prototype |
|----------|-----------|------------------|
| `ship_preprocess` | `ship.c` | `ship.h` line 32: `extern void ship_preprocess(ELEMENT *ElementPtr);` |
| `ship_postprocess` | `ship.c` | `ship.h` line 33: `extern void ship_postprocess(ELEMENT *ElementPtr);` |
| `collision` | `ship.c` | `ship.h` line 34: `extern void collision(ELEMENT *ElementPtr0, POINT *pPt0, ELEMENT *ElementPtr1, POINT *pPt1);` |
| `ship_death` | `tactrans.c` | `tactrans.h` line 40: `extern void ship_death(ELEMENT *ShipPtr);` |

**None are `static`.** No C bridge wrappers are needed for these callbacks. Including `ship.h` and `tactrans.h` is sufficient.

**Compile-time verification step:** After adding the functions, compile `rust_bridge_ships.c` and confirm no "implicit declaration" or "undefined reference" warnings for any of these four symbols. The linker will catch missing symbols at link time, but the compile step must also be warning-clean.

## Header Declaration for New Helpers (C1)

Create or update a header file so that the three new C helper functions have proper prototypes visible to any translation unit that may need them (including Rust FFI via `extern "C"` blocks).

### New file: `sc2/src/uqm/rust_bridge_ships.h`

```c
#ifndef UQM_RUST_BRIDGE_SHIPS_H_
#define UQM_RUST_BRIDGE_SHIPS_H_

#include "libs/compiler.h"
#include "races.h"
#include "element.h"

#if defined(__cplusplus)
extern "C" {
#endif

#ifdef USE_RUST_SHIPS

/* Lifecycle helpers called by Rust FFI — see rust_bridge_ships.c */

BOOLEAN rust_bridge_spawn_element(STARSHIP *StarShipPtr,
        RACE_DESC *RDPtr, BYTE ship_mass, BYTE activity);

SIZE rust_bridge_init_battle_arena(void);

void rust_bridge_uninit_ships(void);

/* Layout verification — all builds (layout mismatch = silent corruption) */
typedef struct {
    size_t race_desc_size;
    size_t ship_data_offset;
    size_t ship_info_offset;
    size_t characteristics_offset;
    size_t ship_data_ship_offset;
    size_t ship_info_crew_offset;
    size_t ship_info_max_crew_offset;
    size_t characteristics_mass_offset;
} RACE_DESC_LAYOUT;

void rust_bridge_get_race_desc_layout(RACE_DESC_LAYOUT *out);

/* Existing helpers */
BYTE uqm_get_current_activity_lobyte(void);

#endif /* USE_RUST_SHIPS */

#if defined(__cplusplus)
}
#endif

#endif /* UQM_RUST_BRIDGE_SHIPS_H_ */
```

**Why a dedicated header:** The new functions are called across translation unit boundaries (Rust FFI → C linker). Without header prototypes, the compiler cannot verify calling conventions or argument types at the call site. Putting them in an existing header (e.g., `ship.h`) would pollute the non-Rust build path. A dedicated header guarded by `USE_RUST_SHIPS` keeps the separation clean.

## Required Includes / Externs in rust_bridge_ships.c

```c
/* Existing includes (already present) */
#include "globdata.h"
#include "libs/compiler.h"
#include "element.h"
#include "weapon.h"
#include "intel.h"
#include "races.h"
#include "collide.h"
#include "colors.h"
#include "status.h"
#include "sounds.h"
#include "units.h"
#include "libs/mathlib.h"
#include "libs/sndlib.h"

/* New includes for lifecycle helpers */
#include "ship.h"       /* ship_preprocess, ship_postprocess, collision */
#include "tactrans.h"   /* ship_death, new_ship */
#include "init.h"       /* InitSpace, UninitSpace, InitDisplayList, InitGalaxy, NUM_PLAYERS */
#include "build.h"      /* queue operations, Build, race_q, Lock/UnlockStarShip */
#include "hyper.h"      /* LoadHyperspace, FreeHyperspace, inHQSpace */
#include "pickship.h"   /* GetInitialStarShips */
#include "process.h"    /* CalculateGravity, TimeSpaceMatterConflict */
#include "setup.h"      /* SpaceContext, StatusContext, ScreenContext, Screen */
#include "encount.h"    /* UpdateShipFragCrew */
#include "cons_res.h"   /* free_gravity_well */
#include "libs/log.h"   /* log_add for debug assertions (C3) */

/* Own header — prototype declarations */
#include "rust_bridge_ships.h"
```

**No manual `extern` declarations needed.** All four callback prototypes come from `ship.h` and `tactrans.h`. `new_ship` comes from `tactrans.h`. The new helper prototypes come from `rust_bridge_ships.h`. This eliminates the risk of hand-written extern declarations drifting from actual signatures.

## Build Integration

### Guard all new code with `USE_RUST_SHIPS`

The three helper functions and the header content must be wrapped in `#ifdef USE_RUST_SHIPS` / `#endif` so the non-Rust build path is unaffected.

In `rust_bridge_ships.c`, wrap each new function:
```c
#ifdef USE_RUST_SHIPS

static COUNT
rust_bridge_CountCrewElements(void)
{
    /* ... */
}

BOOLEAN
rust_bridge_spawn_element(STARSHIP *StarShipPtr, RACE_DESC *RDPtr,
        BYTE ship_mass, BYTE activity)
{
    /* ... */
}

SIZE
rust_bridge_init_battle_arena(void)
{
    /* ... */
}

void
rust_bridge_uninit_ships(void)
{
    /* ... */
}

#endif /* USE_RUST_SHIPS */
```

### Build system TU compilation verification (per REQ-REMED-BUILD-TU)

`rust_bridge_ships.c` must be compiled and linked in the `USE_RUST_SHIPS=1` build profile. The new helper functions will only have symbols if this file is actually included in the build graph.

**Mandatory acceptance checks — BOTH configurations (H3):**

#### Check 1: USE_RUST_SHIPS=1 — File must be in build graph

```bash
# Verify TU is in the build graph when USE_RUST_SHIPS=1
grep -n 'rust_bridge_ships' sc2/src/uqm/Makeinfo
# Expected: conditional inclusion under uqm_USE_RUST_SHIPS == "1"

# After build with USE_RUST_SHIPS=1, verify object file exists:
find sc2/ -name 'rust_bridge_ships.o' -type f 2>/dev/null
# Expected: exactly one .o file

# Verify all three helper symbols are exported:
nm <path-to-rust_bridge_ships.o> | grep -E 'rust_bridge_(spawn_element|init_battle_arena|uninit_ships)'
# Expected: three 'T' (global text) symbols
```

#### Check 2: USE_RUST_SHIPS=0 — File must not cause link errors

```bash
# Two valid approaches:
#   A) Makeinfo conditionally EXCLUDES the file entirely (current approach), OR
#   B) File is always compiled but all code is #ifdef guarded (also valid)

# For approach A (current): verify no .o file exists when built without Rust
grep -A2 'USE_RUST_SHIPS' sc2/src/uqm/Makeinfo
# Expected: conditional inclusion visible

# After building with USE_RUST_SHIPS=0:
find sc2/ -name 'rust_bridge_ships.o' -type f 2>/dev/null
# Expected: no .o file (file not compiled at all)

# Full build must succeed with zero warnings in both configs
```

#### Check 3: No duplicate definitions

```bash
# Verify each function appears exactly twice (prototype + definition):
grep -rn 'rust_bridge_spawn_element\|rust_bridge_init_battle_arena\|rust_bridge_uninit_ships' \
    sc2/src/uqm/*.c sc2/src/uqm/*.h | sort
# Expected: 2 occurrences per function (one in .h, one in .c)
```

**If ANY check fails, P00 is NOT complete.** The Makeinfo entry is the most critical — if it's missing, helpers compile in isolation but produce undefined symbol errors at link time that only appear when the full binary is linked.

### Build system completeness verification

No `Makefile` or `build.vars.in` changes should be needed because `rust_bridge_ships.c` is already compiled and linked when `USE_RUST_SHIPS=1`. However, concrete validation is required using the actual project build system.

**This project uses `sc2/build.sh uqm` as the build entry point.** The build system is configure-script-based (`build.sh` → interactive config → `make`). There is no standalone `make` target that works without prior configuration. The Rust library is built separately via `cargo build --release` in the `rust/` directory.

#### Step A: Include path / compile ordering
1. Verify `rust_bridge_ships.c` is already in the source file list (it is — it compiles today).
2. Verify the new `#include` directives resolve correctly (all referenced headers are in the standard `sc2/src/uqm/` include path).
3. Verify the new header `rust_bridge_ships.h` does not create circular include chains. Its includes (`compiler.h`, `races.h`, `element.h`) are standard leaf headers.

#### Step B: Full clean build — Rust-enabled profile
```bash
# Build Rust library first
cd rust && cargo build --release

# Build C+link with Rust enabled (uses build.config with rust_bridge=enabled)
cd sc2 && ./build.sh uqm
```
**Must pass with:** zero warnings from `rust_bridge_ships.c`, no "implicit function declaration" warnings, no "undefined reference" link errors.

**Note:** `build.sh uqm` runs the interactive configure step. If `build.config` already has `rust_bridge=enabled` from a previous run, it will reuse that config. To force reconfiguration, delete `sc2/config.state` first.

#### Step C: Full clean build — Rust-disabled profile

The non-Rust build profile is selected via the `build.sh` interactive menu by choosing "disabled" for the Rust bridge option. Alternatively, if `build.config` has been configured for disabled:
```bash
# Reconfigure with rust_bridge=disabled, then build
cd sc2 && rm -f config.state && ./build.sh uqm
# (select "disabled" for Rust bridge in the interactive menu)
```
**Must pass with:** zero warnings. All new code excluded by `#ifdef USE_RUST_SHIPS`.

#### Step D: Check for duplicate definitions
```bash
# Verify no duplicate definitions from header inclusion
grep -rn 'rust_bridge_spawn_element\|rust_bridge_init_battle_arena\|rust_bridge_uninit_ships' \
    sc2/src/uqm/*.c sc2/src/uqm/*.h | sort
```
**Expected:** Each function name appears exactly twice — once in `rust_bridge_ships.h` (prototype) and once in `rust_bridge_ships.c` (definition). No other occurrences.

#### Step E: Implicit function declaration check
The project's own build system compiles with warnings enabled. Verify by checking build output for `rust_bridge_ships.c`:
```bash
# After a successful build, check the compile log for warnings from our file:
# (build output goes to stdout during ./build.sh uqm)
# Or recompile the single file manually with the project's include paths:
grep -r 'CFLAGS\|INCLUDES' sc2/build/unix/Makefile  # find actual include paths
```
Zero warnings required from `rust_bridge_ships.c`.

## Verification — Compile and Link Smoke Test

This phase must not be considered done until compile AND link are verified. Do not defer this to P04.

### Step 1: Rust library build
```bash
cd rust && cargo build --release
```
Confirm: Rust library builds cleanly.

### Step 2: Full C+Rust linked build
```bash
# Uses the project's build system with Rust bridge enabled
cd sc2 && ./build.sh uqm
```
Confirm: clean compile (zero warnings from `rust_bridge_ships.c`), clean link (no undefined symbols). The binary is produced at the path shown in build output (typically `sc2/uqm` or similar).

### Step 3: Symbol export verification
```bash
# Find the object file (path depends on build system output directory)
find sc2/ -name 'rust_bridge_ships.o' -type f 2>/dev/null

# Verify new symbols are present and exported (not static/hidden)
nm <path-to-rust_bridge_ships.o> | grep -E 'rust_bridge_(spawn_element|init_battle_arena|uninit_ships|get_race_desc_layout)'
```
Expected output: four `T` (text/global) symbol entries. `rust_bridge_CountCrewElements` should show as `t` (lowercase = local/static) — this is correct since it is a local helper.

### Step 4: No-Rust build verification
```bash
# Reconfigure with rust_bridge=disabled via the interactive build.sh menu
cd sc2 && rm -f config.state && ./build.sh uqm
# (select "disabled" for Rust bridge option)
```
Confirm: compiles cleanly. The new functions and header are excluded by `#ifdef USE_RUST_SHIPS`.

### Step 5: Callback symbol linkage
```bash
# Verify the callbacks referenced in rust_bridge_spawn_element are linkable
find sc2/ -name 'ship.o' -type f -exec nm {} \; | grep -E ' T.*ship_preprocess| T.*ship_postprocess| T.*collision'
find sc2/ -name 'tactrans.o' -type f -exec nm {} \; | grep -E ' T.*ship_death| T.*new_ship'
```
Expected: all five symbols present as `T` (global text).

## Output

- **New file:** `sc2/src/uqm/rust_bridge_ships.h` — prototypes for all rust_bridge_ships helpers
- **Modified:** `sc2/src/uqm/rust_bridge_ships.c` — three new lifecycle helper functions + one local static helper (`rust_bridge_CountCrewElements`) + layout query function + new includes + own header include
- No Rust changes in this phase.

## Completion Criteria

1. All three helper functions compile without warnings under `-Wall`.
2. All symbols in the inventory tables above are resolved (verified by successful compilation).
3. No `static` or TU-local symbols from other translation units are referenced without being explicitly copied or inlined (verified against inventory tables).
4. `nm` confirms four `T` symbols (spawn_element, init_battle_arena, uninit_ships, get_race_desc_layout) and one `t` symbol (CountCrewElements).
5. Both Rust-enabled and Rust-disabled build profiles compile and link cleanly.
6. **C3 guard code-level verification:** The C-side uninit null-guard ordering (Guards 1-4 per C3 section) is present with correct sequencing. **Critical:** The `if (StarShipPtr == NULL)` and `if (StarShipPtr->RaceDescPtr == NULL)` checks are UNCONDITIONAL (not wrapped in `#ifndef NDEBUG`). Only the `log_add` calls inside the guard blocks are `#ifndef NDEBUG`. Verified by code review.
7. All C2 semantic parity checks (C2-A1 through C2-C6) are marked PASS in the PR description. Any FAIL blocks P00 completion.
8. Inlined `BuildSIS()` and copied `CountCrewElements()` have staleness-risk comments noting that changes to `init.c` originals will not auto-propagate.
9. **Spawn Branch Parity (C2) verified:** All 22 fields/side effects in the parity matrix (per REQ-REMED-SPAWN-PARITY) are confirmed present in `rust_bridge_spawn_element()`. The `if (hShip == 0)` block contains ONLY `AllocElement`/`InsertElement` — all field writes are in the common path. The hShip!=0 reuse path is tested via `GetNextStarShip()` replacement ship scenario.
10. **Build system TU check (H3) — BOTH configs:** `sc2/src/uqm/Makeinfo` confirmed to include `rust_bridge_ships.c` when `uqm_USE_RUST_SHIPS=1`. Object file and `nm` symbols verified after build. Non-Rust build verified (no link errors, no .o file or all code guarded).
11. **Activity flag assertion (L1):** `rust_bridge_spawn_element()` includes a debug assertion: `assert(activity == LOBYTE(GLOBAL(CurrentActivity)))`. This verifies the Rust-provided activity byte matches the C global.

## LoC Estimate

~150 lines of C in `rust_bridge_ships.c` (three helpers + CountCrewElements copy + null guards + debug logging).
~35 lines of C in `rust_bridge_ships.h` (header with prototypes).
