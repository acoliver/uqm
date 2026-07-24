# P09: Consolidate game state ownership to Rust

## Worker scope

Make Rust the single owner of `GlobData` (game state + SIS state). C will read
and write through Rust FFI instead of accessing C-owned memory directly.

This is the foundational phase — all subsequent phases depend on Rust owning
the game state so dispatch logic can read state natively without FFI.

### Current state (the problem)

```
C owns:  GLOBDATA GlobData          ← 767 Game_state fields + 15 SIS_state fields
         GLOBAL(f) = GlobData.Game_state.f
         GLOBAL_SIS(f) = GlobData.SIS_state.f

Rust has: GameState (155 bytes, just bit-packed flags)  ← SEPARATE COPY, not synced

Bridge:  C's getGameState() calls Rust's rust_get_game_state_bits_from_bytes()
         but the byte array is in C memory. Rust has its own copy.
```

### Target state (the solution)

```
Rust owns: GlobData equivalent (game state + SIS state + activity flags)
           All reads/writes go through Rust functions

C reads:   GET_GAME_STATE → rust_get_game_state() (already exists)
           GLOBAL(CurrentActivity) → c_get_current_activity() (already exists)
           GLOBAL_SIS(CrewEnlisted) → c_get_crew_enlisted() (already exists)

Bridge:   C calls Rust getters/setters for ALL game state access
           No dual ownership, no sync risk
```

### What already exists (don't rebuild)

- `rust/src/state/game_state.rs`: `GameState` struct with bit-packed flags (155 bytes)
- `rust/src/state/ffi.rs`: 28 `#[no_mangle]` exports including:
  - `rust_get_game_state(key)` / `rust_set_game_state(key, value)`
  - `rust_get_game_state_bits(start, end)` / `rust_set_game_state_bits(...)`
  - `rust_get_game_state_32(start)` / `rust_set_game_state_32(start, value)`
  - `rust_copy_game_state(...)`, `rust_reset_game_state()`
  - State file I/O: `rust_open_state_file`, `rust_close_state_file`, etc.
  - Planet info: `rust_init_planet_info`, `rust_get_planet_info`, etc.
- `sc2/src/uqm/globdata.c`: Already calls `rust_get_game_state_bits_from_bytes` etc.
- `rust/src/mainloop/c_extern.rs`: `get_current_activity()`, `set_current_activity()`,
  `get_next_activity()`, `set_last_activity()`, `uqm_get_crew_enlisted()`,
  `uqm_get_global_flags_and_data()`, etc.

### What needs to happen

1. **Identify all C globals that are still C-owned**
   - `GlobData.Game_state.CurrentActivity` — already has FFI accessor
   - `GlobData.Game_state.LastActivity` — needs FFI accessor
   - `GlobData.Game_state.NextActivity` — needs FFI accessor
   - `GlobData.Game_state.GameState[]` — bit extraction already in Rust, but byte array is C-owned
   - `GlobData.Game_state.velocity` — needs Rust ownership
   - `GlobData.Game_state.GameClock` — needs Rust ownership
   - `GlobData.Game_state.autopilot` — needs Rust ownership
   - `GlobData.Game_state.ip_planet`, `in_orbit` — needs Rust ownership
   - `GlobData.Game_state.npc_built_ship_q` — C queue, needs Rust ownership
   - `GlobData.Game_state.encounter_q`, `built_ship_q`, `avail_race_q` — C queues
   - `GlobData.SIS_state.*` (15 fields) — needs Rust ownership

2. **Move the GameState byte array to Rust ownership**
   - Currently: C's `GlobData.Game_state.GameState[155]` is the source of truth
   - Target: Rust's `GameState` (in `state/game_state.rs`) is the source of truth
   - C's `getGameState()` already calls `rust_get_game_state_bits_from_bytes` — just change
     it to call `rust_get_game_state_bits` (which reads from Rust's copy)
   - Same for `setGameState`, `getGameState32`, `setGameState32`
   - C's `GlobData.Game_state.GameState` becomes a dead field (or removed)

3. **Move activity flags to Rust ownership**
   - `CurrentActivity`, `LastActivity`, `NextActivity` — make Rust the owner
   - C's `GLOBAL(CurrentActivity)` becomes `c_get_current_activity()` (already exists)
   - C's `GLOBAL(LastActivity)` needs a `c_get_last_activity()` / `c_set_last_activity()`
   - C's `NextActivity` needs `c_get_next_activity()` / `c_set_next_activity()`
   - Replace all `GLOBAL(CurrentActivity)` accesses in C with FFI calls

4. **Move SIS state to Rust ownership**
   - Create Rust `SisState` struct mirroring C's `SIS_STATE`
   - Export FFI getters/setters for each field
   - Replace `GLOBAL_SIS(f)` accesses in C with FFI calls

5. **Move remaining GAME_STATE fields to Rust**
   - velocity, GameClock, autopilot, ip_planet, in_orbit, ShipStamp, ShipFacing
   - Queue fields (npc_built_ship_q, etc.) — these are more complex, may need
     a Rust queue implementation or keep in C temporarily

### Approach

Phase this incrementally — don't try to move everything at once:

**P09a**: Move GameState byte array + activity flags (most critical for comm)
**P09b**: Move SIS state
**P09c**: Move remaining GAME_STATE fields (velocity, clock, queues)

Each sub-phase is independently testable: after each, the game must still run
and all automation proofs must pass.

### Test plan

**Unit tests**:
- `GameState` round-trip: write a bit, read it back, verify value
- Activity flags: set/get CurrentActivity, LastActivity, NextActivity
- SIS state: set/get each field, verify C and Rust agree

**Automation proof** (`scripts/state-sync-v1.json`):
- Start game, wait for main menu
- Assert `CurrentActivity` == `SUPER_MELEE` (0) via `assert_activity`
- Start new game
- Assert `CurrentActivity` transitions to `IN_HYPERSPACE` (3)
- Capture, finish

**Regression**: All existing proofs (main-menu-v1, watchdog-v1, etc.) must still pass.

### Files to create/modify
- MODIFY: `rust/src/state/game_state.rs` (add activity/SIS fields if needed)
- CREATE: `rust/src/state/glob_data.rs` (Rust-owned GlobData equivalent)
- MODIFY: `rust/src/state/ffi.rs` (add FFI getters/setters for new fields)
- MODIFY: `rust/src/mainloop/c_extern.rs` (add missing FFI declarations)
- MODIFY: `sc2/src/uqm/globdata.c` (route GLOBAL() through Rust FFI)
- MODIFY: `sc2/src/uqm/globdata.h` (redefine GLOBAL() macro to call FFI)
- CREATE: `rust/scripts/state-sync-v1.json`