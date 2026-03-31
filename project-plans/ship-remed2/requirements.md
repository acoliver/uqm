# Ship FFI Lifecycle Remediation — Requirements

## Problem Statement

The three FFI lifecycle functions (`rust_ships_spawn`, `rust_ships_init`, `rust_ships_uninit`) that wire Rust ship behavior into the C game engine are broken stubs. The `#ifdef USE_RUST_SHIPS` guards in `ship.c`, `init.c`, and `loadship.c` are now active (commit b1817fd1f), meaning these broken paths **will be taken at runtime**, resulting in immediate crashes or silent failures during battle.

## What Is Broken

### REQ-REMED-SPAWN: `rust_ships_spawn()` Never Creates a C ELEMENT

**Current behavior:** `ffi.rs::rust_ships_spawn()` → `lifecycle::spawn_ship()` loads a descriptor and patches crew, but the `ElementConfig` computed at lifecycle.rs:161 is dead code (prefixed with `_`). No C `ELEMENT` is ever allocated.

**Required behavior (C reference: `ship.c` spawn_ship() lines 393-514):**
1. Load descriptor via `load_ship(species, TRUE)` [OK] already works
2. Clear input/status/counters [OK] already works
3. Patch crew for IN_ENCOUNTER/IN_LAST_BATTLE [OK] already works
4. If `hShip == 0`: call `AllocElement()`, `InsertElement(hShip, GetHeadElement())`
5. Store `hShip` back to `StarShipPtr->hShip`
6. `LockElement(hShip)` to get ELEMENT pointer
7. Set ELEMENT fields: `playerNr`, `crew_level=0`, `mass_points`, `state_flags = APPEARING|PLAYER_SHIP|IGNORE_SIMILAR`, `turn_wait=0`, `thrust_wait=0`, `life_span=NORMAL_LIFE`, `colorCycleIndex=0`
8. `SetPrimType(&DisplayArray[PrimIndex], STAMP_PRIM)`
9. Set `current.image.farray = ship_data.ship`
10. Handle Sa-Matra special case (NPC + IN_LAST_BATTLE): facing=0, center position, life_span++
11. Handle HyperSpace special case: use GLOBAL(ShipFacing)
12. Handle normal case: random facing, random position avoiding gravity/conflict
13. Set callbacks: `preprocess_func = ship_preprocess`, `postprocess_func = ship_postprocess`, `death_func = ship_death`, `collision_func = collision`
14. `ZeroVelocityComponents`, `SetElementStarShip`, `hTarget = 0`
15. `UnlockElement(hShip)`

**Impact:** Without ELEMENT creation, ships are invisible, have no collision, no preprocessing, no postprocessing. Battle is completely non-functional.

### REQ-REMED-INIT: `rust_ships_init()` Skips All Arena Setup

**Current behavior:** `ffi.rs::rust_ships_init()` → `lifecycle::init_ships()` increments a ref counter and returns NUM_SIDES. No display, galaxy, or arena objects are created.

**Required behavior (C reference: `init.c` InitShips() lines 182-249):**
1. `InitSpace()` — loads explosion/blast/asteroid animations [OK] ref-counting exists but no resource loading
2. `SetContext(StatusContext)`, `SetContext(SpaceContext)` — graphics context setup
3. `InitDisplayList()` — clear the element display list
4. `InitGalaxy()` — initialize star background
5. For HyperSpace: `ReinitQueue`, `BuildSIS`, `LoadHyperspace`, return 1
6. For battle: set clip rect, set background color, clear drawable
7. For IN_LAST_BATTLE: `free_gravity_well()`
8. For normal battle: spawn 5 asteroids + 1 planet
9. Return `NUM_SIDES`

**Impact:** Battle arena is empty — no background, no asteroids, no planet, no display list. Nothing renders.

### REQ-REMED-UNINIT: `rust_ships_uninit()` Skips Crew Writeback and C Cleanup

