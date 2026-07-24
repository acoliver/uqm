# P09: Port RaceCommunication + InitCommunication to Rust

## Worker scope

Port the comm dispatch entry points from C `comm.c` to Rust, replacing
the FFI calls in `game_loop.rs` with native Rust implementations.

### What to port

1. **`RaceCommunication()`** (comm.c:1503-1600, ~100 lines)
   - Determines which alien race to talk to based on encounter state
   - Calls `InitCommunication(conversation_id)` with the selected race
   - Handles special cases: Talking Pet, Spathi, Ilwrath, Chmmr, Arilou

2. **`InitCommunication(which_comm)`** (comm.c:1359-1470, ~110 lines)
   - Maps conversation ID → ship type
   - Calls `init_race(comm_id)` to get LOCDATA
   - Copies LOCDATA to global CommData
   - Calls `InitEncounter()` or sets BATTLE_SEGUE
   - If HAIL: calls `HailAlien()` to start the dialogue

3. **`init_race(comm_id)`** dispatch table (commglue.c:367-415, ~50 lines)
   - Switch statement mapping conversation IDs to per-race init functions
   - Each returns a `LOCDATA*` with animation config, function pointers

4. **`HailAlien()`** (comm.c:1183-1356, ~170 lines)
   - Sets up the communication screen
   - Loads race resources (graphics, fonts, colormaps, music)
   - Starts the dialogue animation
   - Calls the race's `init_encounter_func` (the dialogue entry point)

### Approach

- Create `rust/src/comm/dispatch.rs` with:
  - `race_communication()` — replaces C `RaceCommunication()`
  - `init_communication(which_comm: u32) -> u32` — replaces C `InitCommunication()`
  - `init_race(comm_id: u32) -> Option<CommData>` — replaces C `init_race()`
  - `hail_alien()` — replaces C `HailAlien()`

- Update `game_loop.rs` `CffiOps::race_communication()` to call
  `comm::dispatch::race_communication()` instead of C FFI

- The per-race `init_X_comm()` functions stay in C for now (P10-P13 ports
  them). `init_race()` in Rust will call the C per-race init functions
  through FFI until each race is ported.

### Test plan

**Unit tests** (in `dispatch.rs`):
- `race_communication` picks correct conversation for each encounter type
- `init_communication` maps conversation IDs to ship types correctly
- `init_race` dispatches to correct per-race init
- `hail_alien` loads resources and starts dialogue

**Automation proof** (`scripts/comm-encounter-v1.json`):
- Start new game (tap select on NewGame)
- Wait for game to load (assert_activity mask=0x00FF equals=0x0003 = IN_HYPERSPACE)
- Wait several ticks for encounter
- Assert IN_ENCOUNTER (mask=0x00FF equals=0x0002)
- Capture the comm screen
- Finish

### Dependencies
- Existing Rust `comm/` module (11K lines, infrastructure complete)
- Existing Rust `comm/ffi.rs` (178 FFI functions already wrapped)
- C `init_race` per-race functions (temporary FFI until P10-P13)

### Files to create/modify
- CREATE: `rust/src/comm/dispatch.rs`
- MODIFY: `rust/src/comm/mod.rs` (add `pub mod dispatch`)
- MODIFY: `rust/src/mainloop/game_loop.rs` (call Rust dispatch)
- MODIFY: `rust/src/mainloop/c_extern.rs` (keep FFI for unported targets)
- CREATE: `rust/scripts/comm-encounter-v1.json`