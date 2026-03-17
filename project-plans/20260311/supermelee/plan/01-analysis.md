# Phase 01: Analysis

## Phase ID
`PLAN-20260314-SUPERMELEE.P01`

## Prerequisites
- Required: Phase 00.5 (Preflight Verification) completed and passed
- All blocking issues resolved

## Purpose

Produce analysis artifacts covering SuperMelee-owned runtime state, setup/menu transitions, team/persistence behavior, battle-facing selection/handoff contracts, netplay-boundary obligations, compatibility-sensitive audit inputs, and explicit mapping from current C code to scoped Rust modules.

## Entity/State Model Analysis

### 1. SuperMelee Setup State Machine

```text
Melee() -> [Initializing]
  -> initialize runtime/menu state, assets, browser state, random state
  -> load persisted setup if valid, else load built-in fallback teams
  -> [MenuActive]
    -> cursor/navigation edits
    -> fleet edit -> [BuildPicking] -> confirm/cancel -> [MenuActive]
    -> team browse/load -> [TeamBrowse] -> select/cancel -> [MenuActive]
    -> save team -> [Saving] -> success/fail -> [MenuActive]
    -> edit team name -> [NameEdit] -> commit/cancel -> [MenuActive]
    -> start battle -> [StartValidation] -> [BattleHandoff] -> battle subsystem
    -> battle returns -> [PostBattle] -> [MenuActive]
    -> exit -> [Teardown]
```

States to model:
- `SetupState::Initializing`
- `SetupState::MenuActive`
- `SetupState::BuildPicking { side, slot }`
- `SetupState::TeamBrowse { side, source }`
- `SetupState::Saving { side }`
- `SetupState::BattleHandoffPending`
- `SetupState::PostBattle`
- `SetupState::Teardown`

### 2. Team Model

```text
MeleeTeam {
  ships: [MeleeShip; MELEE_FLEET_SIZE],
  name: bounded team-name storage,
}

MeleeSetup {
  teams: [MeleeTeam; NUM_SIDES],
  fleet_value: [u16; NUM_SIDES],
  player_control: [PlayerControl; NUM_SIDES],
}
```

Mutation operations:
- `set_ship(side, slot, ship)`
- `clear_slot(side, slot)`
- `set_team_name(side, name)`
- `replace_team(side, source_team)`
- `get_fleet_value(side)`
- `is_playable(side)`

Derived behavior:
- fleet value remains consistent with current non-empty slots
- empty-slot representation stays distinct from occupied slots
- team name remains bounded/valid before persistence or rendering

### 3. Team Browser State

```text
TeamBrowserState {
  entries: Vec<TeamEntry>,
  source_filter: BuiltIn | Saved | Unified,
  cursor_index: usize,
  view_top: usize,
  highlighted_entry: Option<TeamEntry>,
}

TeamEntry = BuiltIn { id, display_name } | SavedFile { path, display_name }
```

### 4. Battle-Facing Combatant Selection State

```text
CombatantSelectionState {
  selected_slots: [[bool; MELEE_FLEET_SIZE]; NUM_SIDES],
  pending_local_selection: [Option<SelectionCommit>; NUM_SIDES],
  pending_remote_selection: [Option<RemoteSelectionCandidate>; NUM_SIDES],
}

SelectionCommit {
  side: usize,
  slot: usize,
  ship: MeleeShip,
  battle_entry: BattleReadyCombatant,
}
```

Analysis goal:
- document the actual battle-facing object consumed by the battle/input boundary,
- document who creates it,
- document how SuperMelee commits that object while remaining owner only of selection policy/order.

### 5. Netplay Boundary State

```text
NetplayBoundaryState {
  enabled: bool,
  setup_sync_ready: bool,
  local_ready: bool,
  remote_ready: bool,
  local_confirmed: bool,
  remote_confirmed: bool,
}
```

SuperMelee-owned transitions:
- local ship-slot change -> emit setup sync event
- local team-name change -> emit setup sync event
- whole-team bootstrap -> emit full-team sync event
- start request -> validate readiness/confirmation preconditions
- local combatant selection -> expose selected outcome to netplay boundary
- remote combatant selection -> semantic validation -> commit/reject

## Edge/Error Handling Map