**Current behavior:** `ffi.rs::rust_ships_uninit()` only calls `free_master_ship_list()`. It does NOT call the existing `lifecycle::uninit_ships()` which has proper crew writeback logic.

**Required behavior (C reference: `init.c` UninitShips() lines 276-360):**
1. `StopSound()` — stop all battle audio
2. `UninitSpace()` — free explosion/blast/asteroid resources
3. `CountCrewElements()` — count floating crew in display list
4. Iterate display list: find PLAYER_SHIP elements, add floating crew to survivor
5. Write back `RaceDescPtr->ship_info.crew_level` → `StarShipPtr->crew_level`
6. `free_ship(RaceDescPtr, TRUE, TRUE)`, `RaceDescPtr = 0`
7. For IN_ENCOUNTER: `UpdateShipFragCrew()` on each surviving side
8. Clear `IN_BATTLE` from `CurrentActivity`
9. For non-IN_ENCOUNTER: `ReinitQueue` both race queues, `FreeHyperspace` if needed

**Impact:** Crew levels are never written back to fleet fragments after battle. Ships appear to have full crew after taking damage. Game state corrupted.

## What Must NOT Be Changed

### REQ-REMED-PRESERVE: Existing Working Code

The following must remain untouched:
- All 28 `ShipBehavior` implementations in `rust/src/ships/races/*.rs` (147 tests pass)
- `ffi.rs` preprocess/postprocess/death callback marshalling (`borrow_starship_from_c`, `build_element_state`, `writeback_starship`, `extract_starship_from_element`)
- Ship catalog, loader, registry, types, traits
- `battle_bridge.rs` FFI wrappers
- `rust_bridge_ships.c` existing C-side macro wrappers (new helpers are additive)
- `writeback.rs` — `battle_teardown_writeback()` logic (well-tested)
- `lifecycle.rs` — `spawn_ship()` descriptor loading and crew patching logic

## Design Constraints

### REQ-REMED-FFI-BACK: Delegate Element/Arena Operations to C

Element allocation, display list management, graphics context, galaxy init, asteroid/planet spawning, and gravity well management are all battle engine operations that remain in C. The Rust side must call BACK into C for these operations via FFI, not reimplement them.

### REQ-REMED-COMPILE: Each Phase Must Compile and Link

Every phase must leave the codebase in a state that compiles, links, and passes existing tests. No phase may break the 147 ship behavior tests. C helpers must have proper header prototypes. Symbol visibility (non-`static`) must be verified with `nm`.

### REQ-REMED-TEST: Integration Testable Without Full Engine

FFI calls into C battle engine functions (AllocElement, InitDisplayList, etc.) cannot be called in `cargo test`. Use `#[cfg(not(test))]` guards and ensure pure-Rust test paths remain functional.

### REQ-REMED-ABI: Cross-Boundary Types Use Canonical Aliases

All FFI declarations must use the type aliases defined in `ffi_contract.rs` (`CByte`, `CBoolean`, `CSize`, `CCount`, `*mut CStarship`, etc.). No raw Rust primitives (`u8`, `i16`) may appear in `extern "C"` function signatures. This ensures every cross-boundary type is traceable to its C counterpart.

### REQ-REMED-HEADER: C Helpers Have Proper Prototypes

All new C helper functions must be declared in `rust_bridge_ships.h` with proper prototypes, guarded by `#ifdef USE_RUST_SHIPS`. This enables cross-TU type checking and prevents implicit declaration warnings.

### REQ-REMED-CANONICAL-FFI: Single ABI Declaration Path

All C helper FFI bindings callable from Rust must be declared ONLY in `ffi_contract.rs`. No local `extern "C" { fn rust_bridge_... }` blocks are permitted in `ffi.rs` or any other Rust module. This ensures a single source of truth for the FFI surface and prevents duplicate declarations from drifting out of sync.

### REQ-REMED-STATIC-INVENTORY: Static Symbol Dependencies Resolved

