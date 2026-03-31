# P04 — Integration Testing & Validation

## Goal

Verify the full spawn/init/uninit lifecycle works end-to-end in a linked binary. Fix any issues discovered during integration. Include concrete, assertion-based checks for edge cases (Sa-Matra, HyperSpace, multi-battle, repeated hyperspace transitions). Verify battle_bridge.rs has no lifecycle-dependent assumptions. Add automated smoke tests for FFI lifecycle.

## Prerequisite

P01, P02, P03 all complete.

## Testing Strategy

### 1. Cargo Test (Rust-only)

Run the full Rust test suite to confirm no regressions:

```bash
cd rust && cargo test --lib
```

**Expected:** All 147 ship behavior tests pass. All ffi.rs, lifecycle.rs, writeback.rs tests pass. No new failures.

### 2. Link Test

Build the Rust library and full binary with USE_RUST_SHIPS enabled:

```bash
cd rust && cargo build --release
cd sc2 && ./build.sh uqm
```

**Expected:** Clean link. No undefined symbols. Verify with:

```bash
# Find object files (path depends on build system)
find sc2/ -name 'rust_bridge_ships.o' -type f -exec nm {} \; | grep rust_bridge_spawn
find sc2/ -name 'rust_bridge_ships.o' -type f -exec nm {} \; | grep rust_bridge_init
find sc2/ -name 'rust_bridge_ships.o' -type f -exec nm {} \; | grep rust_bridge_uninit
```

### 3. battle_bridge.rs Lifecycle Independence Checklist (M1)

**"No changes needed" for battle_bridge.rs must be verified, not assumed.**

Check every wrapper in `battle_bridge.rs` for assumptions that depend on lifecycle state:

| Check | What to verify | Pass criteria |
|-------|---------------|---------------|
| BB-1 | `bridge::alloc_element()` — does it assume display list is initialized? | Call is a thin C wrapper; C's `AllocElement` handles null display list safely |
| BB-2 | `bridge::lock_element(h)` — does it assume h was allocated by init? | Returns null on invalid handle; callers check return |
| BB-3 | `bridge::get_element_starship(e)` — does it assume element has a valid starship? | Returns null on unset starship; callers check return |
| BB-4 | `bridge::create_missile(block)` / `bridge::create_laser(block)` — do they assume arena context? | **Intended conditional dependency:** C's `initialize_missile`/`initialize_laser` require active battle context (display list, element arena). These wrappers are only callable from within battle callbacks (`preprocess`, `postprocess`) that fire after init completes — so the dependency is satisfied by design, not by accident. This is NOT a lifecycle independence violation; it is the correct architecture. Document in `battle_bridge.rs` with a comment: "Weapon creation requires initialized battle context; called only from within battle callbacks." |
| BB-5 | `bridge::process_sound(sound, source)` — does it crash if no sound system? | C's `ProcessSound` checks for null sound system; safe |
| BB-6 | `bridge::set_velocity_vector(...)` — does it depend on element being in display list? | Operates on velocity struct directly; no display list dependency |
| BB-7 | All wrappers in `#[cfg(test)] pub mod bridge` — do test mocks mirror lifecycle assumptions? | Test mocks are stateless (return constants/mock handles); no lifecycle dependency |
| BB-8 | `element_flags` and `status_flags` constants — match C values? | Already tested in `tests::element_flag_values` and `tests::status_flag_values` |

**Procedure:** During implementation, read each wrapper in battle_bridge.rs and confirm it satisfies the "Pass criteria" column. Document any exceptions found. If any wrapper makes a lifecycle assumption (e.g., "arena must be initialized"), add a comment documenting that assumption.

**Expected result:** All wrappers are lifecycle-independent except BB-4 (weapon creation requires active battle context), which is called only from within battle callbacks that can only fire after init completes. No code changes needed, but the checklist is completed and recorded.

#### M1: Lifecycle Invariant Tests and Call-Site Assertions for battle_bridge.rs

While `battle_bridge.rs` doesn't need structural code changes, the following invariant tests and call-site assertions must be added to **verify** the lifecycle dependency analysis is correct (per REQ-REMED-BATTLE-BRIDGE-INVARIANT):

**A. Add invariant tests to `battle_bridge.rs`:**

