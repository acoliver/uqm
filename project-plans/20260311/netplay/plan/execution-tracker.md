# Execution Tracker

Plan ID: PLAN-20260314-NETPLAY
Generated: 2026-03-14

| Phase | Title | Status | Verified | Semantic Verified | Notes |
|------:|-------|--------|----------|-------------------|-------|
| P00   | Overview | DONE | N/A | N/A | Plan structure |
| P00.5 | Preflight Verification | -- | -- | N/A | Toolchain + deps + compatibility/progress validation |
| P01   | Analysis | -- | -- | -- | Domain model, state machines, error map, conflict + progress analysis |
| P01a  | Analysis Verification | -- | -- | -- | Cross-reference with spec/reqs + traceability |
| P02   | Pseudocode | -- | -- | -- | 15 components including conflict/progress/boundary |
| P02a  | Pseudocode Verification | -- | -- | -- | Algorithmic correctness review |
| P03   | Core Types — Stub | -- | -- | -- | NetState, StateFlags, error, options, constants |
| P03a  | Core Types — Stub Verification | -- | -- | -- | Feature gating, type correctness |
| P04   | Core Types — TDD | -- | -- | -- | 16+ tests |
| P04a  | Core Types — TDD Verification | -- | -- | -- | Test quality review |
| P05   | Core Types — Impl | -- | -- | -- | Transition table, predicates, defaults |
| P05a  | Core Types — Impl Verification | -- | -- | -- | All tests pass, wire compat |
| P06   | Packet Codec | -- | -- | -- | 18 types, serialize/deserialize, queue, receive |
| P06a  | Packet Codec Verification | -- | -- | -- | Wire compatibility audit |
| P07   | Connection & Transport | -- | -- | -- | NetConnection, TCP, registry |
| P07a  | Connection & Transport Verification | -- | -- | -- | Loopback test, lifecycle |
| P08   | Protocol Sub-systems | -- | -- | -- | Ready, confirm, reset |
| P08a  | Protocol Sub-systems Verification | -- | -- | -- | Protocol correctness |
| P11   | Battle Input & Checksum | -- | -- | -- | Primitives, delivery, CRC, verification |
| P11a  | Battle Input & Checksum Verification | -- | -- | -- | Compatibility, FIFO, progress loop |
| P09   | Packet Handlers | -- | -- | -- | 18-type dispatch, state validation |
| P09a  | Packet Handlers Verification | -- | -- | -- | Coverage, state enforcement |
| P10   | Setup Sync & Notifications | -- | -- | -- | Per-connection + broadcast |
| P10a  | Setup Sync Verification | -- | -- | -- | State guards, packet correctness |
| P12   | Integration & Event Hooks | -- | -- | -- | `NetplayFacade`, events, conflict + semantic selection handling |
| P12a  | Integration Verification | -- | -- | -- | Public API, encapsulation, convergence |
| P13   | E2E Integration | -- | -- | -- | Loopback + compatibility-proof tests |

## Phase Dependencies

```
P00.5 -> P01 -> P01a -> P02 -> P02a -> P03 -> P03a -> P04 -> P04a -> P05 -> P05a
      -> P06 -> P06a -> P07 -> P07a -> P08 -> P08a -> P11 -> P11a
      -> P09 -> P09a -> P10 -> P10a -> P12 -> P12a -> P13
```

All phases are strictly sequential. No phase may begin until the prior phase's verification has passed.

## Estimated Scope

| Category | LoC |
|----------|-----|
| Core types (state, error, constants, options) | ~500 |
| Packet codec (types, codec, queue, receive, send) | ~900 |
| Connection (net_connection, transport, registry) | ~800 |
| Protocol sub-systems (ready, confirm, reset) | ~700 |
| Packet handlers (init, setup, battle, sync, control, dispatch) | ~1100 |
| Notifications (per_connection, broadcast) | ~600 |
| Battle input (buffer, delivery) | ~400 |
| Checksum (crc, buffer, verify) | ~400 |
| Integration (events, melee_hooks, battle_hooks) | ~500 |
| Tests (all phases) | ~1800 |
| Integration tests (P13) | ~500 |
| **Total** | **~8200** |

## Module Structure Created

