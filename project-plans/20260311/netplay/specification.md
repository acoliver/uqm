# Netplay Subsystem — Functional & Technical Specification

## 1. Scope

This document specifies the desired end-state behavior of the SuperMelee netplay subsystem for the Rust port.

The subsystem is responsible for:

- peer-to-peer connection management for SuperMelee netplay sessions,
- protocol version compatibility validation,
- setup-phase synchronization of network-owned setup state exchange,
- start-of-match negotiation,
- battle-time ship-selection synchronization,
- battle-time input exchange with deterministic input delay,
- optional deterministic sync verification via checksums,
- reset/abort/disconnect handling at the netplay boundary, and
- integration with the surrounding SuperMelee and lower-level networking layers.

This specification does **not** define:

- SuperMelee menu rendering or menu-input UX beyond the netplay-facing contract,
- battle simulation rules,
- ship behavior,
- generic socket/event-loop/timer APIs of the lower-level network library,
- metaserver/discovery behavior unless and until such behavior is explicitly brought into scope.

## 2. Boundary and ownership model

### 2.1 Netplay subsystem boundary

The netplay subsystem shall own the following end-state responsibilities:

1. creating, tracking, and destroying peer connections used for SuperMelee netplay,
2. maintaining a per-connection state machine,
3. encoding and decoding the netplay wire protocol,
4. validating protocol and product compatibility during connection initialization,
5. transporting setup synchronization messages between peers,
6. transporting battle synchronization messages between peers,
7. negotiating readiness and reset transitions required by networked SuperMelee flow,
8. negotiating and enforcing battle input delay,
9. detecting deterministic divergence through checksum comparison when checksum verification is enabled, and
10. surfacing connection/protocol/reset/abort events to the owning SuperMelee/battle layers.

### 2.2 SuperMelee boundary

SuperMelee owns:

- local menu flow,
- local team/fleet state mutation rules,
- player-facing status/feedback presentation,
- match-start intent,
- ship-pick UI behavior,
- handoff into battle and return from battle.

Netplay shall not absorb those responsibilities. Instead, it shall provide the synchronization and coordination contract SuperMelee requires to run a networked session.

### 2.3 Battle boundary

Battle owns:

- frame progression,
- deterministic simulation,
- authoritative local frame state,
- combat resolution,
- local input sampling before netplay transmission where applicable.

Netplay owns only the network-facing synchronization behavior around that simulation, including delayed remote input delivery, battle-end synchronization, and optional checksum verification.

### 2.4 Lower-level network-library boundary

The lower-level networking/callback/async layers own:

- socket creation and OS integration,
- event polling and descriptor readiness,
- deferred callback scheduling,
- timer/alarm scheduling,
- address resolution and connect/listen primitives.

The Rust netplay subsystem may wrap or adapt those services, but it shall not redefine their generic contracts inside this specification.

## 3. End-state subsystem model

### 3.1 Session model

The subsystem shall support direct peer-to-peer SuperMelee sessions between two peers.

The subsystem may be internally structured to permit future generalization, but the required compatibility target for this port is the existing two-peer SuperMelee netplay model.

### 3.2 Connection model

Each active peer connection shall maintain at minimum:

- the local player slot it serves,
- transport attachment state,
- a netplay connection state,
- readiness/confirmation/reset flags or equivalent state,
- negotiated input-delay information,
- protocol-version/product-version compatibility status,
- packet send and receive buffering sufficient to preserve protocol correctness,
- battle-phase transient state required for synchronization,
- callback or event-hook state needed to signal SuperMelee/battle integration points.

The exact Rust data structures are not prescribed, but the externally visible behavior shall match the contract below.

### 3.3 State machine

The subsystem shall preserve a state machine semantically equivalent to the current connection lifecycle:

- `unconnected`
- `connecting`
- `init`
- `in_setup`
- `pre_battle`
- `inter_battle`
- `select_ship`
- `in_battle`
- `ending_battle`
- `ending_battle_2`

Internal representation and naming may differ, but the observable transition semantics shall remain compatible.

## 4. Connection establishment and lifecycle

### 4.1 Opening a connection

When SuperMelee requests a network connection for a player slot, the subsystem shall create a connection object bound to that slot and begin either server-side listening/acceptance flow or client-side connect flow according to the configured peer options.

### 4.2 Established-transport transition

When the underlying transport connection becomes established, the subsystem shall:

- attach the transport handle to the connection,
- transition the connection into initialization state,
- install receive/close/error handling for ongoing protocol traffic, and
- begin protocol initialization by transmitting the initialization packet defined in §6.

