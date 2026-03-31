# Ship FFI Lifecycle Remediation — Specification

## Canonical Source

Design constraints and acceptance criteria are defined in `requirements.md` (the canonical source). This specification describes HOW those requirements are met. Plan documents (P00-P05) describe WHEN and WHERE changes are made. When this document restates a requirement, it should reference the requirement ID (e.g., "per REQ-REMED-UNINIT-GUARD"). In case of conflict, `requirements.md` takes precedence.

## Nomenclature

Throughout this specification and all plan documents, the following terms are used consistently:

| Term | Meaning |
|------|---------|
| **C helper** | A C function in `rust_bridge_ships.c` that encapsulates C-side battle engine operations, called by Rust via FFI. Prefixed `rust_bridge_`. |
| **FFI entry point** | A `#[no_mangle] pub unsafe extern "C" fn` in `ffi.rs` that C calls into Rust. Prefixed `rust_ships_`. |
| **FFI declaration** | A Rust `extern "C" { fn ... }` block declaring a C function that Rust calls. Lives in `ffi_contract.rs`. |
| **lifecycle API** | Functions in `lifecycle.rs` that manage `BattleState` — `mark_ships_initialized()`, `mark_ships_uninitialized()`, `is_ships_initialized_for_uninit()`. |
| **callback** | A C function pointer stored in an ELEMENT (`preprocess_func`, `postprocess_func`, `death_func`, `collision_func`). |
| **descriptor** | A `RACE_DESC` / `RaceDesc` — the ship type definition. Rust-owned, C-borrowed. |
| **element** | A C `ELEMENT` in the display list — represents a renderable/interactive battle object. C-owned. |
| **arena** | The complete battle environment: display list, galaxy background, asteroids, planet, gravity well. |
| **CStarship / starship** | The C `STARSHIP` struct in the race queue. C-owned. Contains `RaceDescPtr`, `hShip`, crew levels, etc. |
| **display list** | C's doubly-linked list of ELEMENTs that the battle loop processes each frame. |

## Phase Ordering and Dependency Graph

**Mandatory execution order: P00 → P05 → P01 → P02 → P03 → P04.**

### Canonical Dependency Graph

```
P00 (C helpers)
 |
 v
P05 (layout verification) ←── HARD GATE: must pass before ANY phase that
 |                              dereferences RaceDesc* across FFI boundary
 |
 +--→ P01 (wire spawn) ──→ P02 (wire init) ──→ P03 (wire uninit) ──→ P04 (integration test)
```

### Gate Conditions

| Gate | Condition | Blocks |
|------|-----------|--------|
| **G1: P05 layout pass** | `verify_race_desc_layout()` runs without abort. If it aborts, implement accessor-function fallback (P05§6) before proceeding. | P01, P03 (both dereference `RaceDesc*` as `RACE_DESC*` in C) |
| **G2: P00 compile+link** | All C helpers compile with `-Wall` zero warnings, `nm` shows expected symbols. | P05, P01, P02, P03 |
| **G3: P02 lifecycle API** | `mark_ships_initialized()` / `mark_ships_uninitialized()` / `is_ships_initialized_for_uninit()` exist and compile. | P03 (uses idempotence guard) |
| **G4: P01+P02+P03 complete** | All three wiring phases compile, link, and pass `cargo test`. | P04 |

### Phase Independence

P01 and P02 are **code-independent** (modify different functions in `ffi.rs`, can compile in either order) but have a **recommended ordering**: P01 before P02, because debugging spawn into a broken arena is harder than spawn without an arena. P03 depends on P02 for the lifecycle API (G3). P05 is a **mandatory hard prerequisite** for both P01 and P03 — no direct `RACE_DESC*` field access is permitted until layout parity is proven or accessor functions are in place.

