# SuperMelee Requirement Traceability Matrix
@plan PLAN-20260314-SUPERMELEE.P12

## Entry, Initialization, and Teardown

| ID | Requirement | Phase | Verification | Audit Dep |
|---|---|---|---|---|
| SM-ENTRY-01 | Initialize valid runtime state on entry | P07 | `test_init_creates_valid_state` | none |
| SM-ENTRY-02 | Restore persisted setup state if present | P06 | `test_valid_config_roundtrip` | none |
| SM-ENTRY-03 | Fall back to built-in defaults if config invalid | P06 | `test_missing_config_uses_defaults` | none |
| SM-ENTRY-04 | Persist state and release resources on exit | P07 | `test_exit_path_persists_state` | none |

## Team and Fleet Model

| ID | Requirement | Phase | Verification | Audit Dep |
|---|---|---|---|---|
| SM-TEAM-01 | Maintain editable team state per side | P05 | `test_set_ship_updates_team` | none |
| SM-TEAM-02 | Team has name + fleet slots | P03-P05 | `test_team_new_defaults` | none |
| SM-TEAM-03 | Empty slots distinct from occupied | P03 | `test_melee_none_sentinel_roundtrip` | none |
| SM-TEAM-04 | Assigned ship available in fleet | P05 | `test_set_ship_and_cost_update` | none |
| SM-TEAM-05 | Removed ship → slot empty | P05 | `test_remove_ship_sets_none` | none |
| SM-TEAM-06 | Replace full team updates all fields | P05 | `test_replace_team_copies_name_and_ships` | none |
| SM-TEAM-07 | Fleet value consistent with contents | P05 | `test_fleet_value_matches_costs` | none |

## Team-Name Behavior

| ID | Requirement | Phase | Verification | Audit Dep |
|---|---|---|---|---|
| SM-NAME-01 | Set/edit preserves bounded name | P05 | `test_set_team_name_bounded` | none |
| SM-NAME-02 | Oversized name truncated | P05 | `test_overlong_name_truncated` | none |

## Setup/Menu Behavior

| ID | Requirement | Phase | Verification | Audit Dep |
|---|---|---|---|---|
| SM-MENU-01 | Flow supports edit/load/save/start | P07 | `test_init_creates_valid_state` | UI/timing |
| SM-MENU-02 | Cancel returns without commit | P07 | `test_apply_pick_with_no_selection` | none |
| SM-MENU-03 | Confirm applies selection | P07 | `test_apply_pick_sets_ship` | none |
| SM-MENU-04 | Cancel leaves team unchanged | P07 | `test_cancel_preserves_team` | none |

## Match-Start and Battle Handoff

| ID | Requirement | Phase | Verification | Audit Dep |
|---|---|---|---|---|
| SM-BATTLE-01 | No start with empty fleet | P07 | `test_start_blocked_if_no_ships` | none |
| SM-BATTLE-02 | Start allowed with valid fleets | P07 | `test_can_start_battle` | none |
| SM-BATTLE-03 | Prepare combatant selection state | P08 | `test_select_initial_combatant` | none |
| SM-BATTLE-04 | Restore post-battle state | P07 | `test_restore_after_battle` | none |
| SM-BATTLE-05 | Handoff preserves BattleReadyCombatant | P08 | `test_combatant_has_handle_and_ship` | none |

## Built-In Team Browsing

| ID | Requirement | Phase | Verification | Audit Dep |
|---|---|---|---|---|
| SM-BROWSE-01 | Built-in catalog available | P06 | `test_builtin_teams_count` | built-in exactness |
| SM-BROWSE-02 | Fallback from built-in catalog | P06 | `test_missing_config_uses_defaults` | none |
| SM-BROWSE-03 | Browse exposes built-in + saved | P06 | `test_enumerate_includes_builtins_and_saved` | none |
| SM-BROWSE-04 | Load built-in copies to active side | P06 | `test_load_builtin_team_into_setup` | none |

## Saved-Team Browsing and Loading

