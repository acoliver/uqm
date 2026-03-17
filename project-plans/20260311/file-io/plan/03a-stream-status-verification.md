# Phase 03a: Stream Status Verification

## Phase ID
`PLAN-20260314-FILE-IO.P03a`

## Prerequisites
- Required: Phase 03 completed
- Expected files modified: `rust/src/io/uio_bridge.rs`

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Structural Verification
- [ ] `uio_feof` reads `stream.status` field
- [ ] `uio_ferror` reads `stream.status` field
- [ ] `uio_clearerr` writes `stream.status = UIO_STREAM_STATUS_OK`
- [ ] `rust_uio_fread` calls `set_stream_status()` on EOF and error paths
- [ ] `uio_fseek` clears EOF flag on success
- [ ] `uio_fclose` frees buffer before freeing struct

## Semantic Verification
- [ ] Test: open file, read to EOF, verify `uio_feof() != 0` and `uio_ferror() == 0`
- [ ] Test: `uio_clearerr` after EOF, verify both return 0
- [ ] Test: seek after EOF, verify `uio_feof() == 0`
- [ ] Test: write failure path sets error flag
- [ ] Integration: SDL RWops read→ferror→feof sequence produces correct classification

## Phase Completion Marker
Create: `project-plans/20260311/file-io/.completed/P03a.md`
