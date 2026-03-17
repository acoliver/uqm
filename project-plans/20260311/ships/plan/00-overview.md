# Plan: Ships Subsystem — Full Rust Port

Plan ID: PLAN-20260314-SHIPS
Generated: 2026-03-14
Total Phases: 35 (P00.5 through P15, with verification sub-phases and an added boundary-contract phase)
Requirements: Canonical REQ IDs defined in `plan/01-analysis.md` from `ships/requirements.md`; later phases must use those exact IDs only

## Context

The ships subsystem is **completely unported**. There is no `USE_RUST_SHIPS` build toggle, no `ships` module in `rust/src/lib.rs`, and the prototype code in `rust/src/game_init/` (init.rs, master.rs) contains only stub placeholders with hardcoded test data — no ship behavior, no descriptors, no race implementations, no integration with C battle code.

All 28 per-race ship implementations remain in C under `sc2/src/uqm/ships/`. The shared runtime (`ship.c`, `loadship.c`, `master.c`, `build.c`, `dummy.c`, `init.c`) and contract surface (`races.h`) are exclusively C-owned.

This plan ports the entire ships subsystem to Rust: shared types, shared runtime, ship catalog, two-tier loading, all 28 race implementations, and the FFI bridge layer that wires Rust ships into the C battle engine.

## Port Strategy

### Trait-based ship system

Replace C's function-pointer `RACE_DESC` callback pattern with a Rust trait:

```rust
pub trait ShipBehavior: Send {
    fn preprocess(&mut self, ship: &mut ShipState, battle: &BattleContext) -> Result<()>;
    fn postprocess(&mut self, ship: &mut ShipState, battle: &BattleContext) -> Result<()>;
    fn init_weapon(&mut self, ship: &ShipState, battle: &BattleContext) -> Result<Vec<WeaponElement>>;
    fn intelligence(&mut self, ship: &ShipState, battle: &BattleContext) -> StatusFlags;
    fn uninit(&mut self) {}
    fn collision_override(&self) -> Option<CollisionHandler> { None }
}
```

Each race provides a struct implementing `ShipBehavior`. The `RaceDesc` holds a `Box<dyn ShipBehavior>` alongside typed characteristic/info/data fields.

### Dispatch model

Replace `dummy.c`'s `CodeResToInitFunc()` switch with a Rust registry plus a mandatory complete descriptor-template table:

```rust
pub fn descriptor_template_for_species(species: SpeciesId) -> RaceDescTemplate {
    match species {
        SpeciesId::Arilou => ARILOU_TEMPLATE,
        SpeciesId::Chmmr => CHMMR_TEMPLATE,
        // ... all 28 races
    }
}
```

Live combat behavior registration may still arrive in race batches, but metadata/template coverage for all 28 species is a prerequisite for loader/catalog work.

### Integration boundary with battle/combat

The battle engine (`battle.c`, `tactrans.c`) remains in C. Integration is via FFI:

- **C calls Rust** for: descriptor creation/loading, catalog queries, queue helpers, spawn/lifecycle entrypoints, per-frame ship callbacks, writeback helpers
- **Rust calls C** for: resource loading/freeing, element creation/manipulation on the display list, collision queries, sound/graphics API, campaign/module-state reads needed by configurable ships, and battle-engine services that remain C-owned
- **Shared data crosses FFI** as: either shared bindgen-generated/layout-compatible C structs or opaque handles with explicit ownership/lifetime rules; no ABI-facing function may return ownership-ambiguous `*const c_void` values

The boundary is: Rust owns ship behavior, descriptor logic, metadata policy, and helper logic. C continues to own the battle loop, display list, selection policy, and any queue/catalog/runtime storage that has not been explicitly migrated behind a thin adapter. The FFI contract is established early and treated as a prerequisite for loader/runtime/lifecycle work.

### Ownership model decision

This plan uses a **C-owned runtime-storage + Rust helper/behavior** model for the first complete port. Specifically:

- C-owned `STARSHIP`, `SHIP_FRAGMENT`, `FLEET_INFO`, and side-queue/runtime storage remain canonical at the integration boundary.
- Rust-owned `MASTER_SHIP_INFO` catalog storage is the explicit exception, exposed through stable typed pointers with documented lifetime in the Phase 03.5 ABI contract.
- Rust logic operates on either bindgen-generated/shared-layout views of C-owned structures or explicit typed Rust-owned catalog snapshots where metadata stability is required.
- Rust does **not** introduce parallel canonical global queues/runtime state that could diverge from C state.
- If a later cleanup chooses Rust-owned canonical storage more broadly, that is a separate follow-up after parity is reached.

