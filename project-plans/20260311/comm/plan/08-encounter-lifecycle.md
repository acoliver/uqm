# Phase 08: Encounter Lifecycle & Entry Points

## Phase ID
`PLAN-20260314-COMM.P08`

## Prerequisites
- Required: Phase 07a completed
- Expected: CommData with full LOCDATA parity, animation engine, phrase state, glue layer, corrected FFI

## Requirements Implemented (Expanded)

### EC-REQ-001–016: Full encounter lifecycle
**Requirement text**: `RaceCommunication` and `InitCommunication` resolve encounter context and race, build fleet, determine hail/attack, enter dialogue or combat. `HailAlien` loads resources, creates contexts, runs dialogue loop, tears down. Lifecycle callbacks follow exactly-once rules across all exit paths.

Behavior contract:
- GIVEN: An encounter is initiated
- WHEN: `RaceCommunication()` or `InitCommunication(which_comm)` is called
- THEN: context resolved → saved-game SIS update if needed → race normalized → fleet built → sphere tracking → `init_race`/LOCDATA load → hail/attack → `HailAlien`/combat

### EC-REQ-009, EC-REQ-015, EC-REQ-016: Callback ordering
**Requirement text**: Each callback invoked at most once per encounter. Attack-without-hail calls post+uninit but not init.

### OL-REQ-002, OL-REQ-004: Resource ownership
**Requirement text**: Encounter-local resources owned by comm and released on teardown.

## Implementation Tasks

### Files to create

- `rust/src/comm/encounter.rs` — Encounter lifecycle orchestration and Rust-owned public entry points
  - marker: `@plan PLAN-20260314-COMM.P08`
  - marker: `@requirement EC-REQ-001 through EC-REQ-016, OL-REQ-002`

  **Functions:**

  - `race_communication() -> CommResult<()>`
    - Read game state to determine encounter context (hyperspace, interplanetary, last-battle, etc.)
    - If a saved game was just loaded, invoke the SIS display update step before further encounter setup
    - Resolve the resulting `CONVERSATION` enum variant
    - Delegate to `init_communication(which_comm)`

  - `init_communication(which_comm: u32) -> CommResult<()>`
    - Resolve CONVERSATION enum to comm_id
    - Handle drone/rebel normalization (EC-REQ-002)
    - Build NPC fleet if combat possible
    - Start sphere tracking
    - Call `c_init_race(comm_id)` through FFI → get LOCDATA
    - Read LOCDATA into CommData
    - Evaluate hail-or-attack:
      - If BATTLE_SEGUE != 0: present choice
      - Attack chosen: call post_encounter_func, uninit_encounter_func (EC-REQ-015), set BATTLE_SEGUE=1, return to combat
      - Talk chosen: clear BATTLE_SEGUE, fall through to `hail_alien`
    - If BATTLE_SEGUE == 0: call `hail_alien` directly (EC-REQ-006)

  - `hail_alien() -> CommResult<()>`
    - Load resources from CommData resource IDs (EC-REQ-007):
      - `LoadGraphic(alien_frame_res)` → store handle
      - `LoadFont(alien_font_res)` → store handle
      - `LoadColorMap(alien_colormap_res)` → store handle
      - `LoadMusic(alien_song_res or alien_alt_song_res)` → store handle
      - `LoadStringTable(conversation_phrases_res)` → store handle
    - Create subtitle cache context (offscreen pixmap)
    - Create animation display context
    - Reset phrase state
    - Initialize comm animations from CommData descriptors
    - **Release COMM_STATE lock** (CB-REQ-008)
    - Call init_encounter_func() (EC-REQ-016: at most once)
    - **Reacquire COMM_STATE lock**
    - Enter main dialogue loop: `do_communication()`
    - On normal exit:
      - **Release lock**
      - Call post_encounter_func() (EC-REQ-009)
      - Call uninit_encounter_func() (EC-REQ-009)
      - **Reacquire lock**
    - On abort/load:
      - Skip post_encounter_func (EC-REQ-010)
      - **Release lock**
      - Call uninit_encounter_func() (EC-REQ-010)
      - **Reacquire lock**
    - Teardown: destroy all loaded resources in reverse order (EC-REQ-014)
    - Leave state valid for reuse (EC-REQ-010)

  - `destroy_encounter_resources(state: &mut CommState)`
    - DestroyStringTable, DestroyMusic, DestroyColorMap, DestroyFont, DestroyDrawable
    - Destroy subtitle cache context, animation context
    - Clear CommData handles

  - `post_dialogue_battle_segue() -> CommResult<bool>`
    - If BATTLE_SEGUE set and NPC fleet non-empty: build player fleet, return true for combat
    - Otherwise clear BATTLE_SEGUE, return false

