# Phase 05a: Stream State Fix — Implementation Verification

## Phase ID
`PLAN-20260314-UIO.P05a`

## Prerequisites
- Required: Phase 05 completed

## Verification Commands

```bash
cd rust && cargo fmt --all --check
cd rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
cd rust && cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] No `TODO`/`FIXME`/`HACK` in stream-related functions
- [ ] `set_errno` called before every error return in: `uio_fopen`, `uio_fclose`, `uio_fread`, `uio_fwrite`, `uio_fseek`, `uio_ftell`, `uio_fgets`, `uio_fgetc`, `uio_fputc`, `uio_fputs`, `uio_fflush`
- [ ] Buffer deallocation in `uio_fclose` uses matching allocator (libc::free for libc::malloc)
- [ ] `uio_fread` sets STATUS_EOF on short read, STATUS_ERROR on I/O failure
- [ ] `uio_fgetc` sets STATUS_EOF when returning -1 at end of file

## Semantic Verification Checklist
- [ ] All 16 stream state tests pass
- [ ] All pre-existing tests pass
- [ ] `sdluio.c` integration point is satisfied: after a read that returns 0 items, `uio_ferror` correctly distinguishes EOF from error
- [ ] Seek-after-EOF works: `uio_fseek` clears EOF, subsequent reads succeed

## Integration Verification
- [ ] Build full project: `cd sc2 && make` — C code links against Rust exports without errors
- [ ] If build passes, verify `sdluio.c` still compiles (it calls `uio_ferror` and `uio_feof`)

## Gate Decision
- [ ] PASS: stream state fix complete, proceed to Phase 06
- [ ] FAIL: fix issues before proceeding

## Phase Completion Marker
Create: `project-plans/20260311/uio/.completed/P05a.md`
