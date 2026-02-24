# Phase 04a: Seek-Past-End Fix — TDD Verification

## Phase ID
`PLAN-20260224-STATE-SWAP.P04a`

## Prerequisites
- Required: Phase P04 completed
- Expected: 10 new tests in `state_file.rs`

## Structural Verification
- [ ] All 10 tests are present in `rust/src/state/state_file.rs` test module
- [ ] Tests have `@plan` and `@requirement` markers
- [ ] Tests compile: `cargo test --workspace --all-features --no-run`

## Semantic Verification
- [ ] `test_seek_past_end_allowed`: asserts cursor == target (not clamped)
- [ ] `test_read_after_seek_past_end_returns_zero`: asserts read returns 0
- [ ] `test_write_after_seek_past_end_extends_buffer`: asserts length, gap zeros, data correct
- [ ] `test_length_returns_used_not_physical`: asserts length() != physical after wb open
- [ ] `test_read_checks_physical_size_not_used`: asserts reads succeed past `used` within physical
- [ ] `test_open_count_can_go_negative`: asserts no panic on close-without-open
- [ ] All NEW tests fail (RED): `cargo test` output shows failures for new tests

## Gate Decision
- [ ] PASS: proceed to P05
- [ ] FAIL: fix tests