Rationale: Spawn (P01) passes Rust-owned `RaceDesc*` to C, which dereferences it as `RACE_DESC*`. Uninit (P03) has C reading `RaceDescPtr->ship_info.crew_level` for crew writeback. Both will silently corrupt memory if the layouts don't match. P05 provides the fail-fast guard that makes P01/P03 safe to implement. There is **no optional path** — either layout parity is verified, or the accessor-function fallback is implemented. Direct C field access without verification is forbidden.

## Architecture Overview

The fundamental design decision is: **Rust calls back into C** for battle infrastructure operations. Element allocation, display list management, graphics contexts, and arena objects (asteroids, planets, gravity wells) are all deeply wired into the C battle engine and cannot be practically reimplemented in Rust. Instead, we create thin C helper functions that encapsulate the C-side work, and Rust calls these via FFI.

```
                    +------------------+
   C game engine    |    ship.c        |
   calls:           |    init.c        |
                    +--------+---------+
                             |
                    #ifdef USE_RUST_SHIPS
                             |
                    +--------v---------+
   Rust FFI         |    ffi.rs        |  <-- FFI entry points
   layer:           |  lifecycle.rs    |  <-- lifecycle API + state tracking
                    |  ffi_contract.rs |  <-- FFI declarations (types + extern "C")
                    +--------+---------+
                             |
                    extern "C" { ... }  (in ffi_contract.rs)
                             |
                    +--------v---------+
   C helpers        | rust_bridge_     |  <-- C helper functions
   (new):           |   ships.c        |
                    | rust_bridge_     |  <-- header with prototypes
                    |   ships.h        |
                    +------------------+
```

## Approach by Function

### A. spawn — "Rust loads descriptor, C allocates element"

**Strategy:** Split spawn into two stages. Stage 1 (Rust): load descriptor, patch crew. Stage 2 (C helper): allocate element, set fields, set callbacks, position ship.

**Hard prerequisite (C1):** P05 layout verification MUST pass (or accessor fallback MUST be implemented) before this function is wired. The C helper dereferences the Rust-owned pointer as `RACE_DESC*` — without verified layout parity, field access is undefined behavior.

**C helper:**
```c
// In rust_bridge_ships.c, prototype in rust_bridge_ships.h
BOOLEAN rust_bridge_spawn_element(
    STARSHIP *StarShipPtr,
    RACE_DESC *RDPtr,
    BYTE ship_mass,
    BYTE activity
);
```

This C helper does everything the C `spawn_ship()` does AFTER the descriptor is loaded:
- `AllocElement()` / `InsertElement()` if `hShip == 0`
- `LockElement()` and set all ELEMENT fields
- Handle Sa-Matra, HyperSpace, and normal positioning
- Set `preprocess_func = ship_preprocess`, etc. (all four callbacks are globally visible via `ship.h` and `tactrans.h`)
- `ZeroVelocityComponents`, `SetElementStarShip`, `hTarget = 0`
- `UnlockElement()`
- Return TRUE/FALSE

**Rust-side changes:**
1. `lifecycle::spawn_ship()` no longer constructs an `ElementConfig` — that struct is removed (element setup is entirely C-side)
2. `ffi.rs::rust_ships_spawn()` calls `lifecycle_spawn()`, then calls `rust_bridge_spawn_element()` via FFI declaration in `ffi_contract.rs`
3. The `CStarship` pointer is passed through so C can set `hShip` and callbacks directly
4. FFI declaration uses `ffi_contract.rs` type aliases (`CByte`, `CBoolean`, `*mut CStarship`) — no raw Rust primitives

**Spawn branch parity (per REQ-REMED-SPAWN-PARITY):** The C `spawn_ship()` has two branches: `hShip == 0` (fresh allocation via `AllocElement`/`InsertElement`) and `hShip != 0` (element reuse from `GetNextStarShip` recycling). Both branches converge at `LockElement` and set ALL element fields unconditionally. The C helper preserves this exactly — the `if (hShip == 0)` block ONLY controls allocation/insertion; all 22 field writes and callback assignments happen in the common path outside that block, executing in both branches. The 22-field parity assertion checklist in REQ-REMED-SPAWN-PARITY is the canonical reference. P00 documents the implementation-level verification procedure.