### C files replaced

| C File | Replaced By | Notes |
|--------|-------------|-------|
| `sc2/src/uqm/races.h` (types only) | `rust/src/ships/types.rs` plus bindgen/shared-layout views | C header remains authoritative for ABI surface |
| `sc2/src/uqm/dummy.c` | `rust/src/ships/registry.rs` | Ship dispatch/registration |
| `sc2/src/uqm/loadship.c` | `rust/src/ships/loader.rs` | Two-tier loading |
| `sc2/src/uqm/master.c` | `rust/src/ships/catalog.rs` | Master ship catalog logic |
| `sc2/src/uqm/build.c` | `rust/src/ships/queue.rs` | Queue/fragment/build helpers over canonical boundary storage |
| `sc2/src/uqm/ship.c` | `rust/src/ships/runtime.rs` | Shared ship runtime pipeline |
| `sc2/src/uqm/init.c` (ship-runtime-owned parts only) | `rust/src/ships/lifecycle.rs` | `InitShips`/`UninitShips` plus ship-runtime asset participation, not wholesale battle-environment ownership |
| `sc2/src/uqm/ships/androsyn/androsyn.c` | `rust/src/ships/races/androsynth.rs` | Per-race impl |
| `sc2/src/uqm/ships/arilou/arilou.c` | `rust/src/ships/races/arilou.rs` | Per-race impl |
| ... (all 28 race dirs) | `rust/src/ships/races/*.rs` | Per-race impls |

### C files modified (guarded, not deleted)

| C File | Change |
|--------|--------|
| `sc2/src/uqm/ship.c` | Guard body behind `#ifndef USE_RUST_SHIPS` |
| `sc2/src/uqm/loadship.c` | Guard body behind `#ifndef USE_RUST_SHIPS` |
| `sc2/src/uqm/master.c` | Guard body behind `#ifndef USE_RUST_SHIPS` |
| `sc2/src/uqm/build.c` | Guard body behind `#ifndef USE_RUST_SHIPS` |
| `sc2/src/uqm/dummy.c` | Guard body behind `#ifndef USE_RUST_SHIPS` |
| `sc2/src/uqm/init.c` | Guard ship-runtime functions behind `#ifndef USE_RUST_SHIPS`; broader environment orchestration remains C-owned where required by the boundary |
| `sc2/build/unix/build.config` | Add `USE_RUST_SHIPS` toggle |
| `sc2/config_unix.h` | Add `#define USE_RUST_SHIPS` |

## Rust Module Structure

```text
rust/src/ships/
  mod.rs                    # Module root, pub exports
  types.rs                  # SpeciesId, descriptor/runtime helper types,
                            #   C-layout bridge wrappers, flags, metadata snapshots
  traits.rs                 # ShipBehavior trait, CollisionHandler
  ffi_contract.rs           # Early C/Rust ABI, ownership, lifetime contracts
  registry.rs               # template table + behavior dispatch, replaces dummy.c
  loader.rs                 # load_ship(), free_ship(), two-tier loading
  catalog.rs                # master catalog metadata snapshots, sorted lookups
  queue.rs                  # Build(), queue/fragment helpers over canonical C-owned storage
  runtime.rs                # ship_preprocess, ship_postprocess, collision, movement pipeline
  lifecycle.rs              # init_ships(), uninit_ships(), spawn_ship(), ship-runtime asset init
  writeback.rs              # Crew writeback, UpdateShipFragCrew bookkeeping
  ffi.rs                    # All extern "C" FFI exports and C-to-Rust bridge
  c_bridge.rs               # Rust-to-C calls (element ops, sound, graphics, campaign/module queries)
  races/
    mod.rs                  # Race module root, all race re-exports
    androsynth.rs
    arilou.rs
    black_urquan.rs
    chenjesu.rs
    chmmr.rs
    druuge.rs
    human.rs
    ilwrath.rs
    samatra.rs
    melnorme.rs
    mmrnmhrm.rs
    mycon.rs
    orz.rs
    pkunk.rs
    probe.rs
    shofixti.rs
    sis_ship.rs
    slylandro.rs
    spathi.rs
    supox.rs
    syreen.rs
    thraddash.rs
    umgah.rs
    urquan.rs
    utwig.rs
    vux.rs
    yehat.rs
    zoqfotpik.rs
```

## Phase Structure

