# Phase 03: .rmp Parser Fix — Stub

## Phase ID
`PLAN-20260224-RES-SWAP.P03`

## Prerequisites
- Required: Phase 02a (Pseudocode Verification) completed
- Expected files: `analysis/pseudocode/component-001.md`

## Requirements Implemented (Expanded)

### REQ-RES-018: TYPE:path Parsing
**Requirement text**: When `process_resource_desc(key, value)` is called, the system
shall parse the value by splitting on the first `:` character to extract the type
name and file path.

Behavior contract:
- GIVEN: A value string `"GFXRES:base/comm/arilou/arilou.ani"`
- WHEN: The value is parsed
- THEN: type = `"GFXRES"`, path = `"base/comm/arilou/arilou.ani"`

Why it matters:
- The existing Rust parser does NOT split on `:` — this is the root cause of the
  format mismatch that prevents loading real .rmp files.

### REQ-RES-R007: Case-Sensitive Keys
**Requirement text**: Resource keys stored in the HashMap shall preserve their
original case exactly as provided by callers (case-sensitive).

Behavior contract:
- GIVEN: A key `"comm.arilou.graphics"` in a .rmp file
- WHEN: The key is parsed and stored
- THEN: The key is stored as `"comm.arilou.graphics"` (not lowercased or uppercased)

Why it matters:
- The existing Rust code lowercases keys in `ResourceIndex` and uppercases in
  `PropertyFile`, causing lookup failures when C queries by original case.

### REQ-RES-006/007/008/009/010/011/012: Parser Correctness
**Requirement text**: Full property file parsing matching C `PropFile_from_string`.

## Implementation Tasks

### Files to modify
- `rust/src/resource/propfile.rs`
  - Add new `parse_propfile()` function signature that:
    - Accepts `&str` data, a callback `FnMut(&str, &str)`, and optional `&str` prefix
    - Preserves key case (no uppercasing)
    - Handles inline `#` comments
    - Handles bare-key-at-EOF and key-without-value warnings
    - Handles prefix prepending (limited to 255 chars)
  - Mark existing `PropertyFile::from_string` as `#[deprecated]`
  - Stub body: `todo!("Parse propfile — see component-001.md")`
  - marker: `@plan PLAN-20260224-RES-SWAP.P03`
  - marker: `@requirement REQ-RES-018, REQ-RES-R007, REQ-RES-006-012`

- `rust/src/resource/index.rs`
  - Add `resource_type: Option<String>` field to `ResourceEntry`
  - Add `parse_type_path()` helper that splits `value` on first `:`
  - Modify `ResourceIndex` to use case-sensitive keys (remove `.to_lowercase()`)
  - Stub `parse_type_path`: `todo!()`
  - marker: `@plan PLAN-20260224-RES-SWAP.P03`
  - marker: `@requirement REQ-RES-018`

### Files to create
- None (modifying existing files)

### Pseudocode traceability
- Uses pseudocode lines: component-001.md lines 1-66

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] `propfile.rs` has new `parse_propfile()` function (stub with `todo!()`)
- [ ] `index.rs` has `resource_type` field on `ResourceEntry`
- [ ] `index.rs` has `parse_type_path()` helper (stub)
- [ ] Key case changed to case-sensitive in `ResourceIndex`
- [ ] Existing `PropertyFile::from_string` marked deprecated
- [ ] Plan/requirement traceability markers present

## Semantic Verification Checklist
- [ ] Compilation succeeds (stubs are allowed in stub phase)
- [ ] Existing tests may need updating for case-sensitivity change
- [ ] No duplicate module/function names

## Deferred Implementation Detection

```bash
# Stubs are ALLOWED in this phase — todo!() is expected
# But verify they are in the RIGHT places
grep -n "todo!()" rust/src/resource/propfile.rs rust/src/resource/index.rs
```

## Success Criteria
- [ ] New function signatures compile
- [ ] Case-sensitivity change applied
- [ ] Existing tests updated for case change (or documented as needing update)

## Failure Recovery
- rollback: `git checkout -- rust/src/resource/propfile.rs rust/src/resource/index.rs`

## Phase Completion Marker
Create: `project-plans/memandres/resource/.completed/P03.md`