**Branch B detail:** When `hShip != 0` (reuse path), the element is already in the display list from a previous ship. No `AllocElement`/`InsertElement` is called. But ALL fields — including `state_flags`, `turn_wait`, `thrust_wait`, `life_span`, callbacks, velocity, position, and `hTarget` — are overwritten unconditionally. This ensures the reused element has no stale state from the previous ship.

**Why this split:** Rust correctly handles species-specific descriptor loading and crew patching. C correctly handles element allocation/positioning because those depend on DisplayArray, GetHeadElement, random position validation, gravity calculations, and hyperspace state — all deeply embedded in C.

### B. init — "C does arena setup, Rust tracks state"

**Strategy:** The C `InitShips()` function does display list init, galaxy init, context setup, asteroid/planet spawning, and hyperspace setup. None of these have ship-specific Rust logic. Rust delegates entirely to a single C helper, then updates its own state tracking.

**C helper:**
```c
// In rust_bridge_ships.c, prototype in rust_bridge_ships.h
SIZE rust_bridge_init_battle_arena(void);
```

This function contains the FULL body of the original C `InitShips()`.

**Rust-side changes:**
1. `ffi.rs::rust_ships_init()` calls `rust_bridge_init_battle_arena()` via FFI
2. On success, calls `lifecycle::mark_ships_initialized()` to set `BattleState.ships_initialized = true`
3. Does NOT call `lifecycle::init_ships()` in non-test mode (avoids double ref-count bump)
4. Test path remains unchanged (calls `init_ships()` directly)

**State tracking:** `BattleState.ships_initialized` is set deterministically via `mark_ships_initialized()` / `mark_ships_uninitialized()` lifecycle API methods. This flag is consumed by the uninit idempotence guard.

### C. uninit — "C does full teardown, Rust tracks state and guards re-entrancy"

**Strategy:** The C `UninitShips()` body is entirely C-side logic (display list iteration, audio stop, crew writeback, descriptor freeing, queue reinit). Rust delegates to a single C helper, with an idempotence guard to prevent double-uninit.

**C helper:**
```c
// In rust_bridge_ships.c, prototype in rust_bridge_ships.h
void rust_bridge_uninit_ships(void);
```

This function contains the FULL body of the original C `UninitShips()`. It calls `free_ship()` for each spawned ship, which dispatches back to `rust_ships_free()` (via `USE_RUST_SHIPS` guard) to free Rust-owned descriptors. The original C code already nulls `RaceDescPtr` after each `free_ship()` call.

**Rust-side changes:**
1. `ffi.rs::rust_ships_uninit()` checks `lifecycle::is_ships_initialized_for_uninit()` — but this flag is NOT the sole authority. Before skipping teardown, reconcile against C-side state by querying `uqm_get_current_activity_lobyte()` for the `IN_BATTLE` flag. If C says `IN_BATTLE` is set but Rust says uninitialized, log a warning and proceed with teardown anyway (C state is authoritative for arena existence). See H2 reconciliation below.
2. Calls `rust_bridge_uninit_ships()` via FFI
3. Calls `lifecycle::mark_ships_uninitialized()` to set `BattleState.ships_initialized = false`
4. Does NOT call `free_master_ship_list()` in non-test mode (catalog outlives battles)
5. `rust_ships_free()` has a null-pointer guard at the top for safety

**Multi-layer protection (per REQ-REMED-UNINIT-GUARD and REQ-REMED-IDEMPOTENT):**

