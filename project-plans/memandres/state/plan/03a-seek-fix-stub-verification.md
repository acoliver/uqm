# Phase 03a: Seek-Past-End Fix — Stub Verification

## Phase ID
`PLAN-20260224-STATE-SWAP.P03a`

## Prerequisites
- Required: Phase P03 completed
- Expected changes: `rust/src/state/state_file.rs` modified

## Structural Verification
- [ ] `StateFile` struct contains `used: usize` field
- [ ] `StateFile` struct contains `open_count: i32`
- [ ] `StateFile::new()` sets `used = 0`
- [ ] `StateFile::length()` body is `self.used` (not `self.data.len()`)
- [ ] `StateFile::seek()` no longer has `self.data.len() as i64` upper clamp
- [ ] `StateFile::open()` for Write mode sets `self.used = 0`
- [ ] `cargo check --workspace` succeeds

## Semantic Verification
- [ ] The `used` field is distinct from `data.len()`
- [ ] `open_count` can represent negative values (i32)
- [ ] `todo!()` markers exist only in seek/read/write paths that are being modified

## Gate Decision
- [ ] PASS: proceed to P04
- [ ] FAIL: fix stub issues
