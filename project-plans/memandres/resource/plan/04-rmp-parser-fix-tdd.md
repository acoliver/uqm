# Phase 04: .rmp Parser Fix — TDD

## Phase ID
`PLAN-20260224-RES-SWAP.P04`

## Prerequisites
- Required: Phase 03a (Stub Verification) completed
- Expected: `parse_propfile()` stub, `parse_type_path()` stub

## Requirements Implemented (Expanded)

### REQ-RES-018: TYPE:path Parsing — Tests
Behavior contract:
- GIVEN: `"GFXRES:base/comm/arilou/arilou.ani"`
- WHEN: `parse_type_path()` is called
- THEN: Returns `(Some("GFXRES"), "base/comm/arilou/arilou.ani")`

### REQ-RES-019: No Colon — Tests
- GIVEN: `"Some string value"` (no colon)
- WHEN: `parse_type_path()` is called
- THEN: Returns `(None, "Some string value")`

### REQ-RES-006-012: Parser Correctness — Tests

## Implementation Tasks

### Files to modify
- `rust/src/resource/propfile.rs` (test section) or `rust/src/resource/tests.rs`
  - marker: `@plan PLAN-20260224-RES-SWAP.P04`

### Tests to write

#### TYPE:path parsing tests
```
test_parse_type_path_gfxres
  Input: "GFXRES:base/comm/arilou/arilou.ani"
  Expected: type="GFXRES", path="base/comm/arilou/arilou.ani"

test_parse_type_path_string
  Input: "STRING:key Up"
  Expected: type="STRING", path="key Up"

test_parse_type_path_boolean
  Input: "BOOLEAN:true"
  Expected: type="BOOLEAN", path="true"

test_parse_type_path_int32
  Input: "INT32:20"
  Expected: type="INT32", path="20"

test_parse_type_path_color
  Input: "COLOR:rgb(0x1a, 0x00, 0x1a)"
  Expected: type="COLOR", path="rgb(0x1a, 0x00, 0x1a)"

test_parse_type_path_3dovid_multi_colon
  Input: "3DOVID:addons/3dovideo/ships/ship00.duk:addons/3dovideo/ships/spin.aif:89"
  Expected: type="3DOVID", path="addons/3dovideo/ships/ship00.duk:addons/3dovideo/ships/spin.aif:89"
  Rationale: Only first colon separates type from path

test_parse_type_path_conversation_multi_colon
  Input: "CONVERSATION:addons/3dovoice/arilou/arilou.txt:addons/3dovoice/arilou/:addons/3dovoice/arilou/arilou.ts"
  Expected: type="CONVERSATION", path contains all three colon-separated paths

test_parse_type_path_no_colon
  Input: "Some string value"
  Expected: type=None, path="Some string value"

test_parse_type_path_empty_path
  Input: "STRING:"
  Expected: type="STRING", path=""
```

#### Property file parser tests
```
test_propfile_basic_keyvalue
  Input: "key = GFXRES:path/to/file"
  Expected: handler called with ("key", "GFXRES:path/to/file")

test_propfile_preserves_key_case
  Input: "comm.Arilou.Graphics = GFXRES:path"
  Expected: key is "comm.Arilou.Graphics" (NOT lowercased)

test_propfile_comment_line
  Input: "# this is a comment\nkey = value"
  Expected: handler called once with ("key", "value")

test_propfile_inline_comment
  Input: "key = value # inline comment"
  Expected: handler called with ("key", "value")

test_propfile_whitespace_trimming
  Input: "  key  =  value  "
  Expected: handler called with ("key", "value")

test_propfile_blank_lines
  Input: "\n\n\nkey = value\n\n"
  Expected: handler called once with ("key", "value")

test_propfile_key_without_value
  Input: "barekey\nkey = value"
  Expected: Warning logged, handler called once with ("key", "value")

test_propfile_bare_key_at_eof
  Input: "barekey"
  Expected: Warning logged, handler never called

test_propfile_prefix_prepended
  Input: "sfxvol = INT32:20" with prefix "config."
  Expected: handler called with ("config.sfxvol", "INT32:20")

test_propfile_null_prefix
  Input: "key = value" with prefix None
  Expected: handler called with ("key", "value")

test_propfile_prefix_length_limit
  Prefix is 250 chars, key is 10 chars → total > 255
  Expected: key truncated to 255 chars

test_propfile_multiple_entries
  Input: multiline .rmp-style content (5+ entries)
  Expected: handler called for each entry with correct key/value

test_propfile_real_rmp_content
  Input: First 10 lines of actual uqm.rmp content
  Expected: All entries parsed correctly with TYPE:path preserved
```

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features -- --test-threads=1 2>&1 | grep -E "test.*FAILED|test result"
```

## Structural Verification Checklist
- [ ] All listed tests exist
- [ ] Tests are in the correct module
- [ ] Tests compile (even if they fail — RED phase)
- [ ] Plan/requirement markers present

## Semantic Verification Checklist
- [ ] Tests cover TYPE:path splitting (including multi-colon)
- [ ] Tests cover case sensitivity
- [ ] Tests cover inline comments
- [ ] Tests cover prefix mechanism
- [ ] Tests cover edge cases (bare key, no colon, empty value)
- [ ] Tests fail with current stub implementation (RED confirmation)

## Success Criteria
- [ ] All tests compile
- [ ] All tests FAIL (RED) — proving they test real behavior, not stubs

## Failure Recovery
- rollback: `git checkout -- rust/src/resource/`

## Phase Completion Marker
Create: `project-plans/memandres/resource/.completed/P04.md`
