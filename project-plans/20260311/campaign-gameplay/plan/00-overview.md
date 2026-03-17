# Plan: Campaign Gameplay Subsystem — Full Rust Port

Plan ID: PLAN-20260314-CAMPAIGN
Generated: 2026-03-14
Total Phases: 35 (P00.5 through P16, with verification sub-phases and one inserted bridge phase)
Requirements: All requirements from `../requirements.md` and controlling subsystem contract/rules from `../specification.md` (campaign loop, transitions, encounter handoff, starbase flow, event progression, save/load, legacy compatibility, verifier-facing inspection obligations)

## Context

## Normative Inputs

- `project-plans/20260311/campaign-gameplay/requirements.md` — verifier-facing obligations and requirement families for this subsystem plan
- `project-plans/20260311/campaign-gameplay/specification.md` — controlling subsystem contract, section-number references, claim-classification rules, inspection-surface rules, and covered-context definitions used throughout this plan
- `project-plans/20260311/campaign-gameplay/plan/*.md` — execution sequencing, implementation tasks, and verification gates for this plan package

The campaign-gameplay subsystem is **entirely unported**. No Rust code exists for campaign flow. All campaign-boundary behavior — new/load entry handling, top-level activity dispatch, hyperspace/encounter/starbase transitions, event handlers, save/load serialization, and campaign-owned verification/export surfaces — remains in C across 30+ files. This is the largest and most complex subsystem, depending on nearly every other subsystem (clock, state, comm, battle, planets, graphics, input, audio, resource, file-I/O).

The subsystem already has **live indirect Rust edges** through `USE_RUST_CLOCK` (game clock backed by `rust/src/time/`) and `USE_RUST_STATE` (game-state bits/state-files backed by `rust/src/state/`). The port must integrate with these existing Rust dependencies and provide a new `USE_RUST_CAMPAIGN` build toggle for incremental adoption.

### C Source Files in Scope

| File | Responsibility | Est. C LoC |
|------|---------------|-----------|
| `sc2/src/uqm/starcon.c` | Campaign-loop-adjacent dispatch and kernel lifecycle seams to validate | ~300 |
| `sc2/src/uqm/restart.c` | New/load/restart entry flow seams to validate | ~400 |
| `sc2/src/uqm/hyper.c` | Hyperspace runtime, transitions, menu | ~1700 |
| `sc2/src/uqm/encount.c` | Encounter handoff, battle segue, post-encounter | ~900 |
| `sc2/src/uqm/starbase.c` | Starbase visit flow, departure | ~550 |
| `sc2/src/uqm/gameev.c` | Campaign event handlers | ~250 |
| `sc2/src/uqm/clock.c` / `clock_rust.c` | Clock integration (already Rust-backed) | ~300 |
| `sc2/src/uqm/save.c` | Campaign save serialization | ~800 |
| `sc2/src/uqm/load.c` | Campaign load deserialization | ~800 |
| `sc2/src/uqm/globdata.c/.h` | GAME_STATE, activity flags, queues | ~1000 |

**Total C LoC in scope: ~7000**

## Port Strategy

The port follows a **bottom-up, seam-validated FFI-bridge** strategy consistent with other subsystem ports in this project, but it does **not** assume up front that Rust will directly own all top-level legacy C entrypoints. Concrete bridge seams, replacement boundaries, and signatures are treated as validated outputs of analysis/preflight rather than pre-decided facts. That flexibility stops once the seam inventory is complete: wiring the live campaign-loop owner in `sc2/src/uqm/starcon.c` and the start/load owner in `sc2/src/uqm/restart.c` is a mandatory implementation obligation of this plan, not an optional late integration candidate.

1. **Validated seam inventory first** — Confirm concrete C↔Rust integration seams, ownership boundaries, queue/state access strategy, and which legacy functions are actually replaced versus wrapped
2. **Types and boundary representations** — Define Rust types for campaign activity vocabulary, transition flags, save summary, event catalog, and boundary-safe access representations
3. **Freeze verifier-facing persistence/export contract early** — Finalize canonical export schema, claim-family surface selection, malformed-save error/result shape, and verifier-report entry schema before save/load semantics harden around the wrong inspection objects
4. **Pure logic extraction** — Port deterministic campaign logic (event handlers, save summary derivation, post-encounter processing, validation/export logic) as Rust functions testable without broad C ownership assumptions
5. **Accessor/bridge layer before deep orchestration** — Create C-state accessors and ownership rules before implementing loop/stateful orchestration that depends on legacy-owned globals and queues
6. **Integration wiring** — Introduce validated `extern "C"` exports/imports, mandatory `restart.c`/`starcon.c` owner-seam wiring, guard only confirmed replacement seams, wire build system, verify end-to-end

