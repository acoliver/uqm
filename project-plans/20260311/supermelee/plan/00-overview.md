# Plan: SuperMelee Subsystem — Setup/Menu, Persistence, Selection, and Battle Handoff Port

Plan ID: PLAN-20260314-SUPERMELEE
Generated: 2026-03-14
Total Phases: 23 (P00.5 through P15, with verification sub-phases and dedicated compatibility/netplay work)
Requirements: statement-level coverage from `supermelee/requirements.md`

## Context

The SuperMelee subsystem remains largely unported on the Rust side. The C implementation under `sc2/src/uqm/supermelee/` still owns setup/menu orchestration, team management, ship-pick, and load/save behavior. The Rust-side presence today is limited to adjacent configuration plumbing such as melee-zoom option parsing in `rust/src/cli.rs` and `rust/src/config.rs`.

This plan is intentionally scoped to the subsystem boundary defined by `specification.md`:
1. **Setup/menu orchestration** — enter SuperMelee, initialize runtime/menu state, drive setup flow, teardown cleanly
2. **Team and fleet model** — editable per-side team state, bounded names, fleet value consistency
3. **Built-in team catalog and browsing** — browse built-in teams and file-backed teams through one load surface
4. **Team persistence** — legacy `.mle` load interoperability, safe save semantics, setup-state persistence
5. **Fleet-edit ship picking** — confirm/cancel ship selection when editing fleets
6. **Battle-facing combatant selection policy** — choose initial/next combatants from prepared fleets while preserving the consuming battle-facing contract
7. **Battle handoff and return** — validate start conditions, package SuperMelee-owned handoff inputs, transfer control to battle, restore menu state after return
8. **Netplay boundary obligations** — local-only behavior when disabled, plus setup/selection synchronization, start gating, and semantic remote-selection validation when supported
9. **Compatibility audit and REQ traceability** — explicitly distinguish mandatory semantic compatibility from audit-gated exact-parity obligations and verify every requirement statement

This plan does **not** port or replace generic battle-engine internals, per-ship combat runtime, collision, AI, tactical transitions, or netplay transport/protocol machinery. Those remain separate integration boundaries owned by other subsystem plans.

## C Files Being Replaced

### SuperMelee Setup Ownership (`sc2/src/uqm/supermelee/`)
| C File | Rust Module | Purpose |
|--------|-------------|---------|
| `melee.c` / `melee.h` | `supermelee::setup::melee` | Entry, menu loop, setup/battle handoff, teardown |
| `meleesetup.c` / `meleesetup.h` | `supermelee::setup::team` | Team/fleet data model |
| `loadmele.c` / `loadmele.h` | `supermelee::setup::persistence` | Team load/save, built-in catalog, team browser |
| `buildpick.c` / `buildpick.h` | `supermelee::setup::build_pick` | Fleet-edit ship picker |
| `pickmele.c` / `pickmele.h` | `supermelee::setup::pick_melee` | Battle-facing initial/next combatant selection |
| `meleeship.h` | `supermelee::types` | Ship ID enum / constants |

### Integration Boundaries Explicitly Not Replaced by This Plan
| Boundary | Current Owner | Why excluded here |
|----------|---------------|-------------------|
| `battle.c`, `process.c`, `collide.c`, `ship.c`, `intel.c`, `tactrans.c`, `element.h` | Battle / ships subsystem plans | Spec excludes generic battle internals and per-ship combat behavior |
| netplay transport/protocol stack | Netplay subsystem plan | Spec limits SuperMelee to setup/selection boundary obligations |
| graphics/input/audio/resource/file-I/O/threading internals | Existing subsystem plans | SuperMelee consumes these interfaces but does not redefine them |

## New Rust Module Structure

```text
rust/src/supermelee/
  mod.rs                        # Module root, re-exports
  types.rs                      # MeleeShip enum, shared constants/types
  error.rs                      # SuperMeleeError enum
  c_bridge.rs                   # Audited C imports for setup/handoff boundaries

  setup/
    mod.rs                      # Setup sub-module root
    team.rs                     # MeleeTeam, MeleeSetup, fleet value
    persistence.rs              # .mle load/save, built-in teams, browser
    config.rs                   # melee.cfg persistence / sanitization
    melee.rs                    # Melee() entry, menu orchestration, battle handoff
    build_pick.rs               # Fleet-edit ship picker
    pick_melee.rs               # Initial/next combatant selection and commit
    netplay_boundary.rs         # Setup/selection sync surface, validation, start gating
    ffi.rs                      # FFI bridge exports for SuperMelee-owned C entry points
```

## Ownership and Interface Boundaries

