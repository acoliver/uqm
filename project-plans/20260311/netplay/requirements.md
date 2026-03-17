# Netplay Subsystem Requirements

## Purpose

This document defines the required externally observable behavior of the SuperMelee netplay subsystem in EARS form. The subsystem covers peer-to-peer connection management, protocol compatibility validation, setup synchronization, confirmation/ready/reset behavior, battle-time ship-selection synchronization, battle input delivery with deterministic delay, checksum-based sync verification when enabled, and disconnect/abort handling.

## Scope boundaries

- SuperMelee menu ownership and UI rendering are outside this subsystem.
- Battle simulation and ship combat behavior are outside this subsystem.
- Generic socket/event-loop/timer implementation details of lower-level libraries are outside this subsystem.
- Metaserver/discovery behavior is outside this subsystem unless explicitly added to scope.

## Connection establishment

- **REQ-NP-CONN-001** — **When** SuperMelee opens a netplay connection for a player slot, **the subsystem shall** create a connection object for that slot and begin either listening or connecting according to the configured peer role.
- **REQ-NP-CONN-002** — **When** the underlying transport connection is established successfully, **the subsystem shall** transition the connection into protocol initialization state, bind the transport to that connection, and begin protocol initialization messaging.
- **REQ-NP-CONN-003** — **When** a connection attempt fails before transport establishment completes, **the subsystem shall** report that failure through the netplay/SuperMelee integration boundary and leave the connection in a clean disconnected state.
- **REQ-NP-CONN-004** — **When** a connection is closed after previously being established, **the subsystem shall** surface the close event through the integration boundary and stop treating the connection as usable.

## Protocol compatibility

- **REQ-NP-PROTO-001** — **When** a peer initialization packet is received, **the subsystem shall** validate the peer's protocol version against the supported protocol version policy.
- **REQ-NP-PROTO-002** — **When** the peer's protocol version is incompatible, **the subsystem shall** reject the connection cleanly and communicate the incompatibility through the abort/failure path.
- **REQ-NP-PROTO-003** — **When** the peer's product version is below the supported minimum compatibility level, **the subsystem shall** reject the connection cleanly and communicate the incompatibility through the abort/failure path.
- **REQ-NP-PROTO-004** — **When** protocol initialization completes successfully for both peers, **the subsystem shall** permit the connection to progress into setup synchronization state.

## Connection state transitions

- **REQ-NP-STATE-001** — **Ubiquitous:** The subsystem shall maintain connection state sufficient to distinguish unconnected, connecting, init, in-setup, pre-battle, inter-battle, select-ship, in-battle, and end-of-battle synchronization phases.
- **REQ-NP-STATE-002** — **When** a connection has transport but has not yet completed initialization, **the subsystem shall** remain in initialization state and shall not behave as if setup synchronization is already active.
- **REQ-NP-STATE-003** — **When** setup confirmation completes successfully for a connection, **the subsystem shall** transition that connection from setup state into pre-battle state.
- **REQ-NP-STATE-004** — **When** battle-start negotiation completes successfully, **the subsystem shall** transition the participating connection into in-battle state before battle-time packet exchange proceeds.
- **REQ-NP-STATE-005** — **When** battle-end synchronization completes successfully, **the subsystem shall** transition the participating connection back to the between-battles/setup-adjacent phase defined by the owning game flow.

## Setup synchronization

- **REQ-NP-SETUP-001** — **When** a connection enters synchronized setup flow, **the subsystem shall** support full bootstrap synchronization of the current network-relevant team state.
- **REQ-NP-SETUP-002** — **When** the local side changes a synchronized fleet slot during setup, **the subsystem shall** transmit a corresponding setup update to connected peers.
- **REQ-NP-SETUP-003** — **When** the local side changes a synchronized team name during setup, **the subsystem shall** transmit a corresponding setup update to connected peers.
- **REQ-NP-SETUP-004** — **When** a valid remote fleet update is received during setup, **the subsystem shall** deliver that update to the SuperMelee integration boundary for application to setup state.
- **REQ-NP-SETUP-005** — **When** a valid remote team-name update is received during setup, **the subsystem shall** deliver that update to the SuperMelee integration boundary for application to setup state.
- **REQ-NP-SETUP-006** — **When** local and remote setup updates conflict while synchronization is in flight, **the subsystem shall** resolve the conflict deterministically so both peers converge on the same setup state.

## Setup confirmation / handshake

- **REQ-NP-CONFIRM-001** — **When** SuperMelee requests setup confirmation during setup state, **the subsystem shall** enter the setup-confirmation protocol for that connection.
- **REQ-NP-CONFIRM-002** — **When** both peers complete setup confirmation successfully, **the subsystem shall** advance the connection toward pre-battle negotiation.
- **REQ-NP-CONFIRM-003** — **When** SuperMelee cancels setup confirmation before completion, **the subsystem shall** cancel the local confirmation state and transmit any protocol-required cancellation signaling.
- **REQ-NP-CONFIRM-004** — **When** a remote setup change invalidates a local confirmation, **the subsystem shall** invalidate the confirmation state and signal that reconfirmation is required.
- **REQ-NP-CONFIRM-005** — **When** confirmation-protocol packets are received outside the states where that protocol is meaningful, **the subsystem shall** treat them as protocol errors.

