# Phase 13: End-to-End Integration & Verification

## Phase ID
`PLAN-20260314-NETPLAY.P13`

## Prerequisites
- Required: Phase 12a (Integration Verification) completed and passed
- Expected: entire netplay subsystem implemented and unit-tested

## Purpose
Verify the complete netplay subsystem works end-to-end using integration tests that simulate full protocol exchanges between two peers over loopback TCP, plus compatibility evidence strong enough to support C-compatibility claims when interoperability is required.

## Requirements Verified

This phase verifies all stable requirement families from `requirements.md`, including:

- `REQ-NP-CONN-*`: Two peers connect over loopback TCP
- `REQ-NP-PROTO-*`: Init packet exchange validates version
- `REQ-NP-STATE-*`: Full lifecycle from `Unconnected` through `InBattle` and back
- `REQ-NP-SETUP-*`: Fleet and team-name exchange between peers, including deterministic crossing-edit resolution
- `REQ-NP-CONFIRM-*`: Handshake0→Handshake1 completes setup confirmation
- `REQ-NP-READY-*`: Ready rendezvous works for init, pre-battle, battle start, and battle end without starving progress
- `REQ-NP-DELAY-*`: Both peers advertise delay, effective delay computed deterministically
- `REQ-NP-SEED-*`: Discriminant side sends, other receives, and battle entry remains blocked until exchange completes
- `REQ-NP-INPUT-*`: Input buffered and delivered with correct delay
- `REQ-NP-SHIP-*`: Ship picks exchanged and semantically invalid remote selection resets correctly
- `REQ-NP-CHECK-*`: CRC computed, exchanged, verified, desync detected
- `REQ-NP-END-*`: Frame count exchange + two-stage ready + agreed terminal frame simulation
- `REQ-NP-RESET-*`: Bidirectional reset returns to setup
- `REQ-NP-ABORT-*`: Version mismatch aborts cleanly
- `REQ-NP-DISC-*`: Unexpected close surfaced as event
- `REQ-NP-COMPAT-003`: compatibility proof uses C-derived fixtures, mixed-peer interop, or both when required

## Implementation Tasks

### Files to create

- `rust/tests/netplay_integration.rs` — Integration test file
  - marker: `@plan PLAN-20260314-NETPLAY.P13`
  - marker: `@requirement REQ-NP-CONN-* REQ-NP-PROTO-* REQ-NP-STATE-* REQ-NP-SETUP-* REQ-NP-CONFIRM-* REQ-NP-READY-* REQ-NP-DELAY-* REQ-NP-SEED-* REQ-NP-INPUT-* REQ-NP-SHIP-* REQ-NP-CHECK-* REQ-NP-END-* REQ-NP-RESET-* REQ-NP-ABORT-* REQ-NP-DISC-* REQ-NP-COMPAT-003`
  - Contents:
    - `#![cfg(feature = "netplay")]`
    - Helper: `fn create_loopback_pair() -> (NetplayFacade, NetplayFacade)`
    - Helper: `fn advance_to_setup(a: &mut NetplayFacade, b: &mut NetplayFacade)`
    - Helper: `fn advance_to_battle(a: &mut NetplayFacade, b: &mut NetplayFacade)`

**Integration test cases:**

1. `test_e2e_connection_establishment`
2. `test_e2e_connection_failed_event_before_establishment`
3. `test_e2e_version_mismatch`
4. `test_e2e_setup_sync_fleet`
5. `test_e2e_setup_sync_team_name`
6. `test_e2e_bootstrap_sync`
7. `test_e2e_confirmation_and_prebattle`
8. `test_e2e_confirmation_cancel`
9. `test_e2e_battle_start_blocked_until_seed_exchange_complete`
10. `test_e2e_battle_input_exchange`
11. `test_e2e_ship_selection`
12. `test_e2e_checksum_match`
13. `test_e2e_checksum_desync`
14. `test_e2e_battle_end_sync`
15. `test_e2e_battle_end_continues_to_agreed_terminal_frame`
16. `test_e2e_reset_protocol`
17. `test_e2e_stale_battle_input_ignored_or_rejected_after_reset`
18. `test_e2e_stale_frame_count_ignored_or_rejected_after_reset`
19. `test_e2e_stale_select_ship_ignored_or_rejected_after_reset`
20. `test_e2e_stale_setup_packets_ignored_or_rejected_after_reset`
21. `test_e2e_disconnect_during_setup`
22. `test_e2e_disconnect_during_battle`
23. `test_e2e_full_session_lifecycle`
24. `test_e2e_crossing_setup_conflict_resolution`
   - both peers perform conflicting edits while sync is in flight
   - both converge to the same final state according to the deterministic policy
25. `test_e2e_semantic_invalid_remote_ship_selection_resets`
   - transport-valid select-ship packet is rejected by owner-side semantic validation hook
   - battle handoff blocked
   - reset initiated and observed
