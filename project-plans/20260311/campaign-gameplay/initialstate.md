# Campaign Gameplay Subsystem Current State

## Scope and boundary verified from code

The campaign-gameplay subsystem is the single-player campaign runtime/orchestration layer that selects and advances the player through the major campaign activities and persists that runtime state. The boundary is evidenced by the central activity state machine in `/Users/acoliver/projects/uqm/sc2/src/uqm/starcon.c`, which initializes a campaign run, starts the game clock and initial events, then dispatches among encounter/starbase flow, interplanetary flow, and hyperspace flow based on `CurrentActivity` and activity flags such as `START_ENCOUNTER` and `START_INTERPLANETARY` (`/Users/acoliver/projects/uqm/sc2/src/uqm/starcon.c:210-307`).

The activity vocabulary used by that orchestration is defined in `GAME_STATE.CurrentActivity` and associated flags in `/Users/acoliver/projects/uqm/sc2/src/uqm/globdata.h:894-962`, especially `IN_LAST_BATTLE`, `IN_ENCOUNTER`, `IN_HYPERSPACE`, `IN_INTERPLANETARY`, `START_ENCOUNTER`, `START_INTERPLANETARY`, `CHECK_LOAD`, `CHECK_RESTART`, and `CHECK_ABORT` (`/Users/acoliver/projects/uqm/sc2/src/uqm/globdata.h:894-917`). That same state struct also contains the campaign clock, navigation position, encounter queues, NPC ship queue, escort queue, and game-state bitfield, which shows that these are runtime campaign concerns today rather than separate orchestration objects (`/Users/acoliver/projects/uqm/sc2/src/uqm/globdata.h:920-962`).

Based on code, this subsystem includes:

- new-game / load-game entry and restart selection (`/Users/acoliver/projects/uqm/sc2/src/uqm/restart.c:340-411`)
- top-level campaign loop and activity dispatch (`/Users/acoliver/projects/uqm/sc2/src/uqm/starcon.c:210-307`)
- hyperspace runtime transitions into encounters and interplanetary (`/Users/acoliver/projects/uqm/sc2/src/uqm/hyper.c:412-565`)
- encounter/battle handoff and post-battle cleanup (`/Users/acoliver/projects/uqm/sc2/src/uqm/encount.c:100-189`, `/Users/acoliver/projects/uqm/sc2/src/uqm/encount.c:476-845`)
- starbase visit flow as a campaign-level branch (`/Users/acoliver/projects/uqm/sc2/src/uqm/starbase.c:430-545`)
- scripted campaign event scheduling/progression tied to the game clock (`/Users/acoliver/projects/uqm/sc2/src/uqm/gameev.c:41-252`, `/Users/acoliver/projects/uqm/sc2/src/uqm/clock.c:91-140`, `/Users/acoliver/projects/uqm/sc2/src/uqm/clock.c:176-299`)
- save/load of campaign runtime state, including activity, clock, encounters, group data, SIS state, and summary (`/Users/acoliver/projects/uqm/sc2/src/uqm/save.c:330-371`, `/Users/acoliver/projects/uqm/sc2/src/uqm/save.c:435-493`, `/Users/acoliver/projects/uqm/sc2/src/uqm/save.c:733-791`, `/Users/acoliver/projects/uqm/sc2/src/uqm/load.c:298-359`, `/Users/acoliver/projects/uqm/sc2/src/uqm/load.c:608-771`)

Based on code, this subsystem does **not** own lower-level domain behavior already assigned elsewhere in the suite:

- solar-system exploration/orbit/scan/surface execution is invoked from campaign orchestration but implemented in the planets/solar-system code; `starcon.c` only routes to `ExploreSolarSys()` when `START_INTERPLANETARY` is set (`/Users/acoliver/projects/uqm/sc2/src/uqm/starcon.c:270-277`)
- dialogue behavior is not owned here; campaign gameplay calls `RaceCommunication()` or `InitCommunication()`/`InitCommunication(COMMANDER_CONVERSATION)` as orchestration edges into the comm subsystem (`/Users/acoliver/projects/uqm/sc2/src/uqm/starcon.c:243-261`, `/Users/acoliver/projects/uqm/sc2/src/uqm/starbase.c:356-364`, `/Users/acoliver/projects/uqm/sc2/src/uqm/starbase.c:451-483`, `/Users/acoliver/projects/uqm/sc2/src/uqm/starbase.c:497-503`)
- per-ship combat runtime is not owned here; campaign gameplay prepares queues and calls `Battle()` or `BuildBattle()` but does not implement combat internals (`/Users/acoliver/projects/uqm/sc2/src/uqm/starcon.c:281-285`, `/Users/acoliver/projects/uqm/sc2/src/uqm/encount.c:100-189`, `/Users/acoliver/projects/uqm/sc2/src/uqm/encount.c:775-841`)
- supermelee menuing/runtime is explicitly a separate branch out of restart/start flow (`/Users/acoliver/projects/uqm/sc2/src/uqm/restart.c:352-358`)

## Top-level runtime ownership

### Entry, restart, and new/load branching

`StartGame()` loops until a valid start is selected, optionally plays the introduction for a fresh campaign, then returns control to the main campaign runtime (`/Users/acoliver/projects/uqm/sc2/src/uqm/restart.c:373-411`). `DoRestart()` marks the menu decision into activity state: load sets `LastActivity = CHECK_LOAD` and `CurrentActivity = IN_INTERPLANETARY`, while new game sets `LastActivity = CHECK_LOAD | CHECK_RESTART` and `CurrentActivity = IN_INTERPLANETARY` (`/Users/acoliver/projects/uqm/sc2/src/uqm/restart.c:146-153`). `TryStartGame()` keeps spinning restart flow until the user starts/loads a game, times out, quits, or detours into supermelee (`/Users/acoliver/projects/uqm/sc2/src/uqm/restart.c:340-370`).

This means the campaign subsystem currently uses menu-selected activity flags rather than a dedicated campaign session object to distinguish new game from load game.

### Main campaign loop

The campaign runtime in `starcon.c` is the highest-level orchestrator in scope. After `StartGame()`, it sets player input, initializes game structures, initializes the game clock, and registers initial game events (`/Users/acoliver/projects/uqm/sc2/src/uqm/starcon.c:210-220`). Then, inside the main loop, it:

- restores `CurrentActivity` from `NextActivity` on fake loads (`/Users/acoliver/projects/uqm/sc2/src/uqm/starcon.c:237-241`)
- dispatches to starbase or race communication if an encounter should start (`/Users/acoliver/projects/uqm/sc2/src/uqm/starcon.c:243-261`)
- dispatches to interplanetary solar-system flow when `START_INTERPLANETARY` is set (`/Users/acoliver/projects/uqm/sc2/src/uqm/starcon.c:270-277`)
- otherwise enters hyperspace/quasispace runtime by setting `IN_HYPERSPACE`, adjusting clock rate, and calling `Battle(&on_battle_frame)` (`/Users/acoliver/projects/uqm/sc2/src/uqm/starcon.c:278-285`)
- handles terminal outcomes such as winning the last battle or player death, then tears down clock and structures when the run ends (`/Users/acoliver/projects/uqm/sc2/src/uqm/starcon.c:290-309`)

That is the clearest evidence that campaign-gameplay today is a C-owned activity/flag-based orchestrator rather than a separate engine subsystem with explicit typed transitions.

## Activity transitions and orchestration behavior

### Hyperspace and interplanetary transitions

`hyper.c` owns the campaign-facing transition logic out of hyperspace. `cleanup_hyperspace()` reorders the collided encounter to the head of the encounter queue so later communication flow uses the correct encounter object (`/Users/acoliver/projects/uqm/sc2/src/uqm/hyper.c:412-442`). `unhyper_transition()` then clears `IN_BATTLE` and selects one of three campaign transitions:

