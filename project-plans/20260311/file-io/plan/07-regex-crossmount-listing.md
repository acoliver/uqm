# Phase 07: Regex Matching & Cross-Mount Directory Listing

## Phase ID
`PLAN-20260314-FILE-IO.P07`

## Prerequisites
- Required: Phase 06a completed
- Expected: Mount ordering is position-based and correct
- Carry-forward: regex-engine compatibility decision from P00a/P01 must already be documented before implementation starts

## Requirements Implemented (Expanded)

### REQ-FIO-DIRLIST-REGEX: Audited regex compatibility for listing behavior
**Requirement text**: When the public API advertises regex matching, the subsystem SHALL implement the externally visible regex behavior required by callers and SHALL NOT substitute simplified heuristics.

Behavior contract:
- GIVEN: `uio_getDirList` called with `match_MATCH_REGEX` and pattern `\.[rR][mM][pP]$`
- WHEN: Directory contains `"foo.rmp"`, `"bar.RMP"`, `"baz.txt"`
- THEN: Only `"foo.rmp"` and `"bar.RMP"` are returned

- GIVEN: Any regex pattern within the audited compatibility set
- WHEN: Used as regex match pattern
- THEN: Matching behavior follows the documented compatibility decision from P00a/P01

Important planning constraint:
- This phase must **not** claim exact POSIX ERE parity unless the selected engine and tests prove it.
- If the chosen implementation offers a narrower compatibility set than full POSIX ERE, the plan output for this phase must document that the supported set still covers all engine callers and externally required patterns.

### REQ-FIO-DIRLIST-UNION: Union directory listing across mounts
**Requirement text**: When multiple mounts contribute entries to a listed directory, the subsystem SHALL merge visibility according to mount precedence rules. When multiple contributing mounts expose the same entry name, the subsystem SHALL apply precedence rules deterministically and SHALL NOT return duplicate names.

Behavior contract:
- GIVEN: Mount A at `/content` with files `["a.txt", "shared.txt"]` and Mount B at `/content` with `["b.txt", "shared.txt"]`
- WHEN: `uio_getDirList(contentDir, "", "", MATCH_LITERAL)` is called
- THEN: Returns a deterministic union with `"shared.txt"` only once

### REQ-FIO-DIRLIST-EMPTY: Empty match returns non-null
**Requirement text**: For a successfully resolved directory with no matches, returns a non-null `uio_DirList` with `numNames == 0`. `NULL` is reserved for actual errors.

### REQ-FIO-MOUNT-AUTOMOUNT: Conditional AutoMount behavior
**Requirement text**: If P00a audit determines AutoMount is required, directory enumeration must apply AutoMount rules, mutate topology per the public contract, and continue listing even when an individual auto-mount attempt fails.

## Implementation Tasks

### Files to modify
- `rust/Cargo.toml`
  - Add the regex dependency or compatibility layer selected during P00a/P01
    - marker: `@plan PLAN-20260314-FILE-IO.P07`
    - marker: `@requirement REQ-FIO-DIRLIST-REGEX`
- `rust/src/io/uio_bridge.rs`
  - **`matches_pattern`**:
    - replace hardcoded `.rmp` and `.zip`/`.uqm` special cases
    - implement REGEX via the audited compatibility layer
    - invalid regex must return no matches, not crash
  - **`uio_getDirList`**:
    - enumerate all active mounts covering the target virtual path
    - preserve deterministic output ordering by keeping first-seen precedence order, not raw `HashSet` iteration order
    - deduplicate by visible name
    - return non-null empty list on successful empty match
    - preserve `uio_DirList` ABI-visible layout for the first two fields while updating allocation strategy
    - ensure returned allocation ownership is self-contained so `uio_DirList_free` remains valid without hidden side channels
    - if AutoMount branch is active, evaluate rules during enumeration and mount new entries at repository bottom while continuing on failures
  - **listing concurrency audit**:
    - confirm directory enumeration observes a coherent topology snapshot or equivalent lock discipline while mounts may change
    - confirm returned `uio_DirList` allocations remain valid until `uio_DirList_free`
  - **errno follow-through**:
    - extend errno mapping for regex compilation failures, listing-allocation failures, and any new branch-specific failures introduced here

### Concrete caller touchpoints
- `sc2/src/options.c`: `mountDirZips()` and `loadIndices()` must continue to discover packages and `.rmp` files through the revised regex/listing behavior

### Pseudocode traceability
- Uses pseudocode lines: PC-09 lines 01–10, PC-10 lines 01–15

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cd sc2 && make clean && make
```

## Structural Verification Checklist
- [ ] regex compatibility layer added to `Cargo.toml` or existing workspace dependency reused
- [ ] `matches_pattern` no longer contains hardcoded archive/index special cases
- [ ] REGEX path uses the audited compatibility layer, not an unsupported assumption
- [ ] `uio_getDirList` iterates over multiple mounts
- [ ] dedup preserves deterministic first-seen precedence order
- [ ] empty-match success returns a non-null `uio_DirList`
- [ ] `uio_DirList` first-two-field ABI contract is preserved explicitly in this phase
- [ ] returned list ownership remains self-contained until `uio_DirList_free`
- [ ] listing topology snapshot/locking rule is documented for shared state touched here
- [ ] errno mapping is extended for regex/list-allocation failure cases introduced here
- [ ] AutoMount behavior is implemented here if and only if P00a required it

## Semantic Verification Checklist (Mandatory)
- [ ] Test: `.rmp` regex matches correctly
- [ ] Test: `.zip`/`.uqm` regex matches correctly
- [ ] Test: additional regex patterns from actual callers are covered by the audited compatibility set
- [ ] Test: invalid regex returns no matches (not a crash)
- [ ] Test: cross-mount union listing returns all entries from both mounts
- [ ] Test: duplicate names across mounts appear only once
- [ ] Test: returned name order is deterministic and precedence-sensitive
- [ ] Test: empty directory returns non-null `uio_DirList` with `numNames == 0`
- [ ] Test: `uio_DirList_free` works on union listing results
- [ ] Verification note: directory-list result lifetime remains safe across concurrent topology changes until free
- [ ] If AutoMount required: auto-mounted entries appear, mount failures are logged/skipped, and new mounts are inserted at repository bottom
- [ ] Game startup `mountDirZips()`/`loadIndices()` still work (regex patterns match archives and index files)

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/io/uio_bridge.rs
```

## Success Criteria
- [ ] Regex compatibility tests pass for all audited caller patterns
- [ ] Cross-mount listing tests pass
- [ ] `uio_DirList` ABI/ownership checks pass
- [ ] AutoMount tests pass if branch is active
- [ ] Game startup discovers and mounts packages correctly
- [ ] Verification commands pass

## Failure Recovery
- Rollback: `git checkout -- rust/src/io/uio_bridge.rs rust/Cargo.toml`

## Phase Completion Marker
Create: `project-plans/20260311/file-io/.completed/P07.md` containing:
- regex compatibility decision actually implemented
- deterministic ordering verification summary
- `uio_DirList` ABI/ownership verification summary
- listing concurrency review note
- AutoMount branch result
