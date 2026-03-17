# Phase 01: Analysis

## Phase ID
`PLAN-20260314-CAMPAIGN.P01`

## Prerequisites
- Required: Phase 00.5 (Preflight Verification) completed
- All blocking issues resolved

## Purpose

Produce domain model, state transition analysis, integration touchpoint inventory, validated seam inventory, boundary-ownership map, and REQ-level traceability before any implementation.

## Domain Model

### Entity Analysis

#### CampaignActivity (§3.1)
Observable campaign modes that determine dispatch, resume, and verification:
- `HyperspaceNavigation` — player traversing hyperspace/quasispace
- `Interplanetary` — player in a solar system
- `Encounter` — campaign encounter being resolved (communication/battle)
- `StarbaseVisit` — player visiting allied starbase
- `LastBattle` — final story battle in progress

Terminal outcomes (not modes): Victory, Defeat

#### Transition Flags (§3.1)
- `StartEncounter` — request encounter/starbase entry from hyperspace/interplanetary
- `StartInterplanetary` — request interplanetary entry from hyperspace
- `CheckLoad` — load requested from sub-activity
- `CheckRestart` — restart requested
- `CheckAbort` — abort requested

#### CampaignSession (§3.2)
Runtime state container holding campaign-owned state plus references or bridge handles to lower-boundary state:
- Current activity mode
- Pending transition flags
- Game clock reference (owned by clock subsystem)
- Navigation position (hyperspace coords or system identity)
- Encounter queue access handle or verified reader/writer strategy
- NPC ship queue access handle or verified reader/writer strategy
- Escort queue access handle or verified reader/writer strategy
- Game-state bitfield reference (owned by state subsystem)
- Autopilot target
- Ship-state persistence tokens (ship stamp/orientation/velocity or verified equivalent boundary representation)
- Orbit flags / interplanetary resume markers
- Starbase-context marker access strategy

#### EventSelector (§8.6)
Enum with 18 variants matching the campaign event catalog:
`ARILOU_ENTRANCE_EVENT` through `SLYLANDRO_RAMP_DOWN`

#### SaveSummary (§9.2)
- `summary_type`: hyperspace | interplanetary | starbase | encounter | last_battle
- `location_id`: format depends on context
- `date`: campaign date at save time

## State Transition Map

```
[Start Flow] ──new-game──> [Init Campaign] ──> [Main Loop]
[Start Flow] ──load-game──> [Restore State] ──> [Main Loop]

[Main Loop] ──deferred-transition──> [Target Activity]
[Main Loop] ──encounter-requested──> [Encounter/Starbase]
[Main Loop] ──interplanetary-requested──> [Solar System]
[Main Loop] ──default──> [Hyperspace]
[Main Loop] ──victory──> [Exit: Win]
[Main Loop] ──death──> [Exit: Lose]
[Main Loop] ──restart/abort──> [Start Flow]

[Hyperspace] ──collision──> [Encounter via START_ENCOUNTER]
[Hyperspace] ──system-entry──> [Interplanetary via START_INTERPLANETARY]
[Hyperspace] ──quasispace-portal──> [Quasispace/Hyperspace]

[Encounter] ──dialogue-only──> [Post-Encounter Cleanup] ──> [Resume Navigation]
[Encounter] ──battle-segue──> [Battle] ──> [Post-Encounter Cleanup] ──> [Resume Navigation]
[Encounter] ──abort/load/death──> [Suppress Cleanup] ──> [Exit/Load]

[Starbase] ──departure──> [Deferred Transition to Interplanetary]
[Starbase] ──load/abort──> [Exit/Load]

[Solar System] ──done──> [Resume Main Loop]
```

### Error/Edge Case Map

| Scenario | Expected Behavior |
|----------|------------------|
| Load fails (corrupt save) | Safe failure: no partial state, return to start flow |
| Load fails (unknown event selector) | Mandatory rejection per §9.4.1 |
| Load fails (malformed event metadata) | Mandatory rejection per §9.4.1 |
| Load from sub-activity (hyper menu) | Clean exit sub-activity, resume from loaded state |
| Load during starbase visit | Resume at correct starbase progression point |
| Save from special context (homeworld) | Apply save-time adjustments for correct resume |
| Encounter abort | Suppress post-encounter processing |
| Starbase bomb-transport sequence | Special gating before normal menu |
| Starbase pre-alliance Ilwrath battle | Conditional battle, return to conversation |
| Multiple mandatory starbase routes latent | First mandatory route surfaces under zero-input settlement |
| Stale state carry-over on restart | All campaign state must be torn down |
| Required adjunct artifact missing | Mandatory load rejection per §9.4.0b |
| Export succeeds but adjunct requirement fails | Claim-local export pass may still be overall covered-context fail |
| Covered valid legacy starbase save lacks sufficient raw comparison object | Use legacy-starbase observational exception path |

## Validated Integration Touchpoints

### Seam Inventory Policy

This plan does **not** pre-commit to replacing named top-level C functions until P00.5/P01 verify the actual ownership seam in source. Every later FFI export/import and every guarded C body must point back to one row in the validated seam inventory below.

### Existing Callers / Candidate Seams (must be validated, not assumed)