| ID | Requirement | Phase | Verification | Audit Dep |
|---|---|---|---|---|
| SM-PERSIST-01 | Enumerate saved .mle files | P06 | `test_enumerate_includes_saved_files` | none |
| SM-PERSIST-02 | Load valid saved team | P06 | `test_save_and_load_team_file_roundtrip` | none |
| SM-PERSIST-03 | Legacy .mle load preserves fleet+name | P06 | `test_deserialize_preserves_ships_and_name` | save-format |
| SM-PERSIST-04 | Invalid ship IDs handled on load | P06 | `test_invalid_ship_id_normalized` | none |
| SM-PERSIST-05 | Malformed file fails cleanly | P06 | `test_load_team_file_too_short_returns_error` | none |

## Team Persistence

| ID | Requirement | Phase | Verification | Audit Dep |
|---|---|---|---|---|
| SM-PERSIST-06 | Save preserves ship slots and name | P06 | `test_save_and_load_team_file_roundtrip` | none |
| SM-PERSIST-07 | Failed save no corrupt artifact | P06 | error return path | none |
| SM-PERSIST-08 | Written file reloadable | P06 | `test_save_and_load_team_file_roundtrip` | save-format |
| SM-PERSIST-09 | Setup persistence preserves control+team | P06 | `test_valid_config_roundtrip` | none |
| SM-PERSIST-10 | Network control sanitized on restore | P06 | `test_network_control_sanitized` | none |

## Battle-Time Combatant Selection

| ID | Requirement | Phase | Verification | Audit Dep |
|---|---|---|---|---|
| SM-SELECT-01 | Provide initial combatants | P08 | `test_select_initial_combatant` | none |
| SM-SELECT-02 | Provide next combatant after loss | P08 | `test_select_next_after_death` | none |
| SM-SELECT-03 | Handoff not weakened to bare IDs | P08 | `test_combatant_has_handle_and_ship` | none |
| SM-SELECT-04 | Operates on fleet/selection state only | P08 | `test_auto_select_uses_fleet_only` | none |

## Netplay Integration Boundary

| ID | Requirement | Phase | Verification | Audit Dep |
|---|---|---|---|---|
| SM-NET-01 | Local-only works without network state | P09 | `test_local_mode_requires_no_network_state` | none |
| SM-NET-02 | Ship-slot sync events | P09 | `test_ship_slot_change_emits_sync_event` | none |
| SM-NET-03 | Team-name sync events | P09 | `test_team_name_change_emits_sync_event` | none |
| SM-NET-04 | Whole-team bootstrap sync | P09 | `test_whole_team_sync_emits_event` | none |
| SM-NET-05 | Start gating on connection/readiness/confirmation | P09 | `test_start_blocked_without_*` (3 tests) | none |
| SM-NET-06 | Local selection outcome exposed | P09 | `test_local_selection_outcome_exposed` | none |
| SM-NET-07 | Valid remote selection accepted+committed | P09 | `test_valid_remote_selection_accepted_and_committed` | none |
| SM-NET-08 | Invalid remote selection rejected | P09 | `test_invalid_remote_selection_rejected_*` (2 tests) | none |
| SM-NET-09 | Consumed slot rejected | P09 | `test_remote_selection_for_consumed_ship_rejected` | none |
| SM-NET-10 | Accepted not re-rejected | P09 | `test_accepted_remote_selection_not_rejected_after_commit` | none |

## Error Handling and Recovery

| ID | Requirement | Phase | Verification | Audit Dep |
|---|---|---|---|---|
| SM-ERR-01 | Malformed config → fallback | P06 | `test_missing_config_uses_defaults` | none |
| SM-ERR-02 | Invalid ship IDs normalized | P06 | `test_invalid_ship_id_normalized` | none |
| SM-ERR-03 | Invalid start → remain in setup | P07 | `test_start_blocked_if_no_ships` | none |
| SM-ERR-04 | Isolated failures don't corrupt | P03-P05 | `SuperMeleeError` typed results | none |

## Compatibility-Sensitive Audit Areas

| ID | Requirement | Phase | Verification | Audit Dep |
|---|---|---|---|---|
| SM-COMPAT-01 | Legacy .mle load interop preserved | P06 | `test_deserialize_preserves_ships_and_name` | save-format |
| SM-COMPAT-02 | Byte-for-byte save if audit requires | P10 | audit decision: ExactParityRequired | save-format |
| SM-COMPAT-03 | Built-in team exactness if audit requires | P10 | audit decision: ExactParityRequired | built-in exactness |
| SM-COMPAT-04 | UI/timing exactness if audit requires | P10 | audit decision: SemanticCompatibilityRequired | UI/timing |