```
rust/src/netplay/
+-- mod.rs                           # P03
+-- error.rs                         # P03-P05
+-- constants.rs                     # P03-P05
+-- options.rs                       # P03-P05
+-- state.rs                         # P03-P05
+-- connection/
|   +-- mod.rs                       # P07
|   +-- net_connection.rs            # P07
|   +-- transport.rs                 # P07
|   +-- registry.rs                  # P07
+-- packet/
|   +-- mod.rs                       # P06
|   +-- types.rs                     # P06
|   +-- codec.rs                     # P06
|   +-- queue.rs                     # P06
|   +-- receive.rs                   # P06
|   +-- send.rs                      # P06
+-- handlers/
|   +-- mod.rs                       # P09
|   +-- init.rs                      # P09
|   +-- setup.rs                     # P09
|   +-- battle.rs                    # P09
|   +-- sync.rs                      # P09
|   +-- control.rs                   # P09
+-- proto/
|   +-- mod.rs                       # P08
|   +-- ready.rs                     # P08
|   +-- confirm.rs                   # P08
|   +-- reset.rs                     # P08
+-- notify/
|   +-- mod.rs                       # P10
|   +-- per_connection.rs            # P10
|   +-- broadcast.rs                 # P10
+-- input/
|   +-- mod.rs                       # P11
|   +-- buffer.rs                    # P11
|   +-- delivery.rs                  # P11
+-- checksum/
|   +-- mod.rs                       # P11
|   +-- crc.rs                       # P11
|   +-- buffer.rs                    # P11
|   +-- verify.rs                    # P11
+-- integration/
    +-- mod.rs                       # P12
    +-- event.rs                     # P12
    +-- melee_hooks.rs               # P12
    +-- battle_hooks.rs              # P12
```

## C Files Replaced (compatibility target)

| C File | Rust Replacement | Phase |
|--------|-----------------|-------|
| `netconnection.c` / `nc_connect.ci` | `connection/net_connection.rs`, `connection/transport.rs` | P07 |
| `netstate.c` | `state.rs` | P03-P05 |
| `netmelee.c` | `connection/registry.rs`, `input/delivery.rs` | P07, P11 |
| `netmisc.c` | distributed across `state.rs`, `integration/event.rs`, `integration/melee_hooks.rs` | P03, P12 |
| `netoptions.c` | `options.rs` | P03-P05 |
| `netplay.h` | `constants.rs` | P03 |
| `packet.c` | `packet/types.rs`, `packet/codec.rs` | P06 |
| `packethandlers.c` | `handlers/*.rs` | P09 |
| `packetsenders.c` | `notify/per_connection.rs` | P10 |
| `packetq.c` | `packet/queue.rs` | P06 |
| `netrcv.c` | `packet/receive.rs` | P06 |
| `netsend.c` | `packet/send.rs` | P06 |
| `notify.c` | `notify/per_connection.rs` | P10 |
| `notifyall.c` | `notify/broadcast.rs` | P10 |
| `netinput.c` | `input/buffer.rs`, `input/delivery.rs` | P11 |
| `checkbuf.c` | `checksum/buffer.rs` | P11 |
| `checksum.c` | `checksum/verify.rs` | P11 |
| `crc.c` | `checksum/crc.rs` | P11 |
| `proto/npconfirm.c` | `proto/confirm.rs` | P08 |
| `proto/ready.c` | `proto/ready.rs` | P08 |
| `proto/reset.c` | `proto/reset.rs` | P08 |

## Traceability Artifact

- `plan/requirements-traceability-matrix.md` is the audit source for stable requirement-to-phase coverage.

## SuperMelee Integration Boundary (Deferred wiring)

The netplay subsystem exposes `NetplayFacade` and `NetplayEvent`. Actual wiring into the melee setup flow, battle loop, ship-pick UI, and battle-end transitions remains deferred to the SuperMelee subsystem plan.

Deferred wiring points:
1. `melee.c` / future Rust melee module → `NetplayFacade::open_connection()`, `close_all_connections()`, `confirm_setup()`, etc.
2. `battle.c` / future Rust battle module → `NetplayFacade::init_battle()`, `send_checksum()`, `verify_checksum()`
3. `pickmele.c` equivalent → `NetplayFacade::send_ship_selected()`, `negotiate_ready()`
4. `tactrans.c` equivalent → `NetplayFacade::signal_battle_end()`, `wait_reset()`
5. UI feedback owner → `NetplayEventSink` implementation
