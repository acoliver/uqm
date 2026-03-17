# Phase 10a: StdioAccess & uio_copyFile Verification

## Phase ID
`PLAN-20260314-FILE-IO.P10a`

## Prerequisites
- Required: Phase 10 completed

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cd sc2 && make clean && make
```

## Structural Verification
- [ ] `uio_getStdioAccess` has the correct 4-parameter signature from `utils.h`
- [ ] `StdioAccessHandleInner` tracks temp files for cleanup
- [ ] direct-path vs temp-copy decision is based on actual object/backing rules
- [ ] `uio_copyFile` is exported with correct ABI
- [ ] StdioAccess/copy failure paths extend errno mapping
- [ ] stdio-access lifetime and cleanup behavior under topology changes is documented
- [ ] conditional temp-mount branch is resolved according to P00a
- [ ] utils ABI audit notes are recorded

## Semantic Verification
- [ ] Direct-path handle: path is host-native, release doesn't delete underlying file
- [ ] Temp-copy handle: decompressed content is correct, release deletes temp artifacts best-effort
- [ ] Directory rejection: `EISDIR`
- [ ] Missing file rejection: `ENOENT`
- [ ] File-location boundary cases (archive entry, merged dir, synthetic archive dir) are tested explicitly
- [ ] Copy file: content matches, partial-failure cleanup works, state remains consistent
- [ ] StdioAccess handle/path remains safely releasable after topology changes until release
- [ ] If temp-mount branch active: repository-visible temp behavior is verified
- [ ] `sc2/src/libs/resource/loadres.c` resource-loading caller path works end-to-end

## Phase Completion Marker
Create: `project-plans/20260311/file-io/.completed/P10a.md` summarizing:
- stdio-access verification result
- copy-file verification result
- stdio-access lifetime/concurrency verification result
- temp-mount branch verification result