26. `test_e2e_blocking_apis_share_one_progress_engine`
   - verifies `poll`, `receive_battle_input`, `negotiate_ready`, and `wait_reset` all exercise the same named progress helper rather than diverging loops

### Compatibility-proof tasks

At least one of the following must be implemented if mixed C/Rust interoperability is required:

- `rust/tests/netplay_c_fixture_compat.rs`
  - loads packet/CRC fixtures derived from the C implementation and verifies Rust encode/decode matches exactly
- `rust/tests/netplay_mixed_peer_interop.rs`
  - runs at least one representative flow with one C peer and one Rust peer
- or both, if feasible

Handwritten expected bytes alone are not sufficient evidence for the final compatibility claim.

### Files to modify (if needed)

- `rust/Cargo.toml` — ensure `serial_test` is available for tests needing port exclusivity

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo test --all-features --test netplay_integration
cargo test --all-features --test netplay_integration -- --nocapture
# If compatibility fixtures/interop are required:
cargo test --all-features --test netplay_c_fixture_compat
cargo test --all-features --test netplay_mixed_peer_interop
```

## Structural Verification Checklist
- [ ] Integration test file exists at `rust/tests/netplay_integration.rs`
- [ ] File is feature-gated with `#![cfg(feature = "netplay")]`
- [ ] At least 26 integration tests defined
- [ ] Loopback helpers create real TCP connections
- [ ] Tests use `serial_test` if port conflicts are possible
- [ ] At least one compatibility-proof test/harness exists when interoperability is required

## Semantic Verification Checklist (Mandatory)
- [ ] Two real `NetplayFacade` instances communicate over TCP loopback
- [ ] Protocol exchange is byte-compatible or otherwise compatibility-proven by evidence stronger than hand-authored expectations
- [ ] State transitions verified at each step of the protocol
- [ ] Events verified at integration boundary (not only internal state inspection)
- [ ] Error cases (version mismatch, disconnect, deferred pre-establishment failure, desync) produce correct events
- [ ] Crossing setup edits converge deterministically and identically on both peers
- [ ] Semantically invalid remote ship selection blocks battle handoff and initiates reset
- [ ] Battle entry remains blocked until required random-seed exchange completes
- [ ] Stale post-reset `BattleInput`, `FrameCount`, `SelectShip`, and setup packets follow the documented shared policy consistently
- [ ] Battle-end flow chooses the agreed max terminal frame and continues simulation until that frame before final ready completion
- [ ] Full session lifecycle test proves the entire state machine works end-to-end
- [ ] Blocking operations (`receive_battle_input`, `negotiate_ready`, `wait_reset`) complete correctly
- [ ] Shared progress-engine reuse is verified, not merely assumed
- [ ] No flaky tests from TCP timing (use retries or generous timeouts)

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "TODO\|FIXME\|HACK\|todo!()\|unimplemented!()\|placeholder" rust/src/netplay/ | grep -v test
```

## Success Criteria
- [ ] All integration tests pass
- [ ] All unit tests from P03-P12 still pass
- [ ] `cargo test --workspace --all-features` clean
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings` clean
- [ ] `cargo fmt --all --check` clean
- [ ] No `todo!()`, `FIXME`, `HACK` in any netplay production code
- [ ] Feature gating works: `cargo test --workspace` (without netplay) still passes
- [ ] Compatibility evidence is archived in a form reviewers can audit

## Failure Recovery
- rollback: `git checkout -- rust/tests/netplay_integration.rs`
- blocking issues: TCP port conflicts (use OS-assigned ports via port 0), timing flakiness (add retry/sleep), lack of C-derived compatibility fixtures/interop harness

## Phase Completion Marker
Create: `project-plans/20260311/netplay/.completed/P13.md`

Contents:
- phase ID: `PLAN-20260314-NETPLAY.P13`
- timestamp
- files changed
- tests added
- all verification command outputs
- semantic verification summary confirming end-to-end behavior
- explicit PASS/FAIL decision

## Final Definition of Done

With P13 complete, the netplay subsystem port is DONE when:

1. [OK] All unit tests pass (P03-P12)
2. [OK] All integration tests pass (P13)
3. [OK] All linting/formatting gates pass
4. [OK] Compatibility claims are backed by C-derived fixtures, mixed-peer interop, or both when required
5. [OK] Public API (`NetplayFacade`) covers all SuperMelee/battle integration needs
6. [OK] Event system delivers all protocol events
7. [OK] Feature gating isolates netplay code completely
8. [OK] No placeholder code remains
9. [OK] Full session lifecycle verified end-to-end over TCP loopback

**Remaining for full game integration** (deferred to SuperMelee plan):
- Wiring `NetplayFacade` into `melee.c` / Rust melee module
- Wiring into `battle.c` / Rust battle module
- Wiring into `pickmele.c` / `tactrans.c` equivalents
- UI feedback presentation
- Runtime configuration UI