### 4.3 Connection identity asymmetry / tie-breaker

Each connected peer pair shall establish a stable asymmetric tie-breaker value or equivalent role distinction that remains fixed for the lifetime of that connection.

That tie-breaker shall be sufficient to preserve the conflict-resolution behavior required by setup synchronization and any other protocol logic that depends on a stable per-connection asymmetry.

### 4.4 Close and destruction behavior

When a connection closes, whether by local action, remote closure, protocol failure, or transport error, the subsystem shall:

- mark the connection disconnected,
- stop using the transport handle,
- emit the appropriate close/error event to its owner,
- transition the connection to an unconnected terminal state or equivalent,
- and release netplay-owned resources associated with that connection.

The subsystem shall not continue to flush queued gameplay traffic against a transport that has already been invalidated.

## 5. Connection state machine contract

### 5.1 `init`

`init` is the state entered immediately after transport establishment and before gameplay/setup synchronization begins.

In this state the subsystem shall:

- exchange initialization/version information,
- validate compatibility,
- and perform the first ready-style rendezvous that permits entry into setup synchronization.

### 5.2 `in_setup`

`in_setup` is the only state in which setup confirmation/cancellation and setup-data synchronization are meaningful.

In this state the subsystem shall accept and emit:

- team/fleet synchronization packets,
- team-name synchronization packets,
- confirmation handshake packets,
- ping/ack packets if the implementation retains them.

### 5.3 `pre_battle`

`pre_battle` shall be used for start-of-match negotiation immediately after setup confirmation completes and before battle begins.

In this state the subsystem shall support:

- RNG seed negotiation/exchange,
- input-delay advertisement or negotiation,
- ready synchronization required to advance toward battle.

### 5.4 `inter_battle`

`inter_battle` shall represent the synchronized between-battles phase used after pre-battle negotiation and after battle-end synchronization.

### 5.5 `select_ship`

`select_ship` shall represent the synchronized ship-selection phase between or before battle rounds.

### 5.6 `in_battle`

`in_battle` shall represent active deterministic battle simulation while battle input exchange is live.

### 5.7 `ending_battle` and `ending_battle_2`

The subsystem shall preserve a two-stage end-of-battle synchronization model semantically equivalent to the current one:

- one stage for exchanging intended terminal frame counts,
- one stage for final readiness after both sides have simulated to the agreed target frame.

The exact internal representation may differ, but both logical stages shall remain observable and correct.

## 6. Protocol wire format and versioning

### 6.1 Packet framing

Each wire packet shall begin with a fixed-size header containing at minimum:

- packet length,
- packet type.

The Rust port shall preserve network-order serialization behavior compatible with the legacy protocol.

### 6.2 Alignment and length handling

If the Rust port must interoperate with the current C implementation or preserve on-the-wire compatibility with it, packet lengths, payload layout, and any padding/alignment behavior relied upon by the current protocol shall be preserved.

Until a compatibility audit proves that a revised wire format is safe, the port shall treat the current packet framing and payload layout as compatibility-significant.

### 6.3 Required packet types

The end-state protocol contract shall preserve the semantic packet set needed by the current subsystem:

- initialization/version packet,
- ping,
- ack,
- ready,
- fleet update,
- team-name update,
- setup confirmation packets,
- seed-random packet,
- input-delay packet,
- select-ship packet,
- battle-input packet,
- frame-count packet,
- checksum packet when checksum verification is enabled,
- abort packet,
- reset packet.

Internal implementation may centralize or refactor handlers, but the externally visible protocol capabilities shall remain available.

### 6.4 Version negotiation rules

On receipt of a peer initialization packet, the subsystem shall:

- require exact compatibility for the netplay protocol version according to the product's chosen compatibility policy,
- reject peers below the product's supported minimum compatible product version,
- communicate rejection through the abort mechanism where protocol-compatible communication is still possible,
- and fail the connection cleanly.

The current compatibility baseline is the legacy protocol family and minimum-UQM-version policy exposed by the C implementation. The Rust port shall preserve that compatibility floor unless an explicit compatibility decision changes it project-wide.

## 7. Handshake / confirmation behavior

### 7.1 Purpose

The subsystem shall provide a setup confirmation protocol that allows both peers to confirm the current setup before advancing into pre-battle negotiation.

### 7.2 Scope restriction

That confirmation protocol shall only be meaningful during `in_setup`.

Packets belonging to the confirmation protocol received in incompatible states shall be treated as protocol errors.

### 7.3 Local confirmation