1. **Rust idempotence guard with C-state reconciliation:** `is_ships_initialized_for_uninit()` is checked first but is NOT the sole authority. Before skipping teardown, reconcile against C-side state via `uqm_get_current_activity_lobyte()`. If C reports active battle but Rust says uninitialized, proceed with teardown (C state authoritative).
2. **C-side mandatory null guards with defined ordering (CODE-LEVEL, NOT DIAGNOSTIC):** The C helper validates pointers in a strict sequence before ANY dereference during display list iteration. **These are unconditional control-flow guards, not optional debug diagnostics:**
   - Guard 1: `LockElement(hElement, &ElementPtr)` — obtain element pointer
   - Guard 2: `GetElementStarShip(ElementPtr, &StarShipPtr)` — extract starship pointer via macro
   - Guard 3: `if (StarShipPtr == NULL)` → `UnlockElement(hElement)` + `continue`. **Fires in ALL builds.** No dereference of StarShipPtr permitted before this check.
   - Guard 4: `if (StarShipPtr->RaceDescPtr == NULL)` → `UnlockElement(hElement)` + `continue`. **Fires in ALL builds.** No dereference of RaceDescPtr permitted before this check.
   - Only after all four guards pass: access `RaceDescPtr->ship_info.crew_level` and other fields
   **The null checks are unconditional production code.** Only the `log_add` messages inside the guard blocks are `#ifndef NDEBUG`. This distinction is critical — removing the guards in release would reintroduce crash risk from partial-init states.
3. **C-side post-free nulling:** `RaceDescPtr = 0` after `free_ship()` prevents C from passing the same pointer twice.
4. **Rust-side null guard:** `rust_ships_free()` checks for null pointer at entry — no-op if already freed.

**Assertion logging (C3):** Both C and Rust layers log state transitions in debug builds (`log_add` on C side, `eprintln!` with `#[cfg(debug_assertions)]` on Rust side). This makes tracing desync issues between the two layers tractable. The logging is supplementary to the mandatory guards — it helps diagnose WHY a guard fired, but the guard prevents the crash regardless of whether logging is enabled.

## New FFI Declarations

**All FFI declarations live ONLY in `ffi_contract.rs`** (H1). This is the single canonical ABI declaration path. No local `extern "C" { fn rust_bridge_... }` blocks are permitted in `ffi.rs` or any other Rust module. This prevents duplicate declarations from drifting out of sync and makes auditing the FFI surface tractable.

**Enforcement acceptance check (H1):** Every phase that adds FFI declarations MUST include this verification step before the phase is considered complete:

```bash
# Must return ZERO matches (excluding #[no_mangle] export functions and
# the pre-existing local imports like uqm_get_current_activity_lobyte):
grep -n 'extern "C"' rust/src/ships/ffi.rs | grep -c 'rust_bridge_'
# Expected: 0
```

If any `rust_bridge_*` extern declaration is found in `ffi.rs` (or any file other than `ffi_contract.rs`), the phase FAILS the acceptance check and must be corrected.

All declarations use the canonical type aliases defined in `ffi_contract.rs`. No raw Rust primitive types (`u8`, `i16`) appear in cross-boundary signatures.

### FFI Surface Scope Limitation (per REQ-REMED-FFI-SCOPE)

The FFI declarations added in `ffi_contract.rs` are limited to **only** the C helper facade functions (`rust_bridge_*`) and the existing required imports. The plan MUST NOT introduce broad primitive export expansion — no new FFI declarations for C functions that are already called indirectly through the C helpers. Specifically:

**Permitted additions to `ffi_contract.rs`:**
- `rust_bridge_spawn_element` — spawn helper
- `rust_bridge_init_battle_arena` — init helper
- `rust_bridge_uninit_ships` — uninit helper
- `rust_bridge_get_race_desc_layout` — layout verification
- Accessor functions (`rust_race_desc_get_*` / `rust_race_desc_set_*`) only if P05 layout verification fails

