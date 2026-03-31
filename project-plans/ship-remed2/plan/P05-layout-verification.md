# P05 — RaceDesc/RACE_DESC Layout Verification

## Goal

Add compile-time and runtime assertions that Rust's `RaceDesc` and C's `RACE_DESC` have matching layouts **at the fields accessed across the FFI boundary**. Hard-fail with a clear diagnostic message if any mismatch is detected. **This phase is a MANDATORY HARD GATE** (per REQ-REMED-LAYOUT) — P01 and P03 MUST NOT proceed until P05 has either confirmed layout parity or the accessor-function fallback (section 6) is implemented. There is no "proceed anyway" option.

## Phase Ordering

**Mandatory order: P00 → P05 → P01 → P02 → P03 → P04.**

Spawn (P01) passes a Rust-owned `RaceDesc*` to C, which dereferences it as `RACE_DESC*` to access `ship_data.ship`, `characteristics.ship_mass`, etc. Uninit (P03) has C reading `RaceDescPtr->ship_info.crew_level` and `ship_info.max_crew` for crew writeback. Both operations perform **direct struct field dereference** across the FFI boundary — if field offsets differ by even one byte, the result is silent memory corruption or crash.

P05 is the gate that determines whether direct dereference is safe. If P05 reveals a mismatch, the accessor-function fallback (section 6) MUST be implemented before P01/P03 proceed. No direct C field access of the Rust `RaceDesc` pointer is permitted without verified layout parity or accessor functions.

## Prerequisite

P00 (for the layout query function in `rust_bridge_ships.c` and `rust_bridge_ships.h`). Can otherwise be done at any time.

## Motivation

`rust_bridge_spawn_element()` receives a `RACE_DESC*` that is actually a Rust `RaceDesc*` cast through `Box::into_raw`. It accesses:
- `RDPtr->ship_data.ship` (frame array pointer)
- `RDPtr->ship_data.ship[0]` (first frame for `SetAbsFrameIndex`)
- `RDPtr->characteristics.ship_mass` (already used by Rust, less risky)
- `RDPtr->ship_info.crew_level` (used by uninit crew writeback)
- `RDPtr->ship_info.max_crew` (used by uninit crew writeback)

If ANY of these fields are at different offsets in Rust vs C, we get silent data corruption or crashes.

**Critical note on current layout:** Rust's `RaceDesc` is NOT `#[repr(C)]` and has different field ordering and types from C's `RACE_DESC`. C's `RACE_DESC` has function pointers (`uninit_func`, `preprocess_func`, etc.) and `CodeRef` that Rust replaces with `behavior: Box<dyn ShipBehavior>`. This means a direct cast is NOT safe unless either:
1. Rust's `RaceDesc` is made `#[repr(C)]` with matching field order and padding, OR
2. Accessor functions are used instead of direct field access.

The verification in this phase will **definitively determine** which approach is needed before P01 proceeds.

## Changes

### 1. C-side static assertions in rust_bridge_ships.c

```c
#include <stddef.h>

// Compile-time verification that RACE_DESC field offsets are plausible.
// These catch gross layout errors (e.g., reordered fields, missing padding).
_Static_assert(sizeof(RACE_DESC) > 0, "RACE_DESC must be non-empty");
_Static_assert(offsetof(RACE_DESC, ship_data) > offsetof(RACE_DESC, characteristics),
    "ship_data must follow characteristics in RACE_DESC");
_Static_assert(offsetof(RACE_DESC, ship_info) < offsetof(RACE_DESC, ship_data),
    "ship_info must precede ship_data in RACE_DESC");
```

### 2. Runtime layout query function in rust_bridge_ships.c

Add a C function that returns critical offsets, callable from Rust. **This is declared in `rust_bridge_ships.h` (added in P00).** All FFI declarations for this function go through `ffi_contract.rs` — no local `extern "C"` in `ffi.rs` (H1).

