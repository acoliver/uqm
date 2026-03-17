# Phase 07: Connection & Transport — Stub/TDD/Impl

## Phase ID
`PLAN-20260314-NETPLAY.P07`

## Prerequisites
- Required: Phase 06a (Packet Codec Verification) completed and passed
- Expected: packet codec fully implemented and tested

## Requirements Implemented (Expanded)

### REQ-NP-CONN-001 / REQ-NP-CONN-002
**Requirement text**: When SuperMelee opens a netplay connection for a player slot, the subsystem shall create a connection object for that slot and begin either listening or connecting according to the configured peer role. When transport establishes successfully, it shall transition into initialization state.

Behavior contract:
- GIVEN: A player slot and peer options
- WHEN: `open_connection()` is called
- THEN: A connection object is created, transport is initiated, state transitions to `Connecting`

### REQ-NP-CONN-003
**Requirement text**: When pre-establishment listen/connect setup fails, the subsystem shall surface failure cleanly, return to `Unconnected`, and make the failure observable through the integration event path without leaking transport resources or stale registry membership.

Behavior contract:
- GIVEN: A player slot begins listen/connect setup
- WHEN: bind, connect, accept, or pre-init transport progress fails before establishment completes
- THEN: the connection returns to `Unconnected`, any transient transport handles are released, and failure is surfaced through the path defined for `NetplayEvent::ConnectionFailed`
- AND: immediate configuration errors may return directly from `open_connection()`, but asynchronous establishment failures must be retained for `poll()` delivery

### REQ-NP-CONN-004
**Requirement text**: When a connection is closed, the subsystem shall mark the connection disconnected, stop using the transport handle, emit the close event, and release resources.

### REQ-NP-COMPAT-003 / specification §18
**Requirement text**: If mixed C/Rust interoperability is required, packet semantics shall be preserved, and waiting/progress behavior must not starve network servicing.

Behavior contract:
- GIVEN: Server and client connect
- WHEN: Connection is established
- THEN: Server has `discriminant=true`, client has `discriminant=false`
- AND: the transport/polling design preserves the shared progress-loop contract validated in preflight/analysis

## Implementation Tasks

### Files to create

- `rust/src/netplay/connection/net_connection.rs` — NetConnection struct and lifecycle
  - marker: `@plan PLAN-20260314-NETPLAY.P07`
  - marker: `@requirement REQ-NP-CONN-001 REQ-NP-CONN-002 REQ-NP-CONN-003 REQ-NP-CONN-004`
  - Contents:
    - `NetConnection` struct with all fields from pseudocode
    - `fn new(player: PlayerId, options: PeerOptions) -> NetConnection`
    - `fn state(&self) -> NetState`
    - `fn set_state(&mut self, state: NetState) -> Result<(), NetplayError>`
    - `fn is_connected(&self) -> bool`
    - `fn is_disconnected(&self) -> bool`
    - `fn player(&self) -> PlayerId`
    - `fn discriminant(&self) -> bool`
    - `fn state_flags(&self) -> &StateFlags` / `fn state_flags_mut(&mut self) -> &mut StateFlags`
    - `fn queue_packet(&mut self, packet: &Packet)`
    - `fn mark_connected(&mut self, stream: TcpStream, is_server: bool)`
    - `fn mark_connection_failed(&mut self, error: NetplayError)` — clears transient transport state, returns state to `Unconnected`, preserves failure details for facade event delivery
    - `fn take_pending_connection_failure(&mut self) -> Option<NetplayError>`
    - `fn mark_disconnected(&mut self)`
    - `fn close(&mut self)`
    - `fn receive_packets(&mut self) -> Result<Vec<Packet>, NetplayError>`
    - `fn flush(&mut self) -> Result<usize, NetplayError>`

- `rust/src/netplay/connection/transport.rs` — TCP transport setup
  - marker: `@plan PLAN-20260314-NETPLAY.P07`
  - marker: `@requirement REQ-NP-CONN-001 REQ-NP-CONN-002 REQ-NP-CONN-003 REQ-NP-READY-004`
  - Contents:
    - `fn listen_on_port(port: u16) -> Result<TcpListener, NetplayError>`
    - `fn accept_connection(listener: &TcpListener) -> Result<TcpStream, NetplayError>`
    - `fn connect_to_peer(host: &str, port: u16, timeout_ms: u64) -> Result<TcpStream, NetplayError>`
    - `fn configure_stream(stream: &TcpStream) -> Result<(), NetplayError>` — set nonblocking + nodelay
    - `enum TransportProgress` or equivalent representing `Pending`, `Established(TcpStream)`, `Failed(NetplayError)` for pre-establishment progress
    - transport notes documenting how this module participates in the shared progress loop
    - transport notes documenting how async establishment failures are surfaced to the facade without direct event emission from the transport layer

