# Phase 09: ZIP/UQM Archive Mount Support

## Phase ID
`PLAN-20260314-FILE-IO.P09`

## Prerequisites
- Required: Phase 08a completed
- Expected: public FileBlock ABI is complete
- Expected: mount ordering and path resolution are correct
- Expected: cross-mount listing works
- Dependency note: Phase 09 may use FileBlock internally or a direct Rust ZIP reader, but the choice must follow the P00a/P01 dependency rationale rather than an unexamined assumption

## Requirements Implemented (Expanded)

### REQ-FIO-ARCHIVE-MOUNT: Archive content accessible through virtual namespace
**Requirement text**: When a caller mounts an archive (ZIP/UQM), the subsystem SHALL index the archive contents at mount time by reading the central directory. When archive-backed entries participate in cross-mount directory listings, the subsystem SHALL apply the same union/dedup rules as for any other mount type. When a caller opens an archive-backed file for read, the subsystem SHALL return decompressed content transparently.

Behavior contract:
- GIVEN: A ZIP file containing `"data/ship.png"` and `"data/readme.txt"`
- WHEN: `uio_mountDir(repo, "/content", UIO_FSTYPE_ZIP, sourceDir, "package.uqm", "/", ...)` is called
- THEN: `uio_getDirList(contentDir, "data", ...)` includes `"ship.png"` and `"readme.txt"`

- GIVEN: An archive mounted at `/content` with file `"sounds/boom.wav"`
- WHEN: `uio_fopen(contentDir, "sounds/boom.wav", "rb")` is called
- THEN: Returns a valid stream; `uio_fread` returns decompressed content

### REQ-FIO-ARCHIVE-EDGE: Archive duplicate-entry, normalization, and lookup semantics
**Requirement text**: The subsystem SHALL normalize archive entry paths (backslashes and leading slashes), SHALL synthesize implied directories, SHALL use case-sensitive lookup, and SHALL select the last central-directory entry when duplicates normalize to the same path.

### REQ-FIO-MOUNT-AUTOMOUNT: Conditional AutoMount interaction with archives
**Requirement text**: If AutoMount parity is required, archive enumeration-triggered mounting behavior must coexist with ZIP support per the public contract.

Why it matters:
- The game's content is distributed as `.uqm` packages (ZIP format). Without ZIP mount support, the game cannot load art, sound, or data assets from packages.

## Implementation Tasks

### Files to create
- `rust/src/io/zip_reader.rs`
  - ZIP central directory parser
  - Entry index (normalized path → archive entry metadata)
  - Duplicate-entry handling (`last entry wins`)
  - Path normalization inside archives (strip leading slash, normalize separators, strip trailing slash on dir entries)
  - Synthetic directory generation
  - Case-sensitive lookup
  - decompression / CRC validation support
  - marker: `@plan PLAN-20260314-FILE-IO.P09`
  - marker: `@requirement REQ-FIO-ARCHIVE-MOUNT`
  - marker: `@requirement REQ-FIO-ARCHIVE-EDGE`

### Files to modify
- `rust/Cargo.toml`
  - Add decompression dependency only if needed by the selected ZIP strategy
- `rust/src/io/mod.rs`
  - Add `pub mod zip_reader;`
- `rust/src/io/uio_bridge.rs`
  - **`MountInfo` struct**: add archive index/storage field
  - **archive mount path** (`register_mount` / `uio_mountDir`):
    - resolve archive file from `sourceDir` + `sourcePath`
    - parse/index ZIP at mount time
    - fail mount with `EIO` on corruption/I/O failure
    - roll back any partially built archive state or registry insertion on mount-time failure
    - set mount active in registry (remove current ZIP exclusion logic)
    - extend errno mapping for mount-time indexing and registration failures introduced here
  - **archive resolution**:
    - path lookup must be case-sensitive
    - file and synthetic-directory results must participate in overlay behavior
  - **archive topology/thread-safety audit**:
    - confirm registry mutation during archive registration/removal is atomic with respect to readers
    - confirm failed archive indexing leaves no residual registry entry visible to later operations
  - **`uio_open` / `uio_fopen`**:
    - read succeeds with transparent decompression
    - write fails with `EACCES`
    - decompression/CRC read-time failure reports `EIO`
  - **`uio_stat`**:
    - report uncompressed size and correct file-type bits for files/dirs including synthetic dirs
  - **`uio_getDirList`**:
    - expose archive entries and synthesized directories in cross-mount listings
  - **`uio_access`**:
    - `F_OK`/`R_OK` succeed, `W_OK` fails, `X_OK` directory succeeds / file fails per spec

