# Phase 05a: Seek-Past-End Fix — Implementation Verification

## Phase ID
`PLAN-20260224-STATE-SWAP.P05a`

## Prerequisites
- Required: Phase P05 completed
- Expected: seek, read, write, open, delete, length all updated in state_file.rs

## Structural Verification
- [ ] No `todo!()` markers remain in `state_file.rs`
- [ ] `StateFile::seek` removes upper clamp — only negative clamp to 0
- [ ] `StateFile::read` checks `self.data.len()` (physical), NOT `self.used`
- [ ] `StateFile::write` updates `self.used = max(self.used, self.ptr)` after advancing cursor
- [ ] `StateFile::write` grows buffer with `max(required, current * 3 / 2)` strategy
- [ ] `StateFile::length()` returns `self.used`
- [ ] `StateFile::open()` pre-allocates to `size_hint` on first open
- [ ] `open_count` is `i32`

## Verification Commands

```bash
cd rust && cargo fmt --all --check
cd rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
cd rust && cargo test --workspace --all-features
```

## Semantic Verification
- [ ] Seek past end: `seek(1000, SEEK_SET)` on 10-byte file → cursor at 1000
- [ ] Read past end: returns 0 bytes
- [ ] Write past end: buffer grows, gap is zero-filled
- [ ] Length after write at offset 100: returns 100 + written_len
- [ ] Length after "wb" open: returns 0
- [ ] Physical read: can read between `used` and `data.len()` (matches C behavior)
- [ ] All tests pass: `cargo test` shows 0 failures

## Deferred Implementation Detection

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/state/state_file.rs
# Expected: no matches
```

## Gate Decision
- [ ] PASS: proceed to P06
- [ ] FAIL: fix implementation