## Ready synchronization

- **REQ-NP-READY-001** — **Ubiquitous:** The subsystem shall provide a reusable ready-synchronization mechanism for phases that require both peers to rendezvous before progressing.
- **REQ-NP-READY-002** — **When** a ready rendezvous is used for initialization, pre-battle, ship-selection, battle-start, or battle-end flow, **the subsystem shall** complete it only after both peers have participated.
- **REQ-NP-READY-003** — **When** both peers complete a given ready rendezvous, **the subsystem shall** trigger the corresponding completion action exactly once for that rendezvous.
- **REQ-NP-READY-004** — **When** higher layers wait for ready completion, **the subsystem shall** continue servicing network traffic and asynchronous callbacks while waiting.

## Fleet/team synchronization boundaries

- **REQ-NP-FLEET-001** — **When** a connection is first established for setup synchronization, **the subsystem shall** support transfer of the full current team/fleet state needed by the remote side.
- **REQ-NP-FLEET-002** — **When** setup synchronization is re-entered after a completed or aborted battle, **the subsystem shall** support re-bootstrap of the relevant setup state.
- **REQ-NP-FLEET-003** — **Ubiquitous:** The subsystem shall transport synchronized setup data but shall not assume ownership of unrelated SuperMelee menu state outside the defined setup synchronization contract.

## Input delay negotiation

- **REQ-NP-DELAY-001** — **When** pre-battle negotiation begins, **the subsystem shall** allow each participating peer to advertise its preferred input delay.
- **REQ-NP-DELAY-002** — **When** all required input-delay advertisements have been processed, **the subsystem shall** choose a deterministic battle input delay according to the product's compatibility rule.
- **REQ-NP-DELAY-003** — **Ubiquitous:** The selected battle input delay shall be at least as large as the required compatibility minimum implied by the participating peers' advertised values and the local configured minimum/preference.
- **REQ-NP-DELAY-004** — **When** a peer advertises an invalid or absurd input delay value, **the subsystem shall** fail cleanly rather than entering battle with corrupted delay state.

## Random-seed synchronization

- **REQ-NP-SEED-001** — **When** pre-battle negotiation requires deterministic RNG synchronization, **the subsystem shall** ensure that exactly one side of each peer pair originates the agreed random seed and the other side applies that seed.
- **REQ-NP-SEED-002** — **When** the agreed random-seed exchange has not completed successfully, **the subsystem shall not** permit battle start.

## Battle input delivery

- **REQ-NP-INPUT-001** — **When** battle begins, **the subsystem shall** initialize battle input buffers in a way that supports deterministic delayed-input startup.
- **REQ-NP-INPUT-002** — **When** battle-time input is generated for transmission, **the subsystem shall** serialize and transmit that input in protocol order.
- **REQ-NP-INPUT-003** — **When** valid remote battle input is received during an active battle phase, **the subsystem shall** buffer it for delayed deterministic delivery to battle.
- **REQ-NP-INPUT-004** — **When** battle requests the next network-controlled input for a player and delayed input is already available, **the subsystem shall** return the next buffered input in order.
- **REQ-NP-INPUT-005** — **When** battle requests the next network-controlled input for a player and delayed input is not yet available, **the subsystem shall** continue servicing network progress while waiting for that input.
- **REQ-NP-INPUT-006** — **When** the connection is lost while battle is waiting for network-controlled input, **the subsystem shall** report the failure through the owning flow and terminate the wait cleanly.
- **REQ-NP-INPUT-007** — **Ubiquitous:** The subsystem shall not deliver remote battle input out of order.

## Ship-selection synchronization

- **REQ-NP-SHIP-001** — **When** a networked ship-selection phase begins, **the subsystem shall** synchronize participating peers into the ship-selection state before remote ship selections are consumed.
- **REQ-NP-SHIP-002** — **When** the local side finalizes a network-relevant ship selection, **the subsystem shall** transmit that selection to connected peers.
- **REQ-NP-SHIP-003** — **When** a valid remote ship-selection packet is received during the ship-selection phase, **the subsystem shall** deliver it to the owning selection flow so the remote choice is reflected locally.
- **REQ-NP-SHIP-004** — **When** an invalid ship-selection packet is received or a ship-selection packet arrives outside the valid ship-selection phase, **the subsystem shall** fail cleanly rather than silently corrupting selection state.
- **REQ-NP-SHIP-005** — **When** a transport-valid remote ship selection fails fleet/rules semantic validation at the SuperMelee boundary, **the integrated system shall** block battle handoff, initiate the reset path, and refuse to commit that invalid selection into battle-facing state.

## Checksum sync verification

