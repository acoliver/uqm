# Ship FFI Lifecycle Remediation — Initial State Inventory

## File Inventory

### Rust Files (to modify)

| File | Lines | Status | Role |
|------|-------|--------|------|
| `rust/src/ships/ffi.rs` | 703 | BROKEN | FFI entry points — spawn/init/uninit are stubs |
| `rust/src/ships/lifecycle.rs` | 900 | PARTIAL | spawn loads desc but no element; init is ref-count only |
| `rust/src/ships/ffi_contract.rs` | 420 | OK | ABI types and extern "C" declarations — has AllocElement etc. |
| `rust/src/ships/writeback.rs` | ~1060 | OK | Crew writeback logic — well-tested, needs to be called from uninit |
| `rust/src/ships/c_bridge.rs` | 477 | OK | Resource loading FFI — no changes needed |
| `rust/src/ships/runtime.rs` | 1376 | OK | Ship preprocess/postprocess pipeline — working |
| `rust/src/ships/types.rs` | 1102 | OK | Core types — no changes needed |
| `rust/src/ships/battle_bridge.rs` | - | OK | MissileBlock, LaserBlock FFI wrappers — working |
| `rust/src/battle/element.rs` | 950 | OK | Element struct, flags, callbacks — fully defined |

### C Files (to modify or add helpers)

| File | Lines | Status | Role |
|------|-------|--------|------|
| `sc2/src/uqm/ship.c` | 591 | GUARDED | `spawn_ship()` line 396: `#ifdef USE_RUST_SHIPS` returns to Rust |
| `sc2/src/uqm/init.c` | 361 | GUARDED | `InitShips()` line 184, `UninitShips()` line 278: both guarded |
| `sc2/src/uqm/loadship.c` | 237 | GUARDED | `load_ship()` line 221, `free_ship()` line 231: both guarded |
| `sc2/src/uqm/rust_bridge_ships.c` | - | OK | C-side macro wrappers for Rust FFI |

### Build System

| File | Status | Notes |
|------|--------|-------|
| `sc2/build/unix/build.config` | ACTIVE | `USE_RUST_SHIPS=1` when enabled action selected |
| `sc2/build.vars.in` | ACTIVE | Exports `USE_RUST_SHIPS` and `SYMBOL_USE_RUST_SHIPS_DEF` |

## Detailed State Analysis

### ffi.rs — Current State

**Working functions:**
- `rust_ships_load_catalog()` / `rust_ships_free_catalog()` — catalog management
- `rust_ships_get_cost_by_index()` — cost lookup
- `rust_ships_load()` / `rust_ships_free()` — descriptor load/free
- `rust_ships_build()` — queue entry allocation via C's AllocLink/PutQueue
- `rust_ships_clone_fragment()` — fragment field copying
- `rust_ships_preprocess()` / `rust_ships_postprocess()` / `rust_ships_death()` — callback marshalling (works IF elements exist)

**Broken functions:**
- `rust_ships_spawn()` (line 261): Constructs Rust `Starship` from `CStarship`, calls `lifecycle_spawn()`, writes back `race_desc_ptr`. But lifecycle_spawn never creates an ELEMENT.
- `rust_ships_init()` (line 324): Calls `init_ships(activity)` which just increments ref counter. Returns NUM_SIDES but arena is empty.
- `rust_ships_uninit()` (line 348): Only calls `free_master_ship_list()`. Does NOT call `lifecycle::uninit_ships()`.

### lifecycle.rs — Current State

**`spawn_ship()` (line 128):**
- Loads descriptor via `load_ship()` -- works
- Clears input/status -- works
- Patches crew for IN_ENCOUNTER/IN_LAST_BATTLE -- works
- Computes `ElementConfig` at line 161 -- DEAD CODE (prefixed `_element_config`)
- Stores descriptor in starship -- works
- MISSING: AllocElement, InsertElement, LockElement, set ELEMENT fields, set callbacks, set position/facing, SetPrimType, UnlockElement

**`init_ships()` (line 257):**
- Calls `init_space()` (ref count only, no resource loading)
- Sets `ships_initialized = true`
- Returns NUM_SIDES for battle, 1 for hyperspace
- MISSING: SetContext, InitDisplayList, InitGalaxy, clip rect, background color, asteroid/planet spawning, HyperSpace setup (BuildSIS, LoadHyperspace), IN_LAST_BATTLE gravity well

**`uninit_ships()` (line 298):**
- Takes `race_queues` and `fragment_queues` as Rust `Vec<Starship>` / `Vec<ShipFragment>`
- Calls `battle_teardown_writeback()` -- LOGIC IS CORRECT but never gets called
- PROBLEM: The FFI entry `rust_ships_uninit()` does NOT call this function
- PROBLEM: Even if called, the signature requires Rust-owned Vec queues, but queues are C-owned

