# P03 — Wire `rust_ships_uninit()` to Perform Full Cleanup

## Goal

Make `rust_ships_uninit()` perform complete battle teardown: stop audio, free space resources, count floating crew, distribute crew to survivor, write back crew levels, free descriptors, clear IN_BATTLE, handle queue reinit. Guard against double-uninit, double-free, and partial-init states.

## Phase Ordering

**Mandatory execution order: P00 → P05 → P01 → P02 → P03 → P04.**

P03 depends on P00 (C helper exists), **P05 (MANDATORY HARD GATE — C-side uninit dereferences `RaceDescPtr->ship_info.crew_level` for crew writeback, requires proven layout parity)**, and P02 (gate G3 — for `mark_ships_uninitialized()` lifecycle API). Logically depends on P01 (elements must exist for uninit to have anything to clean up), but compiles independently.

## Prerequisite

P00 (C helper exists). **P05 (MANDATORY HARD GATE):** layout verification must have confirmed field offsets match, OR accessor-function fallback must be in place. The C uninit helper reads `RaceDescPtr->ship_info.crew_level` and `RaceDescPtr->ship_info.max_crew` — both are direct field access that requires layout parity. P02 (for `mark_ships_uninitialized()` lifecycle API and `is_ships_initialized_for_uninit()`).

## Changes

### 1. Add FFI declaration in ffi_contract.rs (H1)

**All C helper FFI bindings are declared ONLY in `ffi_contract.rs`.** No local `extern "C"` blocks in `ffi.rs`. Single canonical ABI declaration path.

Add to the existing `extern "C"` block, using canonical type aliases:

```rust
extern "C" {
    // ... existing declarations ...

    /// Performs full battle teardown: audio stop, space uninit, crew writeback,
    /// descriptor freeing, IN_BATTLE clearing, queue reinit.
    /// C: void rust_bridge_uninit_ships(void);
    /// Prototype: rust_bridge_ships.h
    pub fn rust_bridge_uninit_ships();
}
```

### 2. Modify `rust_ships_uninit()` in ffi.rs

**Current code (line 348-356):**
```rust
pub unsafe extern "C" fn rust_ships_uninit() {
    let _ = catch_unwind(|| {
        free_master_ship_list();
    });
}
```

**New code:**
```rust
#[no_mangle]
pub unsafe extern "C" fn rust_ships_uninit() {
    let _ = catch_unwind(|| {
        #[cfg(test)]
        {
            // Test mode: defensive catalog cleanup (no C arena to tear down)
            free_master_ship_list();
            lifecycle::mark_ships_uninitialized();
            return;
        }

        #[cfg(not(test))]
        unsafe {
            use crate::ships::ffi_contract::rust_bridge_uninit_ships;

            // H2: Reconcile Rust-side state with C-side state before
            // deciding whether to skip teardown. The Rust flag alone
            // is not authoritative — it can desync from C arena state
            // in failure/partial-init paths.
            let rust_says_initialized = lifecycle::is_ships_initialized_for_uninit();

            if !rust_says_initialized {
                // Check C-side: does CurrentActivity still have IN_BATTLE?
                // IN_BATTLE = 0x80 in the high nibble, but we check the
                // activity byte for evidence of active battle state.
                let c_activity = uqm_get_current_activity_lobyte();
                // IN_ENCOUNTER is 2, which implies battle context may exist
                let c_might_have_arena = c_activity == 2; // IN_ENCOUNTER

                if c_might_have_arena {
                    // H2: Desync detected — Rust says uninitialized but C
                    // may still have arena resources. Proceed with teardown
                    // to prevent resource leak. C state is authoritative.
                    #[cfg(debug_assertions)]
                    eprintln!(
                        "rust_ships_uninit: WARNING desync detected — \
                         Rust says uninitialized but C activity={:#x} \
                         suggests arena may exist. Proceeding with teardown.",
                        c_activity
                    );
                    // Fall through to teardown below
                } else {
                    // Both Rust and C agree: no arena to tear down.
                    #[cfg(debug_assertions)]
                    eprintln!("rust_ships_uninit: idempotence guard fired \
                              (Rust=uninitialized, C activity={:#x})", c_activity);
                    return;
                }
            }

            #[cfg(debug_assertions)]
            eprintln!("rust_ships_uninit: beginning teardown");

            // Delegate full teardown to C helper.
            // This performs:
            //   - StopSound()
            //   - UninitSpace() (free explosion/blast/asteroid resources)
            //   - CountCrewElements() (count floating crew in display list)
            //   - Iterate display list: add floating crew to survivor
            //   - Write back descriptor crew -> StarShipPtr->crew_level
            //   - free_ship() for each spawned ship (calls rust_ships_free)
            //   - Clear IN_BATTLE from CurrentActivity
            //   - UpdateShipFragCrew() for IN_ENCOUNTER
            //   - ReinitQueue / FreeHyperspace for non-IN_ENCOUNTER
            //
            // C-side null guards (C3): The C helper guards against null
            // StarShipPtr and null RaceDescPtr on each element, so
            // partial-init states are handled safely.
            rust_bridge_uninit_ships();

            // Update Rust-side state tracking
            lifecycle::mark_ships_uninitialized();

            #[cfg(debug_assertions)]
            {
                assert!(!lifecycle::is_ships_initialized_for_uninit(),
                    "ships_initialized should be false after uninit");
                eprintln!("rust_ships_uninit: teardown complete, state cleared");
            }
        }
    });
}
```