- **REQ-NP-CHECK-001** — **When** checksum verification is enabled, **the subsystem shall** periodically transmit checksums for deterministic battle state according to the configured checksum interval.
- **REQ-NP-CHECK-002** — **When** checksum verification is enabled, **the subsystem shall** retain enough local and remote checksum history to compare checksums only after the required delayed-input window has elapsed.
- **REQ-NP-CHECK-003** — **When** a checksum packet is received for an implausible frame or invalid checksum interval, **the subsystem shall** discard or otherwise safely ignore that packet without corrupting ongoing sync tracking.
- **REQ-NP-CHECK-004** — **When** delayed checksum comparison for the same frame differs between peers, **the subsystem shall** treat the session as out of sync and initiate sync-loss reset handling.

## Battle-end synchronization

- **REQ-NP-END-001** — **When** one side becomes ready to stop the active battle, **the subsystem shall** participate in the battle-end synchronization flow rather than terminating battle unilaterally.
- **REQ-NP-END-002** — **When** battle-end synchronization is active, **the subsystem shall** exchange target frame-count information required to determine a common terminal frame.
- **REQ-NP-END-003** — **When** one side has not yet reached the agreed terminal frame, **the subsystem shall** continue allowing simulation/input progression until that frame is reached.
- **REQ-NP-END-004** — **When** both peers have reached the effective terminal frame and completed the final readiness rendezvous, **the subsystem shall** permit battle to exit the active battle phase cleanly.

## Reset behavior

- **REQ-NP-RESET-001** — **When** the local side requests a gameplay reset, **the subsystem shall** send reset signaling and enter reset-in-progress state for that connection.
- **REQ-NP-RESET-002** — **When** a remote reset is received before the local side has initiated reset, **the subsystem shall** confirm the reset back to the remote peer.
- **REQ-NP-RESET-003** — **When** both local and remote reset participation have been observed and the owning flow is waiting for reset completion, **the subsystem shall** signal reset completion to the owner.
- **REQ-NP-RESET-004** — **When** a gameplay reset occurs, **the subsystem shall** preserve the distinction between gameplay reset and generic connection loss so the owning flow can return to setup rather than treating the event as an unrelated disconnect.
- **REQ-NP-RESET-005** — **When** sync loss is detected through checksum verification, **the subsystem shall** use the sync-loss reset reason in the reset path.

## Abort behavior

- **REQ-NP-ABORT-001** — **When** a connection encounters an unrecoverable protocol or compatibility failure for which reset is not the correct recovery path, **the subsystem shall** use the abort path rather than the gameplay-reset path.
- **REQ-NP-ABORT-002** — **When** an abort packet is received from a peer, **the subsystem shall** surface the abort reason and terminate the affected connection cleanly.

## Error recovery and protocol robustness

- **REQ-NP-ERROR-001** — **When** a malformed packet is received, **the subsystem shall** fail the affected connection cleanly without corrupting unrelated local netplay state.
- **REQ-NP-ERROR-002** — **When** a structurally valid packet is received in an invalid connection state, **the subsystem shall** treat it as a protocol error.
- **REQ-NP-ERROR-003** — **When** a setup or gameplay packet is received after reset has made it irrelevant according to the reset protocol, **the subsystem shall** ignore or reject it consistently rather than allowing stale gameplay to resume.
- **REQ-NP-ERROR-004** — **Ubiquitous:** The subsystem shall preserve deterministic convergence for valid sessions and shall fail loudly/cleanly for invalid sessions rather than silently diverging.

## Disconnect handling

- **REQ-NP-DISC-001** — **When** a remote peer disconnects unexpectedly during setup, **the subsystem shall** surface the disconnect so SuperMelee can stop waiting for setup progress.
- **REQ-NP-DISC-002** — **When** a remote peer disconnects unexpectedly during battle or battle-end synchronization, **the subsystem shall** surface the disconnect so the owning flow can abort the networked battle cleanly.
- **REQ-NP-DISC-003** — **When** the local side explicitly closes a connection, **the subsystem shall** release transport resources and remove the connection from the active connection set.

## Integration with SuperMelee and battle

- **REQ-NP-INTEG-001** — **When** SuperMelee queries whether a network-controlled side is connected and setup-ready, **the subsystem shall** provide state accurate enough for match-start gating.
- **REQ-NP-INTEG-002** — **When** SuperMelee needs player-facing feedback for connection, abort, reset, error, or confirmation-invalidation events, **the subsystem shall** emit enough structured information for that feedback to be shown.
- **REQ-NP-INTEG-003** — **When** battle starts, **the subsystem shall** provide initialized network state sufficient for battle-time input delivery and optional checksum verification.
- **REQ-NP-INTEG-004** — **When** battle ends or aborts, **the subsystem shall** leave the owning flow in a state from which setup/inter-battle progression can continue correctly or the session can terminate cleanly.

## Compatibility-sensitive obligations

- **REQ-NP-COMPAT-001** — **Ubiquitous:** The subsystem shall preserve the existing two-peer SuperMelee netplay behavior unless a product-wide redesign explicitly changes that contract.
- **REQ-NP-COMPAT-002** — **Ubiquitous:** The subsystem shall preserve the current protocol compatibility floor for version negotiation unless a deliberate compatibility decision revises it.
- **REQ-NP-COMPAT-003** — **Ubiquitous:** If mixed C/Rust interoperability or legacy on-the-wire compatibility is required, **the subsystem shall** preserve packet framing, field encoding, and packet semantics accordingly.