- random encounter transition: saves SIS hyperspace state and sets `START_ENCOUNTER` (`/Users/acoliver/projects/uqm/sc2/src/uqm/hyper.c:546-554`)
- interplanetary transition: sets up system-entry state via `InterplanetaryTransition()` (`/Users/acoliver/projects/uqm/sc2/src/uqm/hyper.c:555-557`)
- Arilou/quasispace transition: uses `ArilouSpaceTransition()` (`/Users/acoliver/projects/uqm/sc2/src/uqm/hyper.c:558-560`)

`InterplanetaryTransition()` itself is campaign orchestration, not planet-exploration logic: it clears orbit context, resets broadcaster state, and either sets `START_INTERPLANETARY` for normal solar-system entry or routes to an encounter when emerging into Arilou homeworld space (`/Users/acoliver/projects/uqm/sc2/src/uqm/hyper.c:452-493`).

The hyperspace menu is also campaign-facing orchestration rather than pure UI. `DoHyperspaceMenu()` can trigger device usage, cargo, roster, save/load game options, starmap, or return to navigation, and it exits immediately when a selected submenu causes `START_ENCOUNTER` or other campaign control flags (`/Users/acoliver/projects/uqm/sc2/src/uqm/hyper.c:1623-1683`). `HyperspaceMenu()` cleans up hyperspace state on exit if not still in battle (`/Users/acoliver/projects/uqm/sc2/src/uqm/hyper.c:1685-1725`).

### Encounter handoff and battle segue

`encount.c` shows campaign gameplay owning the higher-level encounter lifecycle while battle runtime remains elsewhere.

`BuildBattle()` converts campaign ship fragment queues into combat `race_q` state, chooses the battle backdrop based on current activity (`IN_LAST_BATTLE`, `IN_HYPERSPACE`, or planetary battle), and injects the SIS ship for the RPG player (`/Users/acoliver/projects/uqm/sc2/src/uqm/encount.c:100-189`).

`EncounterBattle()` sets `BATTLE_SEGUE`, swaps `CurrentActivity` to `IN_ENCOUNTER` or `IN_LAST_BATTLE`, seeds battle counters, runs `Battle(NULL)`, and restores the previous activity afterward (`/Users/acoliver/projects/uqm/sc2/src/uqm/encount.c:775-841`).

`UninitEncounter()` performs campaign consequences after combat. It suppresses post-processing in abort/load/death/final-battle cases (`/Users/acoliver/projects/uqm/sc2/src/uqm/encount.c:482-488`), otherwise clears `BATTLE_SEGUE` and `BOMB_CARRIER`, computes victory state from battle counters and story state, identifies the encountered race from `npc_built_ship_q`, and drives reward/salvage cleanup by removing dead ships from the relevant campaign queues (`/Users/acoliver/projects/uqm/sc2/src/uqm/encount.c:495-717`). This is evidence that campaign gameplay owns post-encounter progression and resource effects, not just transition dispatch.

### Starbase visit flow

`VisitStarBase()` is clearly campaign orchestration. It handles the special Chmmr-bomb transport case by forcing solar-system reload state and marking the player as in starbase via `GLOBAL_FLAGS_AND_DATA = ~0` (`/Users/acoliver/projects/uqm/sc2/src/uqm/starbase.c:437-444`).

Before the starbase is allied, it runs the Commander conversation, conditionally stages the Ilwrath response battle by cloning an Ilwrath ship into `npc_built_ship_q`, then returns to Commander conversation after that battle (`/Users/acoliver/projects/uqm/sc2/src/uqm/starbase.c:446-483`). When the starbase first becomes available or after bomb installation, it advances time by 14 days and forces a Commander conversation before normal menu flow (`/Users/acoliver/projects/uqm/sc2/src/uqm/starbase.c:488-503`, `/Users/acoliver/projects/uqm/sc2/src/uqm/starbase.c:423-428`).

`DoStarBase()` is the starbase activity loop. It exits on load/abort, runs the Commander/outfit/shipyard choices, and clears `STARBASE_VISITED` on normal departure (`/Users/acoliver/projects/uqm/sc2/src/uqm/starbase.c:294-420`). `CleanupAfterStarBase()` then clears the starbase marker and fakes a load into interplanetary by setting `CurrentActivity = CHECK_LOAD` and `NextActivity = IN_INTERPLANETARY | START_INTERPLANETARY` (`/Users/acoliver/projects/uqm/sc2/src/uqm/starbase.c:517-529`).

