# Netplay Requirements Traceability Matrix

Plan ID: PLAN-20260314-NETPLAY
Requirements Source: `project-plans/20260311/netplay/requirements.md`

## Purpose

Provide stable requirement-to-phase traceability for audit, implementation sequencing, and verification planning. Every substantive requirement family must map to at least one implementation phase and one verification phase.

## Matrix

| Requirement IDs | Summary | Primary implementation phases | Primary verification phases |
|---|---|---|---|
| REQ-NP-CONN-001..004 | Connection creation, establishment, failure, close | P07, P12 | P07a, P12a, P13 |
| REQ-NP-PROTO-001..004 | Init/version validation and setup entry | P06, P09 | P06a, P09a, P13 |
| REQ-NP-STATE-001..005 | State modeling and lifecycle transitions | P03, P05, P08, P12 | P03a, P05a, P08a, P12a, P13 |
| REQ-NP-SETUP-001..006 | Bootstrap sync, fleet/name sync, deterministic conflict resolution | P09, P10, P12 | P09a, P10a, P12a, P13 |
| REQ-NP-CONFIRM-001..005 | Setup confirmation/cancellation | P08, P09, P12 | P08a, P09a, P12a, P13 |
| REQ-NP-READY-001..004 | Generic ready rendezvous and wait-progress behavior | P08, P11, P12 | P08a, P11a, P12a, P13 |
| REQ-NP-FLEET-001..003 | Setup-state transport boundaries and re-bootstrap | P10, P12 | P10a, P12a, P13 |
| REQ-NP-DELAY-001..004 | Input-delay advertisement and deterministic selection | P09, P10, P11, P12 | P09a, P10a, P11a, P12a, P13 |
| REQ-NP-SEED-001..002 | Deterministic random-seed ownership/exchange | P09, P10, P12 | P09a, P10a, P12a, P13 |
| REQ-NP-INPUT-001..007 | Delayed battle input buffering and wait behavior | P09, P11, P12 | P09a, P11a, P12a, P13 |
| REQ-NP-SHIP-001..005 | Ship-selection transport, delivery, semantic invalidation/reset | P09, P12 | P09a, P12a, P13 |
| REQ-NP-CHECK-001..004 | Checksum buffering, delayed comparison, sync loss | P09, P11, P12 | P09a, P11a, P12a, P13 |
| REQ-NP-END-001..004 | Two-stage battle-end synchronization | P09, P12 | P09a, P12a, P13 |
| REQ-NP-RESET-001..005 | Reset signaling and sync-loss reset semantics | P08, P09, P12 | P08a, P09a, P12a, P13 |
| REQ-NP-ABORT-001..002 | Abort path and abort reception | P08, P09, P12 | P08a, P09a, P12a, P13 |
| REQ-NP-ERROR-001..004 | Protocol robustness and stale-packet handling | P06, P09, P12 | P06a, P09a, P12a, P13 |
| REQ-NP-DISC-001..003 | Disconnect handling across setup/battle/close | P07, P11, P12 | P07a, P11a, P12a, P13 |
| REQ-NP-INTEG-001..004 | SuperMelee/battle integration surface | P12 | P12a, P13 |
| REQ-NP-COMPAT-001..003 | Two-peer compatibility and wire compatibility obligations | P00.5, P01, P06, P11, P13 | P00.5, P01a, P06a, P11a, P13 |

## Requirement-by-requirement detail

