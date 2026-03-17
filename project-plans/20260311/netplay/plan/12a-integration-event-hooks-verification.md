# Phase 12a: Integration & Event Hooks — Verification

## Phase ID
`PLAN-20260314-NETPLAY.P12a`

## Prerequisites
- Required: Phase 12 (Integration & Event Hooks) completed
- Expected: `NetplayFacade` and event system implemented with tests passing

## Verification Tasks

### Public API Completeness
- [ ] `NetplayFacade` exposes ALL operations SuperMelee needs for setup:
  - open/close connections
  - query connection state
  - confirm/cancel setup
  - send fleet/team-name changes
  - bootstrap sync
  - poll for events
- [ ] `NetplayFacade` exposes ALL operations battle needs:
  - init/uninit battle buffers
  - send/receive battle input
  - send/verify checksums
  - advertise input delay
  - send random seed
  - compute effective input delay
  - signal battle end
  - negotiate ready
  - wait reset
  - abort
  - reset
- [ ] Method names, parameters, and event shapes match `00-overview.md` exactly

### Encapsulation
- [ ] No internal types (`NetConnection`, `PacketQueue`, `ReadBuffer`, etc.) are exposed publicly
- [ ] Only `NetplayFacade`, `NetplayEvent`, `NetplayEventSink`, `NetState`, `NetplayError`, `PeerOptions`, `NetplayOptions`, `AbortReason`, `ResetReason`, and canonical ID aliases are public

### Event Coverage
- [ ] `Connected` emitted when connection enters `InSetup`
- [ ] `ConnectionFailed` emitted on transport failure
- [ ] `ConnectionClosed` emitted on close/disconnect
- [ ] `ConfirmationInvalidated` emitted when remote setup edit cancels confirmation
- [ ] `ResetReceived` emitted with reason
- [ ] `AbortReceived` emitted with reason
- [ ] `RemoteFleetUpdate` emitted with canonical fleet data shape
- [ ] `RemoteTeamNameUpdate` emitted with side + name
- [ ] `RemoteShipSelected` emitted with canonical ship ID
- [ ] `RandomSeedReceived` emitted with seed
- [ ] `InputDelayReceived` emitted with delay
- [ ] `SyncLoss` emitted when checksum mismatch detected
- [ ] `ConnectionFailed` is emitted only after cleanup has returned the slot to `Unconnected`

### Blocking Behavior
- [ ] `receive_battle_input()` polls network while waiting
- [ ] `negotiate_ready()` polls network while waiting
- [ ] `wait_reset()` polls network while waiting
- [ ] None of the blocking methods starve queued flush, receive, dispatch, callback, or event-delivery work
- [ ] All blocking methods respect timeouts or abort/disconnect conditions
- [ ] `poll`, `receive_battle_input`, `negotiate_ready`, and `wait_reset` all call one named shared progress helper/engine rather than duplicating loops

### Battle Entry / Battle End Semantics
- [ ] `init_battle()` rejects battle entry until required seed agreement is complete
- [ ] `negotiate_ready(NetState::InBattle)` rejects progression until required seed agreement is complete
- [ ] `signal_battle_end()` enters the first ending state, sends frame count, and records local readiness
- [ ] Remote frame counts update the agreed terminal frame using max(local, remote)
- [ ] End-of-battle flow continues simulation/progress until the agreed terminal frame is reached
- [ ] Final ready rendezvous returns the state machine to `InterBattle`

### Deterministic Convergence Boundaries
- [ ] Deterministic setup conflict-resolution policy is implemented in one named owner module/function
- [ ] Crossing setup edits converge identically across peers in tests
- [ ] Semantically invalid remote ship selection blocks battle handoff and initiates reset per specification §12.4

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo test --all-features -- netplay::integration
```

## Gate Decision
- [ ] PASS: proceed to Phase 13
- [ ] FAIL: return to Phase 12 and fix issues

## Phase Completion Marker
Create: `project-plans/20260311/netplay/.completed/P12a.md`