### 3. Idempotence and Partial-Init Protection (C3, H2)

The uninit path has multiple re-entrancy and partial-state risks:

1. **Double-uninit:** `rust_ships_uninit()` called twice (error recovery + normal teardown).
2. **Panic-path desync:** If Rust panics during spawn or init (caught by `catch_unwind`), the Rust-side `ships_initialized` flag may be true while C-side state is partially initialized.
3. **External C call order:** C code might call `UninitShips()` → `rust_ships_uninit()` before `InitShips()` was ever called.
4. **Init failure after partial setup:** `rust_bridge_init_battle_arena()` might call `InitSpace()` and `InitDisplayList()` but fail on `InitGalaxy()` — leaving partial C state with `ships_initialized` never set to true.

#### Layer 1: Rust-side idempotence guard with C-state reconciliation (per REQ-REMED-UNINIT-RECONCILE)

The Rust `ships_initialized` flag is checked first, but it is NOT the sole authority. Before skipping teardown, the guard reconciles against C-side state by querying `uqm_get_current_activity_lobyte()`:

- **Both agree (Rust=false, C=no battle):** Safe skip — no arena to tear down. Handles cases 1 and 3.
- **Desync (Rust=false, C=battle active):** Log warning and proceed with teardown. C state is authoritative for arena existence. Handles case 4 (init failed after partial C setup but before Rust flag was set).
- **Normal path (Rust=true):** Proceed with teardown. Handles case 2 (Rust flag true but C state may be partially valid).

This reconciliation prevents the scenario where a Rust panic during init leaves `ships_initialized = false` but C arena resources are partially allocated and leaking.

#### Layer 2: C-side mandatory null guards (per REQ-REMED-UNINIT-GUARD — Code-Level, Not Diagnostic)

The C helper `rust_bridge_uninit_ships()` (implemented in P00) includes **unconditional** null-pointer guards with a specific ordering that must be followed before any dereference. **These are production control-flow statements, NOT debug-only diagnostics.** The `if (...) { UnlockElement; continue; }` pattern fires in ALL builds. Only the `log_add` messages are `#ifndef NDEBUG`.

**Required guard ordering (per P00 C3 section and REQ-REMED-UNINIT-GUARD):**

1. **Guard 1 — ElementPtr validity:** `LockElement(hElement, &ElementPtr)` must succeed.
2. **Guard 2 — Extract StarShipPtr safely:** `GetElementStarShip(ElementPtr, &StarShipPtr)` via the macro.
3. **Guard 3 — StarShipPtr != NULL:** If null → `UnlockElement` + `continue`. **UNCONDITIONAL.** No dereference of StarShipPtr.
4. **Guard 4 — StarShipPtr->RaceDescPtr != NULL:** If null → `UnlockElement` + `continue`. **UNCONDITIONAL.** No dereference of RaceDescPtr.
5. **Only then:** Access `RaceDescPtr->ship_info.crew_level` and other fields.

```c
/* Guard 3 — UNCONDITIONAL null check (fires in ALL builds) */
if (StarShipPtr == NULL)
{
#ifndef NDEBUG
    /* Diagnostic only — the guard above prevents the crash regardless */
    log_add(log_Debug,
            "rust_bridge_uninit_ships: null StarShipPtr, "
            "skipping element");
#endif
    UnlockElement(hElement);   /* Guard action — unconditional */
    continue;                  /* Guard action — unconditional */
}
/* Guard 4 — UNCONDITIONAL null check (fires in ALL builds) */
if (StarShipPtr->RaceDescPtr == NULL)
{
#ifndef NDEBUG
    log_add(log_Debug,
            "rust_bridge_uninit_ships: null RaceDescPtr on "
            "StarShipPtr=%p, skipping", (void *)StarShipPtr);
#endif
    UnlockElement(hElement);   /* Guard action — unconditional */
    continue;                  /* Guard action — unconditional */
}
/* All guards passed — safe to dereference */
```

