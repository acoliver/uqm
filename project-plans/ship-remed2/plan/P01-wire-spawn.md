# P01 — Wire `rust_ships_spawn()` to Create C ELEMENTs

## Goal

Make `rust_ships_spawn()` actually create a battle ELEMENT by calling the C helper `rust_bridge_spawn_element()` after loading the descriptor.

## Prerequisite

P00 (C helpers exist and compile+link cleanly). **P05 (MANDATORY HARD GATE):** layout verification must have confirmed field offsets match, OR the accessor-function fallback (P05§6) must be implemented. P01 MUST NOT proceed until P05 has run and determined the approach. Direct `RACE_DESC*` field access in `rust_bridge_spawn_element()` is forbidden without proven layout parity.

## Changes

### 1. Add FFI declaration in ffi_contract.rs (H1)

**All C helper FFI bindings are declared ONLY in `ffi_contract.rs`.** No local `extern "C"` blocks are permitted in `ffi.rs` for any C helper function. This is the single canonical ABI declaration path, ensuring no duplicate declarations can drift out of sync.

Add to the existing `extern "C"` block in `ffi_contract.rs`, using the canonical type aliases already defined there:

```rust
extern "C" {
    // ... existing declarations ...

    /// Creates a battle ELEMENT for a spawned ship.
    /// C: BOOLEAN rust_bridge_spawn_element(STARSHIP*, RACE_DESC*, BYTE mass, BYTE activity);
    /// Prototype: rust_bridge_ships.h
    pub fn rust_bridge_spawn_element(
        starship: *mut CStarship,
        race_desc: *mut c_void,
        ship_mass: CByte,
        activity: CByte,
    ) -> CBoolean;
}
```

**ABI type mapping:** All parameter types use the canonical aliases from `ffi_contract.rs`:
- `*mut CStarship` — matches C `STARSHIP*` (shared-layout struct already defined)
- `*mut c_void` — matches C `RACE_DESC*` (opaque to Rust at FFI boundary)
- `CByte` (= `u8`) — matches C `BYTE` (= `uint8`)
- `CBoolean` (= `u8`) — matches C `BOOLEAN` (= `uint8`)

No raw Rust primitive types (`u8`, `i16`) appear in the FFI declaration. This ensures all cross-boundary types are traceable to their C counterparts via the alias definitions in `ffi_contract.rs`.

**Enforcement (H1):** During implementation, run this acceptance check:
```bash
# Must return 0 — no rust_bridge_* extern declarations in ffi.rs
grep -n 'extern "C"' rust/src/ships/ffi.rs | grep -c 'rust_bridge_'
# Must return 0 — no rust_bridge_* extern declarations in any Rust file except ffi_contract.rs
grep -rn 'fn rust_bridge_' rust/src/ships/*.rs | grep -v ffi_contract.rs | grep -vc '#\[no_mangle\]'
```
No new `extern "C" { fn rust_bridge_... }` blocks may appear in `ffi.rs` or any other module. Only `ffi_contract.rs` declares these. This check is a **completion gate** for P01.

### 2. Modify `rust_ships_spawn()` in ffi.rs

**Current code (line 261-316):**
Constructs Rust Starship, calls `lifecycle_spawn()`, writes `race_desc_ptr` back to CStarship. Returns 1.

**Spawn failure rollback contract (H3):** If the C helper `rust_bridge_spawn_element` fails (returns 0), the Rust side must clean up. To minimize inconsistent state, CStarship mutations are structured in two phases:

1. **Pre-helper phase:** Write only the fields that the C helper needs to read (`race_desc_ptr`). Do NOT write back counters/flags yet.
2. **Post-helper phase (success only):** Write back all remaining fields (counters, flags) only after the C helper succeeds.

This ensures that if the C helper fails, the CStarship has minimal mutation. The only field written is `race_desc_ptr`, which gets cleaned up (freed and nulled) on failure.

**Exact rollback contract for CStarship mutation order:**

| Field | When written | Rolled back on failure? |
|-------|-------------|------------------------|
| `race_desc_ptr` | Before C helper call | YES — freed via `Box::from_raw`, set to null |
| `ship_input_state` | After C helper success | N/A — not written on failure path |
| `cur_status_flags` | After C helper success | N/A — not written on failure path |
| `old_status_flags` | After C helper success | N/A — not written on failure path |
| `energy_counter` | After C helper success | N/A — not written on failure path |
| `weapon_counter` | After C helper success | N/A — not written on failure path |
| `special_counter` | After C helper success | N/A — not written on failure path |
| `hShip` | Written by C helper (via pointer) | C helper cleans up on its own failure |
| `ShipFacing` | Written by C helper (via pointer) | Stale value remains (harmless) |

