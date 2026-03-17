# Phase 09: Directory Enumeration Merge & Regex — Stub/TDD/Impl

## Phase ID
`PLAN-20260314-UIO.P09`

## Prerequisites
- Required: Phase 08a completed
- Archive support is functional — ZIP entries can be listed individually
- Mount ordering is correct
- `uio_DirList` ABI strategy from Phase 00a is resolved

## Requirements Implemented (Expanded)

### REQ-UIO-LIST-001: Directory listing enumerates visible entries
**Requirement text**: When a caller requests a directory listing, the subsystem shall enumerate entries visible at the requested virtual directory location.

### REQ-UIO-LIST-002: Merged listing across contributing mounts
**Requirement text**: When the requested virtual directory is contributed by multiple active mounts (including archive mounts), the subsystem shall merge visible entries.

Behavior contract:
- GIVEN: mount A (STDIO) at `/content` with files `a.txt`, `b.txt`
- AND: mount B (ZIP) at `/content` with files `b.txt`, `c.txt`
- AND: mount B has higher precedence than mount A
- WHEN: `uio_getDirList(dir, "/content", "", MATCH_LITERAL)` is called
- THEN: listing contains `a.txt`, `b.txt` (from B), `c.txt` — no duplicate `b.txt`

### REQ-UIO-LIST-003: Deduplication by name
**Requirement text**: When multiple mounts provide entries with the same name, the subsystem shall deduplicate.

Behavior contract:
- GIVEN: mounts A and B both contain `readme.txt` at same virtual directory
- WHEN: listing is requested
- THEN: exactly one `readme.txt` appears (from highest-precedence mount)

### REQ-UIO-LIST-009: Full regex matching
**Requirement text**: When regex matching is requested, the subsystem shall provide general POSIX ERE-compatible semantics.

Behavior contract:
- GIVEN: entries `foo.rmp`, `bar.rmp`, `baz.txt`
- WHEN: `uio_getDirList(dir, path, "\\.[rR][mM][pP]$", MATCH_REGEX)` is called
- THEN: listing contains `foo.rmp`, `bar.rmp` only

### REQ-UIO-LIST-016: Startup .rmp discovery
**Requirement text**: .rmp files enumerated from startup namespace after archive mounting.

### REQ-UIO-LIST-017: Provisional .rmp ordering
**Requirement text**: Deterministic ordering rule for .rmp enumeration.

Behavior contract:
- GIVEN: mount A with `alpha.rmp`, mount B with `beta.rmp`, A higher precedence
- WHEN: .rmp listing requested
- THEN: order is `alpha.rmp`, `beta.rmp` (mount precedence, lexical within mount)

### REQ-UIO-LIST-004 through LIST-008: Match modes
- LITERAL (0): exact match
- PREFIX (1): name starts with pattern
- SUFFIX (2): name ends with pattern
- SUBSTRING (3): name contains pattern
- REGEX (4): POSIX ERE regex

### REQ-UIO-LIST-010: Empty result is valid, not error
**Requirement text**: When no entries match, return empty listing, not null.

### REQ-UIO-LIST-012 / REQ-UIO-LIST-013: DirList ownership and pointer validity
### REQ-UIO-FFI-004 / REQ-UIO-INT-004: ABI-safe public struct layout

## Implementation Tasks

### Files to create