```rust
#[cfg(test)]
mod lifecycle_invariant_tests {
    use super::*;

    /// BB-4 invariant: Weapon creation wrappers are only callable from
    /// within battle callbacks. Verify that calling them without battle
    /// context is handled gracefully (returns error/null, not crash).
    #[test]
    fn weapon_creation_without_battle_context_does_not_crash() {
        // In test mode, C helpers are stubbed. Verify the Rust wrapper
        // handles null/zero returns from stubs without panicking.
        // This documents that weapon creation is context-dependent.
        // Create a MissileBlock with test values, call create_missile,
        // verify it returns 0/null (no element allocated from stub).
    }

    /// BB-1/BB-2 invariant: Element allocation wrappers handle null/zero
    /// returns from C without crashing.
    #[test]
    fn element_alloc_handles_null_return() {
        // In test mode, AllocElement returns 0 (stub).
        // Verify callers handle this gracefully.
    }

    /// BB-5 invariant: Sound processing handles null sound system.
    #[test]
    fn sound_processing_without_sound_system_does_not_crash() {
        // In test mode, ProcessSound is stubbed.
        // Verify no panic on null sound/element.
    }
}
```

**B. Add comments to weapon creation call sites** (in the ship behavior `postprocess` implementations that call `create_missile`/`create_laser`):
```rust
// Weapon creation requires initialized battle context (display list, element arena).
// This is satisfied by design: postprocess callbacks only fire after InitShips completes.
// See BB-4 in P04 lifecycle independence checklist.
```

**C. Add debug-build assertion at weapon creation entry** (in `battle_bridge.rs` wrappers):
```rust
#[cfg(all(not(test), debug_assertions))]
{
    // BB-4: Weapon creation requires active battle context.
    // This assertion fires if the wrapper is called outside a battle callback.
    assert!(lifecycle::is_ships_initialized_for_uninit(),
        "create_missile called outside initialized battle context");
}
```

This makes the lifecycle dependency explicit in both code and tests, not just documentation.

### 4. Super Melee Battle Test

1. Launch the game
2. Enter Super Melee
3. Select ships for both sides
4. Start battle
5. **Verify:** Both ships appear on screen with correct sprites
6. **Verify:** Ships respond to controls (turn, thrust, fire)
7. **Verify:** Weapons fire correctly
8. **Verify:** Ships can damage each other
9. **Verify:** When one ship dies, replacement ship spawns
10. **Verify:** When all ships on one side are dead, battle ends
11. **Verify:** Returning to Super Melee menu works (no crash)
12. **Verify:** Starting another battle works

### 5. Full Game Encounter Test

1. Start a new game (or load a save near an encounter)
2. Enter combat with an alien fleet
3. **Verify:** Same checks as Super Melee
4. **Verify:** After battle, crew levels are correctly preserved
5. **Verify:** Fleet fragment shows correct surviving crew count
6. **Verify:** Game continues normally after battle

### 6. HyperSpace Test — With Concrete Assertions

1. Enter hyperspace
2. **Verify:** Flagship appears
3. **Verify:** Navigation works
4. **Verify:** Transitioning to a star system works

**Concrete state assertions (add as debug logging or debug-build assertions in ffi.rs):**

```rust
// In rust_ships_init(), after rust_bridge_init_battle_arena() returns:
#[cfg(all(not(test), debug_assertions))]
{
    let activity = uqm_get_current_activity_lobyte();
    if is_hyperspace_activity(activity) {
        // HyperSpace: init must return 1 (not NUM_SIDES)
        assert_eq!(num_ships, 1, "HyperSpace init should return 1, got {}", num_ships);
    } else {
        // Battle: init must return NUM_SIDES (2)
        assert_eq!(num_ships, 2, "Battle init should return NUM_SIDES(2), got {}", num_ships);
    }
}
```

Where `is_hyperspace_activity()` checks `activity` for the HyperSpace/QuasiSpace flags (IN_HYPERSPACE / IN_QUASISPACE). Implement this check using the activity constants already defined in the codebase.

### 7. SIS Flagship Lifecycle Across Repeated Hyperspace Transitions (H4)

**Problem:** The plan lacked explicit verification of the SIS flagship lifecycle across repeated hyperspace transitions. The flagship is special: it uses `BuildSIS()` (now inlined in C helper), `SIS_SHIP_ID`, `RPG_PLAYER_NUM`, and `crew_level == 0` (sentinel for SIS). Repeated hyperspace transitions stress the init/uninit cycle specifically for the flagship path.

**Test matrix:**