**P03 verification obligation:** During P03 implementation, code review must confirm that the `if (StarShipPtr == NULL)` and `if (StarShipPtr->RaceDescPtr == NULL)` checks are NOT wrapped in `#ifndef NDEBUG` or any other conditional. The pattern must be: unconditional `if` → conditional `log_add` → unconditional `UnlockElement` + `continue`.

This handles case 2 (panic left partial state) and case 4 (init failed after partial setup). Even if the display list contains elements without valid StarShipPtr or RaceDescPtr, the loop continues safely. The separate guard checks (rather than a combined `||`) provide precise diagnostic logging identifying which pointer was null.

#### Layer 3: C-side post-free nulling

The original `UninitShips()` already nulls `RaceDescPtr` after `free_ship()`:
```c
free_ship(StarShipPtr->RaceDescPtr, TRUE, TRUE);
StarShipPtr->RaceDescPtr = 0;   // prevents double-free
```
This is preserved in the P00 copy. Combined with the null guard above, a double-free attempt on the same element is a no-op.

#### Layer 4: Rust-side null guard in `rust_ships_free()`

The existing `rust_ships_free()` in ffi.rs must tolerate null pointers gracefully. Verify it has a null check at the top:

```rust
#[no_mangle]
pub unsafe extern "C" fn rust_ships_free(
    race_desc: *mut c_void,
    free_icon: CBoolean,
    free_battle: CBoolean,
) {
    let _ = catch_unwind(|| {
        if race_desc.is_null() {
            return;  // Idempotent — already freed
        }
        // ... existing Box::from_raw logic ...
    });
}
```

If the null check is missing, **add it** as part of this phase.

#### Assertion logging around transitions (C3)

Both C and Rust sides log state transitions in debug builds:

**C side (in P00 `rust_bridge_uninit_ships()`):**
```c
#ifndef NDEBUG
    log_add(log_Debug, "rust_bridge_uninit_ships: crew_retrieved=%u",
            (unsigned)crew_retrieved);
    /* ... at end ... */
    log_add(log_Debug, "rust_bridge_uninit_ships: teardown complete");
#endif
```

**Rust side (in `rust_ships_uninit()`):**
```rust
#[cfg(debug_assertions)]
eprintln!("rust_ships_uninit: beginning teardown");
/* ... after mark_ships_uninitialized() ... */
#[cfg(debug_assertions)]
eprintln!("rust_ships_uninit: teardown complete, state cleared");
```

These log lines make it possible to trace the exact uninit sequence when debugging desync issues between Rust and C state.

**State flow with protection layers (updated for H2 reconciliation):**

| Scenario | Layer 1 (Rust guard + C reconciliation) | Layer 2 (C null guard) | Layer 3 (C null ptr) | Layer 4 (Rust null) |
|----------|---------------------------------------------|----------------------|---------------------|---------------------|
| Normal uninit | Rust=true → proceeds | passes | nulls after free | N/A |
| Double uninit | Rust=false, C=no battle → blocks (no-op) | — | — | — |
| Uninit before init | Rust=false, C=no battle → blocks (no-op) | — | — | — |
| Panic during init (flag set) | Rust=true → proceeds | skips null elements | safe | null check |
| Init partial failure (flag NOT set) | Rust=false, C=battle active → **warn + proceed** (H2 reconciliation) | skips null elements | safe | null check |
| Init partial failure, C also clean | Rust=false, C=no battle → blocks (no-op) | — | — | — |

### 4. Key detail: free_ship calls rust_ships_free

The C helper `rust_bridge_uninit_ships()` will call `free_ship(RaceDescPtr, TRUE, TRUE)` for each spawned ship. Because `USE_RUST_SHIPS` is defined, `free_ship()` in loadship.c dispatches to `rust_ships_free()`:

```c
void free_ship(RACE_DESC *raceDescPtr, BOOLEAN FreeIconData, BOOLEAN FreeBattleData)
{
#ifdef USE_RUST_SHIPS
    rust_ships_free(raceDescPtr, FreeIconData, FreeBattleData);
    return;
#endif
    c_free_ship(raceDescPtr, FreeIconData, FreeBattleData);
}
```

So the C helper calls back into Rust's `rust_ships_free()` (in ffi.rs line 127-143) to free the Rust-owned RaceDesc. This is correct — the existing `rust_ships_free()` uses `Box::from_raw` to reclaim the Rust allocation.