This starbase return path is a notable current-state pattern: starbase does not transition directly back to its caller; it relies on the global fake-load handoff consumed later by `starcon.c` (`/Users/acoliver/projects/uqm/sc2/src/uqm/starcon.c:237-241`).

In summary, the current implementation treats starbase as observationally a distinct campaign mode — with its own activity loop, forced-conversation gating, departure mechanism, and save/load resume path — even though it does not have a dedicated top-level activity selector. The evidence for how this is achieved spans three separate mechanisms: (a) dispatch routing goes through the encounter-family path, with `START_ENCOUNTER` routing in `starcon.c` reaching the starbase branch (`/Users/acoliver/projects/uqm/sc2/src/uqm/starcon.c:243-261`); (b) starbase-specific gating and marker behavior uses the `GLOBAL_FLAGS_AND_DATA` bitfield in the starbase code to distinguish the starbase context (`/Users/acoliver/projects/uqm/sc2/src/uqm/starbase.c:437-444`); and (c) save/load resume is evidenced by the starbase activity loop's load/abort exit path in `DoStarBase()` and the fake-load/interplanetary handoff in `CleanupAfterStarBase()` (`/Users/acoliver/projects/uqm/sc2/src/uqm/starbase.c:294-420`, `/Users/acoliver/projects/uqm/sc2/src/uqm/starbase.c:517-529`).

## Event progression and campaign clock

`AddInitialGameEvents()` registers the initial recurring hyperspace encounter event and major campaign timers such as Arilou entrance, Kohr-Ah victory, and Slylandro ramp-up (`/Users/acoliver/projects/uqm/sc2/src/uqm/gameev.c:41-48`). `EventHandler()` is the campaign event switchboard: it updates story flags, fleet destinations/strength, alliance state, genocides, distress calls, shield events, and other strategic-layer changes (`/Users/acoliver/projects/uqm/sc2/src/uqm/gameev.c:50-252`).

The game clock is embedded directly inside `GAME_STATE` (`/Users/acoliver/projects/uqm/sc2/src/uqm/globdata.h:930-933`). In the C implementation, `InitGameClock()` initializes the event queue and seeds the start date to Feb 17 of `START_YEAR` (`/Users/acoliver/projects/uqm/sc2/src/uqm/clock.c:117-130`). `SetGameClockRate()` derives ticks per day from a fixed 24 fps base (`/Users/acoliver/projects/uqm/sc2/src/uqm/clock.c:176-191`). `GameClockTick()` decrements ticks, rolls to the next day, and runs due events via `processClockDayEvents()` (`/Users/acoliver/projects/uqm/sc2/src/uqm/clock.c:283-299`).

`starcon.c` changes the game clock rate according to campaign mode: `INTERPLANETARY_CLOCK_RATE` before `ExploreSolarSys()` and `HYPERSPACE_CLOCK_RATE` before the hyperspace battle-loop runtime (`/Users/acoliver/projects/uqm/sc2/src/uqm/starcon.c:274-285`). `starbase.c` uses `MoveGameClockDays(14)` to advance campaign time during starbase story beats (`/Users/acoliver/projects/uqm/sc2/src/uqm/starbase.c:423-428`).

## Save/load ownership

### What campaign state is serialized

`SaveGameState()` writes campaign-global state including `CurrentActivity`, the embedded `GameClock`, autopilot target, interplanetary location, ship stamp/orientation, orbit flags, velocity, and the `GameState` bitfield (`/Users/acoliver/projects/uqm/sc2/src/uqm/save.c:330-371`). `PrepareSummary()` derives the user-facing save summary from current campaign state, including activity remapping for quasispace, starbase, planet orbit, and Sa-Matra (`/Users/acoliver/projects/uqm/sc2/src/uqm/save.c:435-493`).