**NOT permitted (already called by C helpers internally):**
- `InitDisplayList`, `InitGalaxy`, `SetContext`, etc. — called internally by `rust_bridge_init_battle_arena`
- `AllocElement`, `InsertElement`, `LockElement`, etc. — called internally by `rust_bridge_spawn_element`
- `StopSound`, `UninitSpace`, `CountCrewElements`, etc. — called internally by `rust_bridge_uninit_ships`
- `spawn_asteroid`, `spawn_planet`, `free_gravity_well` — called internally by init helper

The existing FFI declarations in `ffi_contract.rs` (for `AllocElement`, `LockElement`, etc.) remain for use by other Rust modules (e.g., `battle_bridge.rs`). No NEW declarations are added for functions that are only needed inside C helpers.

### In ffi_contract.rs (Rust extern "C" block)

```rust
extern "C" {
    // spawn helper
    pub fn rust_bridge_spawn_element(
        starship: *mut CStarship,
        race_desc: *mut c_void,
        ship_mass: CByte,
        activity: CByte,
    ) -> CBoolean;

    // init helper
    pub fn rust_bridge_init_battle_arena() -> CSize;

    // uninit helper
    pub fn rust_bridge_uninit_ships();

    // layout verification
    pub fn rust_bridge_get_race_desc_layout(out: *mut RaceDescLayout);
}
```

The `RaceDescLayout` struct is also defined in `ffi_contract.rs`:

```rust
#[repr(C)]
pub struct RaceDescLayout {
    pub race_desc_size: usize,
    pub ship_data_offset: usize,
    pub ship_info_offset: usize,
    pub characteristics_offset: usize,
    pub ship_data_ship_offset: usize,
    pub ship_info_crew_offset: usize,
    pub ship_info_max_crew_offset: usize,
    pub characteristics_mass_offset: usize,
}
```

### In rust_bridge_ships.h (C prototypes)

```c
#ifdef USE_RUST_SHIPS

BOOLEAN rust_bridge_spawn_element(STARSHIP *StarShipPtr,
        RACE_DESC *RDPtr, BYTE ship_mass, BYTE activity);
SIZE rust_bridge_init_battle_arena(void);
void rust_bridge_uninit_ships(void);
void rust_bridge_get_race_desc_layout(RACE_DESC_LAYOUT *out);

#endif
```

## Callback Wiring

The C helper `rust_bridge_spawn_element()` sets:
```c
ShipElementPtr->preprocess_func = ship_preprocess;
ShipElementPtr->postprocess_func = ship_postprocess;
ShipElementPtr->death_func = ship_death;
ShipElementPtr->collision_func = collision;
```

**Visibility verification (C2):** All four are globally visible with `extern` prototypes:
- `ship_preprocess`, `ship_postprocess`, `collision` — declared in `ship.h`
- `ship_death` — declared in `tactrans.h`

None are `static`. No bridge wrappers needed.

These C functions already contain `#ifdef USE_RUST_SHIPS` guards that redirect to `rust_ships_preprocess()` / `rust_ships_postprocess()` / `rust_ships_death()`. So the callback chain becomes:

```
C battle loop -> ship_preprocess() [ship.c]
    -> #ifdef USE_RUST_SHIPS -> rust_ships_preprocess() [ffi.rs]
        -> ShipBehavior::preprocess() [races/*.rs]
```

This is the intended design — the existing preprocess/postprocess FFI marshalling in ffi.rs works correctly once elements exist.

### Callback Entry Point Liveness Checks — Extraction-Point Guards (per REQ-REMED-CALLBACK-GUARD)

The callback FFI entry points (`rust_ships_preprocess`, `rust_ships_postprocess`, `rust_ships_death`) extract `StarShip*` and `RaceDesc*` pointers from the element. These pointers may be stale or null if:
- An element outlives its `StarShip` (race queue mutation during battle)
- A callback fires during the same frame an element is freed
- Uninit partially completed but the display list still has elements with stale pointers

