# Phase 12: End-to-End Integration

## Phase ID
`PLAN-20260314-FILE-IO.P12`

## Prerequisites
- Required: Phase 11a completed
- Expected: All individual gap closures are verified

## Purpose
Verify that all gap closures work together in the full engine context. This phase adds no new code — it validates integration across subsystem boundaries.

## Integration Contract

### Existing Callers
- `sc2/src/options.c` → `uio_init`, `uio_openRepository`, `uio_mountDir`, `uio_openDir`, `uio_openDirRelative`, `uio_getDirList`, `uio_stat`, `uio_DirList_free`, `uio_transplantDir`
- `sc2/src/libs/graphics/sdl/sdluio.c` → `uio_fopen`, `uio_fread`, `uio_fseek`, `uio_ftell`, `uio_ferror`, `uio_fclose`
- `sc2/src/libs/network/netplay/packetq.c` → `uio_fprintf`
- `sc2/src/libs/resource/loadres.c` → `uio_getStdioAccess`, `uio_StdioAccessHandle_getPath`, `uio_releaseStdioAccess`
- `rust/src/sound/aiff_ffi.rs` → `uio_open`, `uio_read`, `uio_close`, `uio_fstat`
- `rust/src/sound/heart_ffi.rs` → `uio_fopen`, `uio_fread`, `uio_fseek`, `uio_ftell`, `uio_fclose`

### Existing Code Replaced/Removed
- `sc2/src/libs/uio/uio_fread_shim.c` — removed from active Rust-mode build (Rust exports `uio_fread` directly)
- `uio_feof` hardcoded `1` → stream-state-aware
- `uio_ferror` hardcoded `0` → stream-state-aware
- `uio_vfprintf` stub → functional formatted output
- `uio_access` existence-only → mode-aware
- regex special cases → audited compatibility implementation
- FileBlock stubs → functional ABI-correct implementation
- StdioAccess stubs → functional path-resolution + temp-copy
- ZIP mount exclusion → ZIP mounts active in resolution

### User Access Path
- Game boot → `options.c` mount sequence → content loading → rendering
- Game save/load → `uio_fopen`/`uio_fwrite`/`uio_fread` on save files
- Netplay → `uio_fprintf` debug logging

### End-to-End Verification

```bash
# 1. Full build
cd sc2 && make clean && make

# 2. Rust unit tests
cargo test --workspace --all-features

# 3. Stronger symbol/linkage verification
nm -u build/unix/uqm | grep uio_ || true
nm build/unix/uqm | grep ' uio_' || true
# Review full relevant output; do not truncate with head.

# 4. Game boot test (manual/integration)
# - Start game
# - Verify main menu renders (content loaded from packages)
# - Verify audio plays (sound loaded from packages)
# - Start a new game, verify gameplay renders
# - Save game, reload save
# - Exit cleanly
```

## Integration Test Scenarios

### Scenario 1: Startup Mount Sequence
1. `uio_init()` initializes subsystem
2. `uio_openRepository()` creates repository
3. stdio config/content mounts are created with deterministic ordering
4. `uio_getDirList(..., archive-regex, MATCH_REGEX)` finds `.uqm` packages
5. `uio_mountDir(..., UIO_FSTYPE_ZIP, ...)` mounts each package
6. `uio_getDirList(..., RMP_REGEX, MATCH_REGEX)` finds resource indices
7. If AutoMount branch active, listing-triggered mounts behave per audited contract
8. All mounts are active and resolvable

### Scenario 2: SDL Image Loading
1. `uio_fopen(contentDir, "image.png", "rb")` opens stream from ZIP-backed mount
2. `sdluio_read()` calls `uio_fread()` → returns decompressed data
3. `uio_ferror()` returns 0 (no error)
4. `uio_feof()` returns non-zero at end of file
5. `uio_fclose()` closes without leak

### Scenario 3: Overlay Mutation and Save Game
1. writable save/config mount exists with any read-only overlays above/below as configured
2. `uio_fopen(saveDir, "save.dat", "wb")` resolves to the correct writable mount or fails if shadowed by read-only visible path
3. `uio_fwrite()` writes save data
4. `uio_fclose()` flushes and closes
5. `uio_fopen(saveDir, "save.dat", "rb")` reopens for reading
6. `uio_fread()` reads save data back correctly

### Scenario 4: Resource StdioAccess Boundaries
1. `uio_getStdioAccess(contentDir, "resource.dat", 0, tempDir)` resolves file
2. If stdio-backed concrete file: returns direct host path
3. If ZIP-backed file: creates temp copy, returns temp path
4. Merged directories / synthetic archive dirs are rejected correctly for file-location-style queries
5. `uio_releaseStdioAccess()` cleans up correctly

### Scenario 5: Cleanup After Topology Change
1. Open file/stream/dir/stdio-access handle
2. Unmount the owning mount or close the repository topology as applicable
3. Cleanup operations (`uio_close`, `uio_fclose`, `uio_closeDir`, `uio_releaseStdioAccess`) remain safe

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cd sc2 && make clean && make
```

## Structural Verification Checklist
- [ ] Full build succeeds with no undefined symbols
- [ ] No exported-symbol C shims remain in active Rust-mode build for `uio_*`
- [ ] Linkage verification reviews complete relevant `nm` output rather than truncated samples
- [ ] All Rust tests pass

## Semantic Verification Checklist (Mandatory)
- [ ] Game boots successfully
- [ ] Main menu renders (content loaded)
- [ ] Audio plays (sound loaded)
- [ ] Save/load cycle works with overlay semantics intact
- [ ] StdioAccess direct-path/temp-copy boundaries work in-engine
- [ ] Cleanup after topology change is safe
- [ ] Clean shutdown (no crashes, no error messages)

## Phase Completion Marker
Create: `project-plans/20260311/file-io/.completed/P12.md` containing:
- linkage verification notes
- integration scenario results