### Rust Module Structure

```
rust/src/campaign/
  mod.rs                    — Module root, re-exports
  activity.rs               — CampaignActivity enum, transition flags, dispatch model
  session.rs                — CampaignSession state / boundary-owned runtime state
  state_bridge.rs           — C-state accessor bridge, queue readers/writers, ownership helpers
  loop_dispatch.rs          — Main campaign loop and dispatch logic
  transitions.rs            — Hyperspace→encounter, hyperspace→interplanetary, quasispace transitions
  encounter.rs              — Encounter handoff, BuildBattle, post-encounter processing
  starbase.rs               — Starbase visit flow, special sequences, departure
  events/
    mod.rs                  — Event catalog, EventSelector enum, handler dispatch
    handlers.rs             — Individual event handler implementations
    registration.rs         — Initial event registration (AddInitialGameEvents)
  save/
    mod.rs                  — Save/load module root
    serialize.rs            — Campaign save serialization
    deserialize.rs          — Campaign load deserialization
    summary.rs              — Save summary derivation and normalization
    validation.rs           — Semantic validation, rejection cases (§9.4.1)
    legacy.rs               — Legacy save format compatibility
    export.rs               — Campaign Canonical Export Document (§10.1)
  hyper_menu.rs             — Hyperspace menu campaign-facing orchestration
  ffi.rs                    — Validated extern "C" FFI exports for confirmed C bridge seams
  types.rs                  — Shared types (coordinates, identity tokens, queue-entry representations)
```

## Gap Summary

| # | Gap | Severity | Key Requirements |
|---|-----|----------|-----------------|
| G1 | No campaign activity enum or dispatch model in Rust | Critical | Campaign loop and activity dispatch (all) |
| G2 | No campaign session/runtime state container | Critical | §3.2 runtime state, save/load |
| G2a | No validated C-state accessor bridge or queue ownership model | Critical | §2.2 boundary ownership, §3.2 runtime state, §9.4 load semantics |
| G3 | No main campaign loop implementation | Critical | Campaign loop requirements |
| G4 | No transition logic (hyper→encounter, hyper→interplanetary, quasispace) | Critical | Hyperspace and navigation transitions |
| G5 | No encounter handoff (BuildBattle, EncounterBattle, UninitEncounter) | Critical | Encounter handoff requirements |
| G6 | No starbase visit flow | Critical | Starbase visit flow, save/load resume |
| G7 | No campaign event handlers | Critical | Event progression requirements, §8.6 catalog |
| G8 | No save serialization | Critical | Campaign save requirements |
| G9 | No load deserialization | Critical | Campaign load requirements |
| G10 | No save summary derivation | High | §9.2 summary normalization |
| G11 | No legacy save compatibility layer | High | Legacy save compatibility requirements |
| G12 | No semantic validation for restored schedule state | High | §9.4.1 rejection cases |
| G13a | No canonical export document types/schema in Rust | Medium | §10.1 export surface |
| G13b | No canonical export implementation and verifier reporting flow | Medium | §10.1 export surface, requirements verifier-report obligations |
| G14 | No deferred-transition mechanism | High | §5.3 deferred transitions |
| G15 | No hyperspace menu campaign orchestration | Medium | §7.6 hyperspace menu |
| G16 | No `USE_RUST_CAMPAIGN` build toggle or validated C-side guards | Critical | Integration |
| G17 | No safe-failure load contract implementation | High | §9.4.0b load-failure guarantees |
| G18 | No clock-rate policy integration | Medium | §8.4 clock rate policy |
| G19 | No REQ-level traceability / claim-surface verification matrix | High | `requirements.md` verifier-facing obligations |
| G20 | No fixture/harness corpus for covered-context persistence, corrupt-save, adjunct-failure, and cross-surface verification evidence | High | §9.4.0b safe-failure evidence, §9.7 covered contexts, §10.1 verifier-facing inspection obligations |

## Phase Structure