When SuperMelee requests confirmation for a connection during `in_setup`, the subsystem shall record that the local side has confirmed and shall emit the protocol messages necessary to bring the peer pair toward mutual confirmation.

Duplicate local confirmation requests while an equivalent local confirmation is already active shall fail cleanly rather than silently producing inconsistent protocol state.

### 7.4 Cancellation

When SuperMelee requests cancellation of a setup confirmation that is still active, the subsystem shall transmit whatever cancellation signaling is required by the protocol and shall ensure that the connection does not incorrectly remain in a mutually confirmed state.

### 7.5 Remote invalidation due to setup edits

When a remote setup mutation invalidates an in-progress or completed local confirmation according to the protocol rules, the subsystem shall:

- cancel the relevant confirmation state as required by protocol correctness,
- surface an event or callback that allows SuperMelee to inform the user that reconfirmation is required,
- and prevent stale confirmation from incorrectly advancing the session.

### 7.6 Completion effect

When both sides complete setup confirmation successfully, the subsystem shall transition the connection from `in_setup` to `pre_battle`.

## 8. Ready synchronization behavior

### 8.1 Generic ready primitive

The subsystem shall provide a reusable ready-synchronization primitive that can be used in multiple phases where both peers must rendezvous before progressing.

### 8.2 Required use sites

That primitive shall be usable for at least:

- initialization completion,
- pre-battle negotiation completion,
- battle entry,
- ship-selection entry/completion where required,
- battle-end synchronization.

### 8.3 Callback/event semantics

When both sides become ready for a given ready rendezvous, the subsystem shall trigger the registered completion action exactly once for that rendezvous and clear any one-shot ready callback state required to safely reuse the same connection for later rendezvous.

### 8.4 Blocking/waiting integration

The subsystem shall provide a way for higher layers to wait for ready completion while still servicing network traffic and async callbacks.

The exact mechanism may be callback-driven, future-driven, coroutine-driven, or event-loop-driven in Rust; it need not mimic the C `DoInput()` implementation. It shall, however, preserve the same behavioral outcome: waiting for readiness must not starve network progress.

## 9. Setup synchronization contract

### 9.1 Supported setup data

During `in_setup`, the subsystem shall support synchronization of at least:

- fleet contents,
- individual fleet-slot updates,
- team names.

### 9.2 Bootstrap synchronization

When a connection first enters setup synchronization, or when the surrounding SuperMelee flow explicitly re-enters setup after a battle/reset, the subsystem shall support a full bootstrap synchronization of the current setup state.

That bootstrap shall be sufficient for the remote peer to reconstruct the current fleet and team-name state relevant to the networked session.

### 9.3 Conflict resolution

The Rust port shall preserve the effective externally visible conflict-resolution behavior of the current C implementation for concurrent or crossing setup edits unless and until the project deliberately specifies a new setup conflict model.

In particular, if two peers make conflicting edits while synchronization is in flight, the subsystem shall resolve them deterministically and consistently across both peers.

### 9.4 Separation of concerns

Netplay transports synchronized setup updates, but it does not own the canonical local menu/editing rules. The authoritative application of those changes to SuperMelee's setup state remains on the SuperMelee side of the boundary.

## 10. Pre-battle negotiation

### 10.1 RNG seed synchronization

The subsystem shall provide a pre-battle mechanism by which exactly one side of a peer pair is responsible for originating the random seed used to synchronize deterministic battle startup, while the other side receives and applies it.

The asymmetry used to choose the sending side shall be stable for the lifetime of the connection.

### 10.2 Input-delay negotiation

The subsystem shall support per-peer advertisement of preferred battle input delay during `pre_battle`.

The actual battle input delay used for the session shall be chosen deterministically from the participating peers' advertised values and the local configured minimum/preference.

The current compatibility target is max-of-participants behavior. The Rust port shall preserve that behavior unless the project explicitly changes the rule across the product.

### 10.3 Pre-battle completion

The subsystem shall not permit entry into battle until pre-battle negotiation has completed successfully across all required network-controlled peers.

## 11. Battle input synchronization model

### 11.1 Deterministic delayed-input model

The subsystem shall implement a deterministic delayed-input model for network-controlled battle participants.

For each player with network-delivered battle input, the subsystem shall buffer remote input such that battle simulation consumes it only after the negotiated input delay.

### 11.2 Buffer initialization

At battle start, the subsystem shall initialize battle input buffers in a way that permits deterministic startup before remote input for the earliest delayed frames has been received.

The current compatibility target is equivalent to pre-filling each player's input buffer with `input_delay` neutral input frames.