**Re-entrancy safety:** This creates a C→Rust→C→Rust call chain (`rust_ships_uninit` → `rust_bridge_uninit_ships` → `free_ship` → `rust_ships_free`). This is safe because:
- `rust_ships_free` is a simple `Box::from_raw` + drop on the Rust RaceDesc
- No Rust locks are held when the C helper runs (the `BATTLE_STATE` mutex is NOT held during `rust_bridge_uninit_ships`)
- `catch_unwind` in `rust_ships_free` prevents panics from escaping to C
- The null guard (Layer 4) ensures `Box::from_raw` is never called on the same pointer twice

### 5. No more `free_master_ship_list()` in non-test uninit

The current code calls `free_master_ship_list()` during uninit, which frees the catalog. But UninitShips in C does NOT free the catalog — the catalog outlives individual battles. The catalog is freed separately via `rust_ships_free_catalog()`.

In non-test mode, `free_master_ship_list()` is NOT called. In test mode, it remains as defensive cleanup (tests may load catalogs that need cleanup between test runs).

### 6. Lifecycle.rs — No dangling state

Since P02 adds `mark_ships_initialized()` and this phase adds the call to `mark_ships_uninitialized()`, the `BattleState` is always in a consistent state:
- After init: `ships_initialized = true`
- After uninit: `ships_initialized = false`
- Double-uninit: no-op (guarded by `is_ships_initialized_for_uninit()`)

The `space_init_count` ref counter in `BattleState` is NOT touched in non-test mode (C's `InitSpace`/`UninitSpace` handles its own ref counting). This is correct — the Rust ref counter was designed for a future all-Rust path.

## Important: Descriptor Ownership During Uninit

The C helper iterates the display list and accesses `StarShipPtr->RaceDescPtr`. This pointer was set by `rust_ships_spawn()` (via `Box::into_raw`). The C helper:
1. Reads `RaceDescPtr->ship_info.crew_level` (to write back crew)
2. Calls `free_ship(RaceDescPtr, TRUE, TRUE)` which goes to `rust_ships_free()`
3. Sets `RaceDescPtr = 0`

This is safe because:
- The pointer came from `Box::into_raw` in rust_ships_spawn
- `rust_ships_free` uses `Box::from_raw` to reclaim it
- Between steps 1-2, no Rust code touches the descriptor
- After step 3, the C-side pointer is null (no dangling)
- If called again (double-free), `RaceDescPtr` is 0, `rust_ships_free` gets null and returns immediately (Layer 4)

## Verification

- `cd rust && cargo test --lib` passes (test path uses `free_master_ship_list` + `mark_ships_uninitialized`)
- `cd rust && cargo build --release` succeeds
- `cd sc2 && ./build.sh uqm` compiles and links cleanly (zero warnings)
- H1 acceptance check passes:
  ```bash
  grep -n 'extern "C"' rust/src/ships/ffi.rs | grep -c 'rust_bridge_'
  # Expected: 0
  grep -rn 'fn rust_bridge_' rust/src/ships/*.rs | grep -v ffi_contract.rs | grep -vc '#\[no_mangle\]'
  # Expected: 0
  ```
- Runtime: after battle ends, crew levels should be correctly written back
- Starting a new battle after completing one should work (state properly cleaned up)
- **Double-uninit test:** Call `rust_ships_uninit()` twice in sequence — second call should be a no-op with no crash, and debug build should print the idempotence guard message (with C activity state logged for H2 reconciliation)
- **H2 reconciliation test (debug build):** Force a desync by triggering a panic during init (e.g., via a test-only panic hook), then call uninit. Debug output should show the H2 warning and proceed with teardown.

## Output

- Modified: `rust/src/ships/ffi.rs` — uninit function wired to C helper with idempotence guard and debug logging
- Modified: `rust/src/ships/ffi_contract.rs` — FFI declaration for `rust_bridge_uninit_ships`
- Verified: `rust/src/ships/ffi.rs` — `rust_ships_free()` has null check (add if missing)

## LoC Estimate

~25 lines Rust changed in ffi.rs, ~5 lines added to ffi_contract.rs. Possibly ~3 lines added to `rust_ships_free` if null guard is missing.

## Risk

Low-medium. The main risk is the `free_ship` -> `rust_ships_free` callback chain (C calls Rust from within a Rust-initiated FFI call). Mitigated by the four-layer protection model documented above. The C-side null guards (C3) provide defense-in-depth against partial-init states that the Rust idempotence guard alone cannot catch.