```c
void
rust_bridge_get_race_desc_layout(RACE_DESC_LAYOUT *out)
{
    out->race_desc_size = sizeof(RACE_DESC);
    out->ship_data_offset = offsetof(RACE_DESC, ship_data);
    out->ship_info_offset = offsetof(RACE_DESC, ship_info);
    out->characteristics_offset = offsetof(RACE_DESC, characteristics);
    out->ship_data_ship_offset = offsetof(RACE_DESC, ship_data)
        + offsetof(DATA_STUFF, ship);
    out->ship_info_crew_offset = offsetof(RACE_DESC, ship_info)
        + offsetof(SHIP_INFO, crew_level);
    out->ship_info_max_crew_offset = offsetof(RACE_DESC, ship_info)
        + offsetof(SHIP_INFO, max_crew);
    out->characteristics_mass_offset = offsetof(RACE_DESC, characteristics)
        + offsetof(CHARACTERISTIC_STUFF, ship_mass);
}
```

### 3. FFI declaration in ffi_contract.rs — single canonical path (C2, H1)

**All C helper FFI bindings MUST be declared only in `ffi_contract.rs`. No local `extern "C"` blocks in `ffi.rs` are permitted for any C helper function.** This is the single canonical ABI declaration path. P01/P02/P03 also follow this rule.

Add to `ffi_contract.rs`:

```rust
// ---------------------------------------------------------------------------
// Layout verification (P05)
// ---------------------------------------------------------------------------

/// Layout descriptor for RACE_DESC C-side field offsets.
/// Matches C RACE_DESC_LAYOUT in rust_bridge_ships.h.
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

extern "C" {
    /// Query C-side RACE_DESC field offsets for layout verification.
    /// C: void rust_bridge_get_race_desc_layout(RACE_DESC_LAYOUT *out);
    /// Prototype: rust_bridge_ships.h
    pub fn rust_bridge_get_race_desc_layout(out: *mut RaceDescLayout);
}
```

**Why in `ffi_contract.rs`, not a local extern in `ffi.rs` (C2/H1):** The original plan introduced a local `extern "C"` block for `rust_bridge_get_race_desc_layout` inside `ffi.rs`. This contradicts the plan's own ABI discipline — `ffi_contract.rs` is the single source of truth for all C↔Rust FFI declarations. Local duplicates create maintenance risk (signature drift, missed renames) and make auditing the FFI surface harder. All C helper declarations — including layout verification — go through `ffi_contract.rs`.

### 4. Rust-side layout check — deterministic offset computation (C2)

In `ffi.rs`, add a one-time check called during `rust_ships_init()`. This is NOT debug-only — it runs in all builds because a layout mismatch means silent memory corruption.

**Deterministic offset strategy (C2 fix):** Instead of using `MaybeUninit` + `addr_of` (error-prone manual computation), use `std::mem::offset_of!` (stabilized in Rust 1.77+) for explicit, named fields. If `offset_of!` is not available, use a well-tested helper macro with explicit field names — no "fill in later" placeholders.

