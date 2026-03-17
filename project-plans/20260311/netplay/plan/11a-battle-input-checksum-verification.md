# Phase 11a: Battle Input & Checksum — Verification

## Phase ID
`PLAN-20260314-NETPLAY.P11a`

## Prerequisites
- Required: Phase 11 (Battle Input & Checksum) completed
- Expected: all input and checksum modules implemented with tests passing

## Verification Tasks

### Battle Input Buffer
- [ ] Capacity formula: `input_delay * 2 + 2` matches C `netinput.c:56-89`
- [ ] Pre-fill: `input_delay` zero-input frames inserted at creation
- [ ] FIFO order strictly maintained across push/pop cycles
- [ ] Wrap-around at capacity boundary works correctly
- [ ] push() on full buffer returns error (not undefined behavior)
- [ ] pop() on empty buffer returns None (not panic)

### Input Delivery
- [ ] `receive_input()` tries pop first (no network I/O if data available)
- [ ] `receive_input()` polls network if buffer empty
- [ ] `receive_input()` blocks with timeout if still empty
- [ ] `receive_input()` returns error on connection loss
- [ ] `setup_input_delay()` correctly computes max of all peer delays + local

### CRC32
- [ ] Standard CRC32 test vector: "123456789" → 0xCBF43926
- [ ] `crc32_of_u32` feeds bytes in little-endian order (matching C `crc.c`)
- [ ] Incremental computation matches bulk computation
- [ ] Init value is 0xFFFFFFFF, finalize XORs with 0xFFFFFFFF

### Checksum Buffer
- [ ] Capacity formula matches C `checkbuf.c:41-81`
- [ ] Slot indexing: `(frame / checksum_interval) % capacity`
- [ ] get() returns None for unset frames
- [ ] get() returns None when slot contains different frame (overwritten)

### Checksum Verification
- [ ] Desync only detected when BOTH local and remote checksums present
- [ ] Missing remote checksum → skip (return true)
- [ ] Missing local checksum → error (should always exist)
- [ ] Multiple peers all checked (not just first)
- [ ] `verification_frame()` returns `current_frame - input_delay` when valid

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo test --all-features -- netplay::input
cargo test --all-features -- netplay::checksum
```

## Gate Decision
- [ ] PASS: proceed to Phase 12
- [ ] FAIL: return to Phase 11 and fix issues

## Phase Completion Marker
Create: `project-plans/20260311/netplay/.completed/P11a.md`
