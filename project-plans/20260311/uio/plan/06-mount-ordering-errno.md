# Phase 06: Mount Ordering & errno — Stub/TDD/Impl

## Phase ID
`PLAN-20260314-UIO.P06`

## Prerequisites
- Required: Phase 05a completed
- Stream state fix is complete and verified

## Requirements Implemented (Expanded)

### REQ-UIO-MOUNT-002: Mount placement semantics
**Requirement text**: When a mount is created with top, bottom, above, or below placement semantics, the subsystem shall place the mount according to those placement semantics.

Behavior contract:
- GIVEN: an existing mount A in a repository
- WHEN: mount B is created with `uio_MOUNT_TOP`
- THEN: B has higher precedence than A (B is checked first in resolution)

- GIVEN: mounts A (top) and B (bottom) exist
- WHEN: mount C is created with `uio_MOUNT_ABOVE` relative to B
- THEN: resolution order is A, C, B

- GIVEN: mounts A and B exist
- WHEN: mount C is created with `uio_MOUNT_BELOW` relative to A
- THEN: resolution order is A, C, B

### REQ-UIO-MOUNT-003: Mount precedence for path resolution
**Requirement text**: When multiple mounts can satisfy the same virtual path, the subsystem shall resolve using mount precedence.

Behavior contract:
- GIVEN: mount A (at `/`) and mount B (at `/`, above A) both contain `foo.txt`
- WHEN: a caller opens `/foo.txt`
- THEN: the file from mount B is returned (higher precedence)

- GIVEN: mount A at `/content`, mounted with TOP
- AND: mount B at `/content/packages`, mounted earlier with lower placement precedence
- WHEN: a caller opens `/content/packages/foo.txt`
- THEN: resolution follows the current provisional rule from requirements: explicit placement precedence first, then longest matching mount-point prefix, then recency/insertion order
- AND: this case is covered by explicit tests instead of assuming raw registry insertion order is always equivalent

### REQ-UIO-ERR-002: errno set on failure
**Requirement text**: Functions that fail shall set `errno` to an appropriate POSIX error code.

Behavior contract:
- GIVEN: `uio_open(dir, "nonexistent.txt", O_RDONLY, 0)` fails
- WHEN: caller checks `errno`
- THEN: `errno == ENOENT`

### REQ-UIO-CONC-002: Serialized mount mutations / no torn reads
**Requirement text**: Concurrent readers must not observe torn mount-state updates; mount mutations are serialized.

### REQ-UIO-LIFE-001 / REQ-UIO-LIFE-002: Repository close and unmount invalidation
**Requirement text**: repository close unmounts before free; unmounted mount handles become invalid.

## Implementation Tasks

### Files to modify

#### `rust/src/io/uio_bridge.rs`

- **Replace positional-sort assumptions with explicit placement metadata**
  - marker: `@plan PLAN-20260314-UIO.P06`
  - marker: `@requirement REQ-UIO-MOUNT-002`
  - Replace `sort_mount_registry` with insertion logic that preserves TOP/BOTTOM/ABOVE/BELOW placement relationships and stable recency order
  - Keep enough metadata to resolve ties using the provisional rule rather than assuming Vec order alone is sufficient

- **Update `register_mount` to use placement-aware insertion**
  - marker: `@plan PLAN-20260314-UIO.P06`
  - At `register_mount`: pass placement flags and relative handle through explicit insertion logic
  - Validate:
    - TOP/BOTTOM require null `relative`
    - ABOVE/BELOW require non-null active `relative` in same repository
    - Invalid requests fail without partially registering the mount

- **Update `uio_mountDir` to pass flags/relative to `register_mount`**
  - marker: `@plan PLAN-20260314-UIO.P06`
  - Extract `relative` mount identity and pass to mount registration
  - Set `errno = EINVAL` on invalid placement combinations

- **Update path resolution to encode the provisional ordering rule explicitly**
  - marker: `@plan PLAN-20260314-UIO.P06`
  - marker: `@requirement REQ-UIO-MOUNT-003`
  - Do not rely on “Vec order IS resolution order” alone
  - For a given path, rank matching active mounts by:
    1. explicit placement precedence relation established by TOP/BOTTOM/ABOVE/BELOW
    2. longer matching mount-point prefix
    3. recency / insertion order tie-breaker
  - Apply the same rule consistently in open/stat/access/listing/location lookups that resolve visible content

- **Serialize mount-registry mutations and keep reader views coherent**
  - marker: `@plan PLAN-20260314-UIO.P06`
  - marker: `@requirement REQ-UIO-CONC-002`
  - Ensure reads either operate under the registry lock or from a stable snapshot
  - Avoid exposing partially updated ordering state during mount insert/remove
  - Avoid holding the global mount lock across blocking filesystem I/O where not required for correctness

- **Invalidate mount handles on unmount and close repository via unmount-first path**
  - marker: `@plan PLAN-20260314-UIO.P06`
  - marker: `@requirement REQ-UIO-LIFE-001, REQ-UIO-LIFE-002`
  - `uio_unmountDir`: remove/deactivate mount, make the handle unusable as future `relative` anchor
  - `uio_closeRepository`: unmount all repository mounts before repository free/teardown

