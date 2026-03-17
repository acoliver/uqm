# Phase 01: Analysis

## Phase ID
`PLAN-20260314-NETPLAY.P01`

## Prerequisites
- Required: Phase 00.5 (Preflight Verification) completed and passed
- All toolchain/dependency assumptions verified

## Purpose
Produce domain and flow analysis artifacts covering the entire netplay subsystem before writing any implementation code.

## Expected Outputs

### 1. Entity/State Transition Model

#### NetState State Machine
Document all 10 states and valid transitions with triggers:

| From State | To State | Trigger | Requirements |
|-----------|----------|---------|-------------|
| `Unconnected` | `Connecting` | `open_connection()` called | REQ-NP-CONN-001 |
| `Connecting` | `Init` | TCP connected successfully | REQ-NP-CONN-002 |
| `Connecting` | `Unconnected` | Connect/listen failed | REQ-NP-CONN-003 |
| `Init` | `InSetup` | Both peers Init+Ready complete | REQ-NP-PROTO-004 |
| `Init` | `Unconnected` | Version mismatch / abort | REQ-NP-PROTO-002, REQ-NP-ABORT-001 |
| `InSetup` | `PreBattle` | Both peers confirm setup | REQ-NP-CONFIRM-002, REQ-NP-STATE-003 |
| `PreBattle` | `InterBattle` | Ready rendezvous complete | REQ-NP-READY-002 |
| `InterBattle` | `SelectShip` | Ready rendezvous for ship pick | REQ-NP-SHIP-001 |
| `InterBattle` | `InBattle` | Ready rendezvous for battle | REQ-NP-STATE-004 |
| `SelectShip` | `InBattle` | Ship selection done + ready | REQ-NP-STATE-004 |
| `InBattle` | `EndingBattle` | Both ready to end | REQ-NP-END-001 |
| `EndingBattle` | `EndingBattle2` | Frame counts exchanged | REQ-NP-END-002 |
| `EndingBattle2` | `InterBattle` | Final ready rendezvous | REQ-NP-END-004, REQ-NP-STATE-005 |
| Any gameplay | `InSetup` | Reset complete | REQ-NP-RESET-001..005 |
| Any | `Unconnected` | Disconnect/abort | REQ-NP-DISC-001..003 |

#### Connection State Flags
Document the nested flag model:
- `connected: bool` — transport alive
- `disconnected: bool` — transport dead
- `discriminant: bool` — asymmetric tie-breaker (server=true, client=false)
- `handshake: { local_ok, remote_ok, canceling }` — confirmation state
- `ready: { local_ready, remote_ready }` — ready rendezvous state
- `reset: { local_reset, remote_reset }` — reset protocol state
- `agreement: { random_seed }` — pre-battle agreements
- `input_delay: u32` — negotiated battle input delay
- `checksum_interval: u32` — when checksum enabled

#### Packet Type Catalog
Enumerate all 18 packet types with:
- Wire type ID
- Payload fields and sizes
- Valid states for receipt
- Handler behavior summary
- Canonical Rust type aliases used by each field (`PlayerId`, `FleetSide`, `FleetSlot`, `ShipId`)

### 2. Edge/Error Handling Map

| Error Condition | Detection Point | Response | Requirements |
|----------------|-----------------|----------|-------------|
| Invalid packet type | `receive.rs` | Protocol error → close | REQ-NP-ERROR-001 |
| Packet too short for type | `receive.rs` | Protocol error → close | REQ-NP-ERROR-001 |
| Packet in wrong state | `handlers/*.rs` | Protocol error → close | REQ-NP-ERROR-002 |
| Protocol version mismatch | `handlers/init.rs` | Abort(VersionMismatch) → close | REQ-NP-PROTO-002 |
| UQM version too old | `handlers/init.rs` | Abort(VersionMismatch) → close | REQ-NP-PROTO-003 |
| Duplicate local confirm | `proto/confirm.rs` | Return error (EINVAL equivalent) | REQ-NP-CONFIRM-001 |
| Duplicate local ready | `proto/ready.rs` | Return error | REQ-NP-READY-003 |
| Invalid input delay (>24) | `handlers/sync.rs` | Protocol error → close | REQ-NP-DELAY-004 |
| Invalid fleet ship ID | `handlers/setup.rs` | Protocol error → close | REQ-NP-SETUP-004 |
| Invalid fleet slot index | `handlers/setup.rs` | Protocol error → close | REQ-NP-SETUP-004 |
| Battle input buffer full | `handlers/battle.rs` | Protocol error → close | REQ-NP-INPUT-003 |
| Checksum mismatch | `checksum/verify.rs` | Reset(SyncLoss) | REQ-NP-CHECK-004 |
| Checksum frame out of range | `handlers/sync.rs` | Warn + discard (soft) | REQ-NP-CHECK-003 |
| Semantic-invalid remote ship selection | `integration/melee_hooks.rs` | Block battle handoff + initiate reset | REQ-NP-SHIP-005 |
| TCP send failure | `packet/send.rs` | Close connection | REQ-NP-DISC-003 |
| TCP recv EOF | `packet/receive.rs` | Close connection | REQ-NP-DISC-001, REQ-NP-DISC-002 |
| TCP recv error | `packet/receive.rs` | Error callback → close | REQ-NP-CONN-004 |
| Connect timeout | `connection/transport.rs` | Error callback → unconnected | REQ-NP-CONN-003 |

