# Phase 11: C-Side Bridge Wiring & Build Integration

## Phase ID
`PLAN-20260314-COMM.P11`

## Prerequisites
- Required: Phase 10a completed
- Expected: All Rust-side subsystems implemented (P03–P10)

## Requirements Implemented (Expanded)

### IN-REQ-012: Build switch compatibility
**Requirement text**: When build switches select the integrated communication implementation, externally visible behavior shall be preserved. Intermediate mixed-mode states acceptable provided no regression.

### DS-REQ-003, SC-REQ-001: Script source compatibility
**Requirement text**: All 27 C race scripts compile and behave correctly without modification against the published API.

### SC-REQ-002: PHRASE_ENABLED/DISABLE_PHRASE macros
**Requirement text**: Continue working at source level with existing phrase indices.

### SC-REQ-004: C globals abstraction
**Requirement text**: Scripts don't directly access replaced globals; API abstracts all access.

### CV-REQ-011: Mixed-mode validation
**Requirement text**: Intermediate migration states produce no regressions.

## Phase boundary note

P05b owns first creation of the authoritative trackplayer wrapper seam in `sc2/src/uqm/rust_comm.c` / `sc2/src/uqm/rust_comm.h`. This phase consumes and, if necessary, extends that seam for encounter/UI/build wiring only. It must not defer foundational trackplayer wrappers that P06 depends on.

## Implementation Tasks

### C files to modify

- `sc2/src/uqm/comm.c` — Guard the body behind `#ifndef USE_RUST_COMM`
  - marker: `@plan PLAN-20260314-COMM.P11`
  - marker: `@requirement IN-REQ-012`

  **Specific guards:**
  - Lines ~400-412: Already guarded for init/uninit — verify still correct
  - Lines ~415-462: `draw_response` — guard with `#ifndef USE_RUST_COMM`
  - Lines ~490-549: Speech graphics — guard
  - Lines ~565-712: TalkSegue/DoTalkSegue — guard
  - Lines ~795-920: Conversation summary — guard
  - Lines ~923-1040: Response selection and callback dispatch — guard
  - Lines ~1083-1127: DoCommunication state machine — guard
  - Lines ~1129-1167: Response registration — guard (Rust provides via FFI)
  - Lines ~1171-1296: HailAlien — guard
  - Lines ~1306-1442: InitCommunication — guard
  - Add `RaceCommunication` routing guard so the Rust mode entry point also goes through `rust_RaceCommunication()`
  - Lines ~1586-1642: Subtitle polling — guard
  - Add `#ifdef USE_RUST_COMM` blocks that call Rust FFI equivalents

- `sc2/src/uqm/commglue.c` — Route script API calls to Rust
  - marker: `@plan PLAN-20260314-COMM.P11`
  - marker: `@requirement DS-REQ-003, SC-REQ-001`

  **Specific changes:**
  - Lines ~35-91: `NPCPhrase_cb` — under `USE_RUST_COMM`, call `rust_NPCPhrase_cb(index, cb)` instead of C SpliceTrack
  - Lines ~93-119: `NPCPhrase_splice` — call `rust_NPCPhrase_splice(index)` instead of C splice
  - Lines ~122-239: `NPCNumber`/`NPCNumberPhrase` — call `rust_NPCNumber(number, fmt)` or keep C if Rust delegates back to trackplayer
  - Lines ~260-313: `construct_response` — call `rust_ConstructResponse(buf, ...)` or keep C if simpler
  - Lines ~315-355: `setSegue`/`getSegue` — call `rust_SetSegue(s)` / `rust_GetSegue()`
  - Lines ~357-422: `init_race` — keep the dispatch switch C-owned. Under Rust mode, expose/retain a thin `c_init_race(comm_id) -> LOCDATA*` helper callable from Rust; do **not** move ownership to a public `rust_InitRace` export.

  **Decision point resolved**: The `init_race` switch stays in C. Rust calls `c_init_race(comm_id)` which runs the existing C switch and returns `LOCDATA*`. This avoids duplicating the 27-way switch in Rust and keeps the layering coherent.

- `sc2/src/uqm/commglue.h` — Conditionally adjust macros
  - marker: `@plan PLAN-20260314-COMM.P11`
  - marker: `@requirement SC-REQ-002, SC-REQ-004`

  **Specific changes:**
  - `PHRASE_ENABLED(p)` macro: under `USE_RUST_COMM`, call `rust_PhraseEnabled(p)` instead of checking NUL in string data
  - `DISABLE_PHRASE(p)` macro: under `USE_RUST_COMM`, call `rust_DisablePhrase(p)` instead of NUL-mutating string data
  - Existing `NPCPhrase`, `NPCPhrase_cb`, `NPCPhrase_splice`, `NPCNumber`, `Response`, `DoResponsePhrase`, `construct_response` macros/declarations: may remain as-is if the underlying functions are routed via commglue.c

