# Phase 09a: Directory Enumeration Merge & Regex — Verification

## Phase ID
`PLAN-20260314-UIO.P09a`

## Prerequisites
- Required: Phase 09 completed

## Verification Commands

```bash
cd rust && cargo fmt --all --check
cd rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
cd rust && cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] `dirlist.rs` exists with `get_merged_dir_list`, `matches_pattern`, `build_c_dir_list`, `free_c_dir_list`
- [ ] `regex` crate in `Cargo.toml`
- [ ] No hard-coded regex patterns remain in `uio_bridge.rs`
- [ ] `uio_getDirList` delegates to new dirlist module
- [ ] 14+ tests exist and pass

## Semantic Verification Checklist
- [ ] `.rmp` regex matches case-insensitively (`.rmp`, `.RMP`, `.Rmp`)
- [ ] `.zip`/`.uqm` regex matches case-insensitively
- [ ] Cross-mount merge deduplicates correctly
- [ ] Ordering follows mount precedence with lexical-within-mount for `.rmp` case
- [ ] Empty directory listing returns valid empty DirList (not null)
- [ ] DirList_free handles null input safely
- [ ] All pre-existing tests pass

## Integration Verification
- [ ] Build: `cd sc2 && make`
- [ ] C startup `loadIndices()` at `options.c:490-507` discovers `.rmp` files correctly
- [ ] C startup `mountDirZips()` at `options.c:469-480` discovers `.zip`/`.uqm` files correctly

## Gate Decision
- [ ] PASS: proceed to Phase 10
- [ ] FAIL: fix issues before proceeding

## Phase Completion Marker
Create: `project-plans/20260311/uio/.completed/P09a.md`
