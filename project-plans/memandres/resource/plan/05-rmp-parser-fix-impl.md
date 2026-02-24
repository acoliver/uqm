# Phase 05: .rmp Parser Fix — Implementation

## Phase ID
`PLAN-20260224-RES-SWAP.P05`

## Prerequisites
- Required: Phase 04a (TDD Verification) completed
- Expected: All parser tests exist and fail (RED)

## Requirements Implemented (Expanded)

### REQ-RES-018: TYPE:path Parsing
Behavior contract:
- GIVEN: `"GFXRES:base/comm/arilou/arilou.ani"`
- WHEN: parsed
- THEN: type=`"GFXRES"`, path=`"base/comm/arilou/arilou.ani"`

### REQ-RES-R007: Case-Sensitive Keys
- Keys stored with original case, no transformation

### REQ-RES-006-012: Full Parser Correctness
- Comments, blank lines, inline comments, whitespace trimming,
  bare-key-at-EOF, key-without-value, prefix mechanism

## Implementation Tasks

### Files to modify
- `rust/src/resource/propfile.rs`
  - Implement `parse_propfile()` following component-001.md lines 1-66
  - Character-by-character parsing matching C `PropFile_from_string`
  - Replace `todo!()` with real implementation
  - marker: `@plan PLAN-20260224-RES-SWAP.P05`
  - marker: `@requirement REQ-RES-006-012, REQ-RES-R007`

- `rust/src/resource/index.rs`
  - Implement `parse_type_path()` — split value on first `:`
  - Returns `(Option<&str>, &str)` — type name and path/value
  - If no `:`, returns `(None, entire_value)`
  - Remove case transformation from key storage
  - marker: `@plan PLAN-20260224-RES-SWAP.P05`
  - marker: `@requirement REQ-RES-018, REQ-RES-019`

### Pseudocode traceability
- `parse_propfile`: component-001.md lines 1-66
- `parse_type_path`: component-002.md lines 4-10

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] `parse_propfile()` fully implemented (no `todo!()`)
- [ ] `parse_type_path()` fully implemented (no `todo!()`)
- [ ] Key case preservation confirmed (no `.to_lowercase()` or `.to_uppercase()`)
- [ ] Plan/requirement markers present

## Semantic Verification Checklist
- [ ] All P04 tests pass (GREEN)
- [ ] Parser handles multi-colon values correctly (3DOVID, CONVERSATION)
- [ ] Parser preserves key case exactly
- [ ] Parser handles inline `#` comments
- [ ] Parser handles prefix mechanism with 255-char limit
- [ ] Parser handles bare-key-at-EOF and key-without-value edge cases
- [ ] No placeholder/deferred implementation patterns remain

## Deferred Implementation Detection

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/resource/propfile.rs rust/src/resource/index.rs
# Expected: 0 matches (none allowed in implementation phase)
```

## Success Criteria
- [ ] All P04 tests pass
- [ ] No deferred implementation markers
- [ ] Lint/format/test gates all pass

## Failure Recovery
- rollback: `git checkout -- rust/src/resource/propfile.rs rust/src/resource/index.rs`

## Phase Completion Marker
Create: `project-plans/memandres/resource/.completed/P05.md`
