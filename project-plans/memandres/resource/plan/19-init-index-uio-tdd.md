# Phase 19: Init, Index, and UIO Wrappers — TDD

## Phase ID
`PLAN-20260224-RES-SWAP.P19`

## Prerequisites
- Required: Phase 18a (Init/Index/UIO Stub Verification) completed

## Requirements Implemented (Expanded)

### REQ-RES-002-003: InitResourceSystem — Tests
### REQ-RES-088: UninitResourceSystem — Tests
### REQ-RES-005: LoadResourceIndex — Tests
### REQ-RES-082: Auto-init on API Call — Tests
### REQ-RES-089: Multiple LoadResourceIndex Calls — Tests

## Implementation Tasks

### Tests to write

Note: These tests operate on the internal Rust APIs, not via FFI
(which requires linking with C). FFI integration tests are in P21/P22.

#### Init/Uninit tests
```
test_init_creates_index_with_5_value_types
  InitResourceSystem()
  CountResourceTypes() → 5
  Verify: UNKNOWNRES, STRING, INT32, BOOLEAN, COLOR registered
  (The remaining 9 heap types are registered by C subsystem code
   via InstallResTypeVectors, tested in P12-P14)

test_init_idempotent
  ptr1 = InitResourceSystem()
  ptr2 = InitResourceSystem()
  ptr1 == ptr2

test_uninit_clears_state
  InitResourceSystem()
  UninitResourceSystem()
  State is None
  InitResourceSystem() → creates fresh

test_uninit_safe_to_call_twice
  UninitResourceSystem()
  UninitResourceSystem() → no crash
```

#### LoadResourceIndex tests (using internal API, mock file content)
```
test_load_index_parses_entries
  Mock file content: "comm.arilou.graphics = GFXRES:base/comm/arilou/arilou.ani"
  LoadResourceIndex (internal, from string)
  Lookup "comm.arilou.graphics" → exists, type="GFXRES"

test_load_index_with_prefix
  Content: "sfxvol = INT32:20"
  Load with prefix "config."
  Lookup "config.sfxvol" → exists, value=20

test_load_index_multiple_calls_accumulate
  Load file A: "key.a = STRING:alpha"
  Load file B: "key.b = STRING:beta"
  Both "key.a" and "key.b" exist

test_load_index_last_writer_wins
  Load file A: "music.battle = MUSICRES:base/battle.mod"
  Load file B: "music.battle = MUSICRES:addons/3domusic/battle.ogg"
  Lookup "music.battle" → path="addons/3domusic/battle.ogg"

test_load_index_value_types_parsed_immediately
  Content: "config.sfxvol = INT32:20"
  Load with prefix
  get_integer("config.sfxvol") → 20 (already parsed, no lazy load)

test_load_index_heap_types_deferred
  Content: "comm.arilou.graphics = GFXRES:base/comm/arilou/arilou.ani"
  Load
  Descriptor ptr is NULL (not loaded yet)

test_load_index_boolean_true
  Content: "fullscreen = BOOLEAN:true"
  Load with prefix "config."
  get_boolean("config.fullscreen") → true

test_load_index_boolean_false
  Content: "fullscreen = BOOLEAN:false"
  Load with prefix "config."
  get_boolean("config.fullscreen") → false

test_load_index_color
  Content: "color = COLOR:rgb(0x1a, 0x00, 0x1a)"
  Load with prefix "config."
  get_color("config.color") → Color { r: 0x1a, g: 0x00, b: 0x1a, a: 0xff }

test_load_index_string
  Content: "up.1 = STRING:key Up"
  Load with prefix "keys."
  get_string("keys.up.1") → "key Up"
```

#### Auto-init tests
```
test_auto_init_on_get_string
  No init call
  get_string("nonexistent") → "" (auto-inits, then returns default)

test_auto_init_on_has_key
  No init call
  has_key("nonexistent") → false (auto-inits)
```

#### File I/O wrapper tests (limited, since UIO is C)
```
test_length_res_file_sentinel
  LengthResFile with sentinel (!0 pointer) → 1

test_close_res_file_sentinel
  res_CloseResFile with sentinel → TRUE (no-op)

test_close_res_file_null
  res_CloseResFile(NULL) → TRUE
```

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features 2>&1 | grep "FAILED"
```

## Structural Verification Checklist
- [ ] Init/uninit tests exist
- [ ] LoadResourceIndex tests exist
- [ ] Auto-init tests exist
- [ ] Sentinel tests exist
- [ ] Tests compile

## Semantic Verification Checklist
- [ ] Tests fail with stubs (RED)
- [ ] Init idempotency tested
- [ ] Prefix mechanism tested end-to-end
- [ ] Last-writer-wins tested
- [ ] Value type immediate parsing tested
- [ ] Heap type deferral tested

## Success Criteria
- [ ] All tests compile and FAIL (RED)

## Failure Recovery
- rollback: `git checkout -- rust/src/resource/`

## Phase Completion Marker
Create: `project-plans/memandres/resource/.completed/P19.md`