Before copying C function bodies into `rust_bridge_ships.c`, every symbol used by those bodies must be inventoried and classified (global/static/macro). All `static` or TU-local symbols must be explicitly copied, inlined, or made accessible. The inventory must be verified by successful `-Wall` compilation.

### REQ-REMED-BUILD-MATRIX: Both Build Profiles Verified

Both Rust-enabled (`USE_RUST_SHIPS=1`) and Rust-disabled build profiles must compile and link cleanly with zero warnings. No implicit function declarations. No duplicate definitions. Verified with full clean builds.

### REQ-REMED-SPAWN-ROLLBACK: Spawn Failure Leaves Clean State

If `rust_bridge_spawn_element()` fails (returns 0), the CStarship must not be left in a half-mutated state. CStarship counter/flag writebacks are deferred until after successful element creation. Only `race_desc_ptr` is written before the C helper call, and it is freed and nulled on failure.

### REQ-REMED-UNINIT-GUARD: C-Side Uninit Has Mandatory Guard Ordering (Code-Level, Not Diagnostic)

The C-side uninit helper must validate pointers in a **mandatory, defined order** during display list iteration. **These are code-level control-flow requirements that prevent crashes — they are NOT optional diagnostics and NOT debug-only guards.** Every guard MUST be present as an `if (...) { UnlockElement; continue; }` statement in the production code path:

1. **Guard 1 (ElementPtr):** `LockElement(hElement, &ElementPtr)` — obtain element pointer. If null (should not happen but guard defensively), skip.
2. **Guard 2 (StarShipPtr extraction):** `GetElementStarShip(ElementPtr, &StarShipPtr)` — extract starship pointer safely via the macro (not manual field access).
3. **Guard 3 (StarShipPtr null check):** `if (StarShipPtr == NULL)` → `UnlockElement(hElement)` + `continue`. **MANDATORY in all builds.** No dereference of StarShipPtr may occur before this check passes.
4. **Guard 4 (RaceDescPtr null check):** `if (StarShipPtr->RaceDescPtr == NULL)` → `UnlockElement(hElement)` + `continue`. **MANDATORY in all builds.** No dereference of RaceDescPtr may occur before this check passes.
5. **Only after all four guards pass:** Read/write `StarShipPtr->RaceDescPtr->ship_info.crew_level`, call `free_ship()`, etc.

**Enforcement:** The null checks (`if (StarShipPtr == NULL)`, `if (StarShipPtr->RaceDescPtr == NULL)`) are **unconditional** — they fire in ALL builds (debug AND release). Only the `log_add` diagnostic messages inside the guard blocks are `#ifndef NDEBUG`. This distinction is critical: the guards prevent crashes; the logs assist debugging. Removing the guards in release builds would reintroduce the crash risk.

