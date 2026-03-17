# Phase 14: End-to-End Netplay-Boundary Verification

## Phase ID
`PLAN-20260314-SUPERMELEE.P14`

## Prerequisites
- Required: Phase 13 completed and passed
- Netplay-boundary implementation phase complete
- Requirement matrix rows for netplay obligations defined

## Requirements Verified (Netplay Boundary)

This phase verifies the substantive SuperMelee-owned netplay obligations rather than leaving them as stub hooks:
- setup-time synchronization events for ship-slot changes, team-name changes, and whole-team bootstrap state
- match-start gating on connection/readiness/confirmation preconditions
- exposure of local battle-time combatant-selection outcomes at the SuperMelee/netplay boundary
- acceptance of remote selection updates where integrated mode requires them
- SuperMelee-owned fleet/rules semantic validation of remote selections
- commit/reject behavior for remote selections, including rejection of invalid remote selections and no re-rejection after acceptance

## Verification Tasks

### Integration Test Suite

Create `rust/tests/supermelee_netplay_boundary.rs`:
- `test_setup_ship_slot_change_emits_sync_event`
- `test_setup_team_name_change_emits_sync_event`
- `test_whole_team_bootstrap_emits_sync_event`
- `test_match_start_blocked_without_connection_ready`
- `test_match_start_blocked_without_readiness_confirmation`
- `test_local_selection_outcome_exposed_to_netplay_boundary`
- `test_valid_remote_selection_is_accepted_and_committed`
- `test_invalid_remote_selection_is_rejected_and_not_committed`
- `test_remote_selection_for_consumed_ship_is_rejected`
- `test_accepted_remote_selection_is_not_rejected_later`

### Manual Verification Checklist (Human Tester, if integrated mode available)
- [ ] Start a supported SuperMelee netplay-enabled session
- [ ] Change a ship slot locally and verify the setup sync event is visible at the integration boundary
- [ ] Change a team name locally and verify sync propagation
- [ ] Trigger whole-team bootstrap/synchronization and verify event content
- [ ] Attempt to start before readiness/confirmation and verify start is blocked
- [ ] Reach valid readiness/confirmation state and verify start becomes allowed
- [ ] Select a combatant locally and verify the selection outcome is exposed to the boundary
- [ ] Send a valid remote selection and verify it is committed
- [ ] Send an invalid remote selection and verify it is rejected without silent substitution

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo test --test supermelee_netplay_boundary
```

## Structural Verification Checklist
- [ ] Netplay-boundary integration test file exists
- [ ] Tests exist for setup sync, start gating, local selection exposure, and remote selection validation/commit behavior
- [ ] Requirement matrix rows for all netplay requirements point to concrete tests in this phase

## Semantic Verification Checklist (Mandatory)
- [ ] Netplay requirements are not satisfied merely by cfg-gated stubs
- [ ] Setup-time ship-slot/team-name/whole-team sync events are each tested explicitly
- [ ] Start gating covers connection, readiness, and confirmation preconditions
- [ ] Remote selection semantic validation checks actual fleet/rules availability, not transport-level framing
- [ ] Invalid remote selections are not committed and are surfaced as boundary errors per integration contract
- [ ] Accepted remote selections are treated as committed and not re-rejected later

## Success Criteria
- [ ] All netplay-boundary tests pass
- [ ] Substantive netplay requirements now have concrete verification coverage
- [ ] The prior review gap around netplay REQ coverage is fully addressed

## Phase Completion Marker
Create: `project-plans/20260311/supermelee/.completed/P14.md`
