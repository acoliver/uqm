# PLAN-20260724-MAINLOOP-AND-COMM (Revised)

## Core Problem: Dual Data Ownership

The bridge surface is **712 Rust→C exports, 542 C→Rust imports, 4,537 lines of C bridge files**. But the real risk isn't the function count — it's that **C and Rust each own copies of the same data**, and the bridge manually synchronizes them.

### The three dual-ownership problems

**1. GlobData (767+767 fields)**
- C owns `GLOBDATA GlobData` with `Game_state` (767 fields) and `SIS_state` (15 fields)
- `GLOBAL(f)` = `GlobData.Game_state.f` — reads directly from C memory
- `GLOBAL_SIS(f)` = `GlobData.SIS_state.f` — reads directly from C memory
- Rust has a separate `GameState` (155 bytes, just bit-packed flags) — NOT synced to C's copy
- C's `getGameState()` calls Rust's `rust_get_game_state_bits_from_bytes()` for bit extraction, but the byte array is in C memory
- Rust accesses `CurrentActivity` etc. through 6 FFI functions in `c_extern.rs`

**2. CommData (LOCDATA)**
- C owns `LOCDATA CommData` (global in comm.c:72)
- Rust has its own `CommData` struct in `comm/types.rs` (25+ fields)
- `rust_comm.c` (2156 lines, 189 functions) has 32 LOCDATA field accessors that marshal between them
- Two structs, manually synchronized, high desync risk

**3. Activity flags**
- C owns `CurrentActivity`/`LastActivity`/`NextActivity` as fields of `GlobData.Game_state`
- Rust accesses through FFI getters/setters in `c_extern.rs`
- Rust has `ActivityValue`/`activity_flags` types but they're shadows of C state

### The correct approach: consolidate ownership first, then port logic

Instead of porting C logic to Rust while still reading C globals through FFI (which just adds more bridge surface), we:

1. **Make Rust the single owner of game state** — C reads through Rust FFI
2. **Make Rust the single owner of CommData** — eliminate LOCDATA dual copy
3. **Port dispatch logic natively** — now it reads from Rust-owned state, no FFI for data
4. **Port per-race dialogue state machines** — populate Rust CommData directly
5. **Port remaining dispatch targets** — same pattern

## Phase structure

| Phase | Worker | Verifier | Scope |
|---|---|---|---|
| P09 | Consolidate game state ownership to Rust | P09a | Move GlobData to Rust, make C read through FFI |
| P10 | Consolidate CommData ownership to Rust | P10a | Make Rust CommData the single source, eliminate LOCDATA accessors |
| P11 | Port RaceCommunication + InitCommunication to Rust | P11a | Native dispatch, no FFI for data access |
| P12 | Port Arilou dialogue state machine to Rust | P12a | Reference race port, populates Rust CommData directly |
| P13 | Port remaining race dialogues (batch 1: 6 races) | P13a | |
| P14 | Port remaining race dialogues (batch 2: 6 races) | P14a | |
| P15 | Port remaining race dialogues (batch 3: remaining) | P15a | |
| P16 | Port ExploreSolarSys to Rust | P16a | Planet exploration dispatch |
| P17 | Port VisitStarBase to Rust | P17a | Starbase dispatch |
| P18 | Port InstallBombAtEarth + hyperspace to Rust | P18a | Hyperspace dispatch |
| P19 | Wire Battle dispatch to Rust battle module | P19a | Combat dispatch |
| P20 | Automation proof scripts for all dispatch targets | P20a | Real-binary proofs |
| P21 | Final verification: all proofs pass, all gates green | P21a | Acceptance |

## Testing strategy

### Unit tests
- Game state: round-trip tests (C writes → Rust reads → C reads → same value)
- CommData: field-by-field equivalence after consolidation
- Dispatch logic: mock-based tests for each dispatch target
- Race dialogue: state transition tests for each race

### Automation proof scripts (real binary)
Each proof runs against the real binary with `SDL_VIDEODRIVER=dummy`:
1. **state-sync-v1.json**: Start game, assert activity flags read correctly, verify state sync
2. **comm-encounter-v1.json**: Reach encounter, assert IN_ENCOUNTER, capture
3. **explore-planet-v1.json**: Navigate to planet, assert IN_INTERPLANETARY, capture
4. **starbase-visit-v1.json**: Navigate to starbase, assert IN_STARBASE, capture
5. **battle-v1.json**: Encounter hostile, choose attack, assert IN_BATTLE, capture

Each proof uses `assert_activity` with activity flag masks, captures PNG screenshots,
writes trace.jsonl + teardown receipts, and must exit 0.

### Activity flag reference (from globdata.h)
```
SUPER_MELEE = 0         (main menu)
IN_LAST_BATTLE = 1
IN_ENCOUNTER = 2
IN_HYPERSPACE = 3
IN_INTERPLANETARY = 4
WON_LAST_BATTLE = 5
IN_QUASISPACE = 6
IN_PLANET_ORBIT = 7
IN_STARBASE = 8

Flags (high byte):
CHECK_PAUSE = 0x0100
IN_BATTLE = 0x0200
START_ENCOUNTER = 0x0400
START_INTERPLANETARY = 0x0800
CHECK_LOAD = 0x1000
CHECK_RESTART = 0x2000
CHECK_ABORT = 0x4000
```

### Verification gates (each phase)
- `cargo fmt --all --check`
- `cargo clippy --workspace --all-targets --features audio_heart -- -D warnings`
- `cargo test --lib --features audio_heart -- --test-threads=1`
- `cargo build --bin uqm --release --features audio_heart,linked_c_archive`
- Relevant automation proof script passes against real binary