**Critical requirement:** Liveness checks MUST occur **at the extraction point** — that is, BEFORE any borrow/marshal helper that would dereference the pointer. The distinction is between extracting a raw pointer value (safe — just reading a memory address) and dereferencing that pointer (unsafe — following the address). The checks must fire between extraction and dereference.

**Specifically, checks must fire BEFORE:**
- `borrow_starship_from_c()` — which dereferences the starship pointer to build a Rust Starship
- `extract_starship_from_element()` — which dereferences `element→pParent` to get StarShip*
- `build_element_state()` — which reads element fields through the starship/descriptor

**Required check ordering (implemented in P04):**

```rust
// Step 1: Extract raw StarShip pointer from element — reads element->pParent field value
//         WITHOUT following the pointer. Uses GetElementStarShip or direct field read.
//         If extract_raw_starship_ptr() doesn't exist, add it as a minimal helper.
let starship_ptr = extract_raw_starship_ptr(element_ptr);

// Step 2: LIVENESS CHECK — BEFORE borrow_starship_from_c or any other marshal helper
//         This check fires in ALL BUILDS (debug AND release).
if starship_ptr.is_null() {
    #[cfg(debug_assertions)]
    eprintln!("rust_ships_{name}: null StarShipPtr at entry, skipping");
    return;  // Early return — no further dereference
}

// Step 3: Minimal dereference to read race_desc_ptr field from starship struct
let race_desc_ptr = (*starship_ptr).race_desc_ptr;

// Step 4: LIVENESS CHECK — BEFORE any RaceDesc access
//         This check fires in ALL BUILDS (debug AND release).
if race_desc_ptr.is_null() {
    #[cfg(debug_assertions)]
    eprintln!("rust_ships_{name}: null RaceDescPtr at entry, skipping");
    return;  // Early return — no further dereference
}

// Step 5: NOW safe to call borrow_starship_from_c(), build_element_state(), etc.
//         Both pointers are known non-null at this point.
```

These checks are lightweight (two pointer comparisons) and prevent undefined behavior from propagating into Rust ship behavior code. **The null checks fire in BOTH debug and release builds** — only the `eprintln!` logging is debug-only. The key difference from simple null checks is the **placement**: they must occur at the extraction point, before any conversion/borrowing helper runs. Placing them inside `borrow_starship_from_c()` would be too late — the function signature takes a `*mut CStarship` and may dereference it immediately.

## Lifecycle State Machine

```
         rust_ships_init()
              |
              v
    +----> [UNINITIALIZED] ----> rust_bridge_init_battle_arena()
    |         ^                         |
    |         |                    mark_ships_initialized()
    |         |                         |
    |    mark_ships_                    v
    |    uninitialized()         [INITIALIZED]
    |         |                    |         |
    |         |          rust_ships_spawn()  |
    |         |                    |         |
    |         |                    v         |
    |    rust_bridge_        [BATTLE ACTIVE] |
    |    uninit_ships()            |         |
    |         |          rust_ships_uninit() |
    |         +<-----------+<---------------+
    |                      |
    +--- (double-uninit guard with reconciliation:
          1. Check Rust flag (is_ships_initialized_for_uninit())
          2. If Rust says NO, check C state (IN_BATTLE flag)
          3. If C also says NO → safe skip (no-op)
          4. If C says YES but Rust says NO → warn + proceed with teardown
             (desync recovery — C state is authoritative for arena existence))
```

## Layout Verification

A one-time layout check runs during the first `rust_ships_init()` call. It queries C for `RACE_DESC` field offsets via `rust_bridge_get_race_desc_layout()` and compares them against Rust's `RaceDesc` layout. On mismatch, it prints all divergent offsets and calls `abort()`. This runs in all builds (not just debug) because a layout mismatch means silent memory corruption.

## Risk Mitigation