**New code:**
```rust
#[no_mangle]
pub unsafe extern "C" fn rust_ships_spawn(starship: *mut std::os::raw::c_void) -> CBoolean {
    catch_unwind(|| {
        if starship.is_null() {
            return 0;
        }

        unsafe {
            let starship_c = &mut *(starship as *mut CStarship);

            let mut starship_rust = Starship {
                // ... same field construction as current ...
            };

            #[cfg(test)]
            let activity = 2u8;
            #[cfg(not(test))]
            let activity = uqm_get_current_activity_lobyte();

            match lifecycle_spawn(&mut starship_rust, activity) {
                Ok(_) => {
                    // Extract race_desc_ptr — this is the only CStarship
                    // mutation before the C helper call (H3 rollback contract).
                    let race_desc_ptr = match starship_rust.race_desc {
                        Some(desc) => Box::into_raw(desc) as *mut std::os::raw::c_void,
                        None => return 0,
                    };
                    starship_c.race_desc_ptr = race_desc_ptr;

                    // Get ship mass from descriptor for element creation
                    let ship_mass = (*(race_desc_ptr as *const RaceDesc))
                        .characteristics
                        .ship_mass;

                    // Call C helper to create the ELEMENT
                    #[cfg(not(test))]
                    {
                        use crate::ships::ffi_contract::rust_bridge_spawn_element;

                        let element_ok = rust_bridge_spawn_element(
                            starship as *mut CStarship,
                            race_desc_ptr,
                            ship_mass,
                            activity,
                        );
                        if element_ok == 0 {
                            // H3: Element creation failed — rollback.
                            // Free the descriptor we just allocated.
                            let _desc = Box::from_raw(race_desc_ptr as *mut RaceDesc);
                            // Null the pointer so C doesn't see a dangling ref.
                            starship_c.race_desc_ptr = ptr::null_mut();
                            // Do NOT write back counters/flags — CStarship
                            // remains in its pre-spawn state.
                            return 0;
                        }

                        // C helper succeeded. hShip and ShipFacing are already
                        // set by the C helper via the pointer we passed.
                    }

                    // Post-helper success: NOW write back cleared counters/flags.
                    // These are safe to write because the element exists and the
                    // C side has already consumed race_desc_ptr successfully.
                    starship_c.ship_input_state = starship_rust.ship_input_state;
                    starship_c.cur_status_flags = starship_rust.cur_status_flags.0;
                    starship_c.old_status_flags = starship_rust.old_status_flags.0;
                    starship_c.energy_counter = starship_rust.energy_counter;
                    starship_c.weapon_counter = starship_rust.weapon_counter;
                    starship_c.special_counter = starship_rust.special_counter;

                    1
                }
                Err(_) => 0,
            }
        }
    })
    .unwrap_or_default()
}
```

### 3. Resolve ElementConfig dead code (M1)

**Problem:** The plan previously proposed `drop(element_config)` which silences the warning but leaves dead code that misleads readers into thinking something uses it.

**Resolution:** Remove `ElementConfig` struct and its construction entirely from `lifecycle.rs`. The element configuration values (mass, state_flags, life_span, etc.) are not consumed by Rust — they are hardcoded in the C helper `rust_bridge_spawn_element()`, which copies them directly from the original C `spawn_ship()` source. Keeping a Rust-side struct that shadows these values creates a maintenance burden: if C changes a default (e.g., different `state_flags`), the Rust `ElementConfig` becomes silently stale.

**Changes to lifecycle.rs:**

1. **Remove the `ElementConfig` struct definition** (lines 81-89).
2. **Remove the `_element_config` construction** (lines 161-169).
3. **Add a comment** at the removal site:
   ```rust
   // Element allocation and field setup (mass, state_flags, position,
   // callbacks) are handled by C's rust_bridge_spawn_element().
   // See P00 for the C helper implementation.
   ```
4. **Update `spawn_ship()` return type** to remain `Result<SpawnResult, ShipError>` (unchanged).

**If `ElementConfig` is used by any test:** Check first. If tests reference `ElementConfig`, update them to test the values they need directly from the descriptor or starship instead.

### 4. Spawn Branch Parity (per REQ-REMED-SPAWN-PARITY)

The C helper `rust_bridge_spawn_element()` handles both the `hShip==0` (fresh allocation) and `hShip!=0` (element reuse) branches. Rust's `rust_ships_spawn()` must correctly support both by passing the `CStarship*` through with `hShip` intact. The C helper reads `StarShipPtr->hShip` to decide the branch.

**Rust-side obligation:** The Rust `rust_ships_spawn()` must NOT zero or modify `starship_c.hShip` before calling the C helper. The C helper needs to read the existing value to determine which branch to take. The pointer is passed through as-is.

**Verification for BOTH branches:**

1. **Branch A (hShip == 0 — normal first spawn):** Enter Super Melee, start battle. Both ships appear correctly with proper sprites, facing, position, and callbacks.
2. **Branch B (hShip != 0 — replacement ship):** In Super Melee, when a ship dies and a replacement is spawned, `GetNextStarShip()` copies the old `hShip` to the new `StarShipPtr->hShip` (ship.c line 536) before calling `spawn_ship()` → `rust_ships_spawn()`. The C helper must:
   - Receive `hShip != 0`
   - Skip `AllocElement`/`InsertElement` (element already in display list)
   - Still overwrite ALL 22 element fields unconditionally (no stale state from previous ship)
   - Replacement ship appears with correct sprites, facing, position, and callbacks