| Sequence | Steps | Verify |
|----------|-------|--------|
| T1: HyperSpace → StarSystem → HyperSpace | Enter HyperSpace, enter a star system, return to HyperSpace | Flagship re-appears correctly, navigation works, no crash |
| T2: HyperSpace → Encounter → Battle → HyperSpace | Enter HyperSpace, trigger encounter, fight battle, return to HyperSpace | Crew writeback correct, flagship re-appears, no state leak |
| T3: HyperSpace → QuasiSpace → HyperSpace | Enter HyperSpace, use portal to QuasiSpace, return to HyperSpace | Both transitions work, flagship state preserved |
| T4: Repeated H→S→H cycle (3x) | Enter/exit star systems three times in succession | No memory growth, no display list corruption, no stale element handles |
| T5: HyperSpace → Encounter → Win → HyperSpace → Encounter → Win | Two consecutive encounters with hyperspace between them | Crew correctly writtenback after each battle, no state contamination between battles |
| T6: HyperSpace uninit during transition | Enter star system (triggers uninit of hyperspace state) | `FreeHyperspace()` called correctly, no leak |

**Concrete assertions for T2 and T5:**

```rust
#[cfg(all(not(test), debug_assertions))]
{
    // Before uninit: verify at least one element with PLAYER_SHIP exists
    // (flagship should be in display list during hyperspace)
    // After uninit + re-init: verify init returns 1 for hyperspace
}
```

**Implementation:** These tests are manual (require game interaction) but the debug assertions fire automatically. For T4, check memory usage with:
```bash
# macOS — find the built binary first
BINARY=$(find sc2/ -name 'uqm' -type f -perm +111 2>/dev/null | head -1)
leaks --atExit -- "$BINARY"  # run game, perform T4 sequence, quit
# or use Instruments > Leaks
```

### 8. Sa-Matra Battle Test — With Concrete Assertions

1. Load a save at the final battle (IN_LAST_BATTLE)

**Concrete state assertions (add in C helper or as debug logging):**

In `rust_bridge_spawn_element()`, add debug-build assertions for the Sa-Matra special case:

```c
#ifndef NDEBUG
        if (ShipElementPtr->playerNr == NPC_PLAYER_NUM
                && activity == IN_LAST_BATTLE)
        {
            /* Sa-Matra verification */
            assert(StarShipPtr->ShipFacing == 0);
            assert(ShipElementPtr->current.location.x == (LOG_SPACE_WIDTH >> 1));
            assert(ShipElementPtr->current.location.y == (LOG_SPACE_HEIGHT >> 1));
            assert(ShipElementPtr->life_span == NORMAL_LIFE + 1);
        }
#endif
```

2. **Verify:** Sa-Matra appears at center, facing 0
3. **Verify:** Sa-Matra has incremented life_span (NORMAL_LIFE + 1)
4. **Verify:** Battle plays out correctly
5. **Verify:** After Sa-Matra battle, game state is properly cleaned up (no crash on return)
6. **Verify:** `free_gravity_well()` was called during init (IN_LAST_BATTLE skips asteroid/planet spawning)

### 9. Multi-Battle Sequence Test — With Concrete Assertions

This test verifies that init/uninit cycles don't leak state across battles.

1. Enter Super Melee, fight a battle, let it end normally
2. **Assert:** `ships_initialized` is false after uninit (verified by Rust-side state tracking)
3. Start a second battle immediately
4. **Assert:** `ships_initialized` transitions false → true
5. Fight second battle to completion
6. **Assert:** Crew writeback is correct for the SECOND battle (not contaminated by first)
7. Start a third battle
8. **Assert:** No memory growth (check with Instruments/valgrind if available)

**Concrete Rust-side assertions for multi-battle (add to `rust_ships_init` and `rust_ships_uninit` in debug builds):**

```rust
#[cfg(all(not(test), debug_assertions))]
{
    // After init: verify state is clean
    assert!(lifecycle::is_ships_initialized_for_uninit(),
        "ships_initialized should be true after init");
}

#[cfg(all(not(test), debug_assertions))]
{
    // After uninit: verify state is clean
    assert!(!lifecycle::is_ships_initialized_for_uninit(),
        "ships_initialized should be false after uninit");
}
```

### 10. Double-Uninit Idempotence Test

Verify that calling `rust_ships_uninit()` twice does not crash:

**In Rust test (test-only, calls lifecycle directly):**
```rust
#[test]
fn double_uninit_is_idempotent() {
    // Setup
    let activity = 2u8;
    let _ = init_ships(activity);

    // First uninit
    rust_ships_uninit_test_path();
    assert!(!lifecycle::is_ships_initialized());

    // Second uninit — should be no-op
    rust_ships_uninit_test_path();
    assert!(!lifecycle::is_ships_initialized());
}
```

**In runtime (manual test):** Debug build will print "idempotence guard fired" message when the guard triggers. Trigger by quitting during battle or other error-recovery paths.