`SaveGame()` then writes the summary, full game state, escort queue, NPC queue / battle-group files as applicable, and hyperspace encounter queue (`/Users/acoliver/projects/uqm/sc2/src/uqm/save.c:733-791`). The comments and branching around `START_INTERPLANETARY` show campaign-specific save semantics for homeworld encounter screens and interplanetary re-entry (`/Users/acoliver/projects/uqm/sc2/src/uqm/save.c:747-778`).

`LoadGameState()` restores the same campaign fields, including `CurrentActivity`, `GameClock`, navigation/orbit/velocity fields, and the `GameState` bitfield (`/Users/acoliver/projects/uqm/sc2/src/uqm/load.c:298-359`). `LoadGame()` clears and rebuilds the campaign queues, loads state chunks, restores `NextActivity` from the loaded `CurrentActivity`, and normalizes interplanetary resumes by forcing `START_INTERPLANETARY` when needed (`/Users/acoliver/projects/uqm/sc2/src/uqm/load.c:643-771`).

### State-file dependencies

Campaign save/load is not just one monolithic save blob. `save.c` and `load.c` also depend on state-file helpers for star scan info and battle-group persistence via `OpenStateFile`, `LengthStateFile`, `LoadBattleGroup`, and `SaveBattleGroup` (`/Users/acoliver/projects/uqm/sc2/src/uqm/save.c:595-721`, `/Users/acoliver/projects/uqm/sc2/src/uqm/load.c:453-549`). That matters for Rust-edge verification below, because the campaign save/load boundary indirectly depends on the state subsystem implementation.

## C ownership vs Rust edges

### Campaign-gameplay logic itself is still C-owned

There is no dedicated Rust campaign-gameplay module, no Rust campaign activity state machine, and no campaign-specific Rust toggle in the files inspected.

Evidence:

- Rust library exports modules such as `comm`, `game_init`, `state`, `time`, etc., but no campaign-gameplay module (`/Users/acoliver/projects/uqm/rust/src/lib.rs:1-19`)
- `rust/src/game_init/ffi.rs` exposes initialization helpers such as `rust_init_space`, `rust_init_ships`, `rust_init_game_kernel`, and master-ship-list helpers, but nothing for campaign runtime/orchestration (`/Users/acoliver/projects/uqm/rust/src/game_init/ffi.rs:1-119`)
- top-level campaign runtime remains implemented in C in `starcon.c`, `restart.c`, `hyper.c`, `encount.c`, `starbase.c`, `save.c`, `load.c`, and `gameev.c` (citations above)
- build configuration defines toggles for Rust bridge, clock, state, comm, input, etc., but no campaign-gameplay-specific toggle or symbol (`/Users/acoliver/projects/uqm/sc2/build/unix/build.config:86-107`, `/Users/acoliver/projects/uqm/sc2/build/unix/build.config:656-737`)

Current-state conclusion from the evidence above: **campaign-gameplay has no dedicated Rust port/toggle/module today; its orchestration logic is C-owned.**

### Live Rust edge: clock can be Rust-backed

There is an indirect live Rust edge at the campaign boundary through the game clock.

`build.config` defines `USE_RUST_CLOCK` and, when the Rust bridge is enabled, turns it on together with many other Rust toggles (`/Users/acoliver/projects/uqm/sc2/build/unix/build.config:86-107`, `/Users/acoliver/projects/uqm/sc2/build/unix/build.config` lines around the rust bridge enable action already read in the larger file, specifically the `-DUSE_RUST_CLOCK` enable path in the rust-bridge action shown in the file content from `/Users/acoliver/projects/uqm/sc2/build/unix/build.config:613-739`). The default menu choice for `rust_bridge` is `enabled` (`/Users/acoliver/projects/uqm/sc2/build/unix/build.config` in the rust bridge menu section; visible in the full file and paired with the export section at `/Users/acoliver/projects/uqm/sc2/build/unix/build.config:656-737`).