### Ships subsystem boundary
The ships subsystem owns ship catalog metadata, ship costs/icons, roster validity, battle-ready descriptor loading, queue-entry data model, and combat spawn mechanics.

SuperMelee owns team composition, fleet slot state, selection ordering/policy, and commit of local/remote combatant choice into the battle-facing selection state.

Representative imported capabilities (exact types/signatures to be audited against C headers before implementation):

```rust
pub trait ShipCatalog {
    fn melee_ship_count(&self) -> usize;
    fn ship_cost(&self, ship: MeleeShip) -> Result<u16, SuperMeleeError>;
    fn ship_icons(&self, ship: MeleeShip) -> Result<ShipIconSet, SuperMeleeError>;
    fn is_valid_melee_ship(&self, ship: MeleeShip) -> bool;
}

pub trait BattleShipFactory {
    type CombatantHandle;
    type QueueEntry;

    fn create_combatant_for_slot(
        &self,
        side: usize,
        slot: usize,
        ship: MeleeShip,
    ) -> Result<Self::QueueEntry, SuperMeleeError>;
}
```

### Battle subsystem boundary
SuperMelee does **not** own battle simulation. It must, however, preserve the battle-facing contract by handing off fully prepared combatant entries/handles rather than bare ship IDs or slot numbers.

The exact exported/imported signatures must be audited against the real C headers. Until that audit is complete, plan text distinguishes placeholders from verified interfaces.

### Netplay subsystem boundary
SuperMelee owns:
- emitting setup-time ship-slot/team-name/whole-team sync events,
- gating match start on SuperMelee-visible readiness/confirmation preconditions,
- exposing local combatant-selection outcomes,
- semantically validating remote selections against current fleet state, and
- committing only valid remote selections into battle-facing selection state.

SuperMelee does **not** own wire-format, transport, retransmission, connection discovery, or protocol phase machines.

## Integration Points

### Existing Rust/C subsystems consumed by SuperMelee
| Subsystem | Integration |
|-----------|-------------|
| `config` | `melee_scale` and setup-state persistence locations/configuration |
| `graphics` | Setup/menu frames, ship-pick presentation, transitions consumed through existing APIs |
| `input` | Menu and picker input handling, control-mode mapping |
| `sound` | Menu music / transition audio through existing APIs |
| `resource` | Setup assets, fonts, icons |
| `io` | File reads/writes for `.mle` and `melee.cfg` |
| `state` | `CurrentActivity` / mode flags for menu↔battle transitions |
| battle subsystem | Start battle, return from battle, consume battle-ready combatant entries |
| ships subsystem | ship costs/icons/validation and battle-ready combatant creation |
| netplay subsystem | optional setup-time/battle-time synchronization hooks |

## Phase Structure

| Phase | Title | Est. LoC |
|-------|-------|----------|
| P00.5 | Preflight Verification | 0 |
| P01 | Analysis | 0 |
| P01a | Analysis Verification | 0 |
| P02 | Pseudocode | 0 |
| P02a | Pseudocode Verification | 0 |
| P03 | Core Types & Error — Stub | ~350 |
| P03a | Core Types Stub Verification | 0 |
| P04 | Core Types & Error — TDD | ~300 |
| P04a | Core Types TDD Verification | 0 |
| P05 | Core Types & Error — Impl | ~250 |
| P05a | Core Types Impl Verification | 0 |
| P06 | Team Model & Persistence — Stub/TDD/Impl | ~900 |
| P06a | Team Model & Persistence Verification | 0 |
| P07 | Setup Menu & Fleet Ship Pick — Stub/TDD/Impl | ~1000 |
| P07a | Setup Menu & Fleet Ship Pick Verification | 0 |
| P08 | Battle-Facing Combatant Selection Contract | ~700 |
| P08a | Combatant Selection Verification | 0 |
| P09 | Netplay Boundary Surface & Validation | ~800 |
| P09a | Netplay Boundary Verification | 0 |
| P10 | Compatibility Audit Decision Points | ~0 |
| P10a | Compatibility Audit Verification | 0 |
| P11 | FFI Bridge & C-Side Wiring | ~500 (Rust) + ~200 (C) |
| P11a | FFI Bridge Verification | 0 |
| P12 | Requirement Traceability Matrix | ~200 |
| P12a | Traceability Verification | 0 |
| P13 | End-to-End Local Integration Verification | ~250 |
| P14 | End-to-End Netplay-Boundary Verification | ~250 |
| P15 | Final Integration & Signoff | ~150 |

Total estimated new/modified LoC: ~5700 (Rust/plan-guided implementation) + ~200 (C)

## Execution Order