### 11. FFI Surface Scope Verification (per REQ-REMED-FFI-SCOPE)

Verify that `ffi_contract.rs` has not been over-expanded beyond the planned FFI surface. Run these checks during P04:

```bash
# Count NEW extern "C" declarations added to ffi_contract.rs (compare to baseline)
# Expected new additions: ONLY these rust_bridge_* functions:
#   - rust_bridge_spawn_element
#   - rust_bridge_init_battle_arena
#   - rust_bridge_uninit_ships
#   - rust_bridge_get_race_desc_layout
#   - (optional) rust_race_desc_get_* / rust_race_desc_set_* accessors (only if P05 layout fails)
# NO new declarations for functions called internally by C helpers
# (e.g., InitDisplayList, AllocElement, StopSound, spawn_asteroid, etc.)

grep -c 'pub fn rust_bridge_' rust/src/ships/ffi_contract.rs
# Expected: 4 (or 4 + accessor count if P05 fallback)

# Verify no "internal" C functions were added as new FFI declarations:
git diff HEAD -- rust/src/ships/ffi_contract.rs | grep '^+.*pub fn ' | grep -v rust_bridge_ | grep -v rust_race_desc_
# Expected: 0 lines (no new non-bridge FFI declarations)
```

If the scope has expanded beyond the planned surface, reduce it before P04 is complete.

### 12. Automated FFI Lifecycle Smoke Tests

**Problem:** `#[cfg(not(test))]` gating means `cargo test` won't execute C-helper call paths. The FFI lifecycle is only tested by manual runtime checks.

**Mitigation:** Add lightweight automated smoke tests that verify lifecycle behavior without requiring the full game engine:

#### A. Scripted binary launch check

**Note:** The UQM binary does NOT have a `--quit-after-init` flag or any headless/test mode. The binary requires SDL, a display, and user interaction. Automated binary launch testing is therefore limited to crash-on-startup detection.

```bash
#!/bin/bash
# test_ffi_lifecycle.sh — basic smoke test for FFI lifecycle
# Requires: built binary with USE_RUST_SHIPS=1 (debug build preferred)
# Requires: display server (X11/Wayland/macOS) available

set -e

# Find the built binary (path depends on build system output)
BINARY=$(find sc2/ -name 'uqm' -type f -perm +111 2>/dev/null | head -1)
if [ -z "$BINARY" ]; then
    echo "SKIP: UQM binary not found. Build with: cd sc2 && ./build.sh uqm"
    exit 0
fi

TIMEOUT=15

echo "=== FFI Lifecycle Smoke Test ==="

# Test 1: Binary starts without immediate crash
# Send SIGTERM after timeout to cleanly exit.
# The binary will exercise init paths during startup.
echo "Test 1: Binary launch (will kill after ${TIMEOUT}s)..."
timeout --signal=TERM $TIMEOUT "$BINARY" 2>stderr_output.tmp &
BINARY_PID=$!
sleep 3  # Give it time to initialize

# Check if still running (didn't crash during init)
if kill -0 $BINARY_PID 2>/dev/null; then
    echo "PASS: Binary survived init phase (still running after 3s)"
    kill -TERM $BINARY_PID 2>/dev/null || true
    wait $BINARY_PID 2>/dev/null || true
else
    wait $BINARY_PID 2>/dev/null
    EXIT_CODE=$?
    if [ $EXIT_CODE -ne 0 ] && [ $EXIT_CODE -ne 143 ]; then
        echo "FAIL: Binary crashed during startup (exit code $EXIT_CODE)"
        echo "--- stderr ---"
        cat stderr_output.tmp
        rm -f stderr_output.tmp
        exit 1
    fi
fi

# Test 2: Check debug output for lifecycle messages (debug builds only)
if [ -f stderr_output.tmp ]; then
    if grep -q "rust_ships_init\|mark_ships_initialized\|verify_race_desc_layout" stderr_output.tmp; then
        echo "PASS: Lifecycle debug messages detected in stderr"
    else
        echo "INFO: No lifecycle debug messages (expected in release builds)"
    fi
fi

rm -f stderr_output.tmp
echo "=== Smoke tests complete ==="
```

**Limitations:** This test requires a display server and can only verify that the binary doesn't crash on startup. It cannot verify battle-specific lifecycle behavior (that requires manual testing per sections 4-9). For CI environments without a display, skip this test or use `xvfb-run` on Linux.

#### B. Rust-side lifecycle roundtrip test

Add a test that exercises the full lifecycle in test mode (no C helpers, but verifies state transitions):