3. **22-field checklist:** All fields in the REQ-REMED-SPAWN-PARITY assertion checklist are set in the common path. Code review must confirm no field write is inside the `if (hShip == 0)` block.

### 5. Handle RaceDesc pointer casting

The `race_desc_ptr` stored in `CStarship.race_desc_ptr` is a `*mut c_void` that points to a Rust `RaceDesc`. The C helper `rust_bridge_spawn_element` receives it as `RACE_DESC*`.

**P05 determines whether this is safe.** If P05's layout verification passes, the cast works. If P05 reveals a mismatch, the accessor-function fallback (documented in P05 section 6) must be implemented before P01 can proceed. P01 should not proceed until P05 has run and confirmed the approach.

### 6. Activity Flag Semantics (per REQ-REMED-ACTIVITY-PARITY)

The `activity` parameter passed to `rust_bridge_spawn_element()` must match C's `LOBYTE(GLOBAL(CurrentActivity))` semantics exactly. The Rust side obtains this via `uqm_get_current_activity_lobyte()`, which is a C helper that evaluates `LOBYTE(GLOBAL(CurrentActivity))`.

**Critical parity:** The C `spawn_ship()` uses `LOBYTE(GLOBAL(CurrentActivity))` inline (not via a helper). The Rust path reads it via `uqm_get_current_activity_lobyte()` and passes it as a parameter. The values must match:
- `IN_ENCOUNTER` (value 2) — used for crew patching in `lifecycle_spawn()`
- `IN_LAST_BATTLE` (value 3) — used for Sa-Matra special case in C helper
- `SUPER_MELEE` — used for normal battle
- HyperSpace values — used for facing override in C helper

**Verification:** The P00 C helper `rust_bridge_spawn_element()` includes a permanent debug assertion (added in P00):
```c
#ifndef NDEBUG
    assert(activity == LOBYTE(GLOBAL(CurrentActivity))
        && "activity parameter does not match GLOBAL(CurrentActivity)");
#endif
```
This catches any timing issue where the global changes between Rust's read and C's use. The assertion is part of the P00 implementation and fires in all debug builds.

### 7. ABI Type Verification

After implementing the FFI declaration, verify that the Rust-side types match C types with static assertions:

```rust
#[cfg(test)]
mod spawn_abi_checks {
    use super::*;
    use std::mem;

    #[test]
    fn spawn_element_param_sizes_match_c_abi() {
        // STARSHIP* — pointer sized
        assert_eq!(mem::size_of::<*mut CStarship>(), mem::size_of::<*mut c_void>());
        // RACE_DESC* — pointer sized
        assert_eq!(mem::size_of::<*mut c_void>(), mem::size_of::<usize>());
        // BYTE — 1 byte
        assert_eq!(mem::size_of::<CByte>(), 1);
        // BOOLEAN return — 1 byte
        assert_eq!(mem::size_of::<CBoolean>(), 1);
    }
}
```

These are sanity checks — the real ABI correctness comes from using `ffi_contract.rs` type aliases consistently (not raw `u8`/`i16`).

## Test Behavior

- `#[cfg(test)]`: The `rust_bridge_spawn_element` call is skipped. Existing tests that call `rust_ships_spawn(null)` or test lifecycle::spawn_ship directly are unaffected.
- `#[cfg(not(test))]`: The full path executes.

## Verification

- `cd rust && cargo test --lib` passes (all 147 + ffi tests)
- `cd rust && cargo build --release` succeeds
- `cd sc2 && ./build.sh uqm` compiles and links cleanly (zero warnings)
- H1 acceptance check passes:
  ```bash
  grep -n 'extern "C"' rust/src/ships/ffi.rs | grep -c 'rust_bridge_'
  # Expected: 0
  grep -rn 'fn rust_bridge_' rust/src/ships/*.rs | grep -v ffi_contract.rs | grep -vc '#\[no_mangle\]'
  # Expected: 0
  ```
- Runtime: entering battle should now show ship elements (ships appear on screen)
- If ships appear but don't move, that's expected until preprocess callbacks work (but they should work since elements now have `preprocess_func = ship_preprocess` which redirects to `rust_ships_preprocess`)

## Output

- Modified: `rust/src/ships/ffi.rs` — spawn function wired to C helper with rollback contract
- Modified: `rust/src/ships/ffi_contract.rs` — FFI declaration for `rust_bridge_spawn_element`
- Modified: `rust/src/ships/lifecycle.rs` — `ElementConfig` struct and dead construction removed

## LoC Estimate

~45 lines Rust changed in ffi.rs, ~5 lines added to ffi_contract.rs, ~15 lines removed from lifecycle.rs.

## Risk

Medium. The main risk is RaceDesc/RACE_DESC layout compatibility. **P05 is a mandatory hard gate (not a mitigation) — P01 MUST NOT proceed until P05 has either confirmed layout parity or the accessor-function fallback is implemented.** There is no "proceed anyway" path. If P05 reveals a mismatch and accessor functions are implemented, the C helper modifications from P05§6 must be in place before P01 wiring. The rollback contract (H3) ensures CStarship is not left in a half-mutated state on failure.