```text
P00.5 -> P01 -> P01a -> P02 -> P02a
       -> P03 -> P03a -> P04 -> P04a -> P05 -> P05a
       -> P06 -> P06a
       -> P07 -> P07a
       -> P08 -> P08a
       -> P09 -> P09a
       -> P10 -> P10a
       -> P11 -> P11a
       -> P12 -> P12a
       -> P13 -> P14 -> P15
```

This ordering is deliberate for the scoped subsystem:
1. establish shared types,
2. deliver team model/persistence early,
3. deliver setup/menu and fleet editing,
4. implement battle-facing selection and handoff contract,
5. add concrete netplay-boundary behavior,
6. audit compatibility-sensitive obligations,
7. wire the Rust implementation into the existing C boundary,
8. verify every requirement statement end-to-end.

## Critical Reminders

Before implementing any phase:
1. Preflight verification must confirm only dependencies/interfaces actually required by the existing codebase and integration boundary.
2. C-boundary signatures must be classified as either **verified exact** (audited against headers) or **design placeholder** (subject to later audit).
3. Compatibility-sensitive obligations must remain audit-gated unless `specification.md` or the audit outcome makes them mandatory.
4. Verification gates should prefer targeted tests/scripts over brittle grep heuristics.

## Definition of Done

1. All `cargo test --workspace --all-features` checks relevant to SuperMelee pass
2. All `cargo clippy --workspace --all-targets --all-features -- -D warnings` pass
3. `cargo fmt --all --check` passes
4. SuperMelee setup screen loads and supports local team editing, load/save, built-in browsing, and fleet ship-pick confirm/cancel behavior
5. Match start is blocked for invalid/unplayable fleets and allowed for valid fleets
6. Initial and next combatants are provided via a battle-facing contract that preserves battle-ready handles/queue entries rather than weakening to bare ship IDs
7. Local-only SuperMelee works without netplay support or state
8. When netplay support is enabled, setup-time sync events, start gating, remote-selection semantic validation, and commit/reject behavior are implemented and verified at the SuperMelee boundary
9. Valid legacy `.mle` files load with firm semantic interoperability
10. Save failure does not leave an apparently successful corrupted team artifact behind
11. `melee.cfg` persistence restores valid local startup state and sanitizes transient network-only control modes when required
12. Compatibility-audit outcomes are recorded for built-in team exactness, save-format exactness, and UI/audiovisual parity obligations
13. Statement-level requirement traceability exists for every requirement in `requirements.md`
14. No placeholder stubs or TODO markers remain in SuperMelee implementation code

## Plan Files

```text
plan/
  00-overview.md                                    (this file)
  00a-preflight-verification.md                     P00.5
  01-analysis.md                                   P01
  01a-analysis-verification.md                     P01a
  02-pseudocode.md                                 P02
  02a-pseudocode-verification.md                   P02a
  03-core-types-stub.md                            P03
  03a-core-types-stub-verification.md              P03a
  04-core-types-tdd.md                             P04
  04a-core-types-tdd-verification.md               P04a
  05-core-types-impl.md                            P05
  05a-core-types-impl-verification.md              P05a
  06-team-model-persistence.md                     P06
  06a-team-model-persistence-verification.md       P06a
  07-setup-menu-ship-pick.md                       P07
  07a-setup-menu-ship-pick-verification.md         P07a
  08-combatant-selection-contract.md               P08
  08a-combatant-selection-contract-verification.md P08a
  09-netplay-boundary.md                           P09
  09a-netplay-boundary-verification.md             P09a
  10-compatibility-audit.md                        P10
  10a-compatibility-audit-verification.md          P10a
  11-ffi-bridge-c-wiring.md                        P11
  11a-ffi-bridge-c-wiring-verification.md          P11a
  12-requirement-traceability.md                   P12
  12a-requirement-traceability-verification.md     P12a
  13-e2e-local-integration-verification.md         P13
  14-e2e-netplay-boundary-verification.md          P14
  15-final-integration-signoff.md                  P15
  execution-tracker.md
```

## Deferred Items

The following remain explicitly out of scope for this SuperMelee plan:

- **Generic battle-engine internals**: battle loop simulation, element/display-list internals, collision engine, ship runtime, tactical AI, and ship transition internals
- **Ships subsystem porting**: ship catalog internals, `RACE_DESC`, per-race hooks, spawn mechanics, combat behavior
- **Netplay transport/protocol implementation**: connection management, packet framing, retransmission, remote-state-machine behavior
- **Campaign battle integration redesign**: campaign encounter preparation is not planned here beyond preserving the documented SuperMelee battle-facing boundary
- **Advanced graphics scaling/renderer design**: graphics subsystem concerns stay with the graphics plan