- `sc2/src/uqm/commanim.c` — Guard animation engine
  - marker: `@plan PLAN-20260314-COMM.P11`

  **Specific changes:**
  - Guard all animation functions behind `#ifndef USE_RUST_COMM`
  - `ProcessCommAnimations`, `InitCommAnimations`, `wantTalkingAnim`, `haveTalkingAnim`, `setRunIntroAnim`, `setRunTalkingAnim`, etc.
  - Under `#ifdef USE_RUST_COMM`: provide stub wrappers that call `rust_ProcessCommAnimations(delta)`, etc.

- `sc2/src/uqm/rust_comm.c` — Expand from init/uninit to full bridge
  - marker: `@plan PLAN-20260314-COMM.P11`

  **Add wrapper functions:**
  - `RaceCommunication()` → `rust_RaceCommunication()`
  - `InitCommunication(which_comm)` → `rust_InitCommunication(which_comm)`
  - `HailAlien()` → `rust_HailAlien()` if still needed as a bridge function
  - `DoCommunication()` → `rust_DoCommunication()`
  - `draw_response()` → `rust_DrawResponses()`
  - `ProcessCommAnimations(delta)` → `rust_ProcessCommAnimations(delta)`
  - Resource loading wrappers: `c_LoadGraphic`, `c_LoadFont`, etc. (C functions callable from Rust)
  - Input wrappers: `c_GetPulsedMenuInput`, `c_GetCurrentMenuInput`, `c_DoInput`, `c_SetMenuSounds`, `c_SuppressSpuriousInputAfterLoad` (C functions callable from Rust)
  - Reuse and, if needed, extend the P05b trackplayer wrapper seam rather than recreating it here; P11 may add only wrappers still missing after P05b for encounter/UI wiring
  - Trackplayer/lifecycle verification seam wrappers: `c_PollPendingTrackCompletion`, `c_CommitTrackAdvancement`
  - `c_init_race(comm_id)` wrapper around the existing C dispatch switch

- `sc2/src/uqm/rust_comm.h` — Declare all new FFI exports
  - marker: `@plan PLAN-20260314-COMM.P11`

  **New declarations (adding to existing):**
  - `int rust_NPCPhrase_cb(int index, void (*cb)(void));`
  - `void rust_NPCPhrase_splice(int index);`
  - `int rust_NPCNumber(int number, const char *fmt);`
  - `void rust_ConstructResponse(UNICODE *buf, int R, ...);`
  - `void rust_SetSegue(int segue);`
  - `int rust_GetSegue(void);`
  - `int rust_PhraseEnabled(int index);`
  - `void rust_DisablePhrase(int index);`
  - `int rust_RaceCommunication(void);`
  - `LOCDATA *c_init_race(int comm_id);`
  - All animation control functions
  - All encounter lifecycle functions

### Concrete seam ownership and source-path mapping

All bridge wrappers in this phase must point to the concrete existing implementations below:

- `sc2/src/uqm/comm.c`
  - `RaceCommunication`
  - `InitCommunication`
  - `HailAlien`
  - `DoCommunication`
  - `TalkSegue` / `DoTalkSegue`
  - `RefreshResponses` / `FeedbackPlayerPhrase`
  - `ClearSubtitles` / `CheckSubtitles` / `RedrawSubtitles`
- `sc2/src/uqm/commglue.c`
  - `NPCPhrase_cb`
  - `NPCPhrase_splice`
  - `NPCNumber`
  - `construct_response`
  - `setSegue` / `getSegue`
  - `init_race`
- `sc2/src/uqm/commanim.c`
  - `ProcessCommAnimations`
  - `InitCommAnimations`
  - talking/transition helpers
- `sc2/src/libs/sound/trackplayer.c`
  - `PlayTrack`, `StopTrack`, `JumpTrack`, `PauseTrack`, `ResumeTrack`
  - `FastReverse_Smooth`, `FastForward_Smooth`, `FastReverse_Page`, `FastForward_Page`
  - subtitle enumeration helpers currently used for replay/summary parity checks
- `sc2/src/libs/input/*`
  - low-level input backend remains C-owned; bridge wrappers expose only the menu-state / `DoInput` seam needed by Rust comm

### Mixed-mode invariants to validate continuously

These invariants are not deferred to P12; they must hold after each integration slice lands and are explicitly validated again here:

- C-authoritative in both build modes:
  - race scripts
  - low-level input backend
  - legacy fallback implementation when `USE_RUST_COMM` is off
- Rust-authoritative in Rust mode by the end of P11:
  - encounter lifecycle orchestration
  - phrase state
  - talk/main-loop control flow
  - response/subtitle/speech/summary rendering behavior
- Allowed mixed entry points during migration:
  - C race scripts calling commglue APIs routed into Rust
  - Rust calling thin C wrappers for resource/input/trackplayer/build hooks
- Required after this phase:
  - both build modes compile and link
  - no duplicated ownership of the same visible behavior in Rust mode
  - no regression in externally visible behavior in partially migrated intermediate builds

### Race-script compatibility validation

All 27 race scripts MUST compile without modification. Validation:

```bash
# After C guards and macro changes, rebuild full project
cd sc2 && make clean && make

# Verify all 27 race .o files compile
ls -la src/uqm/comm/*/*.o
```

Specific scripts to spot-check for macro usage:
- `arilouc.c` — uses PHRASE_ENABLED, DISABLE_PHRASE, NPCPhrase, Response
- `starbas.c` — complex dialogue tree, construct_response usage
- `orzc.c` — multi-phase dialogue with setSegue
- `zoqfotc.c` — NPCNumber usage for ship counts
- `melnormc.c` — extensive DISABLE_PHRASE usage

### Pseudocode traceability
- No direct pseudocode — this phase is wiring, not algorithm

## Verification Commands

```bash
# Rust side
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features

# C side (full project rebuild)
cd /Users/acoliver/projects/uqm/sc2 && make clean && make 2>&1 | tail -20

# Race script compilation
for dir in /Users/acoliver/projects/uqm/sc2/src/uqm/comm/*/; do
    race=$(basename "$dir")
    echo "=== $race ==="
    ls -la "${dir}"*.o 2>/dev/null || echo "NO OBJECT FILES"
done
```

## Structural Verification Checklist
- [ ] `comm.c` major sections guarded by `#ifndef USE_RUST_COMM`
- [ ] `comm.c` or bridge layer routes both public entry points (`RaceCommunication`, `InitCommunication`) to Rust in Rust mode
- [ ] `commglue.c` routes NPCPhrase/response/segue to Rust under `USE_RUST_COMM`
- [ ] `commglue.h` macros (PHRASE_ENABLED, DISABLE_PHRASE) route to Rust under `USE_RUST_COMM`
- [ ] `commanim.c` guarded with Rust stubs under `USE_RUST_COMM`
- [ ] `rust_comm.c` expanded with all bridge wrappers
- [ ] `rust_comm.h` declares all new Rust FFI functions
- [ ] All 27 race scripts compile without modification
- [ ] Every wrapper introduced in P08–P11 now cites its concrete backing source file/path

## Semantic Verification Checklist (Mandatory)
- [ ] Full C build succeeds with `USE_RUST_COMM` defined
- [ ] Full C build succeeds with `USE_RUST_COMM` undefined (C fallback preserved)
- [ ] All 27 race .o files generated
- [ ] PHRASE_ENABLED/DISABLE_PHRASE macros work via Rust (unit test from C side)
- [ ] NPCPhrase routes through Rust glue to trackplayer
- [ ] Response macros route through Rust response system
- [ ] setSegue/getSegue route through Rust segue state
- [ ] `RaceCommunication()` routes through Rust in Rust mode and remains C-owned in fallback mode
- [ ] `InitCommunication()` routes through Rust in Rust mode and remains C-owned in fallback mode
- [ ] Intermediate mixed-mode build still runs without ownership ambiguity or duplicated visible behavior
- [ ] No duplicate symbol errors at link time
- [ ] No undefined symbol errors at link time

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" sc2/src/uqm/rust_comm.c sc2/src/uqm/rust_comm.h
```

## Success Criteria
- [ ] Both build modes compile and link
- [ ] Race scripts unchanged and compiling
- [ ] C→Rust routing works for all comm entry points
- [ ] No regressions in C-only mode (IN-REQ-012)
- [ ] Mixed-mode invariants are explicit and verified, not left for P12 alone

## Failure Recovery
- rollback: `git restore sc2/src/uqm/comm.c sc2/src/uqm/commglue.c sc2/src/uqm/commglue.h sc2/src/uqm/commanim.c sc2/src/uqm/rust_comm.c sc2/src/uqm/rust_comm.h`
- blocking: Link errors require checking symbol visibility and export names

## Phase Completion Marker
Create: `project-plans/20260311/comm/.completed/P11.md`