```rust
#[cfg(test)]
mod lifecycle_integration_tests {
    use super::*;

    #[test]
    fn full_lifecycle_roundtrip() {
        // Init
        let result = unsafe { rust_ships_init() };
        assert!(result > 0, "init should return positive ship count");
        assert!(lifecycle::is_ships_initialized());

        // Uninit
        unsafe { rust_ships_uninit() };
        assert!(!lifecycle::is_ships_initialized());

        // Re-init (multi-battle)
        let result2 = unsafe { rust_ships_init() };
        assert!(result2 > 0, "re-init should succeed");
        assert!(lifecycle::is_ships_initialized());

        // Final uninit
        unsafe { rust_ships_uninit() };
        assert!(!lifecycle::is_ships_initialized());
    }

    #[test]
    fn uninit_without_init_is_safe() {
        // Ensure ships are not initialized
        lifecycle::mark_ships_uninitialized();

        // This should be a no-op
        unsafe { rust_ships_uninit() };
        assert!(!lifecycle::is_ships_initialized());
    }
}
```

#### C. Callback Entry Liveness Checks — Extraction-Point Guards (per REQ-REMED-CALLBACK-GUARD)

The callback FFI entry points (`rust_ships_preprocess`, `rust_ships_postprocess`, `rust_ships_death`) extract `StarShip*` and `RaceDesc*` pointers from elements. These pointers may be stale or null during edge cases (element outlives its StarShip, callback fires during uninit, race between spawn and first callback).

**Critical requirement:** Liveness checks MUST occur **at the extraction point** — the moment the raw pointer is obtained from the element, BEFORE any borrow/marshal/conversion helper is called. The key distinction:
- **Extraction** (safe): reading a pointer value from a struct field — e.g., reading `element->pParent` to get the StarShip address
- **Dereference** (unsafe): following that pointer to access fields — e.g., `(*starship_ptr).race_desc_ptr`

The null check must happen between extraction and first dereference. Specifically, checks must fire BEFORE:
- `borrow_starship_from_c()` — dereferences the starship pointer
- `extract_starship_from_element()` — dereferences element→pParent
- `build_element_state()` — reads element fields through the starship/descriptor

**Required check ordering for each callback entry point:**

```rust
// Step 1: Extract raw StarShip pointer from element WITHOUT dereferencing it.
//         This reads the pointer VALUE from the element struct — it does NOT
//         follow the pointer. Uses GetElementStarShip via FFI or direct pParent read.
//         If extract_raw_starship_ptr() doesn't exist, ADD IT as a minimal helper.
let starship_ptr = extract_raw_starship_ptr(element_ptr);

// Step 2: LIVENESS CHECK — fires in ALL BUILDS (debug AND release)
//         BEFORE borrow_starship_from_c or any other marshal helper
if starship_ptr.is_null() {
    #[cfg(debug_assertions)]  // Only the logging is debug-only
    eprintln!("rust_ships_preprocess: null StarShipPtr at entry, skipping");
    return;  // Early return — unconditional
}

// Step 3: Read race_desc_ptr from the starship — single field dereference
//         This is the first dereference of starship_ptr, and it's safe because
//         we just verified starship_ptr is non-null.
let race_desc_ptr = (*starship_ptr).race_desc_ptr;

// Step 4: LIVENESS CHECK — fires in ALL BUILDS (debug AND release)
//         BEFORE any RaceDesc access or build_element_state
if race_desc_ptr.is_null() {
    #[cfg(debug_assertions)]
    eprintln!("rust_ships_preprocess: null RaceDescPtr at entry, skipping");
    return;  // Early return — unconditional
}

// Step 5: NOW safe to call borrow_starship_from_c(), build_element_state(), etc.
//         Both pointers are known non-null at this point.
```

**The null checks (`if ... is_null()` + `return`) fire in ALL builds (debug AND release).** Only the `eprintln!` logging is `#[cfg(debug_assertions)]`. The cost is two pointer comparisons per callback invocation — negligible. This prevents undefined behavior from propagating into Rust ship behavior code when elements have stale references.

**Implementation for all three callbacks:**

| Callback | Where to add check | What calls to guard | Return type |
|----------|-------------------|---------------------|-------------|
| `rust_ships_preprocess` | Top of function, before `borrow_starship_from_c` | `borrow_starship_from_c`, `build_element_state` | `void` (return) |
| `rust_ships_postprocess` | Top of function, before `borrow_starship_from_c` | `borrow_starship_from_c`, `build_element_state` | `void` (return) |
| `rust_ships_death` | Top of function, before `extract_starship_from_element` | `extract_starship_from_element`, element field reads | `void` (return) |