### 3. Integration Touchpoints List

| Integration Point | Rust Module | C File | Direction | Nature |
|-------------------|-------------|--------|-----------|--------|
| Open connection | `netplay::registry` | `melee.c:378-392` | SuperMelee→Netplay | API call |
| Close all connections | `netplay::registry` | `melee.c:1411-1435` | SuperMelee→Netplay | API call |
| Connected feedback | `netplay::integration::event` | `melee.c:2144` | Netplay→SuperMelee | Event/callback |
| Close feedback | `netplay::integration::event` | `melee.c:2157` | Netplay→SuperMelee | Event/callback |
| Error feedback | `netplay::integration::event` | `melee.c:2170` | Netplay→SuperMelee | Event/callback |
| Abort feedback | `netplay::integration::event` | `melee.c:2180` | Netplay→SuperMelee | Event/callback |
| Reset feedback | `netplay::integration::event` | `melee.c:2212` | Netplay→SuperMelee | Event/callback |
| Confirmation cancelled | `netplay::integration::event` | `melee.c:2113` | Netplay→SuperMelee | Event/callback |
| Fleet change notify | `netplay::notify` | `melee.c:2374` | SuperMelee→Netplay | API call |
| Team name notify | `netplay::notify` | `melee.c:2397` | SuperMelee→Netplay | API call |
| Remote fleet update | `netplay::integration::event` | `melee.c:2495` | Netplay→SuperMelee | Event/callback |
| Remote team name | `netplay::integration::event` | `melee.c:2563` | Netplay→SuperMelee | Event/callback |
| Confirm setup | `netplay::proto::confirm` | `melee.c:1521` | SuperMelee→Netplay | API call |
| Init battle input | `netplay::input` | `battle.c:437` | Battle→Netplay | API call |
| Init checksum buffers | `netplay::checksum` | `battle.c:441` | Battle→Netplay | API call |
| Send battle input | `netplay::notify` | `battle.c` loop | Battle→Netplay | API call |
| Receive battle input | `netplay::input::delivery` | `battle.c` loop | Battle→Netplay | API call |
| Send checksum | `netplay::notify` | `battle.c:268` | Battle→Netplay | API call |
| Verify checksum | `netplay::checksum::verify` | `battle.c:287` | Battle→Netplay | API call |
| Ship selected notify | `netplay::notify` | `pickmele.c:931` | SuperMelee→Netplay | API call |
| Remote ship selected | `netplay::integration::event` | `pickmele.c:907` | Netplay→SuperMelee | Event/callback |
| Semantic ship-selection validation/reject | `netplay::integration::melee_hooks` | `pickmele.c` / melee ship-pick owner | Netplay↔SuperMelee | Boundary contract |
| Battle end sync | `netplay::integration::battle_hooks` | `tactrans.c:152` | Battle→Netplay | API call |
| Random seed update | `netplay::integration::event` | `melee.c:2106` | Netplay→SuperMelee | Event/callback |

### 4. Deterministic Setup Conflict-Resolution Analysis

Produce an explicit artifact at `project-plans/20260311/netplay/analysis/setup-conflict-resolution.md` for specification §9.3 / REQ-NP-SETUP-006 covering:
- canonical conflict unit (`FleetSide`, `FleetSlot`, `ShipId` or team-name field)
- whether conflict detection uses local pending edits, remote receipt order, or both
- how the stable discriminant decides winners for crossing edits
- how losing local edits invalidate confirmation state
- which module owns the policy (`integration/melee_hooks.rs::resolve_setup_conflict(...)` or equivalent)
- tests for simultaneous crossing fleet edits, simultaneous team-name edits, and mixed fleet/name traffic

### 5. Blocking/Progress Model Analysis

Produce an explicit artifact at `project-plans/20260311/netplay/analysis/progress-model.md` for specification §8.4 and §18 covering:
- the shared progress loop contract used by `receive_battle_input`, `negotiate_ready`, and `wait_reset`
- ordering between flush, recv, dispatch, event delivery, callbacks, and awaited-condition checks
- how timeouts/abort/disconnect are surfaced without starving queued work
- whether `std::net` alone satisfies the contract or whether an adapter crate is required

