# P02 — Wire `rust_ships_init()` to Initialize Battle Arena

## Goal

Make `rust_ships_init()` fully initialize the battle arena by calling `rust_bridge_init_battle_arena()`, which contains the original C `InitShips()` body.

## Phase Ordering

**Mandatory execution order: P00 → P05 → P01 → P02 → P03 → P04.**

P02 depends on P00 (C helper exists) and logically follows P05 (layout verification confirms RaceDesc/RACE_DESC compatibility). P02 is code-independent of P01 (spawn wiring) — they modify different functions in `ffi.rs`. However, the recommended execution order has P01 before P02 because spawn correctness should be verified before arena init is wired (spawning into a broken arena is harder to debug than spawning without an arena). P03 depends on P02 for the lifecycle API (gate G3).

## Prerequisite

P00 (C helper exists and compiles). P05 (layout verification complete — init calls the verification guard).

## InitShips Mode Matrix — Acceptance Table (per REQ-REMED-INIT-MODE-MATRIX)

The C `InitShips()` function branches based on `inHQSpace()` and `LOBYTE(GLOBAL(CurrentActivity))`. The Rust `rust_ships_init()` delegates entirely to `rust_bridge_init_battle_arena()`, which must handle ALL mode combinations identically to the original. This acceptance table defines the expected pre/post state for each mode and serves as a **mandatory verification checklist with explicit assertions per mode bit combination**.

### Mode Branches

| Mode | Condition | C Reference Lines | Return Value |
|------|-----------|-------------------|--------------|
| **HyperSpace** | `inHQSpace() == TRUE` | 197-206 | 1 |
| **Battle (normal)** | `!inHQSpace() && LOBYTE(activity) != IN_LAST_BATTLE` | 208-241 | NUM_SIDES (2) |
| **Battle (Sa-Matra / IN_LAST_BATTLE)** | `!inHQSpace() && LOBYTE(activity) == IN_LAST_BATTLE` | 208-232 | NUM_SIDES (2) |

### Common Side Effects (ALL modes)

These operations occur unconditionally regardless of mode:

| Operation | C Reference |
|-----------|-------------|
| `InitSpace()` — ref count increment | 188 |
| `SetContext(StatusContext)` then `SetContext(SpaceContext)` | 190-191 |
| `InitDisplayList()` | 193 |
| `InitGalaxy()` | 195 |

### Per-Mode Pre/Post State Assertions

#### HyperSpace Mode (`inHQSpace() == TRUE`)

| # | Pre-condition | Post-condition | Verification Method |
|---|---------------|----------------|---------------------|
| H1 | `race_q[0]` and `race_q[1]` may have stale entries | Both queues reinitialized (`ReinitQueue`) | Debug assertion: check queue state after init |
| H2 | No SIS entry in `race_q[0]` | SIS entry built via inlined `BuildSIS()` with `playerNr = RPG_PLAYER_NUM` | Debug assertion: lock `race_q[0]` head, verify `SpeciesID == SIS_SHIP_ID` |
| H3 | Hyperspace resources not loaded | `LoadHyperspace()` called | Implicit — failure would crash |
| H4 | `InitSpace()` ref count at N | Ref count at N+1 | Verified by `UninitSpace()` symmetry in P04 multi-battle test |
| H5 | Return value undefined | Returns exactly `1` | **Rust assertion: `assert_eq!(num_ships, 1)`** |

#### Battle Mode (normal: `IN_ENCOUNTER` or `SUPER_MELEE`)

| # | Pre-condition | Post-condition | Verification Method |
|---|---------------|----------------|---------------------|
| B1 | `SpaceContext` not active | `SetContext(SpaceContext)` called, FG frame set, clip rect set | Implicit — rendering would fail |
| B2 | Background not cleared | `BLACK_COLOR` background, `ClearDrawable()` on `ScreenContext` | Visual: black background |
| B3 | No asteroids | 5 asteroids spawned (`spawn_asteroid(NULL)` × 5) | Visual: asteroids visible; count verifiable with display list iteration |
| B4 | No planet | 1 planet spawned (`spawn_planet()`) | Visual: planet visible |
| B5 | `InitSpace()` ref count at N | Ref count at N+1 | Verified by `UninitSpace()` symmetry |
| B6 | Return value undefined | Returns exactly `NUM_SIDES` (2) | **Rust assertion: `assert_eq!(num_ships, 2)`** |

#### Battle Mode (Sa-Matra: `IN_LAST_BATTLE`)

| # | Pre-condition | Post-condition | Verification Method |
|---|---------------|----------------|---------------------|
| S1 | Same graphics setup as normal battle | Same graphics setup | Shared code path |
| S2 | Gravity well may exist | `free_gravity_well()` called | Debug: no gravity well in display list |
| S3 | No arena objects | NO asteroids or planet spawned | Visual: empty arena (Sa-Matra only) |
| S4 | Return value undefined | Returns exactly `NUM_SIDES` (2) | **Rust assertion: `assert_eq!(num_ships, 2)`** |