| Phase | Title | Gaps Addressed | Est. LoC |
|-------|-------|---------------|----------|
| P00.5 | Preflight Verification | -- | 0 |
| P01 | Analysis | canonical requirement index, subsystem inventory | 0 |
| P01a | Analysis Verification | -- | 0 |
| P02 | Pseudocode | -- | 0 |
| P02a | Pseudocode Verification | -- | 0 |
| P03 | Core Types & Enums | Shared types, bridge-safe wrappers, requirement normalization | ~800 |
| P03a | Types Verification | -- | 0 |
| P03.5 | FFI Boundary & Ownership Contract | ABI/lifetime/layout contract, explicit per-type decisions, early mixed-path smoke surface | ~350 |
| P03.5a | Boundary Verification | -- | 0 |
| P04 | ShipBehavior Trait & Registry | Hook contract, full 28-species template coverage, metadata-safe dispatch | ~400 |
| P04a | Trait Verification | -- | 0 |
| P05 | Two-Tier Loader | Loading, freeing, metadata/resource bridge, separation of metadata vs live behavior coverage | ~500 |
| P05a | Loader Verification | -- | 0 |
| P06 | Master Ship Catalog | Catalog snapshots, stable metadata ownership, lookups without live-behavior dependency | ~450 |
| P06a | Catalog Verification | -- | 0 |
| P07 | Queue & Build Primitives | Queue, fragment, fleet-info helpers over canonical boundary state, including campaign/build parity helpers | ~600 |
| P07a | Queue Verification | -- | 0 |
| P08 | Shared Runtime Pipeline | Preprocess, postprocess, movement, energy, gravity/AI/collision semantics | ~700 |
| P08a | Runtime Verification | -- | 0 |
| P09 | Ship Spawn & Lifecycle | spawn_ship, init/uninit_ships, explicit split between ship-runtime and C battle-environment ownership | ~500 |
| P09a | Spawn Verification | -- | 0 |
| P10 | Crew Writeback & Death | Writeback bookkeeping, fragment matching, death | ~400 |
| P10a | Writeback Verification | -- | 0 |
| P11 | Race Batch 1 — Simple Ships (8 races) | Per-race behavior | ~2400 |
| P11a | Batch 1 Verification | -- | 0 |
| P12 | Race Batch 2 — Mode-Switching Ships (8 races) | Per-race behavior | ~3200 |
| P12a | Batch 2 Verification | -- | 0 |
| P13 | Race Batch 3 — Complex & Non-Melee Ships (12 races) | Per-race behavior, SIS/final-battle integration | ~4000 |
| P13a | Batch 3 Verification | -- | 0 |
| P14 | C-Side Bridge Wiring | Build toggle, guards, final wiring enablement | ~400 (C) |
| P14a | Bridge Verification | -- | 0 |
| P15 | End-to-End Integration & Verification | All | ~200 |

Total estimated new/modified LoC: ~14,900 (Rust) + ~400 (C)

## Execution Order

```text
P00.5 -> P01 -> P01a -> P02 -> P02a
       -> P03 -> P03a -> P03.5 -> P03.5a
       -> P04 -> P04a -> P05 -> P05a
       -> P06 -> P06a -> P07 -> P07a
       -> P08 -> P08a -> P09 -> P09a
       -> P10 -> P10a -> P11 -> P11a
       -> P12 -> P12a -> P13 -> P13a
       -> P14 -> P14a -> P15
```

Each phase MUST be completed and verified before the next begins. No skipping.

## Critical Reminders

Before implementing any phase:
1. Preflight verification is complete (Phase 0.5)
2. Integration points are explicitly listed
3. TDD cycle is defined per slice
4. Lint/test/coverage gates are declared
5. No phase may assume unresolved ABI ownership, pointer lifetime, or C-layout details
6. P05/P06 may not depend on later live race batches for metadata completeness
7. Ship-runtime lifecycle work may not silently absorb broader battle-environment ownership from `init.c`

## Definition of Done

1. All `cargo test --workspace --all-features` pass
2. All `cargo clippy --workspace --all-targets --all-features -- -D warnings` pass
3. `cargo fmt --all --check` passes
4. Mixed C/Rust smoke tests pass for loader, catalog, spawn, callbacks, teardown, queue helpers, campaign helper wiring, and symbol/link validation before the final end-to-end phase
5. Game boots with `USE_RUST_SHIPS=1` and SuperMelee combat works
6. All 28 race ships spawn, fight, die, and transition correctly
7. Master ship catalog matches C: same ships, same costs, same sort order
8. Two-tier loading works: metadata-only for catalog, battle-ready for combat
9. Crew writeback preserves campaign encounter semantics
10. Non-melee ships (SIS, Sa-Matra, Probe) function correctly
11. No descriptor mutation restrictions break existing race behaviors
12. `races.h` C types remain available for C consumers of shared enums
13. No placeholder stubs or panic-on-use temporary implementations remain in implementation code
14. Every FFI-facing function has an explicit signature, ownership, lifetime, and layout contract in the plan
15. Full metadata/template coverage for all 28 species exists before loader/catalog completion
16. Campaign/build helper parity covers escort feasibility, sphere tracking, and required fleet-info semantics from the active boundary
17. The split between ships-owned runtime init and C-owned battle/environment orchestration is explicit and preserved