| Phase | Title | Gaps Addressed | Est. LoC |
|-------|-------|---------------|----------|
| P00.5 | Preflight Verification | — | 0 |
| P01 | Analysis | — | 0 |
| P01a | Analysis Verification | — | 0 |
| P02 | Pseudocode | — | 0 |
| P02a | Pseudocode Verification | — | 0 |
| P03 | Types, Enums, Domain Model | G1, G2 | ~600 |
| P03a | Types Verification | — | 0 |
| P03.5 | C-State Accessor Bridge & Ownership Model | G2a | ~500 |
| P03.5a | C-State Accessor Bridge Verification | — | 0 |
| P04 | Campaign Event Catalog & Handlers | G7 | ~900 |
| P04a | Event Handlers Verification | — | 0 |
| P05 | Save Summary & Export Types | G10, G13a, G13b, G19 | ~750 |
| P05a | Save Summary Verification | — | 0 |
| P06 | Save Serialization | G8 | ~800 |
| P06a | Save Serialization Verification | — | 0 |
| P07 | Load Deserialization & Validation | G9, G12, G17 | ~900 |
| P07a | Load Verification | — | 0 |
| P08 | Legacy Save Compatibility | G11 | ~500 |
| P08a | Legacy Save Verification | — | 0 |
| P09 | Deferred Transitions & Activity Dispatch | G3, G14 | ~600 |
| P09a | Dispatch Verification | — | 0 |
| P10 | Hyperspace & Navigation Transitions | G4 | ~700 |
| P10a | Transitions Verification | — | 0 |
| P11 | Encounter Handoff & Post-Encounter | G5 | ~800 |
| P11a | Encounter Verification | — | 0 |
| P12 | Starbase Visit Flow | G6 | ~700 |
| P12a | Starbase Verification | — | 0 |
| P13 | Hyperspace Menu & Clock Rate Policy | G15, G18 | ~400 |
| P13a | Hyper Menu Verification | — | 0 |
| P14 | Campaign Canonical Export Document Execution | G13b, G19 | ~350 |
| P14a | Export Verification | — | 0 |
| P15 | C-Side Bridge & Build Toggle | G16 | ~500 (C) |
| P15a | Bridge Verification | — | 0 |
| P16 | End-to-End Integration & Verification | All, G20 | ~300 |

Total estimated new/modified LoC: ~8000 (Rust) + ~500 (C bridge)

## Execution Order

```
P00.5 -> P01 -> P01a -> P02 -> P02a
       -> P03 -> P03a -> P03.5 -> P03.5a
       -> P04 -> P04a -> P05 -> P05a
       -> P06 -> P06a -> P07 -> P07a
       -> P08 -> P08a -> P09 -> P09a
       -> P10 -> P10a -> P11 -> P11a
       -> P12 -> P12a -> P13 -> P13a
       -> P14 -> P14a -> P15 -> P15a
       -> P16
```

Each phase MUST be completed and verified before the next begins. No skipping.

## Dependency Map

### Subsystems This Plan Depends On (Must Be Available)

| Subsystem | Rust Module | Toggle | Status | Used For |
|-----------|------------|--------|--------|----------|
| Clock/Time | `rust/src/time/` | `USE_RUST_CLOCK` | [OK] Ported | Game clock, event scheduling, date/tick |
| State | `rust/src/state/` | `USE_RUST_STATE` | [OK] Ported | Game-state bits, state files |
| Comm | `rust/src/comm/` | `USE_RUST_COMM` | Partial | Encounter communication dispatch |
| Game Init | `rust/src/game_init/` | — | [OK] Ported | Master ship list, initialization |
| File I/O | `rust/src/io/` | `USE_RUST_FILE` | [OK] Ported | Save file I/O |
| Resource | `rust/src/resource/` | `USE_RUST_RESOURCE` | [OK] Ported | Resource loading |
| Input | `rust/src/input/` | `USE_RUST_INPUT` | [OK] Ported | Player input handling |
| Graphics | `rust/src/graphics/` | `USE_RUST_GFX` | Partial | Rendering (used via FFI) |
| Sound | `rust/src/sound/` | `USE_RUST_AUDIO` | [OK] Ported | Music/SFX |
| Threading | `rust/src/threading/` | `USE_RUST_THREADS` | [OK] Ported | Thread primitives |

### Subsystems That Will Depend On This

| Subsystem | Integration Point |
|-----------|------------------|
| Planet/SolarSys | Campaign dispatches to `ExploreSolarSys()` |
| Battle/Combat | Campaign invokes `Battle()` via encounter handoff |
| SuperMelee | Separate branch from restart flow (not owned here) |

## Critical Reminders