| Scenario | Expected Behavior | Requirement area |
|----------|-------------------|------------------|
| Malformed `.mle` file | Fail cleanly, leave active team/setup state unchanged | saved-team loading |
| Invalid ship ID in persisted team data | Normalize consistently or fail cleanly per compatibility contract | invalid ship identifiers |
| Empty/unplayable fleet at match start | Block start, remain in setup/menu flow | match-start validation |
| Save fails mid-write | No apparently successful corrupted artifact remains | save failure |
| `melee.cfg` missing/unreadable/invalid | Fall back to usable built-in default setup | fallback initialization |
| Restored transient network-control startup mode | Sanitize/downgrade to valid local startup state | netplay-mode persistence |
| Fleet-edit picker canceled | Team state unchanged | picker cancel |
| Team-load subview canceled | Team state unchanged | subview cancel |
| Remote selection references unavailable ship | Reject commit; treat as boundary error via netplay integration | remote selection validation |
| Netplay start requested without readiness/confirmation | Block start | netplay start gating |

## Integration Touchpoints

### From SuperMelee to Other Subsystems

| Call Direction | Interface | Purpose |
|----------------|-----------|---------|
| SM -> Graphics | Existing setup/menu drawing APIs | Menu, browser, picker, transitions |
| SM -> Sound | Existing menu music / transition APIs | Setup audiovisual flow |
| SM -> Input | Existing menu/picker input APIs | Navigation, confirm/cancel, edit actions |
| SM -> Resource | Existing asset loading APIs | Frames, icons, fonts |
| SM -> State | `CurrentActivity` / equivalent | Mode ownership and battle return |
| SM -> IO | team-file/config file APIs | `.mle` and setup persistence |
| SM -> Battle boundary | battle entry + combatant handoff | Start match and return |
| SM -> Netplay boundary | setup sync events, selection outcomes | optional synchronized sessions |

### From Other Subsystems to SuperMelee

| Call Direction | Interface | Purpose |
|----------------|-----------|---------|
| Ships -> SM | cost/icons/validity lookups | team value, picker rendering, validation |
| Ships/Battle -> SM | request initial/next combatants | selection policy + commit |
| Battle -> SM | return control after battle | restore menu state |
| Netplay -> SM | decoded remote setup/selection updates | semantic validation + commit/reject |

## Old Code to Replace/Remove

### Immediate Replacement (guarded or redirected in the C bridge)
- `sc2/src/uqm/supermelee/melee.c` — SuperMelee entry/menu/battle handoff ownership
- `sc2/src/uqm/supermelee/meleesetup.c` — team/fleet data model ownership
- `sc2/src/uqm/supermelee/loadmele.c` — built-in catalog, load/save, team browse ownership
- `sc2/src/uqm/supermelee/buildpick.c` — fleet-edit picker ownership
- `sc2/src/uqm/supermelee/pickmele.c` — battle-facing selection policy ownership

### Out-of-Scope C Files Explicitly Not Replaced Here
- `sc2/src/uqm/battle.c`
- `sc2/src/uqm/process.c`
- `sc2/src/uqm/collide.c`
- `sc2/src/uqm/ship.c`
- `sc2/src/uqm/intel.c`
- `sc2/src/uqm/tactrans.c`
- `sc2/src/uqm/element.h`

Those remain integration dependencies and should move under a separate tactical combat/battle-engine plan if they are to be ported.

## Requirement Mapping

| Requirement area | Plan phase |
|------------------|------------|
| Entry/initialization/teardown | P07, P13, P15 |
| Team/fleet model | P03–P06 |
| Team-name behavior | P03–P06 |
| Setup/menu behavior | P07, P13 |
| Built-in team browsing | P06, P10, P13 |
| Saved-team browsing/loading | P06, P13 |
| Team persistence | P06, P10, P13 |
| Match-start validation | P07, P09, P13 |
| Fleet-edit ship picker | P07, P13 |
| Battle-facing initial/next combatant selection | P08, P13 |
| Handoff not weakened to bare IDs | P08, P11, P12, P13 |
| Battle handoff/return | P07, P08, P11, P13 |
| Local-only behavior when netplay disabled | P09, P14 |
| Setup-time netplay sync events | P09, P14 |
| Netplay start gating | P09, P14 |
| Remote selection acceptance/rejection semantics | P09, P14 |
| Error handling/recovery | P06–P15 |
| Compatibility-sensitive audit areas | P10, P13, P15 |
| Statement-level traceability | P12, P15 |

## Verification Commands

```bash
# No code changes in this phase - verify analysis artifact exists
ls -la project-plans/20260311/supermelee/plan/01-analysis.md
```

## Success Criteria
- [ ] SuperMelee-owned state machines and entities are documented
- [ ] Battle-engine/runtime internals are explicitly excluded from this plan's ownership analysis
- [ ] Combatant-selection contract analysis identifies the real battle-facing handoff object/owner
- [ ] Netplay-boundary obligations are decomposed into concrete SuperMelee-owned states and transitions
- [ ] Compatibility-sensitive questions are captured as audit inputs, not pre-decided implementation obligations
- [ ] Requirement areas map to scoped implementation phases