| Requirement ID | Summary | Implementation phases | Verification phases |
|---|---|---|---|
| REQ-NP-CONN-001 | Open connection creates per-player connection object and starts connect/listen flow | P07, P12 | P07a, P12a, P13 |
| REQ-NP-CONN-002 | Established transport binds to connection and begins init messaging | P07, P09, P12 | P07a, P09a, P12a, P13 |
| REQ-NP-CONN-003 | Pre-establishment failure reports cleanly and leaves disconnected state | P07, P12 | P07a, P12a, P13 |
| REQ-NP-CONN-004 | Established close is surfaced and connection becomes unusable | P07, P12 | P07a, P12a, P13 |
| REQ-NP-PROTO-001 | Init packet validates protocol version | P03, P06, P09 | P03a, P06a, P09a, P13 |
| REQ-NP-PROTO-002 | Incompatible protocol version aborts cleanly | P06, P09, P12 | P06a, P09a, P12a, P13 |
| REQ-NP-PROTO-003 | Too-old product version aborts cleanly | P06, P09, P12 | P06a, P09a, P12a, P13 |
| REQ-NP-PROTO-004 | Successful initialization permits setup progression | P01, P08, P09, P12 | P01a, P08a, P09a, P12a, P13 |
| REQ-NP-STATE-001 | Netplay models all required lifecycle phases | P03, P05 | P03a, P05a, P13 |
| REQ-NP-STATE-002 | Connection stays in init until initialization completes | P05, P09 | P05a, P09a, P13 |
| REQ-NP-STATE-003 | Successful setup confirmation advances to pre-battle | P08, P12 | P08a, P12a, P13 |
| REQ-NP-STATE-004 | Battle-start negotiation enters in-battle before battle packets proceed | P08, P12 | P08a, P12a, P13 |
| REQ-NP-STATE-005 | Battle-end synchronization returns to owning post-battle phase | P09, P12 | P09a, P12a, P13 |
| REQ-NP-SETUP-001 | Setup entry/re-entry supports full bootstrap sync | P10, P12 | P10a, P12a, P13 |
| REQ-NP-SETUP-002 | Local fleet change sends corresponding update | P10, P12 | P10a, P12a, P13 |
| REQ-NP-SETUP-003 | Local team-name change sends corresponding update | P10, P12 | P10a, P12a, P13 |
| REQ-NP-SETUP-004 | Valid remote fleet update delivered to integration boundary | P09, P12 | P09a, P12a, P13 |
| REQ-NP-SETUP-005 | Valid remote team-name update delivered to integration boundary | P09, P12 | P09a, P12a, P13 |
| REQ-NP-SETUP-006 | Crossing setup updates resolve deterministically | P01, P02, P10, P12 | P01a, P02a, P10a, P12a, P13 |
| REQ-NP-CONFIRM-001 | Local setup confirmation enters confirmation protocol | P08, P12 | P08a, P12a, P13 |
| REQ-NP-CONFIRM-002 | Mutual setup confirmation advances toward pre-battle | P08, P12 | P08a, P12a, P13 |
| REQ-NP-CONFIRM-003 | Local confirmation cancellation sends protocol-required cancellation | P08, P12 | P08a, P12a, P13 |
| REQ-NP-CONFIRM-004 | Remote setup edit invalidates confirmation and signals reconfirmation | P09, P12 | P09a, P12a, P13 |
| REQ-NP-CONFIRM-005 | Confirmation packets outside valid states are protocol errors | P08, P09 | P08a, P09a, P13 |
| REQ-NP-READY-001 | Reusable ready mechanism exists for multi-phase rendezvous | P08, P12 | P08a, P12a, P13 |
| REQ-NP-READY-002 | Ready completes only after both peers participate | P08, P12 | P08a, P12a, P13 |
| REQ-NP-READY-003 | Ready completion action fires exactly once | P08, P12 | P08a, P12a, P13 |
| REQ-NP-READY-004 | Waiting for readiness continues servicing network/callback progress | P00.5, P01, P02, P11, P12 | P00.5, P01a, P02a, P11a, P12a, P13 |
| REQ-NP-FLEET-001 | Initial setup sync transfers full current team/fleet state | P10, P12 | P10a, P12a, P13 |
| REQ-NP-FLEET-002 | Re-entered setup supports re-bootstrap | P10, P12 | P10a, P12a, P13 |
| REQ-NP-FLEET-003 | Netplay transports setup data without owning unrelated menu state | P01, P10, P12 | P01a, P10a, P12a |
| REQ-NP-DELAY-001 | Peers advertise preferred input delay in pre-battle | P09, P10, P12 | P09a, P10a, P12a, P13 |
| REQ-NP-DELAY-002 | Effective input delay chosen deterministically from participants | P11, P12 | P11a, P12a, P13 |
| REQ-NP-DELAY-003 | Effective input delay respects compatibility minimum/max-of-participants rule | P11, P12 | P11a, P12a, P13 |
| REQ-NP-DELAY-004 | Invalid input delay values fail cleanly | P09, P12 | P09a, P12a, P13 |
| REQ-NP-SEED-001 | Exactly one side originates deterministic random seed | P09, P10, P12 | P09a, P10a, P12a, P13 |
| REQ-NP-SEED-002 | Battle may not start until seed exchange completes | P08, P12 | P08a, P12a, P13 |
| REQ-NP-INPUT-001 | Battle input buffers initialize for delayed deterministic startup | P11, P12 | P11a, P12a, P13 |
| REQ-NP-INPUT-002 | Local battle input serializes/transmits in protocol order | P10, P12 | P10a, P12a, P13 |
| REQ-NP-INPUT-003 | Valid remote battle input buffers for delayed delivery | P09, P11, P12 | P09a, P11a, P12a, P13 |
| REQ-NP-INPUT-004 | Available delayed input is returned in order | P11, P12 | P11a, P12a, P13 |
| REQ-NP-INPUT-005 | Empty-buffer waits continue servicing network progress | P00.5, P01, P02, P11, P12 | P00.5, P01a, P02a, P11a, P12a, P13 |
| REQ-NP-INPUT-006 | Disconnect during input wait terminates wait cleanly | P11, P12 | P11a, P12a, P13 |
| REQ-NP-INPUT-007 | Remote battle input is never delivered out of order | P11, P12 | P11a, P12a, P13 |
| REQ-NP-SHIP-001 | Ship-selection phase synchronizes before consuming remote picks | P08, P12 | P08a, P12a, P13 |
| REQ-NP-SHIP-002 | Local finalized ship selection is transmitted | P10, P12 | P10a, P12a, P13 |
| REQ-NP-SHIP-003 | Transport-valid remote ship selection delivered to owner flow | P09, P12 | P09a, P12a, P13 |
| REQ-NP-SHIP-004 | Invalid or out-of-phase ship selection fails cleanly | P09, P12 | P09a, P12a, P13 |
| REQ-NP-SHIP-005 | Owner-side semantic rejection blocks battle handoff and initiates reset | P01, P02, P12 | P01a, P02a, P12a, P13 |
| REQ-NP-CHECK-001 | Checksum verification periodically transmits checksums when enabled | P10, P11, P12 | P10a, P11a, P12a, P13 |
| REQ-NP-CHECK-002 | Local and remote checksum history retained for delayed comparison | P11, P12 | P11a, P12a, P13 |
| REQ-NP-CHECK-003 | Implausible checksum packets are ignored safely | P09, P11 | P09a, P11a, P13 |
| REQ-NP-CHECK-004 | Checksum mismatch triggers sync-loss reset handling | P11, P12 | P11a, P12a, P13 |
| REQ-NP-END-001 | Battle-end sync participates instead of unilateral exit | P08, P12 | P08a, P12a, P13 |
| REQ-NP-END-002 | Battle-end sync exchanges target frame counts | P09, P10, P12 | P09a, P10a, P12a, P13 |
| REQ-NP-END-003 | Simulation continues until agreed terminal frame | P09, P12 | P09a, P12a, P13 |
| REQ-NP-END-004 | Final readiness rendezvous permits clean battle exit | P08, P12 | P08a, P12a, P13 |
| REQ-NP-RESET-001 | Local gameplay reset sends reset signaling and enters reset-in-progress | P08, P12 | P08a, P12a, P13 |
| REQ-NP-RESET-002 | Remote reset before local reset is confirmed back to peer | P08, P09, P12 | P08a, P09a, P12a, P13 |
| REQ-NP-RESET-003 | Waiting owner receives reset completion when both sides participate | P08, P12 | P08a, P12a, P13 |
| REQ-NP-RESET-004 | Gameplay reset remains distinct from generic disconnect | P08, P12 | P08a, P12a, P13 |
| REQ-NP-RESET-005 | Sync loss uses sync-loss reset reason | P11, P12 | P11a, P12a, P13 |
| REQ-NP-ABORT-001 | Unrecoverable compatibility/protocol failures use abort path | P08, P09, P12 | P08a, P09a, P12a, P13 |
| REQ-NP-ABORT-002 | Received abort surfaces reason and closes cleanly | P09, P12 | P09a, P12a, P13 |
| REQ-NP-ERROR-001 | Malformed packet fails affected connection cleanly | P06, P09, P12 | P06a, P09a, P12a, P13 |
| REQ-NP-ERROR-002 | Structurally valid packet in invalid state is protocol error | P09, P12 | P09a, P12a, P13 |
| REQ-NP-ERROR-003 | Stale post-reset setup/gameplay packets ignored or rejected consistently | P08, P09, P12 | P08a, P09a, P12a, P13 |
| REQ-NP-ERROR-004 | Valid sessions converge; invalid sessions fail loudly/cleanly | P03, P09, P12 | P03a, P09a, P12a, P13 |
| REQ-NP-DISC-001 | Unexpected disconnect during setup is surfaced promptly | P07, P10, P12 | P07a, P10a, P12a, P13 |
| REQ-NP-DISC-002 | Unexpected disconnect during battle/end sync is surfaced promptly | P07, P11, P12 | P07a, P11a, P12a, P13 |
| REQ-NP-DISC-003 | Explicit local close releases transport and active-set membership | P07, P12 | P07a, P12a, P13 |
| REQ-NP-INTEG-001 | Integration exposes connected/setup-ready state for match gating | P12 | P12a, P13 |
| REQ-NP-INTEG-002 | Integration emits structured feedback events | P12 | P12a, P13 |
| REQ-NP-INTEG-003 | Battle start receives initialized network input/checksum state | P11, P12 | P11a, P12a, P13 |
| REQ-NP-INTEG-004 | Battle end/abort leaves owning flow in correct continuation state | P12 | P12a, P13 |
| REQ-NP-COMPAT-001 | Existing two-peer SuperMelee behavior is preserved | P00.5, P01, P12, P13 | P00.5, P01a, P12a, P13 |
| REQ-NP-COMPAT-002 | Current version-compatibility floor is preserved | P00.5, P01, P06, P09 | P00.5, P01a, P06a, P09a, P13 |
| REQ-NP-COMPAT-003 | Wire framing/encoding/semantics preserved when interoperability is required | P00.5, P06, P11, P13 | P00.5, P06a, P11a, P13 |

## Notes

- P00.5 and P01 own compatibility/design validation that must happen before transport design is locked.
- P12 is the canonical public API phase and owns API consistency across overview, notifications, handlers, and event delivery.
- P13 must include at least one compatibility-proof mechanism stronger than handwritten expectations alone: C-derived fixtures, mixed-peer interop, or both.
- Requirement IDs are the only traceability labels to use in later plan edits; broad labels like "REQ-NP: protocol compatibility" are not audit-sufficient.