**Scenarios covered:** panic-path desync (Rust panic during init leaves partial state), init failure after partial setup (C arena partially allocated), unexpected call ordering (uninit before init), double-uninit (second pass sees null RaceDescPtr from first pass's post-free nulling), element with no associated starship (orphaned display list entry).

### REQ-REMED-IDEMPOTENT: Uninit Must Be Re-Entrant Safe

`rust_ships_uninit()` must tolerate being called multiple times (double-uninit). The second and subsequent calls must be no-ops. `rust_ships_free()` must tolerate null pointers. C-side `RaceDescPtr` must be nulled after each `free_ship()` call.

### REQ-REMED-LAYOUT: Struct Layout Must Be Verified (Hard Gate)

The `RaceDesc` (Rust) / `RACE_DESC` (C) layout compatibility must be verified at runtime with hard-fail on mismatch. A silent layout divergence would cause memory corruption in `rust_bridge_spawn_element()` and `rust_bridge_uninit_ships()`. **This verification is a mandatory hard gate** — no phase that dereferences `RaceDesc*` across the FFI boundary (P01, P03) may proceed until layout parity is proven or accessor functions are in place. There is no "proceed with risk" option.

### REQ-REMED-UNINIT-RECONCILE: Uninit Must Reconcile Rust and C State

The Rust `ships_initialized` flag is not the sole authority for whether C-side arena state exists. In failure and partial-init paths, the Rust flag may desync from C state (e.g., Rust panic during init leaves flag false but C arena is partially allocated). Before skipping uninit teardown, reconcile Rust-side state against C-side state (query `IN_BATTLE`/`CurrentActivity` via existing FFI). If C reports active battle state but Rust says uninitialized, log a warning and proceed with teardown — C state is authoritative for arena resource existence.

### REQ-REMED-CALLBACK-GUARD: Callback Entry Points Must Validate Liveness at Extraction Point Before Borrow/Marshal

All FFI callback entry points (`rust_ships_preprocess`, `rust_ships_postprocess`, `rust_ships_death`) must validate that `StarShip*` and `RaceDesc*` pointers are non-null **at the extraction point** — that is, BEFORE any borrow/marshal/conversion helper is invoked. The required check ordering is:

1. **Extract raw StarShip pointer** from the element using a minimal accessor (e.g., reading `element->pParent` directly or via `GetElementStarShip` FFI). This accessor must NOT dereference the starship pointer itself — it only reads the pointer value.
2. **Null-check StarShipPtr.** If null → early return. This fires BEFORE `borrow_starship_from_c()`, `extract_starship_from_element()`, or any other helper that would dereference it.
3. **Read `race_desc_ptr` from `(*starship_ptr).race_desc_ptr`** — a single-field dereference.
4. **Null-check race_desc_ptr.** If null → early return. This fires BEFORE `build_element_state()` or any helper that would access `RaceDesc` fields.
5. **Only then:** Call `borrow_starship_from_c()`, `build_element_state()`, or other conversion helpers.

**Key distinction from simple null checks:** The checks must occur at the **extraction point** (step 1-4 above), not after conversion. Calling `borrow_starship_from_c(starship_ptr)` when `starship_ptr` is null would dereference it inside the helper — the null check must happen BEFORE the helper is called, not inside it.

Null checks fire in ALL builds (debug AND release). Only the `eprintln!` logging is `#[cfg(debug_assertions)]`. If `extract_raw_starship_ptr()` does not exist, it must be added as a minimal helper in `ffi.rs` that reads the pointer without deep dereference.

### REQ-REMED-SPAWN-PARITY: Spawn Must Handle Both hShip Branches with Full Field Initialization

The C helper `rust_bridge_spawn_element()` must correctly handle BOTH spawn branches from `ship.c`:
- **Branch A (hShip == 0):** Fresh allocation via `AllocElement()`/`InsertElement()`, then full field init
- **Branch B (hShip != 0):** Element reuse (from `GetNextStarShip` recycling an existing handle), then full field init

**Critical:** Both branches MUST converge at `LockElement` and set ALL 22 element fields, callbacks, position, facing, and velocity **unconditionally**. The branch difference is ONLY whether `AllocElement()`/`InsertElement()` are called. Every side effect listed below must occur in both branches:

**Branch parity assertion checklist (mandatory verification in P00 and P01):**

| # | Field / Side Effect | Branch A | Branch B | C ref line |
|---|---------------------|----------|----------|------------|
| 1 | `ShipElementPtr->playerNr = StarShipPtr->playerNr` | YES | YES | 447 |
| 2 | `ShipElementPtr->crew_level = 0` | YES | YES | 448 |
| 3 | `ShipElementPtr->mass_points = ship_mass` | YES | YES | 449 |
| 4 | `ShipElementPtr->state_flags = APPEARING \| PLAYER_SHIP \| IGNORE_SIMILAR` | YES | YES | 450 |
| 5 | `ShipElementPtr->turn_wait = 0` | YES | YES | 451 |
| 6 | `ShipElementPtr->thrust_wait = 0` | YES | YES | 452 |
| 7 | `ShipElementPtr->life_span = NORMAL_LIFE` | YES | YES | 453 |
| 8 | `ShipElementPtr->colorCycleIndex = 0` | YES | YES | 454 |
| 9 | `SetPrimType(&DisplayArray[...], STAMP_PRIM)` | YES | YES | 456 |
| 10 | `current.image.farray = RDPtr->ship_data.ship` | YES | YES | 457 |
| 11 | `current.image.frame` set via `SetAbsFrameIndex` | YES | YES | 464/488 |
| 12 | `current.location.x` positioned | YES | YES | 467/493 |
| 13 | `current.location.y` positioned | YES | YES | 468/496 |
| 14 | `StarShipPtr->ShipFacing` set | YES | YES | 463/473 |
| 15 | `preprocess_func = ship_preprocess` | YES | YES | 501 |
| 16 | `postprocess_func = ship_postprocess` | YES | YES | 502 |
| 17 | `death_func = ship_death` | YES | YES | 503 |
| 18 | `collision_func = collision` | YES | YES | 504 |
| 19 | `ZeroVelocityComponents(&velocity)` | YES | YES | 505 |
| 20 | `SetElementStarShip(ShipElementPtr, StarShipPtr)` | YES | YES | 507 |
| 21 | `hTarget = 0` | YES | YES | 508 |
| 22 | `UnlockElement(hShip)` | YES | YES | 510 |
| 23 | `life_span++` (Sa-Matra only, conditional) | YES | YES | 469 |

**Structural requirement:** The C helper's `if (hShip == 0)` block must ONLY contain `AllocElement()`/`InsertElement()`. All field writes, callback assignments, and `UnlockElement` must be in the common path OUTSIDE that block. This mirrors ship.c lines 431-510 exactly.

**Branch B trigger:** Branch B occurs when `GetNextStarShip()` copies `LastStarShipPtr->hShip` to `StarShipPtr->hShip` before calling `spawn_ship()` (ship.c line 536). The replacement-ship scenario in Super Melee is the primary test case.

### REQ-REMED-INIT-MODE-MATRIX: Init Mode Branching Must Be Verified with Assertion Matrix

The C helper `rust_bridge_init_battle_arena()` branches based on `inHQSpace()` and `LOBYTE(GLOBAL(CurrentActivity))`. A mode matrix acceptance table with **explicit pre/post state checks per mode bit combination** (documented in P02) must be verified. The Rust caller must assert the return value matches the expected mode.

**Mode combinations and required side effects:**

| Mode | Condition | Return | Queue Ops | Arena Objects | Special |
|------|-----------|--------|-----------|---------------|---------|
| HyperSpace | `inHQSpace() == TRUE` | 1 | `ReinitQueue` both, `BuildSIS()` | None (no asteroids/planet) | `LoadHyperspace()` |
| Battle (normal) | `!inHQSpace() && activity != IN_LAST_BATTLE` | NUM_SIDES (2) | None | 5 asteroids + 1 planet | Clip rect, black background |
| Battle (Sa-Matra) | `!inHQSpace() && activity == IN_LAST_BATTLE` | NUM_SIDES (2) | None | None (no asteroids/planet) | `free_gravity_well()` |

**Mandatory per-mode pre/post assertions (debug builds):**

1. **Return value parity:** Rust caller asserts `num_ships == 1` for HyperSpace, `num_ships == NUM_SIDES` for battle modes.
2. **HyperSpace queue state:** After init, `race_q[0]` head should be SIS entry with `SpeciesID == SIS_SHIP_ID` and `playerNr == RPG_PLAYER_NUM`.
3. **InitSpace symmetry:** `InitSpace()` ref count incremented (verified by `UninitSpace()` symmetry — tested in P04 multi-battle).
4. **Sa-Matra exclusion:** In `IN_LAST_BATTLE` mode, NO asteroids or planets should be spawned; `free_gravity_well()` must be called.
5. **Normal battle inclusion:** In `IN_ENCOUNTER`/`SUPER_MELEE` mode, exactly 5 asteroids and 1 planet must be spawned.

**This table is the canonical reference.** P02 documents the implementation-level assertions; this requirement defines what must be true.

### REQ-REMED-BUILD-TU: Build System Must Include rust_bridge_ships.c (Verified Both Configs)

The build system must be verified to include `rust_bridge_ships.c` in the compile graph when `USE_RUST_SHIPS=1` and exclude it (or guard its contents) when disabled. This must be checked against the actual `Makeinfo` entry, not assumed.

**Mandatory acceptance checks (both configurations):**

1. **USE_RUST_SHIPS=1:** Verify `Makeinfo` conditional includes `rust_bridge_ships.c`. After build, confirm `rust_bridge_ships.o` exists and `nm` shows the expected `T` symbols for all three helpers.
2. **USE_RUST_SHIPS=0:** Verify either (a) `Makeinfo` excludes the file from the build graph entirely, OR (b) the file compiles but all new functions are `#ifdef USE_RUST_SHIPS` guarded (producing an empty object file). Confirm no link errors from missing symbols in the non-Rust build.

**Why both configs:** Without this dual verification, the helpers may compile in isolation but fail to link in one configuration. A `Makeinfo` entry that's missing for `USE_RUST_SHIPS=1` means undefined symbol errors at link time. A missing `#ifdef` guard means the `USE_RUST_SHIPS=0` build pulls in dependencies on Rust-only symbols.

### REQ-REMED-FFI-SCOPE: FFI Surface Must Be Minimized

New FFI declarations in `ffi_contract.rs` are limited to the C helper facade (`rust_bridge_*`) and accessor functions (if P05 layout verification fails). No new declarations for C functions that are already called internally by the C helpers (e.g., `InitDisplayList`, `AllocElement`, `StopSound`). Existing declarations for functions used by other Rust modules remain unchanged.

### REQ-REMED-ACTIVITY-PARITY: Activity Flags Must Match C LOBYTE(GLOBAL(CurrentActivity)) Semantics

The `activity` parameter passed from Rust to C helpers via `uqm_get_current_activity_lobyte()` must match `LOBYTE(GLOBAL(CurrentActivity))` exactly. This is critical because the C `spawn_ship()` reads `LOBYTE(GLOBAL(CurrentActivity))` inline, while the Rust path reads it via the helper function and passes it as a parameter. A mismatch means the C helper takes wrong branches (wrong positioning, wrong crew patching, wrong Sa-Matra handling).

**Mandatory verification:**
1. **Debug assertion in C helper:** `rust_bridge_spawn_element()` must assert `activity == LOBYTE(GLOBAL(CurrentActivity))` in debug builds. This catches any timing issue where the global changes between Rust's read and C's use.
2. **Init does NOT pass activity:** `rust_bridge_init_battle_arena()` reads `LOBYTE(GLOBAL(CurrentActivity))` directly from the global (no parameter). Rust does not need to pass it.
3. **Critical values:** `IN_ENCOUNTER` (used for crew patching in lifecycle_spawn), `IN_LAST_BATTLE` (Sa-Matra special case in C helper), `SUPER_MELEE` (normal battle), HyperSpace values (facing override). All must use the same macro expansion path as the original C code.

### REQ-REMED-BATTLE-BRIDGE-INVARIANT: battle_bridge.rs Lifecycle Dependencies Must Be Tested with Invariant Assertions

The `battle_bridge.rs` lifecycle independence analysis must be verified with **invariant tests and assertions around call sites**, not just documented. Specifically:

1. **Weapon creation wrappers** (`create_missile`, `create_laser`) are intentional conditional dependencies on initialized battle context. These wrappers are only callable from within battle callbacks (`preprocess`, `postprocess`) that fire after `InitShips` completes. This must be:
   - Documented with comments at each call site explaining the dependency.
   - Tested with invariant tests that verify graceful handling when context is absent (stub/null returns, no crash).
2. **Element allocation wrappers** (`alloc_element`, `lock_element`) must be tested for null/zero return handling.
3. **Sound wrappers** (`process_sound`) must be verified safe when no sound system is initialized.

The checklist (BB-1 through BB-8 in P04) is a **completion gate** — each item must be verified during implementation, not assumed.

### REQ-REMED-VERIFICATION-HOOKS: Debug Builds Must Have Scripted Scenario Checks with Expected State Transitions

Debug builds must include automated verification hooks that fire during gameplay without manual intervention. Beyond passive logging, these hooks must include **deterministic assertions that verify expected state transitions** for critical scenarios:

1. **Lifecycle event tracing:** Log init/spawn/uninit transitions with activity state (automatic).
2. **Multi-battle state leak detection:** Assert `ships_initialized == false` at init entry (catches leaks from previous battles).
3. **Sa-Matra correctness:** Assert facing==0, centered position, life_span==NORMAL_LIFE+1 after Sa-Matra spawn (automatic in debug builds).
4. **Init/uninit symmetry counters:** Track init count and verify uninit keeps pace (detects orphaned init calls).
5. **Crew writeback verification logging:** Log pre/post crew values during uninit to verify writeback correctness.
6. **Scripted scenario smoke test:** A shell script that launches the debug binary and verifies it survives startup without crashing (P04 section 11A). On macOS, `leaks --atExit` can detect memory leaks from repeated hyperspace transitions.
7. **Rust-side lifecycle roundtrip test:** Automated `cargo test` that exercises init→spawn→uninit→re-init→uninit in test mode (P04 section 11B).

### REQ-REMED-SEMANTIC-PARITY: Copied C Helper Bodies Must Be Semantically Equivalent

Copying C function bodies from their original translation units into `rust_bridge_ships.c` may introduce semantic drift if macros, globals, or static state behave differently in the new TU context. Build success alone is insufficient. Each copied body must have explicit parity verification: side effects, state transitions (queue state, activity bits, hyperspace path behavior, crew writeback outcomes) must match the original functions. Verification is documented per-function with PASS/FAIL status.

### REQ-REMED-STATE: Lifecycle State Tracking Must Be Deterministic

The `BattleState.ships_initialized` flag must be set consistently in both test and non-test paths via explicit lifecycle API methods (`mark_ships_initialized()` / `mark_ships_uninitialized()`). No path may leave this flag in an ambiguous state.

### Canonical Source Note

This requirements document is the **canonical source of truth** for all design constraints (REQ-REMED-*). The specification document describes HOW these requirements are met. Plan documents (P00-P05) describe WHEN and WHERE changes are made. When requirements and specification overlap, this document takes precedence. Plan documents should reference requirement IDs (e.g., "per REQ-REMED-UNINIT-GUARD") rather than restating the full requirement text.

## Acceptance Criteria

### Core Functionality
1. `rust_ships_spawn()` creates a real C ELEMENT with correct fields and callbacks
2. `rust_ships_init()` fully initializes the battle arena (display list, galaxy, asteroids/planet)
3. `rust_ships_uninit()` performs crew writeback, frees descriptors, stops audio, clears IN_BATTLE
4. All 147 existing ship behavior tests continue to pass
5. The binary links and runs a battle encounter without crashing
6. Crew writeback after battle is correct (surviving ship's crew is preserved in fleet fragments)

### Spawn Parity (C2)
7. Spawn handles BOTH hShip branches: `hShip==0` (fresh allocation) and `hShip!=0` (element reuse). All 22 fields/callbacks in the parity checklist (REQ-REMED-SPAWN-PARITY) are set unconditionally in both branches. Verified by code inspection AND replacement-ship test in Super Melee.
8. Spawn failure leaves CStarship in a clean state (descriptor freed and nulled, counters not written back)

### Uninit Safety (C3)
9. C-side uninit null guards are **code-level mandatory** (not debug-only): Guards 1-4 per REQ-REMED-UNINIT-GUARD fire in ALL builds. Only `log_add` diagnostics are `#ifndef NDEBUG`. Verified by code review of `rust_bridge_uninit_ships()`.
10. C-side uninit tolerates partial-init states (null StarShipPtr, null RaceDescPtr) without crashing
11. Double-uninit does not crash (idempotence) — verified by Rust-side guard with C-state reconciliation and C-side null guards

### Init Mode Matrix (H1)
12. Init mode matrix (REQ-REMED-INIT-MODE-MATRIX) verified: return value parity (1 for HyperSpace, NUM_SIDES for battle), queue operations per mode, arena objects per mode. Debug assertions in Rust caller verify return value matches expected mode.
13. Sa-Matra (IN_LAST_BATTLE), HyperSpace, and multi-battle sequences work correctly with debug-build assertions verifying state

### Callback Guards (H2)
14. Callback entry liveness checks fire BEFORE any borrow/marshal helper per REQ-REMED-CALLBACK-GUARD. `extract_raw_starship_ptr` (or equivalent) extracts pointer WITHOUT deep dereference. Null checks on both StarShipPtr and RaceDescPtr fire in ALL builds. Only logging is debug-only.

### Build System (H3)
15. Both Rust-enabled and Rust-disabled build profiles compile and link cleanly with zero warnings
16. `rust_bridge_ships.c` TU inclusion verified in BOTH `USE_RUST_SHIPS=1` and `=0` configurations per REQ-REMED-BUILD-TU. Object file and `nm` symbol checks performed.

### battle_bridge.rs (M1)
17. `battle_bridge.rs` lifecycle independence checklist (BB-1 through BB-8) completed with invariant tests. Weapon creation wrappers documented as **intended conditional dependency** on initialized battle context, with tests verifying graceful handling of null/zero returns from stubs.

### FFI Surface (M2)
18. All FFI declarations use `ffi_contract.rs` type aliases and are declared ONLY in `ffi_contract.rs` (no local duplicates). Scope limited to `rust_bridge_*` facade + accessor functions if needed. Enforced by `grep` acceptance check: `grep -rn 'fn rust_bridge_' rust/src/ships/*.rs | grep -v ffi_contract.rs | grep -vc '#\[no_mangle\]'` returns 0.

### Verification Hooks (M3)
19. Debug builds include scripted scenario checks with deterministic state transition assertions: lifecycle trace logger, multi-battle leak detection, Sa-Matra correctness assertions, init/uninit symmetry counters, crew writeback verification logging. These fire automatically during gameplay without manual intervention.
20. Rust-side lifecycle roundtrip test exercises init→uninit→re-init→uninit in test mode.

### Layout & ABI
21. Layout verification runs on first init and aborts with clear message on mismatch — **or** accessor-function fallback is implemented (P05 is a hard gate, not optional)
22. C helper prototypes are in `rust_bridge_ships.h` and `USE_RUST_SHIPS`-guarded
23. All `static`/TU-local symbol dependencies in C helpers are inventoried and resolved (BuildSIS inlined, CountCrewElements copied). Semantic parity verified (C2-A through C2-C checks marked PASS/FAIL in PR).

### Activity Flags (L1)
24. `activity` parameter from Rust matches `LOBYTE(GLOBAL(CurrentActivity))` exactly. Debug assertion in C helper verifies parity.

### State Management
25. Uninit reconciliation: if Rust flag says uninitialized but C `CurrentActivity` has `IN_BATTLE`, teardown proceeds with warning (C state authoritative). Verified by debug build output.
26. Debug builds emit assertion-logging on lifecycle state transitions in both C and Rust layers
27. Repeated hyperspace transitions (HyperSpace → StarSystem → HyperSpace, 3+ cycles) work without state leaks