**If `extract_raw_starship_ptr` doesn't exist:** Add a minimal helper in `ffi.rs` that reads the element's `pParent` field (or uses `GetElementStarShip` via FFI) and returns the raw `*mut CStarship` pointer without any further dereference. This is the "extraction-point check" that REQ-REMED-CALLBACK-GUARD requires.

**Verification:**
1. Code review: confirm null checks are NOT inside `#[cfg(debug_assertions)]` blocks — only the logging is conditional.
2. Runtime: confirm with debug build that no false positives fire during normal battle gameplay.
3. The checks should only trigger in error/edge cases (stale elements, partial uninit).

#### D. Temporary debug assertions for pointer validity (M3)

Add debug-build assertions in the C helper and Rust FFI layer that verify pointer validity during state transitions:

**In `rust_bridge_spawn_element()` (C side):**
```c
#ifndef NDEBUG
    /* M3: Verify inputs */
    assert(StarShipPtr != NULL && "spawn_element: null StarShipPtr");
    assert(RDPtr != NULL && "spawn_element: null RaceDescPtr");
    /* Verify after LockElement */
    assert(ShipElementPtr != NULL && "spawn_element: LockElement returned null");
    /* Verify frame array is loaded */
    assert(RDPtr->ship_data.ship != NULL && "spawn_element: ship frames not loaded");
#endif
```

**In `rust_ships_spawn()` (Rust side):**
```rust
#[cfg(all(not(test), debug_assertions))]
{
    assert!(!race_desc_ptr.is_null(),
        "spawn: race_desc_ptr is null after lifecycle_spawn");
    assert!(ship_mass > 0,
        "spawn: ship_mass is 0 (descriptor may be corrupt)");
}
```

**In `rust_bridge_uninit_ships()` (C side):**
```c
#ifndef NDEBUG
    /* M3: Verify element pointer before field access */
    assert(ElementPtr != NULL && "uninit: LockElement returned null");
    if (StarShipPtr != NULL && StarShipPtr->RaceDescPtr != NULL)
    {
        /* Verify crew_level is plausible */
        assert(StarShipPtr->RaceDescPtr->ship_info.crew_level <=
               StarShipPtr->RaceDescPtr->ship_info.max_crew &&
               "uninit: crew_level exceeds max_crew");
    }
#endif
```

### 13. Concrete Verification Hooks — Scripted Scenario Checks (per REQ-REMED-VERIFICATION-HOOKS)

Manual testing (sections 4-9) is necessary but insufficient for regression detection. The following deterministic verification hooks provide automated state-transition checking that fires during gameplay without manual intervention.

#### A. Debug-Build State Transition Logger

Add a lifecycle event logger that records init/spawn/uninit transitions with timestamps and activity state. This creates an auditable trace for post-hoc verification.

**In `ffi.rs`, add a debug-only lifecycle event log:**

```rust
#[cfg(all(not(test), debug_assertions))]
mod lifecycle_trace {
    use std::sync::Mutex;
    static TRACE: Mutex<Vec<String>> = Mutex::new(Vec::new());

    pub fn log_event(event: &str) {
        if let Ok(mut trace) = TRACE.lock() {
            let activity = unsafe { super::uqm_get_current_activity_lobyte() };
            trace.push(format!("[activity={:#04x}] {}", activity, event));
            eprintln!("LIFECYCLE: [activity={:#04x}] {}", activity, event);
        }
    }
}
```

Instrumentation points:
- `rust_ships_init()` entry and exit (with return value)
- `rust_ships_spawn()` entry and exit (with species ID and success/fail)
- `rust_ships_uninit()` entry and exit (with idempotence guard outcome)
- `rust_ships_free()` entry (with pointer address)

#### B. Multi-Battle State Leak Detection

Add assertions that verify state is clean at init entry (catches leaks from previous battles):

```rust
// At the top of rust_ships_init(), in non-test debug builds:
#[cfg(all(not(test), debug_assertions))]
{
    if lifecycle::is_ships_initialized_for_uninit() {
        eprintln!(
            "WARNING: rust_ships_init called while ships still initialized! \
             Previous uninit may have been skipped."
        );
    }
}
```

#### C. Sa-Matra Correctness Assertions

In `rust_bridge_spawn_element()` (C side), after the Sa-Matra branch:

```c
#ifndef NDEBUG
    if (ShipElementPtr->playerNr == NPC_PLAYER_NUM
            && activity == IN_LAST_BATTLE)
    {
        /* M3: Sa-Matra state transition verification */
        assert(StarShipPtr->ShipFacing == 0
            && "Sa-Matra must face 0");
        assert(ShipElementPtr->current.location.x == (LOG_SPACE_WIDTH >> 1)
            && "Sa-Matra must be centered X");
        assert(ShipElementPtr->current.location.y == (LOG_SPACE_HEIGHT >> 1)
            && "Sa-Matra must be centered Y");
        assert(ShipElementPtr->life_span == NORMAL_LIFE + 1
            && "Sa-Matra life_span must be NORMAL_LIFE+1");
        log_add(log_Debug, "M3: Sa-Matra spawn verified: facing=0, centered, life_span=%u",
                (unsigned)ShipElementPtr->life_span);
    }
#endif
```

#### D. HyperSpace Init/Uninit Symmetry Check

Add a debug counter that tracks init/uninit calls and asserts they're balanced:

```rust
#[cfg(all(not(test), debug_assertions))]
static INIT_COUNT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

// In rust_ships_init(), after success:
#[cfg(all(not(test), debug_assertions))]
{
    let count = INIT_COUNT.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    eprintln!("LIFECYCLE: init #{}", count + 1);
}

// In rust_ships_uninit(), after teardown:
#[cfg(all(not(test), debug_assertions))]
{
    let count = INIT_COUNT.load(std::sync::atomic::Ordering::SeqCst);
    eprintln!("LIFECYCLE: uninit (init count was {})", count);
}
```

#### E. Crew Writeback Verification

In `rust_bridge_uninit_ships()` (C side), add debug logging that records pre- and post-writeback crew values:

```c
#ifndef NDEBUG
    /* M3: Crew writeback verification */
    if (StarShipPtr->RaceDescPtr->ship_info.crew_level)
    {
        COUNT pre_crew = StarShipPtr->RaceDescPtr->ship_info.crew_level;
        /* ... crew writeback logic ... */
        log_add(log_Debug,
                "M3: crew writeback player=%d: pre=%u post=%u max=%u floating=%u",
                StarShipPtr->playerNr,
                (unsigned)pre_crew,
                (unsigned)StarShipPtr->RaceDescPtr->ship_info.crew_level,
                (unsigned)StarShipPtr->RaceDescPtr->ship_info.max_crew,
                (unsigned)crew_retrieved);
    }
#endif
```

These hooks run automatically in debug builds during any gameplay session. No manual test harness required — just play the game in debug mode and check the output log for assertion failures or unexpected state transitions.

#### F. Scripted Scenario State Transition Matrix

The following matrix defines the expected state transitions for specific gameplay scenarios. Debug-build assertions and lifecycle trace logging (section A above) make these transitions verifiable from the log output without manual state inspection.

| Scenario | Expected Lifecycle Trace | Expected Assertions (all must pass) |
|----------|-------------------------|-------------------------------------|
| **Super Melee: single battle** | init(activity=SUPER_MELEE) → spawn(p0) → spawn(p1) → [battle] → uninit | init returns 2, both spawns return 1, uninit completes, ships_initialized=false after |
| **Super Melee: replacement ship** | init → spawn(p0) → spawn(p1) → [p0 dies] → spawn(p0-replacement, hShip!=0) → ... → uninit | Replacement spawn succeeds (returns 1), replacement ship has correct sprites/callbacks |
| **Super Melee: 3 consecutive battles** | init→spawns→uninit → init→spawns→uninit → init→spawns→uninit | Each init returns 2. No init called while ships_initialized=true (leak check). Third uninit completes cleanly. |
| **Encounter: battle + crew writeback** | init(activity=IN_ENCOUNTER) → spawn(p0) → spawn(p1) → [battle] → uninit | Crew writeback log shows pre/post crew values (section E). UpdateShipFragCrew called. |
| **Sa-Matra battle** | init(activity=IN_LAST_BATTLE) → spawn(NPC,facing=0,centered) → spawn(player) → [battle] → uninit | Sa-Matra assertions fire (section C): facing=0, centered, life_span=NORMAL_LIFE+1. free_gravity_well called during init. |
| **HyperSpace → StarSystem → HyperSpace** | init(HQ) → spawn(SIS) → uninit → init(battle) → spawns → uninit → init(HQ) → spawn(SIS) | HQ init returns 1, battle init returns 2, second HQ init returns 1. FreeHyperspace called during uninit. |
| **HyperSpace: 3 transitions (T4)** | init(HQ)→uninit → init(HQ)→uninit → init(HQ)→uninit | No memory growth, no stale element handles, init/uninit count balanced (section D). |