- `rust/src/netplay/connection/registry.rs` — Global connection array
  - marker: `@plan PLAN-20260314-NETPLAY.P07`
  - marker: `@requirement REQ-NP-CONN-001 REQ-NP-CONN-003 REQ-NP-CONN-004 REQ-NP-READY-004`
  - Contents:
    - `ConnectionRegistry` struct: `connections: [Option<NetConnection>; NUM_PLAYERS]`
    - `fn new() -> ConnectionRegistry`
    - `fn add(&mut self, player: PlayerId, conn: NetConnection) -> Result<(), NetplayError>`
    - `fn remove(&mut self, player: PlayerId) -> Option<NetConnection>`
    - `fn get(&self, player: PlayerId) -> Option<&NetConnection>`
    - `fn get_mut(&mut self, player: PlayerId) -> Option<&mut NetConnection>`
    - `fn num_connected(&self) -> usize`
    - `fn close_all(&mut self)`
    - `fn close_disconnected(&mut self)`
    - `fn for_each_connected<F>(&mut self, f: F)`
    - `fn poll_all(&mut self, timeout_ms: u32) -> Result<Vec<(PlayerId, Packet)>, NetplayError>` — preserve source identity for later dispatch
    - `fn collect_connection_failures(&mut self) -> Vec<(PlayerId, NetplayError)>` — drains pending pre-establishment failures after cleanup so P12 can emit `ConnectionFailed`
    - `fn flush_all(&mut self) -> Result<(), NetplayError>`

### Files to modify

- `rust/src/netplay/connection/mod.rs` — Add sub-module declarations

### Tests

**connection/net_connection.rs tests:**
- `test_new_connection_defaults`
- `test_set_state_valid`
- `test_set_state_invalid`
- `test_mark_connected`
- `test_mark_connection_failed_clears_transient_transport`
- `test_take_pending_connection_failure_drains_once`
- `test_mark_disconnected`
- `test_queue_packet`
- `test_close_cleans_transport`

**connection/transport.rs tests:**
- `test_listen_on_port`
- `test_connect_to_peer_localhost`
- `test_connect_timeout`
- `test_configure_stream_nonblocking`
- `test_configure_stream_nodelay`
- `test_transport_progress_reports_pending_then_established`
- `test_transport_progress_reports_pre_establishment_failure`
- `test_transport_progress_loop_can_poll_without_starvation` — validates the chosen transport design participates in the shared wait-progress contract

**connection/registry.rs tests:**
- `test_registry_empty`
- `test_registry_add_remove`
- `test_registry_add_duplicate_fails`
- `test_registry_close_all`
- `test_registry_for_each_connected`
- `test_registry_num_connected`
- `test_poll_all_preserves_source_player_identity`
- `test_collect_connection_failures_drains_failed_connecting_entries`

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo test --all-features -- netplay::connection
```

## Structural Verification Checklist
- [ ] All 3 connection sub-module files exist
- [ ] `connection/mod.rs` declares all sub-modules
- [ ] `NetConnection` struct has all fields from pseudocode
- [ ] Transport functions use the approved transport design from preflight/analysis
- [ ] Registry manages `[Option<NetConnection>; NUM_PLAYERS]`
- [ ] `poll_all()` preserves source connection identity
- [ ] Pre-establishment failure state is captured in connection/registry types, not dropped in logs only
- [ ] At least 21 tests defined

## Semantic Verification Checklist (Mandatory)
- [ ] TCP loopback test: server listens, client connects, both get valid streams
- [ ] Discriminant correctly assigned: server=true, client=false
- [ ] State transition enforcement works through `set_state()`
- [ ] `close()` leaves connection in Unconnected state
- [ ] `receive_packets()` uses the ReadBuffer from P06 correctly
- [ ] `flush()` uses the PacketQueue flush from P06 correctly
- [ ] Registry prevents duplicate player slot assignment
- [ ] Transport/polling behavior is consistent with the shared progress-loop contract from analysis
- [ ] Pre-establishment transport failures return to `Unconnected` with transport handles cleaned up
- [ ] Async connect/listen failures are retained for later facade/event delivery rather than disappearing as internal errors

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "TODO\|FIXME\|HACK\|todo!()\|unimplemented!()\|placeholder" rust/src/netplay/connection/ | grep -v test
```

## Success Criteria
- [ ] All connection tests pass including TCP loopback
- [ ] No `todo!()` remains
- [ ] Connection lifecycle is fully testable without external services
- [ ] Early registry API does not force later handler/integration churn
- [ ] Failure path is explicit enough for P12 to emit `ConnectionFailed` deterministically

## Failure Recovery
- rollback: `git checkout -- rust/src/netplay/connection/`
- blocking issues: port binding conflicts in tests (use port 0 for OS-assigned), TCP timing, transport/progress mismatch

## Phase Completion Marker
Create: `project-plans/20260311/netplay/.completed/P07.md`