At compile time, `clock.c` refuses to build when `USE_RUST_CLOCK` is enabled (`/Users/acoliver/projects/uqm/sc2/src/uqm/clock.c:19-20`). `clock_rust.c` provides the replacement C ABI wrapper and forwards `InitGameClock`, `UninitGameClock`, `SetGameClockRate`, `GameClockTick`, `MoveGameClockDays`, `LockGameClock`, `UnlockGameClock`, and `GameClockRunning` to Rust functions (`/Users/acoliver/projects/uqm/sc2/src/uqm/clock_rust.c:150-227`).

Because campaign gameplay calls those clock functions directly in `starcon.c` and `starbase.c` (`/Users/acoliver/projects/uqm/sc2/src/uqm/starcon.c:217-220`, `/Users/acoliver/projects/uqm/sc2/src/uqm/starcon.c:274-285`, `/Users/acoliver/projects/uqm/sc2/src/uqm/starbase.c:423-428`), the campaign subsystem has a real indirect Rust dependency through the clock when `USE_RUST_CLOCK` is active.

### Live Rust edge: game-state bits and state files can be Rust-backed

There is also an indirect live Rust edge through state management.

`globdata.c` conditionally routes `getGameState`, `setGameState`, `getGameState32`, `setGameState32`, and `copyGameState` to Rust byte-buffer helpers when `USE_RUST_STATE` is enabled (`/Users/acoliver/projects/uqm/sc2/src/uqm/globdata.c:50-100`). Since campaign gameplay heavily uses `GET_GAME_STATE` / `SET_GAME_STATE` in the orchestrator, encounter, starbase, hyperspace, save/load, and event code, campaign behavior can run against Rust-backed bitfield helpers even though the orchestrating functions remain in C.

`state.c` similarly conditionally routes `OpenStateFile`, `CloseStateFile`, `DeleteStateFile`, `LengthStateFile`, `ReadStateFile`, and `WriteStateFile` to Rust FFI (`/Users/acoliver/projects/uqm/sc2/src/uqm/state.c:53-119`). Save/load code uses those helpers for campaign-adjacent state-file chunks (`/Users/acoliver/projects/uqm/sc2/src/uqm/save.c:595-721`, `/Users/acoliver/projects/uqm/sc2/src/uqm/load.c:453-549`).

On the Rust side, `rust/src/state/ffi.rs` exports the matching game-state-bit and state-file functions, including `rust_get_game_state_bits_from_bytes`, `rust_set_game_state_bits_in_bytes`, `rust_get_game_state32_from_bytes`, `rust_set_game_state32_in_bytes`, `rust_copy_game_state_bits_between_bytes`, `rust_open_state_file`, `rust_close_state_file`, `rust_length_state_file`, `rust_read_state_file`, `rust_write_state_file`, and `rust_seek_state_file` (`/Users/acoliver/projects/uqm/rust/src/state/ffi.rs:24-188`, `/Users/acoliver/projects/uqm/rust/src/state/ffi.rs:225-303`).

So campaign-gameplay is not ported to Rust, but it can execute atop Rust-backed state/serialization primitives.

## Hybrid areas and current-state limitations evidenced in code

1. **Orchestration is distributed across many globals and fake-load transitions.**
   The subsystem uses global mutable state (`CurrentActivity`, `NextActivity`, `LastActivity`, `GameClock`, multiple queues, and `GameState`) rather than a cohesive campaign runtime object (`/Users/acoliver/projects/uqm/sc2/src/uqm/globdata.h:920-962`, `/Users/acoliver/projects/uqm/sc2/src/uqm/load.c:38`, `/Users/acoliver/projects/uqm/sc2/src/uqm/starcon.c:237-241`). Starbase exit explicitly "fake[s] a load" to return to interplanetary (`/Users/acoliver/projects/uqm/sc2/src/uqm/starbase.c:524-527`), and load/resume paths also depend on this pattern (`/Users/acoliver/projects/uqm/sc2/src/uqm/starcon.c:237-241`).

2. **Starbase flow is acknowledged in code as hacky.**
   `DoStarBase()` begins with the comment `// XXX: This function is full of hacks and otherwise strange code` (`/Users/acoliver/projects/uqm/sc2/src/uqm/starbase.c:294-297`). Additional comments show that `InitCommunication()` clears flags that starbase logic then has to restore manually (`/Users/acoliver/projects/uqm/sc2/src/uqm/starbase.c:361-364`, `/Users/acoliver/projects/uqm/sc2/src/uqm/starbase.c:481-483`, `/Users/acoliver/projects/uqm/sc2/src/uqm/starbase.c:501-503`).