### Queue Operation Parity (Side-Effect Matrix)

| Operation | HyperSpace | Battle (normal) | Battle (Sa-Matra) |
|-----------|-----------|-----------------|-------------------|
| `ReinitQueue(&race_q[0])` | YES | NO | NO |
| `ReinitQueue(&race_q[1])` | YES | NO | NO |
| `BuildSIS()` (inlined) | YES | NO | NO |
| `LoadHyperspace()` | YES | NO | NO |
| `free_gravity_well()` | NO | NO | YES |
| `spawn_asteroid(NULL)` × 5 | NO | YES | NO |
| `spawn_planet()` × 1 | NO | YES | NO |
| Clip rect / background / ClearDrawable | NO | YES | YES |

### Activity Flag Semantics for Init (per REQ-REMED-ACTIVITY-PARITY)

The C `InitShips()` reads `LOBYTE(GLOBAL(CurrentActivity))` directly. The C helper `rust_bridge_init_battle_arena()` also reads it directly (same macro, same global). No activity parameter is passed from Rust — the C helper reads the global itself. This means the Rust side does NOT need to pass activity flags for init (unlike spawn, where it's a parameter).

**Mandatory Rust-side return value assertion (fires in debug builds):**

```rust
#[cfg(all(not(test), debug_assertions))]
{
    let activity = uqm_get_current_activity_lobyte();
    // HyperSpace modes return 1, battle modes return NUM_SIDES (2)
    if is_hyperspace_activity(activity) {
        assert_eq!(num_ships, 1,
            "HyperSpace init should return 1, got {}", num_ships);
    } else {
        assert_eq!(num_ships, 2,
            "Battle init should return NUM_SIDES(2), got {}", num_ships);
    }
}
```

Where `is_hyperspace_activity()` checks via `inHQSpace()` (or the equivalent Rust-side check). This catches mode/return-value parity drift. This assertion is a **completion gate** for P02 — it must be present in the implementation.

## Changes

### 1. Add FFI declaration in ffi_contract.rs (H1)

**All C helper FFI bindings are declared ONLY in `ffi_contract.rs`.** No local `extern "C"` blocks in `ffi.rs`. This is the single canonical ABI declaration path.

Add to the existing `extern "C"` block in `ffi_contract.rs`, using canonical type aliases:

```rust
extern "C" {
    // ... existing declarations ...

    /// Initializes the battle arena (display list, galaxy, asteroids/planet, hyperspace).
    /// C: SIZE rust_bridge_init_battle_arena(void);
    /// Returns num_ships (NUM_SIDES for battle, 1 for hyperspace).
    /// Prototype: rust_bridge_ships.h
    pub fn rust_bridge_init_battle_arena() -> CSize;
}
```

### 2. Modify `rust_ships_init()` in ffi.rs

**Current code (line 324-338):**
```rust
pub unsafe extern "C" fn rust_ships_init() -> CCount {
    catch_unwind(|| {
        #[cfg(test)]
        let activity = 2u8;
        #[cfg(not(test))]
        let activity = unsafe { uqm_get_current_activity_lobyte() };

        match init_ships(activity) {
            Ok(num_players) => num_players as CCount,
            Err(_) => 0,
        }
    })
    .unwrap_or_default()
}
```

**New code:**
```rust
#[no_mangle]
pub unsafe extern "C" fn rust_ships_init() -> CCount {
    catch_unwind(|| {
        #[cfg(test)]
        {
            let activity = 2u8;
            return match init_ships(activity) {
                Ok(num_players) => num_players as CCount,
                Err(_) => 0,
            };
        }

        #[cfg(not(test))]
        unsafe {
            use crate::ships::ffi_contract::rust_bridge_init_battle_arena;

            // P05: One-time layout verification — aborts with clear message
            // on RaceDesc/RACE_DESC mismatch. Must run before any spawn.
            static LAYOUT_VERIFIED: std::sync::Once = std::sync::Once::new();
            LAYOUT_VERIFIED.call_once(|| {
                verify_race_desc_layout();
            });

            // Delegate arena setup entirely to C — this calls the original
            // InitShips() body which handles InitSpace, display list, galaxy,
            // asteroids, planets, hyperspace setup, etc.
            let num_ships = rust_bridge_init_battle_arena();
            if num_ships <= 0 {
                return 0;
            }

            // Track initialization state on the Rust side
            lifecycle::mark_ships_initialized();

            num_ships as CCount
        }
    })
    .unwrap_or_default()
}
```

### 3. BattleState Lifecycle API — Deterministic Approach

**Problem:** The original plan was inconsistent about whether `BattleState.ships_initialized` would be set in non-test builds. The text debated multiple approaches without committing to one, leaving the state tracking ambiguous.

**Resolution:** Add explicit `mark_ships_initialized()` / `mark_ships_uninitialized()` lifecycle methods to `lifecycle.rs`. These provide a clean, deterministic API that both test and non-test paths use for state tracking.

**Rationale for choosing this approach over "don't track state":**
- `ships_initialized` IS read outside tests — `uninit_ships()` in lifecycle.rs checks it to decide whether teardown is needed, and future Rust-side lifecycle code may also need it.
- Even if no current non-test path reads it today, leaving it permanently false in production would be a latent bug — any future code that checks `ships_initialized` would silently get the wrong answer.
- The cost is ~10 lines of code for correctness guarantees.

**Changes to lifecycle.rs:**

```rust
/// Mark battle ships as initialized (called after successful arena setup).
/// Safe to call from both test and non-test paths.
pub(crate) fn mark_ships_initialized() {
    let mut state = BATTLE_STATE
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    state.ships_initialized = true;
}

/// Mark battle ships as uninitialized (called after teardown).
/// Safe to call from both test and non-test paths.
pub(crate) fn mark_ships_uninitialized() {
    let mut state = BATTLE_STATE
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    state.ships_initialized = false;
}

/// Query whether ships are currently initialized.
/// Used by uninit idempotence guard (P03) — not test-only.
pub(crate) fn is_ships_initialized_for_uninit() -> bool {
    let state = BATTLE_STATE
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    state.ships_initialized
}

/// Query whether ships are currently initialized (test convenience).
#[cfg(test)]
pub(crate) fn is_ships_initialized() -> bool {
    is_ships_initialized_for_uninit()
}
```

**State flow (deterministic):**
1. `rust_ships_init()` → C helper succeeds → `mark_ships_initialized()` → `ships_initialized = true`
2. `rust_ships_uninit()` → C helper completes → `mark_ships_uninitialized()` → `ships_initialized = false`
3. Test path: `init_ships()` already sets `ships_initialized = true` directly (unchanged).

**No double-init of `init_space()`:** In non-test mode, `init_ships()` is NOT called — only `rust_bridge_init_battle_arena()` (which calls `InitSpace()` once) and `mark_ships_initialized()`. The ref-count asymmetry concern from the original plan is eliminated.

### 4. Verify BattleState consumers

Grep for `ships_initialized` and `space_init_count` usage. Confirm all non-test consumers are either:
- Bypassed in non-test mode (because `init_ships()`/`uninit_ships()` aren't called), OR
- Correctly handled by the new `mark_ships_initialized()`/`mark_ships_uninitialized()` calls.

This verification must be done during implementation, not deferred.

### 5. Debug assertions for state transitions (M3)

Add debug-build assertions to verify state consistency:

```rust
#[cfg(all(not(test), debug_assertions))]
{
    // After init: verify state is clean
    assert!(lifecycle::is_ships_initialized_for_uninit(),
        "ships_initialized should be true after successful init");
}
```

## Verification

- `cd rust && cargo test --lib` passes (test path unchanged — still calls `init_ships()`)
- `cd rust && cargo build --release` succeeds
- `cd sc2 && ./build.sh uqm` compiles and links cleanly (zero warnings)
- H1 acceptance check passes:
  ```bash
  grep -n 'extern "C"' rust/src/ships/ffi.rs | grep -c 'rust_bridge_'
  # Expected: 0
  grep -rn 'fn rust_bridge_' rust/src/ships/*.rs | grep -v ffi_contract.rs | grep -vc '#\[no_mangle\]'
  # Expected: 0
  ```
- Runtime: entering battle should show background stars, asteroids, planet
- HyperSpace entry should show flagship in hyperspace
- After init, `ships_initialized` is `true` in both test and non-test paths

## Output

- Modified: `rust/src/ships/ffi.rs` — init function wired to C helper, layout verification call
- Modified: `rust/src/ships/ffi_contract.rs` — FFI declaration for `rust_bridge_init_battle_arena`
- Modified: `rust/src/ships/lifecycle.rs` — added `mark_ships_initialized()`, `mark_ships_uninitialized()`, `is_ships_initialized_for_uninit()`, `is_ships_initialized()` (test)

## LoC Estimate

~20 lines Rust changed in ffi.rs, ~5 lines added to ffi_contract.rs, ~25 lines added to lifecycle.rs.

## Risk

Low. We're delegating to C code that was previously working before the USE_RUST_SHIPS guards were added. The C helper is literally the original `InitShips()` body. The new lifecycle API adds deterministic state tracking with minimal surface area.