```rust
/// Verify that Rust RaceDesc and C RACE_DESC have matching field offsets.
/// Called once during init. Hard-fails with a clear message on mismatch.
///
/// This function uses ffi_contract::RaceDescLayout and
/// ffi_contract::rust_bridge_get_race_desc_layout — the canonical FFI
/// declarations in ffi_contract.rs. No local extern "C" blocks.
#[cfg(not(test))]
fn verify_race_desc_layout() {
    use crate::ships::ffi_contract::{RaceDescLayout, rust_bridge_get_race_desc_layout};

    unsafe {
        let mut c_layout = std::mem::zeroed::<RaceDescLayout>();
        rust_bridge_get_race_desc_layout(&mut c_layout);

        let rust_size = std::mem::size_of::<RaceDesc>();

        // Collect ALL mismatches before reporting (developer can fix them all at once)
        let mut mismatches: Vec<String> = Vec::new();

        // --- Size check ---
        if c_layout.race_desc_size != rust_size {
            mismatches.push(format!(
                "RACE_DESC size: C={} Rust={}",
                c_layout.race_desc_size, rust_size
            ));
        }

        // --- Deterministic offset checks using offset_of! ---
        // Each check names the exact field path and compares C vs Rust offset.
        // offset_of! was stabilized in Rust 1.77. If using an older toolchain,
        // replace with the memoffset crate's offset_of! macro.

        macro_rules! check_offset {
            ($rust_type:ty, $field:ident, $c_offset:expr, $name:expr) => {
                let rust_offset = std::mem::offset_of!($rust_type, $field);
                if rust_offset != $c_offset {
                    mismatches.push(format!(
                        "{}: C={} Rust={}",
                        $name, $c_offset, rust_offset
                    ));
                }
            };
        }

        check_offset!(RaceDesc, ship_info, c_layout.ship_info_offset, "ship_info");
        check_offset!(RaceDesc, characteristics, c_layout.characteristics_offset, "characteristics");
        check_offset!(RaceDesc, ship_data, c_layout.ship_data_offset, "ship_data");

        // For nested field offsets (ship_data.ship, ship_info.crew_level, etc.),
        // compute the Rust offset as parent_offset + nested_offset:
        let rust_ship_data_offset = std::mem::offset_of!(RaceDesc, ship_data);
        let rust_ship_data_ship_offset = rust_ship_data_offset
            + std::mem::offset_of!(ShipData, ship);
        if rust_ship_data_ship_offset != c_layout.ship_data_ship_offset {
            mismatches.push(format!(
                "ship_data.ship: C={} Rust={}",
                c_layout.ship_data_ship_offset, rust_ship_data_ship_offset
            ));
        }

        let rust_ship_info_offset = std::mem::offset_of!(RaceDesc, ship_info);
        let rust_crew_offset = rust_ship_info_offset
            + std::mem::offset_of!(ShipInfo, crew_level);
        if rust_crew_offset != c_layout.ship_info_crew_offset {
            mismatches.push(format!(
                "ship_info.crew_level: C={} Rust={}",
                c_layout.ship_info_crew_offset, rust_crew_offset
            ));
        }

        let rust_max_crew_offset = rust_ship_info_offset
            + std::mem::offset_of!(ShipInfo, max_crew);
        if rust_max_crew_offset != c_layout.ship_info_max_crew_offset {
            mismatches.push(format!(
                "ship_info.max_crew: C={} Rust={}",
                c_layout.ship_info_max_crew_offset, rust_max_crew_offset
            ));
        }

        let rust_char_offset = std::mem::offset_of!(RaceDesc, characteristics);
        let rust_mass_offset = rust_char_offset
            + std::mem::offset_of!(Characteristics, ship_mass);
        if rust_mass_offset != c_layout.characteristics_mass_offset {
            mismatches.push(format!(
                "characteristics.ship_mass: C={} Rust={}",
                c_layout.characteristics_mass_offset, rust_mass_offset
            ));
        }

        if !mismatches.is_empty() {
            // HARD FAIL: Layout mismatch means silent memory corruption.
            // Print ALL mismatches so the developer can fix them all at once.
            let msg = format!(
                "FATAL: RaceDesc/RACE_DESC layout mismatch detected!\n\
                 The Rust RaceDesc struct does not match the C RACE_DESC layout.\n\
                 This WILL cause memory corruption during ship spawn/uninit.\n\
                 Mismatches:\n  {}\n\
                 \n\
                 Fix options:\n\
                 1. Make RaceDesc #[repr(C)] with matching field order/padding in types.rs\n\
                 2. Switch to accessor-function approach (see P05 fallback plan)\n\
                 See specification.md section 'Layout Verification' for details.",
                mismatches.join("\n  ")
            );
            eprintln!("{}", msg);
            // abort() rather than panic!() — we're in an FFI context,
            // unwinding across FFI is UB. panic!() would be caught by
            // catch_unwind and swallowed, causing silent corruption later.
            std::process::abort();
        }
    }
}
```

**Why `abort()` not `panic!()`:** We're inside a `catch_unwind` in FFI context. `panic!()` would be caught and swallowed, returning 0 from `rust_ships_init()`. The game would continue with an uninitialized arena and crash later. `abort()` is the only correct response to a layout mismatch.