### External C functions needed via FFI

```rust
extern "C" {
    // Resource loading
    fn c_LoadGraphic(res: u32) -> u32;
    fn c_LoadFont(res: u32) -> u32;
    fn c_LoadColorMap(res: u32) -> u32;
    fn c_LoadMusic(res: u32) -> u32;
    fn c_LoadStringTable(res: u32) -> u32;
    // Resource destruction
    fn c_DestroyDrawable(handle: u32);
    fn c_DestroyFont(handle: u32);
    fn c_DestroyColorMap(handle: u32);
    fn c_DestroyMusic(handle: u32);
    fn c_DestroyStringTable(handle: u32);
    // Encounter flow
    fn c_BuildBattle(fleet_id: i32);
    fn c_EncounterBattle() -> i32;
    fn c_InitEncounter();
    fn c_UninitEncounter();
    fn c_StartSphereTracking(race: i32);
    // Game state queries and entry-point helpers
    fn c_GetCurrentActivity() -> u32;
    fn c_GetBattleSegue() -> i32;
    fn c_SetBattleSegue(val: i32);
    fn c_SavedGameJustLoaded() -> i32;
    fn c_UpdateSISForCurrentContext();
    fn c_ResolveRaceCommunicationFromGameState() -> i32;
    // Graphics contexts
    fn c_CreateContext(name: *const c_char) -> u32;
    fn c_DestroyContext(ctx: u32);
    // Input bridge prerequisites for P09
    fn c_GetPulsedMenuInput() -> u32;
    fn c_GetCurrentMenuInput() -> u32;
    fn c_DoInput(mode: i32) -> i32;
    fn c_SetMenuSounds(move_sounds: u32, select_sounds: u32);
    fn c_SuppressSpuriousInputAfterLoad();
}
```

### Files to modify

- `rust/src/comm/state.rs`
  - Add `encounter_active: bool` field
  - Add callback tracking: `init_called: bool`, `post_called: bool`, `uninit_called: bool`
  - Extend `clear()` to reset encounter tracking
  - marker: `@plan PLAN-20260314-COMM.P08`

- `rust/src/comm/ffi.rs`
  - Replace `rust_InitCommunication` with full encounter lifecycle:
    ```rust
    #[no_mangle]
    pub extern "C" fn rust_InitCommunication(which_comm: c_uint) -> c_int {
        match encounter::init_communication(which_comm) {
            Ok(()) => 1,
            Err(_) => 0,
        }
    }
    ```
  - Add `rust_RaceCommunication() -> c_int` export for the second public entry point
  - Add `rust_HailAlien() -> c_int` export only if a separate C bridge entry is still needed
  - marker: `@plan PLAN-20260314-COMM.P08`

- `rust/src/comm/mod.rs`
  - Add `pub mod encounter;`

### C-side integration

- `sc2/src/uqm/rust_comm.c`
  - Add C wrapper functions for resource loading that Rust can call through FFI
  - Add thin helper wrappers for saved-game SIS update and `RaceCommunication()` context resolution
  - Add input bridge wrappers needed before P09: pulsed menu input snapshot, current menu input snapshot, `DoInput` driving hook, menu-sound hook, and load-transition/spurious-input suppression hook tied to `LastActivity`
  - These are thin wrappers around existing UQM resource / encounter-state APIs

### Concrete seam ownership and source-path mapping

The wrappers introduced in this phase MUST be anchored to existing C implementations, not rediscovered later:

- `c_BuildBattle`, `c_EncounterBattle`, `c_InitEncounter`, `c_UninitEncounter`
  - source implementation: `sc2/src/uqm/comm.c`
  - called from existing `InitCommunication()` encounter flow
- `c_ResolveRaceCommunicationFromGameState`, `c_UpdateSISForCurrentContext`
  - source implementation: `sc2/src/uqm/comm.c`
  - wrapper logic extracted from existing `RaceCommunication()` / `InitCommunication()` branches that currently handle `LastActivity & CHECK_LOAD`, HQ/interplanetary dispatch, and title/SIS redraw behavior
- `c_init_race(comm_id)`
  - source implementation: `sc2/src/uqm/commglue.c` (`init_race` switch)
