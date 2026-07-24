# PLAN-20260724-MAINLOOP-AND-COMM

## Goal

Port the remaining C game-loop dispatch targets and the per-race dialogue
state machines to Rust, with test coverage and real-binary automation proofs
that exercise both paths against the live UQM binary.

## Two workstreams

### A. Game-Loop Activity Dispatch (starcon.c → Rust)

The Rust `game_loop.rs` already owns the two-level loop (outer: new/load
game, inner: activity state machine). What remains is porting the 5 C
dispatch targets it calls through FFI:

| C function | C file | LoC | What it does |
|---|---|---|---|
| `RaceCommunication()` | comm.c:1503 | ~100 | Picks which alien to talk to, calls `InitCommunication` |
| `InitCommunication(conv)` | comm.c:1359 | ~100 | Maps conversation→ship, calls `init_race`, starts encounter |
| `ExploreSolarSys()` | planets.c | 483 | Interplanetary exploration dispatch |
| `VisitStarBase()` | starbase.c:431 | 602 | Starbase visit (outfit, build, talk) |
| `InstallBombAtEarth()` | hyper.c | 1747 | Hyperspace navigation + bomb installation |
| `Battle(&callback)` | battle.c:397 | 517 | Combat dispatch (Rust `battle/` module already exists) |

**Strategy**: Port in order of dependency and testability:
1. `RaceCommunication` + `InitCommunication` — depends on comm infrastructure
   (already 11K lines of Rust) and `init_race` dispatch table
2. `ExploreSolarSys` — depends on planets module (not yet ported)
3. `VisitStarBase` — depends on starbase UI
4. `InstallBombAtEarth` — hyperspace navigation (largest, most complex)
5. `Battle` — Rust `battle/` module already exists, just needs wiring

### B. Per-Race Dialogue State Machines (comm/*/Xc.c → Rust)

18 race dialogue files (~14,437 lines total C) contain:
- A `LOCDATA` struct (animation config, colors, fonts, music — all resource
  keys, NOT embedded text)
- An `init_X_comm()` function (sets function pointers + segue mode)
- Dialogue state machine functions (branching logic using `NPCPhrase(index)`
  to speak text loaded from resource files)
- Response handling (player picks response → next state machine function)

**Text is already externalized**: All dialogue text lives in `.rmp`/`.pkg`
content files, loaded by string index at runtime. The C files contain only
the branching logic and resource key references.

**Strategy**: Port race-by-race using a table-driven approach:
1. Define a Rust `RaceDialogue` trait with `init()`, `intro()`, `respond()`
2. Port `init_race()` dispatch table to Rust
3. Port one race (Arilou) as the reference implementation
4. Port remaining races in batches
5. Each race: translate the C state machine to Rust match arms, keeping the
   same string indices and resource keys

## Testing strategy

### Unit tests
- Each ported dispatch target gets unit tests with mock `GameLoopOps`
- Each race dialogue gets unit tests for state transitions
- `assert_activity` automation action tests activity flags

### Automation proof scripts (real binary)
We already have the automation system working (main-menu-v1, watchdog-v1,
inactive-smoke, hard-hang proofs all pass). We'll add:

1. **comm-encounter-v1.json**: Start new game, wait for hyperspace, navigate
   to an encounter, assert `IN_ENCOUNTER` activity, capture the comm screen,
   tap through dialogue responses, finish. This exercises the comm dispatch
   path end-to-end.

2. **explore-planet-v1.json**: Start new game, wait for hyperspace, navigate
   to a planet, assert `IN_INTERPLANETARY` activity, capture, finish. This
   exercises the ExploreSolarSys dispatch.

3. **starbase-visit-v1.json**: Start new game, navigate to starbase, assert
   `IN_STARBASE` activity, capture, finish.

4. **battle-v1.json**: Start new game, encounter hostile alien, choose
   attack, assert `IN_BATTLE` activity, capture, finish.

Each proof:
- Runs against the real binary with `SDL_VIDEODRIVER=dummy`
- Uses `assert_activity` with activity flag masks to verify game state
- Captures PNG screenshots at key transitions
- Writes trace.jsonl + teardown-complete.json
- Must exit 0 with correct terminal class

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

## Phase structure

| Phase | Worker | Verifier | Scope |
|---|---|---|---|
| P09 | Port `RaceCommunication` + `InitCommunication` to Rust | P09a | comm dispatch, init_race table |
| P10 | Port Arilou dialogue state machine | P10a | Reference race port |
| P11 | Port remaining race dialogues (batch 1: 6 races) | P11a | Batch porting |
| P12 | Port remaining race dialogues (batch 2: 6 races) | P12a | Batch porting |
| P13 | Port remaining race dialogues (batch 3: 5 races) | P13a | Batch porting |
| P14 | Port `ExploreSolarSys` to Rust | P14a | Planet exploration dispatch |
| P15 | Port `VisitStarBase` to Rust | P15a | Starbase dispatch |
| P16 | Port `InstallBombAtEarth` + hyperspace to Rust | P16a | Hyperspace dispatch |
| P17 | Wire `Battle` dispatch to Rust battle module | P17a | Combat dispatch |
| P18 | Automation proof scripts for all dispatch targets | P18a | Real-binary proofs |
| P19 | Final verification: all proofs pass, all gates green | P19a | Acceptance |

## Verification gates (each phase)
- `cargo fmt --all --check`
- `cargo clippy --workspace --all-targets --features audio_heart -- -D warnings`
- `cargo test --lib --features audio_heart -- --test-threads=1`
- `cargo build --bin uqm --release --features audio_heart,linked_c_archive`
- Relevant automation proof script passes against real binary