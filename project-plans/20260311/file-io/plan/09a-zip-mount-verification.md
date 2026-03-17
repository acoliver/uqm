# Phase 09a: ZIP Mount Verification

## Phase ID
`PLAN-20260314-FILE-IO.P09a`

## Prerequisites
- Required: Phase 09 completed

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cd sc2 && make clean && make
# Boot game — verify content loads from .uqm packages
```

## Structural Verification
- [ ] `zip_reader.rs` parses ZIP end-of-central-directory and central directory entries
- [ ] stored and deflated entries are both supported as required by the chosen strategy
- [ ] synthetic directories are generated for implicit paths
- [ ] duplicate-entry conflict resolution is explicit and verified
- [ ] archive path normalization is explicit and verified
- [ ] case-sensitive lookup is explicit and verified
- [ ] ZIP entries participate in mount resolution and cross-mount directory listings
- [ ] mount-time archive failure rolls back partial registry/state changes
- [ ] archive registration/removal topology contract is documented
- [ ] archive failure and read failure paths extend errno mapping

## Semantic Verification
- [ ] Test ZIP with normalized-path edge cases → correct visible names
- [ ] Test ZIP with duplicate entries → last entry wins
- [ ] Test ZIP lookup case sensitivity
- [ ] Test ZIP with stored entries → correct content
- [ ] Test ZIP with deflated entries → correct decompressed content
- [ ] Test ZIP with CRC/decompression failure → `EIO` on read
- [ ] Test ZIP without explicit directory entries → synthesized dirs visible and stat-able
- [ ] Test corrupt/unreadable ZIP mount → failure return, `NULL`, `errno = EIO`, and no residual registry entry
- [ ] Test cross-mount listing with ZIP + stdio mounts → union works
- [ ] Archive registration failure/success integrity review is complete for shared topology state
- [ ] Game starts and successfully renders content from packages

## Integration Verification
- [ ] `options.c` `mountDirZips()` mounts archives via Rust UIO
- [ ] `options.c` `mountDirZips()` handles corrupt archives without leaving partial mounts behind
- [ ] `options.c` `loadIndices()` finds `.rmp` files in mounted archives
- [ ] SDL RWops adapter successfully reads image data from ZIP-backed streams
- [ ] Sound decoders successfully read audio data from ZIP-backed files

## Phase Completion Marker
Create: `project-plans/20260311/file-io/.completed/P09a.md` summarizing:
- archive semantic verification results
- mount-failure rollback verification result
- integration verification results