| C File | Candidate Function / Call Site | Candidate Rust Responsibility | Validation Required in P01 |
|--------|-------------------------------|-------------------------------|----------------------------|
| `restart.c` | entry-flow new/load/restart functions | Start-flow orchestration or delegated helper(s) | Confirm exact function names, signatures, and whether Rust replaces function bodies or a narrower helper seam |
| `starcon.c` | campaign-loop-adjacent dispatch / kernel lifecycle call path | Campaign loop orchestration | Confirm exact loop owner, caller/callee shape, and whether Rust owns full loop or only selected dispatch steps |
| `gameev.c` | `AddInitialGameEvents()` / `EventHandler()` | Event registration / event dispatch | Verify direct replacement feasibility and signatures |
| `save.c` | save entrypoints and summary derivation call path | Campaign save / summary logic | Verify exact save seam and whether summary is produced by helper or top-level API |
| `load.c` | load entrypoints and state-restore call path | Campaign load / validation logic | Verify exact load seam and failure propagation path |
| `encount.c` | encounter handoff / post-encounter seams | Encounter campaign-boundary logic | Verify concrete function-level seams |
| `starbase.c` | starbase visit / departure seams | Starbase campaign-boundary logic | Verify concrete function-level seams |
| `hyper.c` | transition and hyperspace-menu seams | Transition orchestration / hyper menu | Verify which functions are campaign-owned versus lower-boundary |
| `globdata.c/.h` | global activity flags / queues / markers | Bridge/accessor layer only | Verify read/write ownership and representation |

### Existing Code Replaced/Removed

No concrete C body is marked for replacement in this phase. Instead, P01 must produce a **validated replacement/guard table** with the following fields for each later P15 seam:
- source file
- function or call site name
- current owner
- proposed replacement mode (`replace_body`, `wrap_call_site`, `read_only_accessor`, `write_accessor`, `leave_in_c`)
- validated Rust export/import name (if any)
- caller/callee signature
- evidence location (file + function + line reference captured during implementation time)

### Cross-Subsystem Calls (Rust campaign calling other subsystems)

| Target Subsystem | Function/API Family | Purpose | Validation Notes |
|-----------------|--------------------|---------|------------------|
| Clock (`rust/src/time/`) | clock lifecycle / rate / day-advance APIs | Clock lifecycle, rate policy, day advancement | Confirm exact exported names already in tree |
| Clock (`rust/src/time/`) | event scheduling API | Register/query scheduled events | Confirm ownership split between clock persistence and campaign semantic validation |
| State (`rust/src/state/`) | bitfield access APIs | Campaign progression flags | Confirm exact accessor functions |
| State (`rust/src/state/`) | state-file helpers | Battle-group and per-system state files | Confirm failure propagation path |
| Comm (`rust/src/comm/`) or C boundary | communication entrypoints | Encounter dialogue dispatch | Confirm whether Rust calls Rust module or C seam |
| Battle (C) | battle entrypoint | Combat invocation | Confirm exact call boundary |
| Planets (C) | solar-system exploration entrypoint | Solar system exploration dispatch | Confirm lifecycle init/uninit seam owned by campaign |
| Game Init | kernel/session init/free API family | Session setup/teardown | Confirm exact owner and signatures |
| File I/O (`rust/src/io/`) | file open/read/write/close | Save file operations | Confirm slot/path helper strategy |

## Boundary Ownership / Migration Notes

- `CurrentActivity` / `NextActivity` / `LastActivity` are treated as **legacy-owned representation until bridge validation proves otherwise**; Rust may mirror them in `CampaignSession`, but source-of-truth ownership must be explicit per phase
- `GAME_STATE.GameClock` is already Rust-backed; campaign references it via the clock API, not direct struct access
- `GAME_STATE.GameState` bitfield is already Rust-backed; campaign accesses it via get/set bit API
- Queue data (`npc_built_ship_q`, `escort_q`, encounter queue, related race/group queues) must not be assumed Rust-owned in P03; P03.5 establishes reader/writer/accessor policy first
- Ship-state persistence fields (ship stamp/orientation/velocity or verified equivalents) must be grounded to actual source declarations before Rust type names are frozen
- Save-file I/O uses the same persistence infrastructure as current C code, accessed through Rust I/O or validated bridge helpers

## Requirements Coverage Matrix

All normative requirement families from `requirements.md` are mapped to implementation phases. The detailed paragraph-level traceability appendix is maintained in `requirements-traceability.md`.

| Requirement Area | Phase(s) |
|-----------------|---------|
| New-game entry, initialization, start flow | P03.5, P09 |
| Load-game entry, state restoration | P03.5, P07, P08 |
| Campaign loop and activity dispatch | P09 |
| Deferred-transition behavior | P09 |
| Hyperspace and navigation transitions | P10 |
| Encounter handoff and post-encounter | P11 |
| Starbase visit flow | P12 |
| Starbase save/load resume | P07, P12, P16 |
| Event progression and campaign clock | P04, P13 |
| Campaign save | P06 |
| Campaign load | P07 |
| Load-failure contract | P07, P16 |
| Scheduled-event semantic validation | P07, P14, P16 |
| Save/load round-trip fidelity | P06, P07, P08, P16 |
| Legacy save compatibility | P08, P16 |
| Error handling and robustness | P03.5, P07, P09, P11, P12 |
| Save summary normalization | P05 |
| Campaign Canonical Export Document | P05, P14 |
| Claim-family inspection-surface selection / no-mixing rule | P14, P16 |
| Verifier report minimum fields | P14, P16 |
| Export-success vs overall covered-context distinction | P14, P16 |
| Legacy-starbase observational exception | P08, P14, P16 |
| §8.6 row-specific normalization rules | P04, P14, P16 |
| §10.1 load/export outcome-class handling | P14, P16 |