1. **Circular FFI:** C calls Rust (rust_ships_spawn), Rust calls C (rust_bridge_spawn_element). This is safe because Rust's catch_unwind boundary prevents panics from propagating, and the C helpers are leaf functions (no callbacks back into Rust during spawn). The uninit path does have a C→Rust callback (`free_ship` → `rust_ships_free`), but no Rust locks are held at that point.

2. **Pointer lifetime:** `CStarship*` passed to `rust_bridge_spawn_element` is C-owned queue storage with stable address. `RACE_DESC*` is Rust-owned via Box::into_raw and remains valid until free_ship. Post-free nulling prevents use-after-free.

3. **Double-uninit (C3):** Four-layer defense: (1) Rust idempotence guard via `is_ships_initialized_for_uninit()` with C-state reconciliation (see item 10), (2) C-side null guards on every element during iteration, (3) C-side post-free nulling of `RaceDescPtr`, (4) Rust-side null check in `rust_ships_free()`. Both layers emit debug logging on state transitions.

4. **Spawn failure rollback (H3):** CStarship mutations are ordered to minimize inconsistent state on failure. Only `race_desc_ptr` is written before the C helper call; all counter/flag writebacks happen after success. On failure, `race_desc_ptr` is freed and nulled. See P01 rollback contract table.

5. **Partial init / uninit desync (C3):** The C-side uninit helper guards against null `StarShipPtr` and null `RaceDescPtr` on every element. This handles panic-path desync, init failure after partial setup, and external call ordering issues. Debug logging traces all transitions.

6. **Test isolation:** All C FFI calls are `#[cfg(not(test))]` guarded. Pure-Rust test paths remain unchanged.

7. **Incremental delivery:** Phases are ordered P00 → P05 → P01 → P02 → P03 → P04. P05 (layout verification) is a **mandatory hard gate** before P01/P03 — not optional. Each phase is independently compilable.

8. **ABI safety (H1):** All cross-boundary type declarations use `ffi_contract.rs` aliases and are declared ONLY in `ffi_contract.rs`. No local `extern "C"` duplicates permitted. Enforcement: each phase includes a `grep` acceptance check that no `rust_bridge_*` extern declarations exist outside `ffi_contract.rs`. Layout verification hard-fails on mismatch with `abort()`.

9. **Static symbol dependencies (C1):** Every symbol used by C helper bodies has been inventoried and classified (global/static/macro). Two `static` functions from `init.c` (`BuildSIS`, `CountCrewElements`) are copied/inlined into `rust_bridge_ships.c` rather than made public, preserving init.c's encapsulation.

10. **Uninit state reconciliation (H2):** The Rust `ships_initialized` flag is not the sole authority for whether C-side arena state exists. In failure/partial-init paths, the Rust flag may desync from C state. Before the Rust idempotence guard skips teardown, it reconciles against C-side state: query `GLOBAL(CurrentActivity) & IN_BATTLE` via the existing `uqm_get_current_activity_lobyte()` FFI. If C reports `IN_BATTLE` active but Rust says uninitialized, log a warning and proceed with teardown (C state is authoritative for arena existence). If both agree (Rust says uninitialized AND C says no `IN_BATTLE`), skip teardown safely. This prevents the scenario where a Rust panic during init leaves `ships_initialized = false` but C arena resources are partially allocated.

11. **Callback chain liveness (H2):** All FFI callback entry points (`rust_ships_preprocess`, `rust_ships_postprocess`, `rust_ships_death`) must validate that the `StarShip*` and `RaceDesc*` pointers extracted from the element are non-null BEFORE any borrow/marshal helper (`borrow_starship_from_c`, `extract_starship_from_element`, `build_element_state`) is called. Checks occur at the extraction point, not after conversion. This guards against stale element references, races between spawn and first callback, and desync after partial uninit.