**Why runtime exact compare, not weak static asserts (C2 fix):** The original plan proposed `_Static_assert(offsetof(...) > 0)` which only checks "non-zero" — it would pass even if offsets were entirely wrong. The runtime check above does **exact equality comparison** of every critical offset. The C-side `_Static_assert` checks (section 1) are retained only as compile-time sanity checks for gross errors (e.g., empty struct, reordered top-level fields). The authoritative check is the runtime exact compare.

### 5. Call `verify_race_desc_layout()` from `rust_ships_init()`

In the non-test path of `rust_ships_init()`, call the verification before the C helper:

```rust
#[cfg(not(test))]
unsafe {
    // One-time layout verification — aborts with clear message on mismatch
    static LAYOUT_VERIFIED: std::sync::Once = std::sync::Once::new();
    LAYOUT_VERIFIED.call_once(|| {
        verify_race_desc_layout();
    });

    let num_ships = rust_bridge_init_battle_arena();
    // ...
}
```

Use `std::sync::Once` to ensure it only runs on the first init call (subsequent init calls after uninit skip the verification since the layout is immutable at compile time).

### 6. Fallback: Accessor functions if layout diverges (MANDATORY if P05 fails)

If layout verification reveals mismatches that cannot be fixed in Rust types (likely, given that Rust's `RaceDesc` is not `#[repr(C)]` and contains `Box<dyn ShipBehavior>`), the fallback is to NOT pass `RaceDesc*` to C for direct field access. Instead, add C-callable accessor functions in `ffi.rs`.

**This is NOT optional.** If `verify_race_desc_layout()` would abort, the accessor approach MUST be implemented **in this same phase (P05)** before P01 or P03 can proceed. P01/P03 are hard-gated on P05 completion, and P05 is not complete until either (a) layout parity is confirmed or (b) accessor functions are implemented and the C helpers are modified to use them.

**Accessor functions to add in `ffi.rs`:**

```rust
/// C calls this to get ship_data.ship frame array pointer.
#[no_mangle]
pub unsafe extern "C" fn rust_race_desc_get_ship_frames(
    rd: *const c_void,
) -> *mut *mut c_void {
    let desc = &*(rd as *const RaceDesc);
    desc.ship_data.ship.as_ptr() as *mut *mut c_void
}

/// C calls this to get characteristics.ship_mass.
#[no_mangle]
pub unsafe extern "C" fn rust_race_desc_get_ship_mass(
    rd: *const c_void,
) -> CByte {
    let desc = &*(rd as *const RaceDesc);
    desc.characteristics.ship_mass
}

/// C calls this to get ship_info.crew_level.
#[no_mangle]
pub unsafe extern "C" fn rust_race_desc_get_crew_level(
    rd: *const c_void,
) -> CCount {
    let desc = &*(rd as *const RaceDesc);
    desc.ship_info.crew_level as CCount
}

/// C calls this to set ship_info.crew_level (for crew writeback during uninit).
#[no_mangle]
pub unsafe extern "C" fn rust_race_desc_set_crew_level(
    rd: *mut c_void,
    crew: CCount,
) {
    let desc = &mut *(rd as *mut RaceDesc);
    desc.ship_info.crew_level = crew as u16;
}

/// C calls this to get ship_info.max_crew.
#[no_mangle]
pub unsafe extern "C" fn rust_race_desc_get_max_crew(
    rd: *const c_void,
) -> CCount {
    let desc = &*(rd as *const RaceDesc);
    desc.ship_info.max_crew as CCount
}
```

**C helper modifications (if accessor path is taken):**

Modify `rust_bridge_spawn_element()`:
```c
// Instead of: RDPtr->ship_data.ship
// Use: rust_race_desc_get_ship_frames(RDPtr)
FRAME *ship_frames = (FRAME *)rust_race_desc_get_ship_frames(RDPtr);
ShipElementPtr->current.image.farray = ship_frames;
```

Modify `rust_bridge_uninit_ships()` crew writeback loop:
```c
// Instead of: StarShipPtr->RaceDescPtr->ship_info.crew_level
// Use: rust_race_desc_get_crew_level(StarShipPtr->RaceDescPtr)
COUNT crew = rust_race_desc_get_crew_level(StarShipPtr->RaceDescPtr);
COUNT max_crew = rust_race_desc_get_max_crew(StarShipPtr->RaceDescPtr);
if (crew)
{
    if (crew_retrieved >= max_crew - crew)
        crew = max_crew;
    else
        crew += crew_retrieved;
}
rust_race_desc_set_crew_level(StarShipPtr->RaceDescPtr, crew);
StarShipPtr->crew_level = crew;
```

This eliminates the layout dependency entirely. The cost is additional FFI calls during spawn/uninit (one-time per ship, not per frame — negligible).

**FFI scope note (per REQ-REMED-FFI-SCOPE):** If accessor functions are needed, they are `rust_race_desc_get_*` / `rust_race_desc_set_*` — these are the ONLY permitted expansion beyond the `rust_bridge_*` facade. They are explicitly listed as permitted additions in the specification's FFI Surface Scope Limitation section. No other new FFI declarations are permitted.

**Decision point:** The layout verification in step 4 will determine which path is needed. If all offsets match, direct cast works and `verify_race_desc_layout()` is kept as a runtime safety net. If they don't match, implement the accessor approach in this same phase, modify the C helpers, and disable the abort in `verify_race_desc_layout()` (replace with a log message noting accessor mode is active).

### 7. Runtime debug assertions for pointer validity (M3)

Add temporary debug assertions in both C helper and Rust FFI for pointer validity and state transitions:

```c
/* In rust_bridge_spawn_element(), after LockElement: */
#ifndef NDEBUG
    assert(ShipElementPtr != NULL && "LockElement returned null");
    assert(RDPtr != NULL && "RaceDesc pointer is null");
    assert(RDPtr->ship_data.ship != NULL && "ship_data.ship is null (frames not loaded)");
    assert(RDPtr->ship_data.ship[0] != NULL && "ship_data.ship[0] is null (first frame)");
#endif
```

```rust
// In verify_race_desc_layout(), add assertion that RaceDesc has expected alignment:
#[cfg(all(not(test), debug_assertions))]
{
    let align = std::mem::align_of::<RaceDesc>();
    assert!(align >= std::mem::align_of::<usize>(),
        "RaceDesc alignment ({}) is less than pointer alignment", align);
}
```

## Verification

- `cd rust && cargo test --lib` passes
- `cd rust && cargo build --release` succeeds
- `cd sc2 && ./build.sh uqm` compiles and links cleanly (static assertions pass at compile time)
- H1 acceptance check passes:
  ```bash
  grep -rn 'fn rust_bridge_' rust/src/ships/*.rs | grep -v ffi_contract.rs | grep -vc '#\[no_mangle\]'
  # Expected: 0
  ```
- First `rust_ships_init()` call in runtime: no abort → layout matches (or accessor approach is in use)
- If layout DOES mismatch: abort with clear message listing all divergent offsets, including both C and Rust values for each field. Then implement accessor fallback (section 6) before marking P05 complete.
- **P05 completion gate:** P05 is complete ONLY when one of these is true:
  1. Layout verification passes (all offsets match) and runtime confirms no abort, OR
  2. Accessor functions are implemented, C helpers are modified to use them, and the build compiles+links cleanly.
- After verification, document the confirmed offsets (or accessor-mode status) in a comment block for future reference

## Output

- Modified: `sc2/src/uqm/rust_bridge_ships.c` — layout query function + static assertions + pointer validity debug assertions
- Modified: `rust/src/ships/ffi_contract.rs` — `RaceDescLayout` struct + `rust_bridge_get_race_desc_layout` FFI declaration (canonical location, H1)
- Modified: `rust/src/ships/ffi.rs` — `verify_race_desc_layout()` with deterministic offset computation + hard-fail + `Once` guard
- (P00 already added the `RACE_DESC_LAYOUT` typedef and prototype to `rust_bridge_ships.h`)
- Possibly: accessor functions in `ffi.rs` + C helper modifications if layout diverges (fallback path)

## LoC Estimate

~30 lines C (layout function + static assertions + debug assertions), ~80 lines Rust (verification function with deterministic offsets, mismatch collection, abort, Once guard). If accessor fallback is needed: additional ~30 lines Rust + ~20 lines C.