Before implementing any phase:
1. Preflight verification is complete (Phase 0.5)
2. Integration points are explicitly listed and validated, not assumed
3. TDD cycle is defined per slice
4. Lint/test/coverage gates are declared
5. REQ-level traceability and verifier-facing inspection obligations remain preserved

## Definition of Done

1. All `cargo test --workspace --all-features` pass
2. All `cargo clippy --workspace --all-targets --all-features -- -D warnings` pass
3. `cargo fmt --all --check` passes
4. Game boots with `USE_RUST_CAMPAIGN=1` and campaign gameplay works correctly
5. New-game entry reaches Sol in interplanetary mode
6. Save/load round-trip preserves all campaign-boundary observables
7. Legacy C saves load correctly with semantic equivalence
8. All 18 campaign event handlers produce correct campaign effects
9. Starbase visit flow handles all mandatory special sequences
10. Deferred transitions work without fake-load side effects
11. Campaign Canonical Export Document produces valid JSON for contexts whose chosen inspection surface requires export
12. Load-failure safe-failure guarantees hold (no partial state application, no save/adjunct mutation)
13. `sc2/src/uqm/restart.c` and `sc2/src/uqm/starcon.c` are wired at validated runtime-owner seams so `USE_RUST_CAMPAIGN=1` actually switches live start/load flow and main campaign dispatch to Rust
14. C-side files are guarded only at validated replacement seams behind `#ifndef USE_RUST_CAMPAIGN`
15. No placeholder stubs or TODO markers remain in implementation code
16. All 27 race scripts compile without modification against updated headers
17. REQ-level traceability appendix and claim-surface verification matrix are complete and exercised
18. Verifier-facing reports capture claim-local result, overall covered-context result, and adjunct sensitivity where required

## Plan Files

```
plan/
  00-overview.md                              (this file)
  00a-preflight-verification.md               P00.5
  01-analysis.md                              P01
  01a-analysis-verification.md                P01a
  02-pseudocode.md                            P02
  02a-pseudocode-verification.md              P02a
  03-types-domain-model.md                    P03
  03a-types-domain-model-verification.md      P03a
  03b-c-state-accessor-bridge.md              P03.5
  03ba-c-state-accessor-bridge-verification.md P03.5a
  04-event-catalog-handlers.md                P04
  04a-event-catalog-handlers-verification.md  P04a
  05-save-summary-export-types.md             P05
  05a-save-summary-export-types-verification.md  P05a
  06-save-serialization.md                    P06
  06a-save-serialization-verification.md      P06a
  07-load-deserialization-validation.md       P07
  07a-load-deserialization-validation-verification.md  P07a
  08-legacy-save-compatibility.md             P08
  08a-legacy-save-compatibility-verification.md  P08a
  09-deferred-transitions-dispatch.md         P09
  09a-deferred-transitions-dispatch-verification.md  P09a
  10-hyperspace-navigation-transitions.md     P10
  10a-hyperspace-navigation-transitions-verification.md  P10a
  11-encounter-handoff-post-encounter.md      P11
  11a-encounter-handoff-post-encounter-verification.md  P11a
  12-starbase-visit-flow.md                   P12
  12a-starbase-visit-flow-verification.md     P12a
  13-hyper-menu-clock-rate.md                 P13
  13a-hyper-menu-clock-rate-verification.md   P13a
  14-canonical-export-document.md             P14
  14a-canonical-export-document-verification.md  P14a
  15-c-side-bridge-build-toggle.md            P15
  15a-c-side-bridge-build-toggle-verification.md  P15a
  16-e2e-integration-verification.md          P16
  execution-tracker.md
  requirements-traceability.md
```

## Deferred Items

The following are explicitly out of scope:

- **Solar-system exploration internals**: Campaign dispatches to `ExploreSolarSys()` but does not own orbit/scan/surface/lander
- **Dialogue tree porting**: 27 race scripts remain in C; comm subsystem owns their runtime
- **Battle simulation porting**: Campaign invokes `Battle()` but does not own combat internals
- **SuperMelee flow**: Separate branch from restart flow, outside campaign boundary
- **HQxx scalers / advanced graphics**: Graphics subsystem concerns
- **Netplay protocol**: Network subsystem concern
- **Binary-compatible save format**: Only semantic compatibility with legacy saves is required (§10.1)
- **Exact encounter generation parameters**: Open audit-sensitive area (§11)
- **Exact clock tick-rate values**: Open audit-sensitive area (§11)