### 6. Connection Failure/Event Delivery Analysis

Produce an explicit artifact at `project-plans/20260311/netplay/analysis/connection-failure-path.md` covering:
- which `open_connection()` failures are returned immediately vs converted into deferred `NetplayEvent::ConnectionFailed`
- where async listen/connect progress is polled before establishment
- how transport setup failures are cleaned up before surfacing the failure event
- how pre-establishment failures converge back to `Unconnected` without leaking registry membership or transport handles

### 7. Old Code to Replace/Remove List

The Rust port replaces the following C files entirely (when `USE_RUST_NETPLAY` or equivalent is defined):

| C File | Lines (approx) | Replacement |
|--------|----------------|-------------|
| `netconnection.c` | ~240 | `netplay::connection::net_connection` |
| `nc_connect.ci` | ~290 | `netplay::connection::transport` |
| `netstate.c` | ~40 | `netplay::state` |
| `netmelee.c` | ~730 | `netplay::connection::registry` + `netplay::input::delivery` |
| `netmisc.c` | ~130 | distributed across `netplay::state`, `netplay::integration::event`, `netplay::integration::melee_hooks` |
| `netoptions.c` | ~40 | `netplay::options` |
| `packet.c` | ~150 | `netplay::packet::codec` |
| `packethandlers.c` | ~640 | `netplay::handlers::*` |
| `packetsenders.c` | ~190 | `netplay::notify` |
| `packetq.c` | ~130 | `netplay::packet::queue` |
| `netrcv.c` | ~170 | `netplay::packet::receive` |
| `netsend.c` | ~90 | `netplay::packet::send` |
| `notify.c` | ~100 | `netplay::notify::per_connection` |
| `notifyall.c` | ~150 | `netplay::notify::broadcast` |
| `netinput.c` | ~90 | `netplay::input::buffer` + `netplay::input::delivery` |
| `checkbuf.c` | ~90 | `netplay::checksum::buffer` |
| `checksum.c` | ~300 | `netplay::checksum` + `netplay::checksum::verify` |
| `crc.c` | ~60 | `netplay::checksum::crc` |
| `proto/npconfirm.c` | ~85 | `netplay::proto::confirm` |
| `proto/ready.c` | ~95 | `netplay::proto::ready` |
| `proto/reset.c` | ~135 | `netplay::proto::reset` |

Total: ~3945 lines of C replaced by ~6750 lines of Rust (includes tests and documentation).

No transitional C/Rust bridge implementation phase exists in this plan. Any `#ifdef USE_RUST_NETPLAY` wiring remains deferred to the SuperMelee plan.

## Verification Commands

```bash
# No code changes in analysis phase — verify existing build still passes
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] State machine document covers all 10 NetState values
- [ ] All 18 packet types are cataloged with valid-state rules
- [ ] Error map covers all error paths from initialstate.md and the semantic-invalid-selection path from specification §12.4
- [ ] Integration touchpoints list covers all C↔Rust boundaries owned by this plan
- [ ] Replacement list covers all 21 C source files
- [ ] Deterministic setup conflict-resolution artifact exists at `project-plans/20260311/netplay/analysis/setup-conflict-resolution.md`
- [ ] Blocking/progress model artifact exists at `project-plans/20260311/netplay/analysis/progress-model.md`
- [ ] Connection failure/event delivery artifact exists at `project-plans/20260311/netplay/analysis/connection-failure-path.md`

## Semantic Verification Checklist (Mandatory)
- [ ] State transitions match the transitions documented in initialstate.md
- [ ] Error handling behavior matches specification §15-16
- [ ] Setup conflict-resolution analysis matches specification §9.3 and REQ-NP-SETUP-006
- [ ] Ship-selection semantic boundary analysis matches specification §12.4 and REQ-NP-SHIP-005
- [ ] Blocking/progress model analysis matches specification §8.4 and §18
- [ ] Connection-failure/event delivery analysis matches REQ-NP-CONN-003 and the integration event contract
- [ ] Integration points match the boundary model from specification §16-17
- [ ] No C file is left unaccounted for in the replacement plan

## Success Criteria
- [ ] Complete analysis documents produced
- [ ] All requirement IDs from `requirements.md` are represented in analysis or the traceability matrix
- [ ] Integration contract is explicit and complete

## Failure Recovery
- rollback: N/A (no code changes)
- blocking issues: Missing C source understanding → re-read initialstate.md

## Phase Completion Marker
Create: `project-plans/20260311/netplay/.completed/P01.md`