## Race Assignment to Batches

### Batch 1 — Simple Ships (P11): Straightforward weapon/special, minimal state
1. Arilou (autoaim laser + teleport)
2. Earthling/Human (point-defense laser + nuke)
3. Spathi (forward gun + rear BUTT missile)
4. Supox (lateral thrust special)
5. Thraddash (afterburner trail)
6. Yehat (pulse cannon + shield)
7. Druuge (recoil cannon + crew sacrifice)
8. Ilwrath (hellfire + cloak)

### Batch 2 — Mode-Switching & Complex State Ships (P12): Private data, mode switches
1. Androsynth (bubble/blazer mode switch, private data)
2. Mmrnmhrm (X-form dual mode transform)
3. Orz (marine boarding, space marine elements)
4. Pkunk (resurrection, insult)
5. Shofixti (glory device self-destruct)
6. Syreen (siren song crew steal)
7. Utwig (energy absorb shield)
8. Vux (limpet + warp-in advantage, private data)

### Batch 3 — Complex & Non-Melee Ships (P13): Heavy state, unique mechanics
1. Chmmr (ZapSat satellites, tractor beam)
2. Chenjesu (crystal shard + DOGI)
3. Mycon (homing plasmoid, regeneration)
4. Melnorme (charge-shot, confusion)
5. Umgah (antimatter cone, zip)
6. Ur-Quan (fighters + fusion blast)
7. Kohr-Ah/Black Ur-Quan (spinning blades + FRIED)
8. Slylandro (probe lightning)
9. ZoqFotPik (tongue + stinger)
10. SIS Ship (configurable flagship, non-melee)
11. Sa-Matra (final battle, non-melee)
12. Probe (autonomous, non-melee)

## Plan Files

```text
plan/
  00-overview.md                                (this file)
  00a-preflight-verification.md                 P00.5
  01-analysis.md                                P01
  01a-analysis-verification.md                  P01a
  02-pseudocode.md                              P02
  02a-pseudocode-verification.md                P02a
  03-core-types-enums.md                        P03
  03a-core-types-enums-verification.md          P03a
  03b-ffi-boundary-ownership.md                 P03.5
  03b-ffi-boundary-ownership-verification.md    P03.5a
  04-trait-registry.md                          P04
  04a-trait-registry-verification.md            P04a
  05-two-tier-loader.md                         P05
  05a-two-tier-loader-verification.md           P05a
  06-master-catalog.md                          P06
  06a-master-catalog-verification.md            P06a
  07-queue-build-primitives.md                  P07
  07a-queue-build-primitives-verification.md    P07a
  08-shared-runtime-pipeline.md                 P08
  08a-shared-runtime-pipeline-verification.md   P08a
  09-spawn-lifecycle.md                         P09
  09a-spawn-lifecycle-verification.md           P09a
  10-crew-writeback-death.md                    P10
  10a-crew-writeback-death-verification.md      P10a
  11-race-batch-1-simple.md                     P11
  11a-race-batch-1-simple-verification.md       P11a
  12-race-batch-2-mode-switching.md             P12
  12a-race-batch-2-mode-switching-verification.md P12a
  13-race-batch-3-complex-nonmelee.md           P13
  13a-race-batch-3-complex-nonmelee-verification.md P13a
  14-c-side-bridge-wiring.md                    P14
  14a-c-side-bridge-wiring-verification.md      P14a
  15-e2e-integration-verification.md            P15
  execution-tracker.md
```

## Deferred Items

The following are explicitly out of scope:

- **Battle engine porting**: `battle.c`, `tactrans.c`, `pickship.c` remain in C. This plan ports ships, not the combat loop.
- **Netplay synchronization**: Ship selection sync for netplay is handled at the battle/netplay boundary, not in ships.
- **Advanced AI strategy**: The AI intelligence hook is ported per-race, but AI strategy improvements are not in scope.
- **Save/load of ship private data**: Race-specific private data (`RACE_DESC.data`) is transient combat state. Save/load compatibility is a campaign subsystem concern.
- **HQxx scalers / rendering pipeline**: Ship frame rendering is a graphics subsystem concern.