### 11.3 Buffer capacity

The subsystem shall size battle input buffers to tolerate the worst-case skew implied by the deterministic delayed-input model and the current peer-to-peer lockstep behavior.

### 11.4 Delivery to battle

When battle requests network-controlled input for a player, the subsystem shall:

- return the next buffered input if available,
- continue servicing network traffic while waiting if not yet available,
- abort the session cleanly if the connection is lost during such a wait,
- and avoid returning speculative or out-of-order input.

### 11.5 Wire transport of battle input

The subsystem shall serialize and deliver battle-input packets in the active battle phases and preserve in-order semantics compatible with the current protocol.

## 12. Ship-selection synchronization

### 12.1 Selection phase

The subsystem shall synchronize entry into ship-selection phases before the networked battle/input layer begins consuming remote ship-selection choices.

### 12.2 Remote selection delivery

During `select_ship`, the subsystem shall accept remote ship-selection packets, validate them against the current selection phase context, and deliver them to the battle/SuperMelee side in a form sufficient to update the local selection UI/state.

### 12.3 Local selection reporting

When the local side finalizes a network-relevant ship selection, the subsystem shall transmit that result to the connected peers in the form required by the protocol.

### 12.4 Ship-selection commit/validation boundary contract

The following ownership split applies at the netplay↔SuperMelee boundary for remote ship selections:

- **Transport-level validation** is netplay-owned: packet framing, protocol phase correctness (e.g., selection packet received during `select_ship` phase), and wire-format integrity. A transport-level validation failure is a netplay protocol error.
- **Fleet/rules semantic validation** is SuperMelee-owned: whether the selected ship identity is valid for the remote fleet at that moment (e.g., ship exists in the remote fleet and has not already been eliminated). Netplay delivers the decoded selection identity; SuperMelee determines whether it is semantically valid.
- **Commit authority:** SuperMelee is authoritative for committing a remote selection into the battle-facing selection state. Netplay delivers a transport-valid selection; SuperMelee applies it, and may reject it if fleet/rules semantic validation fails. A semantically invalid remote selection shall not be committed into battle-facing state.
- **Invalid remote selection handling:** If a remote selection fails fleet/rules semantic validation, the integrated system shall treat this as a sync/protocol error. The required netplay-side behavior is:
  1. The battle handoff shall be blocked immediately; the semantically invalid selection shall not proceed to battle.
  2. Netplay shall initiate the session reset path (§15.1), not a graceful recovery or retry. A semantically invalid remote selection is an unrecoverable sync divergence because both peers should agree on the valid fleet roster.
  3. If reset protocol exchange succeeds, control returns to the SuperMelee setup flow. If the peer does not confirm reset, the session shall abort (§15.4).
  4. SuperMelee shall not silently substitute a different ship or proceed with an invalid selection.
- **Post-acceptance:** SuperMelee shall not re-reject a remote selection that it has already accepted and committed into battle-facing state.

This boundary contract is mirrored in `supermelee/specification.md` §9.4.

## 13. Checksum verification and sync-loss handling

### 13.1 Purpose

When checksum verification is enabled, the subsystem shall support periodic exchange of checksums over deterministic battle state for desync detection.

### 13.2 Sampling model

The checksum model shall preserve the delayed verification semantics of the current system:

- local battle state checksum is sampled at configured frame intervals,
- transmitted to peers,
- stored locally and remotely,
- and compared only after enough delayed frames have elapsed for both peers to have observed the relevant frame.

### 13.3 Verification scope

The Rust port shall preserve the existing deterministic compatibility target for what is checksummed unless a broader simulation audit deliberately revises that boundary.

### 13.4 Handling malformed or mistimed checksum packets

Checksum packets received for invalid intervals or implausible frame ranges shall not by themselves force immediate connection teardown if the protocol can safely ignore them.

The current compatibility target is warning/discard behavior for such packets.

### 13.5 Sync loss

When delayed checksum verification detects a mismatch between peers for the same frame, the subsystem shall treat that as synchronization loss and initiate the session reset path using the sync-loss reset reason.

## 14. End-of-battle synchronization

### 14.1 Required two-stage flow

The subsystem shall preserve a battle-end synchronization flow semantically equivalent to the current implementation:

1. each side signals readiness to stop active battle,
2. each side communicates a target ending frame count,
3. both sides continue simulation until the agreed maximum target frame count,
4. both sides perform a final readiness rendezvous,
5. the session returns to `inter_battle` or the next owning phase.

### 14.2 Frame-count correctness