12. **Semantic parity of copied C helpers (C2):** Build success alone does not prove that copied C function bodies behave identically in `rust_bridge_ships.c` vs their original translation units. Each copied body must have explicit parity verification: (a) side effects and state transitions match the original (queue mutations, activity bit changes, hyperspace path behavior, crew writeback outcomes), (b) no macro/global behavior differs due to TU context, (c) P04 integration tests include specific assertions for each state transition the helper performs. Additionally, spawn_ship's two branches (hShip==0 and hShip!=0) must BOTH be verified against the 22-field parity assertion checklist in REQ-REMED-SPAWN-PARITY — this means confirming that all field writes and callback assignments are in the common path OUTSIDE the `if (hShip == 0)` block, not inside it.

13. **Build system TU inclusion (per REQ-REMED-BUILD-TU):** `rust_bridge_ships.c` must be verified as part of the build graph in BOTH configurations. When `USE_RUST_SHIPS=1`: Makeinfo conditional inclusion confirmed, `rust_bridge_ships.o` exists after build, `nm` shows expected `T` symbols. When `USE_RUST_SHIPS=0`: either file is excluded from build graph entirely, or all code is `#ifdef USE_RUST_SHIPS` guarded (no link errors from missing Rust symbols). Both configurations must be verified — not just the Rust-enabled one.

14. **FFI surface scope (M2):** New FFI declarations in `ffi_contract.rs` are limited to the C helper facade (`rust_bridge_*`) and accessor functions (if needed). No broad expansion of primitive C function imports that are already called internally by the C helpers.

15. **Activity flag semantics (per REQ-REMED-ACTIVITY-PARITY):** The `activity` parameter passed from Rust to C helpers must match `LOBYTE(GLOBAL(CurrentActivity))` exactly. Verified by a debug assertion in the C helper: `assert(activity == LOBYTE(GLOBAL(CurrentActivity)))`. The C `spawn_ship()` reads the global inline; the Rust path reads it via `uqm_get_current_activity_lobyte()` and passes it as a parameter. The assertion catches timing drift. For init, the C helper reads the global directly (no parameter from Rust).

## Files Changed Summary

| File | Change Type | Description |
|------|-------------|-------------|
| `sc2/src/uqm/rust_bridge_ships.h` | NEW | Prototypes for all `rust_bridge_` helpers + `RACE_DESC_LAYOUT` typedef, `USE_RUST_SHIPS`-guarded |
| `sc2/src/uqm/rust_bridge_ships.c` | ADD functions | `rust_bridge_spawn_element` (with C2 branch parity + debug assertions), `rust_bridge_init_battle_arena` (with inlined BuildSIS), `rust_bridge_uninit_ships` (with C3 mandatory guard ordering + debug logging + M3 crew writeback verification), `rust_bridge_CountCrewElements` (local static copy), `rust_bridge_get_race_desc_layout` |
| `rust/src/ships/ffi.rs` | MODIFY | Wire spawn/init/uninit to C helpers, add layout verification (`verify_race_desc_layout` with `Once` guard), add uninit idempotence guard with H2 C-state reconciliation, add spawn failure rollback, callback liveness checks (H2) BEFORE borrow/marshal helpers, debug assertions, M3 lifecycle trace logger |
| `rust/src/ships/ffi_contract.rs` | ADD | FFI declarations for C helpers using canonical type aliases (single source of truth, H1). Scope limited to `rust_bridge_*` facade + accessor functions if needed (M2 — no broad primitive expansion). `RaceDescLayout` struct. |
| `rust/src/ships/lifecycle.rs` | MODIFY | Remove `ElementConfig`, add `mark_ships_initialized()`, `mark_ships_uninitialized()`, `is_ships_initialized_for_uninit()` |
| `rust/src/ships/battle_bridge.rs` | VERIFY + TEST | Lifecycle independence checklist completed. Lifecycle invariant tests added (M1). Weapon creation wrappers (`create_missile`, `create_laser`) documented as **intended conditional dependency** on initialized battle context. See P04 BB-4. |
