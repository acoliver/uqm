# Phase 09: Netplay Boundary Surface & Validation

## Phase ID
`PLAN-20260314-SUPERMELEE.P09`

## Prerequisites
- Required: Phase 08a completed and passed
- Combatant-selection contract implemented

## Purpose

Implement the substantive SuperMelee-owned netplay boundary obligations called out by the requirements and specification:
- complete local behavior when netplay is disabled,
- setup-time synchronization events,
- start gating on connection/readiness/confirmation,
- local combatant-selection outcome exposure,
- semantic validation and commit/reject behavior for remote selections.

This phase intentionally excludes transport/protocol framing and other netplay-owned machinery.

## Implementation Tasks

### Files to create or complete

- `rust/src/supermelee/setup/netplay_boundary.rs`
  - marker: `@plan PLAN-20260314-SUPERMELEE.P09`
  - Implement:
    - local no-netplay fast path returning valid behavior without network state
    - ship-slot change sync event emission
    - team-name change sync event emission
    - whole-team bootstrap/sync emission
    - start precondition validation for connection/readiness/confirmation
    - exposure of local combatant-selection outcomes to the boundary
    - semantic validation for remote selection updates against current fleet/selection state
    - commit of valid remote selections into battle-facing selection state
    - rejection path for invalid remote selections without silent substitution or post-acceptance re-rejection

### Tests to create

- `rust/src/supermelee/setup/netplay_boundary_tests.rs`
  - `test_local_mode_requires_no_network_state`
  - `test_ship_slot_change_emits_setup_sync_event`
  - `test_team_name_change_emits_setup_sync_event`
  - `test_whole_team_sync_emits_setup_sync_event`
  - `test_start_blocked_without_connection_ready`
  - `test_start_blocked_without_readiness_or_confirmation`
  - `test_local_selection_outcome_is_exposed_to_boundary`
  - `test_valid_remote_selection_is_accepted_and_committed`
  - `test_invalid_remote_selection_is_rejected_and_not_committed`
  - `test_remote_selection_for_consumed_ship_is_rejected`
  - `test_accepted_remote_selection_is_not_rejected_after_commit`

### Files to modify

- `rust/src/supermelee/setup/mod.rs`
  - ensure `netplay_boundary` module is declared and tests are wired according to local conventions

### Pseudocode traceability
- Uses pseudocode lines: 148–174

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features netplay_boundary_tests
```

## Structural Verification Checklist
- [ ] `netplay_boundary.rs` exists under `setup/`
- [ ] Dedicated netplay-boundary tests exist
- [ ] The phase plans real implementation work before later E2E netplay verification

## Semantic Verification Checklist (Mandatory)
- [ ] Local-only SuperMelee behavior is explicitly supported without requiring network state
- [ ] Setup-time sync events for ship-slot changes, team-name changes, and whole-team bootstrap are each specified and tested separately
- [ ] Start gating covers connection, readiness, and confirmation preconditions
- [ ] Remote selection validation is semantic fleet/rules validation, not transport-level validation
- [ ] Invalid remote selections are rejected without silent substitution or accidental commit
- [ ] Accepted remote selections are committed once and not re-rejected later

## Success Criteria
- [ ] The plan now includes a concrete implementation phase for the netplay-boundary requirements instead of verification-only coverage
- [ ] All substantive SuperMelee-owned netplay obligations have phase-local implementation tasks and tests

## Failure Recovery
- rollback: `git checkout -- rust/src/supermelee/setup/netplay_boundary.rs`

## Phase Completion Marker
Create: `project-plans/20260311/supermelee/.completed/P09.md`
