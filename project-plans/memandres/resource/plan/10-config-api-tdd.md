# Phase 10: Config API — TDD

## Phase ID
`PLAN-20260224-RES-SWAP.P10`

## Prerequisites
- Required: Phase 09a (Config API Stub Verification) completed

## Requirements Implemented (Expanded)

### REQ-RES-047-059: Config Get/Put — Tests
### REQ-RES-060-065: SaveResourceIndex — Tests

## Implementation Tasks

### Tests to write

#### Put + Get Roundtrip tests
```
test_put_get_string_roundtrip
  Put "test.key" = "hello world"
  Get "test.key" → "hello world"

test_put_get_integer_roundtrip
  Put "test.num" = 42
  Get "test.num" → 42

test_put_get_boolean_true_roundtrip
  Put "test.flag" = true
  Get "test.flag" → true

test_put_get_boolean_false_roundtrip
  Put "test.flag" = false
  Get "test.flag" → false

test_put_get_color_roundtrip
  Put "test.color" = Color { r: 0x1a, g: 0x00, b: 0x1a, a: 0xff }
  Get "test.color" → Color { r: 0x1a, g: 0x00, b: 0x1a, a: 0xff }

test_put_get_color_with_alpha
  Put "test.color" = Color { r: 0xff, g: 0x00, b: 0x00, a: 0x80 }
  Get "test.color" → Color { r: 0xff, g: 0x00, b: 0x00, a: 0x80 }
```

#### Auto-creation tests
```
test_put_string_auto_creates
  Get "nonexistent" → "" (default)
  Put "nonexistent" = "value"
  Get "nonexistent" → "value"
  IsString "nonexistent" → true

test_put_integer_auto_creates
  Get "nonexistent" → 0 (default)
  Put "nonexistent" = 99
  Get "nonexistent" → 99
  IsInteger "nonexistent" → true

test_put_boolean_auto_creates
  Get "nonexistent" → false (default)
  Put "nonexistent" = true
  Get "nonexistent" → true
  IsBoolean "nonexistent" → true

test_put_color_auto_creates
  Get "nonexistent" → Color(0,0,0,0) (default)
  Put "nonexistent" = Color(255,0,0,255)
  Get "nonexistent" → Color(255,0,0,255)
  IsColor "nonexistent" → true
```

#### Type checking tests
```
test_get_string_wrong_type_returns_default
  Put as INT32, then GetString → ""

test_get_integer_wrong_type_returns_default
  Put as STRING, then GetInteger → 0

test_is_type_correct
  Put as STRING, IsString → true, IsInteger → false

test_has_key
  HasKey "nonexistent" → false
  Put "somekey" = "value"
  HasKey "somekey" → true
```

#### SaveResourceIndex tests
```
test_save_basic
  Put config.sfxvol = 20 (INT32)
  Put config.fullscreen = true (BOOLEAN)
  Put config.scaler = "no" (STRING)
  SaveResourceIndex with root="config.", strip_root=true
  Read output file → contains "sfxvol = INT32:20\n" etc.
  Note: Entry order may vary (HashMap)

test_save_preserves_type_prefix
  Save with strip_root=false
  Output contains "config.sfxvol = INT32:20"

test_save_filters_by_root
  Put config.a = 1, keys.b = 2
  SaveResourceIndex with root="config."
  Output contains "a" but NOT "keys.b"

test_save_load_roundtrip
  Put values, save, reload, verify values match

test_save_color_format
  Put color = Color(0x1a, 0x00, 0x1a, 0xff)
  Save → output contains "COLOR:rgb(0x1a, 0x00, 0x1a)"

test_save_color_with_alpha_format
  Put color = Color(0xff, 0x00, 0x00, 0x80)
  Save → output contains "COLOR:rgba(0xff, 0x00, 0x00, 0x80)"

test_save_skips_heap_types
  Create entries with heap-type vtable (toString=None)
  Save → those entries are NOT in output
```

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features 2>&1 | grep "FAILED"
```

## Structural Verification Checklist
- [ ] All listed tests exist and compile
- [ ] Tests cover Put+Get roundtrip for all 4 types
- [ ] Tests cover auto-creation
- [ ] Tests cover type checking
- [ ] Tests cover SaveResourceIndex with root filtering and strip_root

## Semantic Verification Checklist
- [ ] Tests fail with stub implementation (RED)
- [ ] Roundtrip tests verify behavioral correctness, not internals
- [ ] SaveResourceIndex tests verify file output format

## Success Criteria
- [ ] All tests compile and FAIL (RED)

## Failure Recovery
- rollback: `git checkout -- rust/src/resource/`

## Phase Completion Marker
Create: `project-plans/memandres/resource/.completed/P10.md`