### ZIP index structure guidance
```
struct ZipEntry {
    path: String,            // normalized forward-slash relative path
    compressed_size: u64,
    uncompressed_size: u64,
    compression_method: u16,
    crc32: u32,
    local_header_offset: u64,
    is_directory: bool,
}

struct ZipIndex {
    entries: HashMap<String, ZipEntry>,   // last duplicate central-dir entry wins
    directories: HashSet<String>,         // includes synthesized dirs
    archive_path: PathBuf,
}
```

### Concrete caller touchpoints
- `sc2/src/options.c`: `mountDirZips()` must observe successful ZIP mounts and clean failure for corrupt archives
- `sc2/src/options.c`: `loadIndices()` must continue to find `.rmp` files in mounted archives after indexing changes

### Pseudocode traceability
- Uses pseudocode lines: PC-06 lines 14–33, PC-10 lines 01–15, PC-12 lines 09–14

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cd sc2 && make clean && make
```

## Structural Verification Checklist
- [ ] `zip_reader.rs` module exists with ZIP parsing logic
- [ ] archive dependency choice follows the documented strategy
- [ ] `MountInfo` can hold archive index/state
- [ ] ZIP mounts are no longer excluded from active registry
- [ ] archive path normalization is explicit
- [ ] duplicate-entry handling is explicit (`last central-directory entry wins`)
- [ ] case-sensitive lookup is explicit
- [ ] synthetic directory generation is explicit
- [ ] mount-time archive failure rolls back partial registration/state
- [ ] archive registration/removal topology contract is documented for shared state touched here
- [ ] `uio_open`/`uio_fopen` can read from ZIP entries
- [ ] `uio_stat` returns correct metadata for ZIP entries
- [ ] `uio_getDirList` includes ZIP entries in results
- [ ] errno mapping is extended for mount-time and read-time archive failures introduced here

## Semantic Verification Checklist (Mandatory)
- [ ] Test: mount a test ZIP file; list entries; verify correct names
- [ ] Test: archive entries with backslashes/leading slashes normalize to expected visible paths
- [ ] Test: duplicate archive entries with same normalized path resolve to the last central-directory entry
- [ ] Test: archive lookup is case-sensitive
- [ ] Test: open and read a stored (uncompressed) entry → correct content
- [ ] Test: open and read a deflated entry → correct decompressed content
- [ ] Test: CRC mismatch or decompression failure → `EIO` on read
- [ ] Test: corrupt/unreadable archive mount fails with `NULL`, `errno = EIO`, and leaves no residual registry entry
- [ ] Test: synthetic directories appear in listings and stats
- [ ] Test: write to ZIP entry → `EACCES`
- [ ] Test: `uio_stat` on ZIP entry → correct uncompressed size
- [ ] Test: cross-mount listing includes both ZIP and stdio entries
- [ ] Verification note: archive registration failure and successful registration are both safe under concurrent topology observation
- [ ] If AutoMount required: archive discovery/mount flow remains correct
- [ ] Game boots and loads content from `.uqm` packages
- [ ] `mountDirZips()` in `sc2/src/options.c` successfully mounts archive packages and handles corrupt-archive failures cleanly
- [ ] `loadIndices()` in `sc2/src/options.c` finds `.rmp` files in mounted archives

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/io/uio_bridge.rs rust/src/io/zip_reader.rs
```

## Success Criteria
- [ ] ZIP mount tests pass
- [ ] Archive edge-case tests pass
- [ ] Mount-failure rollback tests pass
- [ ] Game loads content from `.uqm` packages
- [ ] Verification commands pass

## Failure Recovery
- Rollback: `git checkout -- rust/src/io/uio_bridge.rs rust/src/io/mod.rs rust/Cargo.toml`
- Remove: `rust/src/io/zip_reader.rs`
- Risk: large phase; split into parser/integration slices if needed during execution

## Phase Completion Marker
Create: `project-plans/20260311/file-io/.completed/P09.md` containing:
- ZIP support verification summary
- archive edge-case verification summary
- mount-failure rollback verification summary
- archive topology-concurrency review note
- chosen FileBlock-vs-direct-reader implementation note
