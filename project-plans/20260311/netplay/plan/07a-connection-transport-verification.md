# Phase 07a: Connection & Transport — Verification

## Phase ID
`PLAN-20260314-NETPLAY.P07a`

## Prerequisites
- Required: Phase 07 (Connection & Transport) completed
- Expected: connection module fully implemented with tests passing

## Verification Tasks

### TCP Integration
- [ ] Server listen + client connect on loopback succeeds in test
- [ ] Stream is set to nonblocking after `configure_stream()`
- [ ] Stream has TCP_NODELAY set after `configure_stream()`
- [ ] Connect timeout test does not hang indefinitely
- [ ] Pre-establishment failure path reports `Pending`/`Failed` distinctly from `Established`

### NetConnection Lifecycle
- [ ] `new()` → state is Unconnected
- [ ] `mark_connected()` → state_flags.connected is true, discriminant set
- [ ] `mark_connection_failed()` → transient transport is cleared, state is Unconnected, failure retained once for facade delivery
- [ ] `close()` → transport is None, state is Unconnected, disconnected flag set
- [ ] `set_state()` enforces transition rules from P05

### Registry
- [ ] Add + get + remove cycle works for both player slots
- [ ] `close_all()` closes all connections
- [ ] `num_connected()` returns accurate count
- [ ] `for_each_connected` skips None and disconnected entries
- [ ] `collect_connection_failures()` drains pending pre-establishment failures after cleanup

### Thread Safety (if applicable)
- [ ] Connection struct does not require `Send` if single-threaded polling model
- [ ] Registry does not require `Sync` if single-threaded
- [ ] Document threading model decision

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo test --all-features -- netplay::connection
```

## Gate Decision
- [ ] PASS: proceed to Phase 08
- [ ] FAIL: return to Phase 07 and fix issues

## Phase Completion Marker
Create: `project-plans/20260311/netplay/.completed/P07a.md`