- `c_CreateContext`, `c_DestroyContext`
  - source implementation path to wrap: graphics/context helpers already used by `HailAlien()` in `sc2/src/uqm/comm.c`
- input bridge wrappers required by P09 and established here as prerequisites:
  - `c_GetPulsedMenuInput` / `c_GetCurrentMenuInput`
    - source state: `PulsedInputState` / `CurrentInputState` used by `DoTalkSegue()` and `PlayerResponseInput()` in `sc2/src/uqm/comm.c`
  - `c_DoInput`
    - source call pattern: `DoInput(...)` used by `TalkSegue()`, `DoConvSummary()`, `DoCommunication()`, and `HailAlien()` in `sc2/src/uqm/comm.c`
  - `c_SetMenuSounds`
    - source call sites: `SetMenuSounds(...)` in `TalkSegue()` and `DoCommunication()` in `sc2/src/uqm/comm.c`
  - `c_SuppressSpuriousInputAfterLoad`
    - source state transition: `LastActivity |= CHECK_LOAD` / `LastActivity &= ~CHECK_LOAD` in `HailAlien()` and `InitCommunication()` in `sc2/src/uqm/comm.c`

### Intermediate build invariants (must hold after P08/P08a)

- C remains authoritative for actual input polling and `DoInput` event-loop mechanics; Rust may call only the wrappers defined in this phase.
- Rust becomes authoritative for encounter lifecycle orchestration and callback-order tracking in Rust mode.
- Mixed mode after this phase is valid only if:
  - `rust_InitCommunication()` / `rust_RaceCommunication()` route through Rust lifecycle orchestration,
  - `init_race` dispatch remains C-owned behind `c_init_race`, and
  - the input bridge seam above exists and is verified before P09 begins.
- Both build modes must still compile after this phase, even though later phases still own response/subtitle/speech rendering behavior.

### Pseudocode traceability
- Uses pseudocode lines: 170-216 (public entry-point routing and encounter lifecycle)

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] `encounter.rs` created with `race_communication`, `init_communication`, `hail_alien`, teardown
- [ ] `RaceCommunication()` and `InitCommunication()` are both planned as Rust-owned public entry points in Rust mode
- [ ] Callback ordering: init→post→uninit on normal exit
- [ ] Callback ordering: init→uninit on abort (skip post)
- [ ] Callback ordering: post→uninit on attack-without-hail (skip init)
- [ ] Lock release before every C callback
- [ ] Lock reacquire after every C callback return
- [ ] Resource load in init, destroy in teardown (reverse order)
- [ ] Encounter flow FFI declarations present
- [ ] Saved-game SIS update hook is explicit in the entry-point flow
- [ ] Input bridge seam for pulsed/current menu state, `DoInput`, menu sounds, and post-load suppression is established before P09
- [ ] Every new wrapper in this phase names its concrete C source file/path

## Semantic Verification Checklist (Mandatory)
- [ ] Test: `race_communication()` resolves context correctly for representative encounter contexts
- [ ] Test: saved-game SIS update step runs before encounter setup when load flag is set
- [ ] Test: normal exit calls init, post, uninit exactly once each
- [ ] Test: abort skips post, calls uninit exactly once
- [ ] Test: attack-without-hail calls post and uninit, not init
- [ ] Test: double-init rejected (EC-REQ-016 at most once)
- [ ] Test: resource teardown destroys all loaded handles
- [ ] Test: state valid for reuse after teardown (new encounter can init)
- [ ] Test: BATTLE_SEGUE evaluated correctly for hail/attack decision
- [ ] Test: sphere tracking started for correct race
- [ ] Test: encounter_active flag correctly managed
- [ ] Test: `c_DoInput` / menu-input wrappers exist and are callable before P09 begins

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/comm/encounter.rs
```

## Success Criteria
- [ ] Full encounter lifecycle implemented
- [ ] Both public entry points owned in Rust mode and verified
- [ ] Saved-game SIS update ordering preserved
- [ ] All exit paths tested
- [ ] Callback ordering correct per EC-REQ-009, EC-REQ-015, EC-REQ-016
- [ ] Resource lifecycle correct
- [ ] Input bridge prerequisites for P09 are closed, not left as a later discovery item

## Failure Recovery
- rollback: `git restore rust/src/comm/encounter.rs rust/src/comm/state.rs`
- blocking: Resource loading and input-bridge FFI wrappers must exist in C

## Phase Completion Marker
Create: `project-plans/20260311/comm/.completed/P08.md`
