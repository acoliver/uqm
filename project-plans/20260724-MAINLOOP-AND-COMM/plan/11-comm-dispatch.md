# P11: Port RaceCommunication + InitCommunication to Rust

## Worker scope

Port the comm dispatch entry points from C `comm.c` to Rust, now that Rust
owns the game state (P09) and CommData (P10). The dispatch logic reads from
Rust-owned state natively, without FFI for data access.

### What to port

1. **`RaceCommunication()`** (comm.c:1503-1600, ~100 lines)
   - Determines which alien race to talk to based on encounter state
   - Calls `InitCommunication(conversation_id)` with the selected race
   - Handles special cases: Talking Pet, Spathi, Ilwrath, Chmmr, Arilou
   - Reads: `GLOBAL(CurrentActivity)`, `GET_GAME_STATE(...)`, `GLOBAL_SIS(CrewEnlisted)`, `npc_built_ship_q`
   - All of these are now Rust-owned (from P09)

2. **`InitCommunication(which_comm)`** (comm.c:1359-1470, ~110 lines)
   - Maps conversation ID → ship type
   - Calls `init_race(comm_id)` to get LOCDATA → now populates Rust CommData (P10)
   - Copies LOCDATA to CommData → no copy needed, already Rust-owned
   - Calls `InitEncounter()` or sets BATTLE_SEGUE
   - If HAIL: calls `HailAlien()` (already Rust: `rust_HailAlien()` / `hail::hail_alien()`)

3. **`init_race(comm_id)` dispatch table** (commglue.c:367-415, ~50 lines)
   - Switch statement mapping conversation IDs to per-race init functions
   - Each returns a `LOCDATA*` → now populates Rust `CommData` through FFI
   - Per-race functions stay in C until P12-P15 ports them

### Approach

- Create `rust/src/comm/dispatch.rs`:
  - `race_communication()` — reads from Rust-owned game state, no FFI for data
  - `init_communication(which_comm: u32) -> u32` — same
  - `init_race(comm_id: u32) -> Option<CommData>` — calls C per-race init through FFI (temporary)

- Update `game_loop.rs` `CffiOps::race_communication()` to call
  `comm::dispatch::race_communication()` instead of C FFI

- The key insight: after P09+P10, the dispatch logic can read `CurrentActivity`,
  `GET_GAME_STATE`, `GLOBAL_SIS(CrewEnlisted)` etc. from Rust state directly.
  No FFI needed for data reads — only for calling the C per-race init functions
  (which will be eliminated in P12-P15).

### Test plan

**Unit tests** (in `dispatch.rs`):
- `race_communication` picks correct conversation for each encounter type
- `init_communication` maps conversation IDs to ship types correctly
- `init_race` dispatches to correct per-race init
- Activity flags read from Rust state, not C

**Automation proof** (`scripts/comm-encounter-v1.json`):
- Start new game (tap select on NewGame)
- Wait for game to load (assert_activity mask=0x00FF equals=0x0003 = IN_HYPERSPACE)
- Wait several ticks for encounter
- Assert IN_ENCOUNTER (mask=0x00FF equals=0x0002)
- Capture the comm screen
- Finish

### Dependencies
- P09 (game state ownership)
- P10 (CommData ownership)

### Files to create/modify
- CREATE: `rust/src/comm/dispatch.rs`
- MODIFY: `rust/src/comm/mod.rs` (add `pub mod dispatch`)
- MODIFY: `rust/src/mainloop/game_loop.rs` (call Rust dispatch)
- MODIFY: `rust/src/mainloop/c_extern.rs` (remove RaceCommunication FFI)
- CREATE: `rust/scripts/comm-encounter-v1.json`