#### `rust/src/io/uio/dirlist.rs`
- marker: `@plan PLAN-20260314-UIO.P09`
- **Functions:**
  - `pub fn get_merged_dir_list(dir_path: &Path, pattern: &str, match_type: c_int) -> Result<Vec<String>, std::io::Error>`
    - Iterates all active mounts contributing to the requested directory using the Phase 06 ordering rule
    - For each STDIO mount: calls `fs::read_dir` on physical dir
    - For each ZIP mount: calls `archive::list_archive_directory`
    - Deduplicates by name (first-seen wins)
    - Filters by `matches_pattern`
    - Returns `Vec<String>` of matching names
    - Preserves the scoped provisional `.rmp` ordering rule without claiming that rule for all arbitrary merged listings

  - `pub fn matches_pattern(name: &str, pattern: &str, match_type: c_int) -> bool`
    - Empty pattern → true
    - LITERAL: `name == pattern`
    - PREFIX: `name.starts_with(pattern)`
    - SUFFIX: `name.ends_with(pattern)`
    - SUBSTRING: `name.contains(pattern)`
    - REGEX: compile with `regex::Regex`, match against name

  - `pub fn build_c_dir_list(names: Vec<String>) -> *mut uio_DirList`
    - Allocates a C-compatible `uio_DirList` public struct
    - Uses a private bookkeeping strategy outside the ABI-visible `uio_DirList` fields
    - Does not rely on any extra Rust-only field inside the public struct

  - `pub fn free_c_dir_list(dirlist: *mut uio_DirList)`
    - Frees all name strings, name array, and DirList storage
    - Null-safe
    - No dependency on side-channel registry

### Files to modify

#### `rust/Cargo.toml`
- Add `regex = "1"` to `[dependencies]`
- marker: `@plan PLAN-20260314-UIO.P09`

#### `rust/src/io/uio/mod.rs`
- Add `pub mod dirlist;`

#### `rust/src/io/uio_bridge.rs`

- **Replace `uio_getDirList` implementation**
  - marker: `@plan PLAN-20260314-UIO.P09`
  - marker: `@requirement REQ-UIO-LIST-001, REQ-UIO-LIST-002, REQ-UIO-LIST-003`
  - Remove hard-coded regex patterns
  - Remove single-dir `fs::read_dir`
  - Call `dirlist::get_merged_dir_list` instead
  - Call `dirlist::build_c_dir_list` to produce the C-compatible output
  - Set errno on failure

- **Replace `uio_DirList_free` implementation**
  - marker: `@plan PLAN-20260314-UIO.P09`
  - marker: `@requirement REQ-UIO-LIST-012`
  - Call `dirlist::free_c_dir_list`
  - Remove dependency on buffer-size side-channel registry
  - Ensure the final implementation depends only on the ABI-safe allocation strategy resolved in Phase 00a

- **Remove `matches_pattern` from `uio_bridge.rs`**
  - The old hard-coded version is replaced by the new general-purpose version

### Tests to add (in `rust/src/io/uio/dirlist.rs`)

- **`test_matches_pattern_literal_exact`**
  - Assert `matches_pattern("foo.txt", "foo.txt", MATCH_LITERAL)` → true
  - Assert `matches_pattern("foo.txt", "bar.txt", MATCH_LITERAL)` → false

- **`test_matches_pattern_prefix`**
  - Assert `matches_pattern("readme.md", "read", MATCH_PREFIX)` → true
  - Assert `matches_pattern("readme.md", "write", MATCH_PREFIX)` → false

- **`test_matches_pattern_suffix`**
  - Assert `matches_pattern("data.rmp", ".rmp", MATCH_SUFFIX)` → true
  - Assert `matches_pattern("data.txt", ".rmp", MATCH_SUFFIX)` → false

- **`test_matches_pattern_substring`**
  - Assert `matches_pattern("hello_world.txt", "world", MATCH_SUBSTRING)` → true

- **`test_matches_pattern_regex_rmp`**
  - Assert `matches_pattern("data.rmp", "\\.[rR][mM][pP]$", MATCH_REGEX)` → true
  - Assert `matches_pattern("data.RMP", "\\.[rR][mM][pP]$", MATCH_REGEX)` → true
  - Assert `matches_pattern("data.txt", "\\.[rR][mM][pP]$", MATCH_REGEX)` → false

- **`test_matches_pattern_regex_zip_uqm`**
  - Assert `matches_pattern("content.uqm", "\\.([zZ][iI][pP]|[uU][qQ][mM])$", MATCH_REGEX)` → true
  - Assert `matches_pattern("content.ZIP", "\\.([zZ][iI][pP]|[uU][qQ][mM])$", MATCH_REGEX)` → true
  - Assert `matches_pattern("content.txt", "\\.([zZ][iI][pP]|[uU][qQ][mM])$", MATCH_REGEX)` → false