**How to verify:** Run the game in a debug build. After completing the scenario, check stderr/log output for:
1. No assertion failures
2. Lifecycle trace events in the expected order
3. Crew writeback values are plausible (pre ≤ max, post ≤ max)
4. Init/uninit counts are balanced
5. No "WARNING desync" messages (unless testing partial-init recovery)

## Known Issues to Watch For

### A. RaceDesc/RACE_DESC Layout Mismatch

**Symptom:** Ships appear but with wrong sprites, or crash when accessing `ship_data.ship` frame array.

**Diagnosis:** The Rust `RaceDesc` returned by `rust_ships_load()` is passed to C's `rust_bridge_spawn_element()` as a `RACE_DESC*`. If field offsets differ, `RDPtr->ship_data.ship` reads garbage.

**Fix:** P05 adds runtime verification with hard-fail. If layout diverges, accessor-function approach is used (see P05 section 6).

### B. Callback Chain Crash

**Symptom:** Crash in `ship_preprocess` after spawn.

**Diagnosis:** `ship_preprocess` has `#ifdef USE_RUST_SHIPS` which calls `rust_ships_preprocess`. This calls `extract_starship_from_element` which reads `element->p_parent` as `CStarship*`. If `SetElementStarShip` in the C helper didn't set `p_parent` correctly, the pointer is garbage.

**Fix:** Verify `SetElementStarShip` sets the pParent field. Check Element struct layout in element.rs matches C's ELEMENT exactly. Particularly verify offset of `p_parent`.

### C. Double-Free on Descriptor

**Symptom:** Crash during uninit when `free_ship()` is called.

**Diagnosis:** `rust_bridge_uninit_ships()` calls `free_ship(RDPtr)` which dispatches to `rust_ships_free()` which calls `Box::from_raw()`. If the pointer is already freed or was never a valid Box allocation, this crashes.

**Fix:** P03 adds four-layer protection (Rust idempotence guard, C null guards, C post-free nulling, Rust null check in `rust_ships_free`).

### D. Missing ELEMENT Fields

**Symptom:** Ships appear but don't interact (no collision, no physics).

**Diagnosis:** The C helper may not set all required ELEMENT fields. Compare field-by-field with the original spawn_ship in ship.c.

**Fix:** Audit the C helper against the original, field by field.

## Post-Testing Cleanup

After all tests pass:

1. Remove any temporary debug prints added during testing (keep `#[cfg(debug_assertions)]` assertions — those are permanent safety nets)
2. Verify no warnings in Rust compilation (`cargo test 2>&1 | grep warning`)
3. Verify no warnings in C compilation
4. Run `cargo clippy` if available
5. Confirm all tests are deterministic (run 3 times)

## Output

- Bug fixes discovered during integration (files TBD)
- Debug-build assertions added to ffi.rs and rust_bridge_ships.c for Sa-Matra, HyperSpace, multi-battle, and pointer validity
- **Callback entry extraction-point guards (per REQ-REMED-CALLBACK-GUARD):** Null-pointer checks at extraction point BEFORE any borrow/marshal helper in `rust_ships_preprocess`, `rust_ships_postprocess`, `rust_ships_death`. Null checks fire in ALL builds; logging is debug-only.
- **battle_bridge.rs lifecycle invariants (per REQ-REMED-BATTLE-BRIDGE-INVARIANT):** Independence checklist completed. Invariant tests added. BB-4 documented as **intended conditional dependency** with debug assertion at call site.
- **FFI surface scope verification (per REQ-REMED-FFI-SCOPE):** Grep check confirms no over-expansion of `ffi_contract.rs` declarations.
- Automated lifecycle roundtrip test added to ffi.rs
- Smoke test script for binary launch (uses SIGTERM-after-timeout, NOT `--quit-after-init` which doesn't exist)
- Hyperspace transition test matrix (H4) executed
- **Concrete verification hooks (per REQ-REMED-VERIFICATION-HOOKS):** debug-build lifecycle trace logger, multi-battle state leak detection, Sa-Matra correctness assertions, init/uninit symmetry counter, crew writeback verification logging, scripted scenario state transition matrix
- Confidence that the spawn/init/uninit lifecycle is functional across all game modes

## LoC Estimate

~60-70 lines of debug assertions and verification hooks (C + Rust). ~20-25 lines callback entry liveness checks (H2, 3 callbacks × ~7 lines each). ~30 lines lifecycle roundtrip tests. ~15 lines lifecycle invariant tests (M1). ~30 lines lifecycle trace logger (M3). ~30 lines smoke test script. Bug fix LoC depends on bugs found — estimate 0-50 lines.
