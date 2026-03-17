# Phase 03a: Stream State Fix — Stub Verification

## Phase ID
`PLAN-20260314-UIO.P03a`

## Prerequisites
- Required: Phase 03 completed
- All stub changes in `rust/src/io/uio_bridge.rs` are in place

## Verification Commands

```bash
cd rust && cargo fmt --all --check
cd rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
cd rust && cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] `set_errno` function added to `rust/src/io/uio_bridge.rs`
- [ ] `uio_feof` no longer contains hardcoded `1`
- [ ] `uio_ferror` no longer contains hardcoded `0`
- [ ] `uio_clearerr` is no longer an empty body
- [ ] `uio_fseek` includes status reset on success path
- [ ] `uio_fclose` includes `libc::free` for buffer
- [ ] `uio_fflush(NULL)` returns -1 with errno set
- [ ] `uio_fwrite`, `uio_fputc`, `uio_fputs` set stream operation/status

## Semantic Verification Checklist
- [ ] `cargo check` passes — all changes compile
- [ ] `cargo test` passes — no existing tests broken
- [ ] Manual inspection: `uio_feof` body reads `(*stream).status` field
- [ ] Manual inspection: `uio_ferror` body reads `(*stream).status` field
- [ ] Manual inspection: `uio_clearerr` writes `UIO_STREAM_STATUS_OK` to `(*stream).status`
- [ ] No new `TODO`/`FIXME`/`HACK` markers introduced

## Deferred Implementation Detection

```bash
grep -n "always returns\|hardcoded\|intentionally leak" rust/src/io/uio_bridge.rs
```

Expected: no hits for the functions modified in this phase.

## Gate Decision
- [ ] PASS: proceed to Phase 04
- [ ] FAIL: fix issues before proceeding

## Phase Completion Marker
Create: `project-plans/20260311/uio/.completed/P03a.md`