### ffi_contract.rs — FFI Declarations Available

Already declares (line 324-356):
- `AllocElement() -> HElement`
- `InsertElement(h, after)`
- `LockElement(h, &mut element_ptr)`
- `UnlockElement(h)`
- `GetHeadElement() -> HElement`
- `GetSuccElement(element) -> HElement`
- `ProcessSound(sound, element)`

NOT yet declared (will need to be added):
- `InitDisplayList()`
- `InitGalaxy()`
- `SetContext(context) -> context`
- `SetContextFGFrame(frame)`
- `SetContextClipRect(rect)`
- `SetContextBackGroundColor(color)`
- `ClearDrawable()`
- `spawn_asteroid(element)`
- `spawn_planet()`
- `free_gravity_well()`
- `StopSound()`
- `BuildSIS() -> HSTARSHIP`
- `LoadHyperspace()` / `FreeHyperspace()`
- `ReinitQueue(queue)`
- `SetPrimType(prim, type)`
- `SetAbsFrameIndex(frame, index) -> frame`
- `ZeroVelocityComponents(velocity)`
- `SetElementStarShip(element, starship)`
- `TFB_Random() -> u32`
- `WRAP_X`, `WRAP_Y`, `DISPLAY_ALIGN_X`, `DISPLAY_ALIGN_Y` (macros)
- `CalculateGravity(element) -> bool`
- `TimeSpaceMatterConflict(element) -> bool`
- `inHQSpace() -> bool`
- `CountCrewElements() -> COUNT`
- `UpdateShipFragCrew(starship)`
- `FleetIsInfinite(side) -> bool`

### C Reference: Key Function Signatures

```c
// ship.c
static BOOLEAN spawn_ship(STARSHIP *StarShipPtr);  // static — NOT directly callable from other TUs
void ship_preprocess(ELEMENT *ElementPtr);           // GLOBAL — declared extern in ship.h:32
void ship_postprocess(ELEMENT *ElementPtr);          // GLOBAL — declared extern in ship.h:33
void collision(ELEMENT *E0, POINT *P0, ELEMENT *E1, POINT *P1);  // GLOBAL — declared extern in ship.h:34

// tactrans.c
void ship_death(ELEMENT *ShipPtr);                   // GLOBAL — declared extern in tactrans.h:40

// init.c
BOOLEAN InitSpace(void);
void UninitSpace(void);
SIZE InitShips(void);
void UninitShips(void);
static COUNT CountCrewElements(void);                // static — only visible within init.c
static HSTARSHIP BuildSIS(void);                     // static — only visible within init.c

// loadship.c
RACE_DESC *load_ship(SPECIES_ID id, BOOLEAN battle);
void free_ship(RACE_DESC *rd, BOOLEAN icon, BOOLEAN battle);
```

**Callback visibility summary:** All four element callbacks (`ship_preprocess`, `ship_postprocess`, `collision`, `ship_death`) are **globally visible** with `extern` prototypes in public headers (`ship.h`, `tactrans.h`). None are `static`. No C bridge wrappers are needed for callback assignment in `rust_bridge_spawn_element()`.

### Test State

- 147 ship behavior tests: all pass (`cargo test` in ships module)
- `ffi.rs` tests: 7 tests, all pass (use stubs/null-pointer safety)
- `lifecycle.rs` tests: 26 tests, all pass (pure Rust, no C calls)
- `writeback.rs` tests: ~20 tests, all pass (pure Rust)

### Existing extern "C" Imports in ffi.rs (line 31-44)

```rust
extern "C" {
    fn AllocLink(queue: *mut c_void) -> *mut c_void;
    fn PutQueue(queue: *mut c_void, link: *mut c_void);
    fn LockLink(queue, link) -> *mut c_void;
    fn UnlockLink(queue, link);
    fn GetLinkSize(queue) -> usize;
    fn uqm_get_current_activity_lobyte() -> u8;
}
```

## Key Architectural Insight

The overall strategy is "Rust handles ship behavior, C handles battle infrastructure." The remediation must make the FFI layer correctly delegate infrastructure calls to C while keeping ship-specific logic in Rust. This means:

1. **spawn**: Rust loads descriptor + patches crew, then calls C for element creation/setup
2. **init**: Rust delegates entirely to a C helper that does arena setup, then gets back control for ship-specific init
3. **uninit**: Rust iterates C-owned display list and queues via C helper calls, performs crew writeback, then delegates C-side cleanup