- **Apply `set_errno` to all non-stream API error paths**
  - marker: `@plan PLAN-20260314-UIO.P06`
  - marker: `@requirement REQ-UIO-ERR-002`
  - Functions to update:
    - `uio_open` → ENOENT (not found), EROFS (read-only mount), EINVAL (bad args)
    - `uio_close` → EINVAL (null handle)
    - `uio_read` → EINVAL (null), EIO (read failure)
    - `uio_write` → EINVAL (null), EIO (write failure), EROFS (read-only)
    - `uio_lseek` → EINVAL (null/bad whence), EIO (seek failure)
    - `uio_fstat` → EINVAL (null), ENOENT (can't stat)
    - `uio_stat` → ENOENT (not found)
    - `uio_unlink` → ENOENT (not found), EROFS (read-only)
    - `uio_rename` → ENOENT, EROFS
    - `uio_mkdir` → EEXIST, EROFS, ENOENT
    - `uio_rmdir` → ENOENT, ENOTEMPTY, EROFS
    - `uio_access` → ENOENT, EACCES
    - `uio_openDir` → ENOENT
    - `uio_openDirRelative` → ENOENT
    - `uio_getDirList` → ENOENT (target dir not found)
    - `uio_mountDir` → EINVAL (bad args)
    - `uio_getFileLocation` → ENOENT (not found)

### Tests to add

- **`test_mount_top_has_highest_precedence`**
  - Mount A at bottom, mount B at top
  - Both provide same file, assert B's version is resolved

- **`test_mount_bottom_has_lowest_precedence`**
  - Mount A at top, mount B at bottom
  - Both provide same file, assert A's version is resolved

- **`test_mount_above_relative`**
  - Mount A, mount B below A, mount C above B
  - Assert resolution order: A, C, B

- **`test_mount_below_relative`**
  - Mount A, mount B, mount C below A
  - Assert resolution order: A, C, B

- **`test_mount_specificity_tie_break_after_placement`**
  - Overlapping but non-identical mount points (for example `/content` and `/content/packages`)
  - Assert the path winner follows the documented provisional rule, not accidental Vec iteration

- **`test_mount_ordering_survives_unmount`**
  - Mount A (top), B (bottom), C (above B)
  - Unmount C
  - Assert A and B still resolve correctly

- **`test_unmounted_mount_handle_cannot_be_relative_anchor`**
  - Unmount a mount, then attempt ABOVE/BELOW using that old handle
  - Assert mount request fails with `EINVAL`

- **`test_concurrent_mount_readers_do_not_observe_torn_state`**
  - One thread mutates mount order / unmounts under synchronization
  - Readers resolve repeatedly from stable snapshots or locked state
  - Assert no panic, no invalid intermediate state, deterministic valid results only

- **`test_close_repository_unmounts_before_free`**
  - Create repository with mounts
  - Close repository
  - Assert repository-owned mounts are removed from active resolution

- **`test_errno_set_on_open_nonexistent`**
  - Call `uio_open` on nonexistent file
  - Assert `errno == ENOENT`

- **`test_errno_set_on_stat_nonexistent`**
  - Call `uio_stat` on nonexistent path
  - Assert `errno == ENOENT`

- **`test_errno_set_on_null_handle`**
  - Call `uio_close(null)`
  - Assert `errno == EINVAL`

### Pseudocode traceability
- Uses pseudocode Component 002 and Component 003

## Verification Commands

```bash
cd rust && cargo fmt --all --check
cd rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
cd rust && cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] placement-aware insertion logic exists
- [ ] `sort_mount_registry` removed or reduced to non-authoritative helper use only
- [ ] `register_mount` uses placement-aware insertion
- [ ] `uio_mountDir` passes flags and relative to mount registration
- [ ] resolution logic encodes the provisional rule explicitly
- [ ] mount mutations are serialized and reader views are coherent
- [ ] mount handles are invalidated after unmount
- [ ] repository close unmounts before free
- [ ] `set_errno` called in all listed functions' error paths
- [ ] 10+ new tests added

## Semantic Verification Checklist
- [ ] Mount ordering tests verify correct precedence behavior
- [ ] Overlapping-prefix tests verify the documented provisional rule
- [ ] errno tests verify correct error code values
- [ ] concurrency test verifies no torn-state observation during mount updates
- [ ] repository close / unmount lifecycle tests pass
- [ ] All existing tests still pass (no mount ordering regression)
- [ ] No placeholder/deferred implementation patterns

## Deferred Implementation Detection

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder" rust/src/io/uio_bridge.rs
```

## Success Criteria
- [ ] Mount placement flags correctly control resolution order
- [ ] Overlap cases follow the provisional rule from requirements
- [ ] errno set on all non-stream function error paths
- [ ] mount registry updates are serialized without torn reader state
- [ ] unmounted mount handles cannot be reused as anchors
- [ ] All tests pass
- [ ] Verification commands pass

## Failure Recovery
- rollback: `git stash` or `git checkout -- rust/src/io/uio_bridge.rs`
- blocking issues: relative mount handle lookup logic, placement-order data model, snapshot-vs-lock approach for concurrent readers

## Phase Completion Marker
Create: `project-plans/20260311/uio/.completed/P06.md`