- **`test_matches_pattern_empty_matches_all`**
  - Assert `matches_pattern("anything", "", MATCH_REGEX)` → true
  - Assert `matches_pattern("anything", "", MATCH_LITERAL)` → true

- **`test_merged_dir_list_single_stdio_mount`**
  - Create temp dir with known files
  - Mount as STDIO
  - Get merged listing
  - Assert all files present

- **`test_merged_dir_list_dedup_across_mounts`**
  - Create two temp dirs with overlapping files
  - Mount both (first higher precedence)
  - Get merged listing
  - Assert no duplicates, first-seen wins

- **`test_merged_dir_list_stdio_plus_zip`**
  - Create temp dir with `a.txt`
  - Create ZIP with `b.txt`, `a.txt`
  - Mount both
  - Assert listing contains `a.txt` (from higher-precedence), `b.txt`

- **`test_merged_dir_list_rmp_ordering`**
  - marker: `@requirement REQ-UIO-LIST-016, REQ-UIO-LIST-017`
  - Mount A (higher) with `beta.rmp`, `alpha.rmp`
  - Mount B (lower) with `gamma.rmp`, `alpha.rmp`
  - Get listing with `.rmp` regex filter
  - Assert order: `alpha.rmp`, `beta.rmp`, `gamma.rmp` (lexical within each mount, first-seen dedup)

- **`test_build_c_dir_list_produces_valid_public_layout`**
  - Build DirList from known names
  - Assert `numNames` correct
  - Assert each name string accessible and correct
  - Free with `free_c_dir_list`
  - Verify only the public `names` and `numNames` fields are exposed via the returned pointer

- **`test_free_c_dir_list_null_safe`**
  - Call `free_c_dir_list(null)` — should not crash

### Pseudocode traceability
- Uses pseudocode Component 005

## Verification Commands

```bash
cd rust && cargo fmt --all --check
cd rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
cd rust && cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] `rust/src/io/uio/dirlist.rs` exists with all listed functions
- [ ] `regex` crate added to `Cargo.toml`
- [ ] Hard-coded regex patterns removed from `uio_bridge.rs`
- [ ] `uio_getDirList` delegates to `dirlist::get_merged_dir_list`
- [ ] `uio_DirList_free` uses ABI-safe proper deallocation
- [ ] public `uio_DirList` layout matches the C header exactly
- [ ] 13+ tests added
- [ ] Plan/requirement markers present

## Semantic Verification Checklist
- [ ] All 5 match types work correctly (literal, prefix, suffix, substring, regex)
- [ ] Regex patterns from `options.c` (`.rmp` and `.zip`/`.uqm`) work correctly
- [ ] Empty pattern matches all entries
- [ ] Cross-mount merge produces deduplicated listing
- [ ] First-seen dedup respects mount precedence
- [ ] `.rmp` ordering follows the scoped provisional rule only for the acceptance case that requires it
- [ ] C-compatible `uio_DirList` struct is valid and freeable
- [ ] No memory leaks in DirList allocation/free cycle
- [ ] All existing tests pass

## Deferred Implementation Detection

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder" rust/src/io/uio/dirlist.rs rust/src/io/uio_bridge.rs
```

## Success Criteria
- [ ] General regex matching works for all startup patterns
- [ ] Cross-mount merge works for STDIO+ZIP combinations
- [ ] `uio_DirList` memory management is ABI-safe and clean (no side-channel, no extra public field)
- [ ] All tests pass
- [ ] Verification commands pass

## Failure Recovery
- rollback: `git stash`
- blocking issues: regex crate API changes, `uio_DirList` allocation mismatch

## Phase Completion Marker
Create: `project-plans/20260311/uio/.completed/P09.md`