The subsystem shall guarantee that each peer knows the effective terminal frame boundary far enough in advance that it does not deadlock waiting for battle-input data after the last meaningful frame.

## 15. Reset, abort, error, and disconnect behavior

### 15.1 Reset semantics

The subsystem shall provide a reset protocol that terminates the current game/session round and returns control to the SuperMelee setup flow rather than treating the event as a generic disconnect.

A reset shall complete only after the equivalent of both local and remote reset participation has been observed.

### 15.2 Remote reset confirmation

If one side receives a reset before it has initiated one locally, the subsystem shall confirm the reset back to the peer as required by protocol correctness.

### 15.3 Reset completion callback/event

The subsystem shall provide a callback, event, or awaitable completion hook so higher layers can wait for reset completion while still servicing network progress.

### 15.4 Abort semantics

The subsystem shall provide an abort mechanism for unrecoverable compatibility/protocol failures where the session should terminate rather than return to setup as an ordinary gameplay reset.

### 15.5 Disconnect handling

When the transport disconnects unexpectedly during setup, selection, or battle, the subsystem shall surface the disconnect promptly to higher layers so the owning flow can abort or recover according to the surrounding product behavior.

### 15.6 Protocol errors

Packets that are malformed, structurally invalid, or received in impossible states shall be treated as protocol errors.

The subsystem shall fail such sessions cleanly and consistently, preserving local state integrity.

## 16. Integration contract with SuperMelee

### 16.1 Menu/setup integration

The subsystem shall provide the information SuperMelee needs to determine:

- whether a network-controlled side is connected,
- whether the connection is in setup-ready state,
- whether confirmation is in progress or invalidated,
- whether match start may proceed from the network perspective.

### 16.2 Setup mutation integration

The subsystem shall expose APIs or events sufficient for SuperMelee to:

- send local setup changes,
- receive remote setup changes,
- bootstrap full setup sync when entering or re-entering setup.

### 16.3 Feedback integration

The subsystem shall emit enough structured events for SuperMelee to present player-facing feedback for at least:

- connection established,
- connection failed,
- connection closed,
- confirmation invalidated,
- reset reason,
- abort reason.

This specification does not require those events to remain text-based or callback-shaped exactly as in the C implementation.

## 17. Integration contract with battle

### 17.1 Battle entry

The subsystem shall provide battle with:

- initialized input-delay state,
- initialized battle-input buffers,
- initialized checksum buffers when enabled,
- synchronized readiness to begin battle.

### 17.2 Battle loop servicing

The subsystem shall support continued network progress during the battle loop without requiring battle to understand the full protocol details.

### 17.3 Battle exit

On battle abort, sync loss, or manual reset, the subsystem shall cooperate with battle and SuperMelee so control returns to the appropriate setup/inter-battle flow cleanly.

## 18. Lower-level network-library dependencies

### 18.1 Event-driven transport servicing

The subsystem shall be compatible with an event-driven lower-level networking layer that exposes readiness notifications and deferred callbacks.

### 18.2 Callback/async progress

The subsystem shall ensure that waiting for protocol progression does not starve:

- network receive processing,
- queued deferred callbacks,
- timer/alarm-driven async work.

### 18.3 Internal freedom

The Rust port may implement this using futures, tasks, explicit polling loops, or structured callbacks so long as the behavioral contract above is preserved.

## 19. Compatibility and non-goals

### 19.1 Compatibility targets

The Rust port shall preserve the observable behavior of the current C netplay subsystem with respect to:

- two-peer SuperMelee netplay sessions,
- connection state progression,
- initialization/version rejection rules,
- setup synchronization,
- setup confirmation/cancellation,
- input-delay negotiation,
- ship-selection synchronization,
- battle input delivery,
- checksum-based desync detection when enabled,
- battle-end synchronization,
- reset/abort/disconnect outcomes.

### 19.2 Non-goals

This specification does not require:

- preserving current C file decomposition,
- preserving callback pointer layouts or struct layouts,
- preserving the exact C waiting implementation via `DoInput()`,
- expanding the subsystem to own metaserver/discovery behavior,
- redesigning SuperMelee UI flow.

## 20. Audit-sensitive areas

The following areas should be treated as compatibility-sensitive questions rather than assumed freedoms:

- whether the current packet wire layout must remain byte-for-byte compatible for mixed C/Rust interoperability,
- whether ping/ack must remain fully implemented even if currently underused,
- whether currently unused abort reasons such as `invalid_hash` must be preserved for future parity,
- whether any hidden external tooling or historical builds depend on the precise current protocol version values.
