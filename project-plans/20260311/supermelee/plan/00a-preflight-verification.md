# Phase 0.5: Preflight Verification

## Phase ID
`PLAN-20260314-SUPERMELEE.P00.5`

## Purpose
Verify assumptions about the codebase, toolchain, and integration boundaries before any implementation begins.

## Toolchain Verification
- [ ] `cargo --version` — confirm Rust 2021 edition support
- [ ] `rustc --version` — confirm stable toolchain
- [ ] `cargo clippy --version` — confirm clippy available
- [ ] `cargo llvm-cov --version` — verify whether coverage gates are feasible

## Dependency Verification
Verify only dependencies that are actually required by existing project conventions or the scoped SuperMelee boundary:
- [ ] `thiserror` crate present in `Cargo.toml` if the project standardizes on it for typed errors
- [ ] `libc` crate present if required for audited FFI signatures
- [ ] `tempfile` crate available for persistence tests, or an established project alternative exists
- [ ] Confirm whether any additional crate used by the plan is already present and idiomatic in this repository before adopting it

Explicitly do **not** treat speculative implementation preferences as preflight gates:
- `sdl2` is not a SuperMelee-owned dependency gate here
- `crossbeam` is not assumed for any concurrent processing in this plan
- `parking_lot` is not assumed for SuperMelee runtime state
- `anyhow` is optional and must not be required unless project conventions already rely on it for this layer

## Type/Interface Verification

### Existing Rust Types Required
- [ ] `rust/src/config.rs` — confirm current config surface relevant to `melee_scale` and setup persistence paths
- [ ] `rust/src/graphics/` — confirm frame/drawable abstractions used by setup/menu rendering already exist
- [ ] `rust/src/input/` — confirm menu/picker input abstractions exist
- [ ] `rust/src/sound/` — confirm menu music / transition audio hooks exist
- [ ] `rust/src/state/` — confirm game-state/activity flag types exist for menu↔battle transitions
- [ ] `rust/src/io/` or equivalent — confirm file-I/O abstractions used by persistence already exist

### C Types/Contracts That Must Be Audited Before Final Signatures Are Locked
- [ ] `MELEE_STATE` struct in `sc2/src/uqm/supermelee/melee.h` — setup runtime state
- [ ] `MeleeTeam` / `MeleeSetup` in `sc2/src/uqm/supermelee/meleesetup.h` — team model and serialized size assumptions
- [ ] `GETMELEE_STATE` and selection-related declarations in `sc2/src/uqm/supermelee/pickmele.h` — battle-facing selection state
- [ ] Exact built-in-team source in `sc2/src/uqm/supermelee/loadmele.c` — confirm whether names/compositions are content-only or compatibility-significant after audit
- [ ] `.mle` load/save format definitions in the current subsystem — confirm firm semantic compatibility floor and whether stronger byte-for-byte obligations exist only after audit
- [ ] Setup-persistence format for `melee.cfg` — confirm control-mode/team-state content and transient-network sanitization behavior
- [ ] Existing battle-facing consumer declarations for initial/next combatant requests — determine the actual queue-entry/handle type returned to battle
- [ ] Existing netplay-boundary declarations used by SuperMelee setup/selection integration — determine actual hook points and data passed across the boundary

### Globals/Subsystem Hooks That Must Be Accessible
- [ ] `PlayerControl[]` — per-player control-mode flags used by setup and start gating
- [ ] `GLOBAL(CurrentActivity)` or equivalent — activity state flags
- [ ] Setup asset/resource loaders used by `melee.c`, `buildpick.c`, and `loadmele.c`
- [ ] Battle entry point callable from SuperMelee handoff code
- [ ] Ship catalog helpers for ship cost/validity/icon lookup
- [ ] Optional netplay readiness/confirmation state accessible through the existing integration boundary when supported

## Call-Path Feasibility

### Setup/Menu Path
```text
Melee() -> LoadMeleeInfo() -> LoadMeleeConfig()
  -> menu/input loop
    -> DoLoadTeam() / DoSaveTeam() / BuildPickShip()
    -> StartMeleeButtonPressed() -> StartMelee() -> battle boundary
  -> WriteMeleeConfig() -> FreeMeleeInfo()
```

- [ ] Verify menu state machine can be driven from Rust while preserving existing setup/menu ownership boundaries
- [ ] Verify team serialization format is documented and testable at the semantic level
- [ ] Verify `.mle` load interoperability target is well-defined from current C behavior
- [ ] Verify setup-state persistence inputs/outputs are well-defined enough to preserve usable startup state

### Battle-Facing Selection Path
```text
StartMelee()
  -> prepare battle-facing selection state
  -> GetInitialMeleeStarShips() / GetNextMeleeStarShip()
  -> battle subsystem consumes fully prepared combatant entries/handles
```

- [ ] Verify the consuming battle boundary expects a concrete battle-ready handle/queue entry, not merely a ship ID
- [ ] Identify which part of the existing system creates/commits that battle-ready combatant object
- [ ] Verify SuperMelee can preserve that contract while still owning only selection policy/order

### Netplay Boundary Path
```text
local setup mutation -> expose sync event
remote setup update -> semantic validation -> commit/reject
match start request -> readiness/confirmation gate
battle-time local selection -> expose outcome
battle-time remote selection -> semantic validation -> commit/reject
```

- [ ] Verify setup-time ship-slot/team-name/whole-team sync hooks exist or can be added at the SuperMelee boundary
- [ ] Verify readiness/confirmation inputs required for start gating are accessible without redefining the netplay state machine
- [ ] Verify remote-selection semantic validation inputs are available (fleet contents, eliminated/available status, current phase)

## Test Infrastructure Verification
- [ ] `cargo test --workspace` succeeds currently
- [ ] Test files can be created in the SuperMelee area used by this project
- [ ] `tempfile` crate or equivalent is available for `.mle` persistence tests
- [ ] Determine whether any property/fuzz test framework already exists before introducing one

## Compatibility Audit Inputs
- [ ] Identify source of truth for built-in team names/compositions
- [ ] Identify source of truth for `.mle` semantic load contract
- [ ] Identify whether exact save-byte layout must remain audit-gated rather than mandatory from the start
- [ ] Identify whether setup-screen UI navigation/timing/audio details require exact parity or semantic compatibility only

## Blocking Issues
[List any blockers found during verification. If non-empty, revise plan before proceeding.]

Potential blockers:
1. If the battle-facing combatant-return type cannot be audited, Phase P08 cannot lock its public signatures.
2. If setup-time or battle-time netplay hook points are not discoverable, Phase P09 must be revised around documented deferred contracts.
3. If `.mle` or `melee.cfg` format behavior is under-specified in the current code, Phase P10 compatibility audit must explicitly resolve the uncertainty before final acceptance criteria are locked.

## Gate Decision
- [ ] PASS: proceed to Phase 01
- [ ] FAIL: revise plan — document specific issues and required changes