3. **Subsystem responsibilities are intertwined with lower-level rendering/UI loops.**
   The campaign orchestrator and starbase/encounter flows directly manage graphics contexts, music, fades, and input loops instead of delegating pure state transitions (`/Users/acoliver/projects/uqm/sc2/src/uqm/starcon.c:235-288`, `/Users/acoliver/projects/uqm/sc2/src/uqm/starbase.c:303-333`, `/Users/acoliver/projects/uqm/sc2/src/uqm/starbase.c:403-416`, `/Users/acoliver/projects/uqm/sc2/src/uqm/encount.c:551-745`).

4. **Rust integration exists only at dependency seams, not at the campaign orchestration seam.**
   Clock and state backends can be Rust-backed (`/Users/acoliver/projects/uqm/sc2/src/uqm/clock_rust.c:150-227`, `/Users/acoliver/projects/uqm/sc2/src/uqm/globdata.c:50-100`, `/Users/acoliver/projects/uqm/sc2/src/uqm/state.c:53-119`), but the activity graph and transition logic remain in C (`/Users/acoliver/projects/uqm/sc2/src/uqm/starcon.c:210-307`, `/Users/acoliver/projects/uqm/sc2/src/uqm/restart.c:340-411`, `/Users/acoliver/projects/uqm/sc2/src/uqm/starbase.c:430-529`, `/Users/acoliver/projects/uqm/sc2/src/uqm/encount.c:775-841`). That makes the current state hybrid at the primitive/backing-service layer, not at the campaign module layer.

5. **Build config enables broad Rust bridge support, but campaign has no isolated opt-in/out.**
   `build.config` treats Rust as a broad bridge bundle and exports `USE_RUST_CLOCK` and `USE_RUST_STATE` among many others (`/Users/acoliver/projects/uqm/sc2/build/unix/build.config:86-107`, `/Users/acoliver/projects/uqm/sc2/build/unix/build.config:656-737`). There is no campaign-specific toggle to isolate or test just campaign-gameplay migration.

## Current-state summary

Today, campaign-gameplay is a **C-owned global-state orchestrator** centered on `starcon.c` and `CurrentActivity`/flag dispatch. It owns the single-player campaign loop, new/load branching, high-level navigation transitions, starbase entry/exit sequencing, encounter handoff, event progression scheduling, and save/load of campaign runtime state (`/Users/acoliver/projects/uqm/sc2/src/uqm/starcon.c:210-307`, `/Users/acoliver/projects/uqm/sc2/src/uqm/restart.c:340-411`, `/Users/acoliver/projects/uqm/sc2/src/uqm/hyper.c:412-565`, `/Users/acoliver/projects/uqm/sc2/src/uqm/starbase.c:430-529`, `/Users/acoliver/projects/uqm/sc2/src/uqm/gameev.c:41-252`, `/Users/acoliver/projects/uqm/sc2/src/uqm/save.c:733-791`, `/Users/acoliver/projects/uqm/sc2/src/uqm/load.c:608-771`).

There is **no dedicated Rust campaign-gameplay module or toggle** in the inspected codebase (`/Users/acoliver/projects/uqm/rust/src/lib.rs:1-19`, `/Users/acoliver/projects/uqm/sc2/build/unix/build.config:86-107`). However, there are **live indirect Rust edges** at this boundary through the clock and state subsystems when `USE_RUST_CLOCK` and `USE_RUST_STATE` are enabled (`/Users/acoliver/projects/uqm/sc2/src/uqm/clock_rust.c:150-227`, `/Users/acoliver/projects/uqm/sc2/src/uqm/globdata.c:50-100`, `/Users/acoliver/projects/uqm/sc2/src/uqm/state.c:53-119`, `/Users/acoliver/projects/uqm/rust/src/state/ffi.rs:24-188`).
