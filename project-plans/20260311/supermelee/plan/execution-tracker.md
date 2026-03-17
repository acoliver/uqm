# Execution Tracker

Plan ID: PLAN-20260314-SUPERMELEE
Generated: 2026-03-14

| Phase | Title | Status | Verified | Semantic Verified | Notes |
|------:|-------|--------|----------|-------------------|-------|
| P00   | Overview | ⬜ | N/A | N/A | Scoped plan structure |
| P00.5 | Preflight Verification | ⬜ | ⬜ | N/A | Toolchain + real dependency/interface audit |
| P01   | Analysis | ⬜ | ⬜ | ⬜ | Scoped SuperMelee model + boundary analysis |
| P02   | Pseudocode | ⬜ | ⬜ | ⬜ | Team/persistence/setup/selection/netplay/audit |
| P03   | Core Types — Stub | ⬜ | ⬜ | ⬜ | MeleeShip, team model, error types |
| P04   | Core Types — TDD | ⬜ | ⬜ | ⬜ | Core model tests |
| P05   | Core Types — Impl | ⬜ | ⬜ | ⬜ | Enum conversion, serialization, bounded names |
| P06   | Team Model & Persistence | ⬜ | ⬜ | ⬜ | `.mle`, `melee.cfg`, built-in catalog |
| P07   | Setup Menu & Fleet Ship Pick | ⬜ | ⬜ | ⬜ | `Melee()`, picker, local start flow |
| P08   | Combatant Selection Contract | ⬜ | ⬜ | ⬜ | Initial/next combatants, battle-ready handoff |
| P09   | Netplay Boundary Surface & Validation | ⬜ | ⬜ | ⬜ | Setup sync, start gating, remote validation |
| P10   | Compatibility Audit Decision Points | ⬜ | ⬜ | ⬜ | Built-ins/save-format/UI timing audit |
| P11   | FFI Bridge & C Wiring | ⬜ | ⬜ | ⬜ | Scoped setup/selection entry points only |
| P12   | Requirement Traceability Matrix | ⬜ | ⬜ | ⬜ | Statement-level REQ coverage |
| P13   | E2E Local Integration | ⬜ | ⬜ | ⬜ | Local setup/persistence/handoff verification |
| P14   | E2E Netplay Boundary | ⬜ | ⬜ | ⬜ | Netplay boundary verification |
| P15   | Final Integration Signoff | ⬜ | ⬜ | ⬜ | Matrix-driven final signoff |

## Phase Dependencies

```text
P00.5 -> P01 -> P02 -> P03 -> P04 -> P05 -> P06 -> P07 -> P08 -> P09 -> P10 -> P11 -> P12 -> P13 -> P14 -> P15
```

All phases are strictly sequential. No phase may begin until the prior phase's verification has passed.

## Estimated Scope

| Category | LoC |
|----------|-----|
| Core types (`types.rs`, `error.rs`) | ~500 |
| Team model + persistence | ~1200 |
| Setup menu + ship pick | ~1200 |
| Combatant selection contract | ~700 |
| Netplay boundary | ~800 |
| FFI bridge | ~500 |
| Tests / verification harness | ~1800 |
| **Total** | **~6700** |

## Module Structure Created

```text
rust/src/supermelee/
├── mod.rs
├── types.rs                  # P03–P05
├── error.rs                  # P03–P05
├── c_bridge.rs               # P11
└── setup/
    ├── mod.rs
    ├── team.rs               # P03–P06
    ├── persistence.rs        # P06
    ├── config.rs             # P06
    ├── melee.rs              # P07
    ├── build_pick.rs         # P07
    ├── pick_melee.rs         # P08
    ├── netplay_boundary.rs   # P09
    └── ffi.rs                # P11
```

## C Files Replaced (Scoped SuperMelee ownership only)

| C File | Rust Replacement | Phase |
|--------|-----------------|-------|
| `sc2/src/uqm/supermelee/melee.c` | `setup/melee.rs` | P07, P11 |
| `sc2/src/uqm/supermelee/meleesetup.c` | `setup/team.rs` | P05, P11 |
| `sc2/src/uqm/supermelee/loadmele.c` | `setup/persistence.rs` | P06, P11 |
| `sc2/src/uqm/supermelee/buildpick.c` | `setup/build_pick.rs` | P07, P11 |
| `sc2/src/uqm/supermelee/pickmele.c` | `setup/pick_melee.rs` | P08, P11 |

## Explicitly Out-of-Scope Dependencies

These files/systems remain integration dependencies and are not port targets in this plan:
- `sc2/src/uqm/battle.c`
- `sc2/src/uqm/process.c`
- `sc2/src/uqm/collide.c`
- `sc2/src/uqm/ship.c`
- `sc2/src/uqm/intel.c`
- `sc2/src/uqm/tactrans.c`
- generic graphics/input/audio/resource/file-I/O/threading subsystem internals
- netplay transport/protocol stack internals

## Boundary Contracts Requiring Audit

1. **Battle-facing combatant contract** — exact queue-entry/handle type returned for initial/next combatants
2. **Netplay boundary contract** — exact setup-sync and selection-update hook signatures
3. **Compatibility obligations** — whether exact built-in content, save bytes, and UI timing must preserve legacy parity or only semantic behavior
