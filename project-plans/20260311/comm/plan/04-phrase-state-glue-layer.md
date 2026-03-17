# Phase 04: Phrase State & Glue Layer

## Phase ID
`PLAN-20260314-COMM.P04`

## Prerequisites
- Required: Phase 03a completed
- Expected files: `rust/src/comm/locdata.rs`, expanded `CommData` in `types.rs`
- Required gate: P00.5 phrase-disable audit completed and recorded before relying on narrowed semantics

## Requirements Implemented (Expanded)

### PS-REQ-001–007: Phrase enable/disable lifecycle
**Requirement text**: Phrase state shall be encounter-local, initialized to enabled on encounter start, disabled via DISABLE_PHRASE, queryable via PHRASE_ENABLED, not re-enableable within an encounter, and reset on encounter teardown.

Behavior contract:
- GIVEN: A fresh encounter
- WHEN: `DISABLE_PHRASE(5)` is called, then `PHRASE_ENABLED(5)` is queried
- THEN: Returns false. `PHRASE_ENABLED(3)` for a non-disabled phrase returns true.

### DS-REQ-005–010: Phrase emission and glue
**Requirement text**: NPCPhrase_cb resolves phrase indices, handles special indices, and feeds the trackplayer-facing queueing layer. NPCPhrase_splice preserves continuous flow. NPCNumber synthesizes spoken numbers. construct_response builds composite strings.

**Phase boundary note**: P04 establishes script-facing API shape, phrase resolution, phrase-state semantics, response composition, and segue state. The full trackplayer-owned callback/history/commit semantics are completed in P06; P04 should not claim those requirements complete until P06 wiring exists.

### DS-REQ-011: Segue state
**Requirement text**: setSegue/getSegue expose segue behavior consistent with encounter flow.

### SB-REQ-001–006: Segue outcomes
**Requirement text**: Peace, Hostile, Victory, Defeat segues produce correct BATTLE_SEGUE and side effects.

## Implementation Tasks

### Files to create

- `rust/src/comm/phrase_state.rs` — Phrase enable/disable tracking
  - marker: `@plan PLAN-20260314-COMM.P04`
  - marker: `@requirement PS-REQ-001, PS-REQ-002, PS-REQ-003`
  - `PhraseState` struct with `HashSet<i32>` for disabled indices
  - `is_enabled(index: i32) -> bool`
  - `disable(index: i32)`
  - `reset()` — clears all disable state
  - Unit tests for all PS-REQ requirements
  - If the P00.5 audit finds a violating script path, include the compatibility hook required to preserve legacy NUL-mutation semantics for that path only

- `rust/src/comm/glue.rs` — Script-facing glue functions
  - marker: `@plan PLAN-20260314-COMM.P04`
  - marker: `@requirement DS-REQ-005, DS-REQ-006, DS-REQ-007, DS-REQ-008, DS-REQ-009, DS-REQ-010`
  - `npc_phrase_cb(index: i32, callback: Option<PhraseCallback>)` — resolve phrase and forward to the trackplayer-facing queueing API
  - `npc_phrase_splice(index: i32)` — request page-break-free append through the trackplayer-facing API
  - `npc_number(number: i32, fmt: *const c_char)` — synthesize spoken number
  - `construct_response(buf: &mut String, fragments: &[i32])` — build composite response text
  - Phrase resolution: handles GLOBAL_PLAYER_NAME, GLOBAL_SHIP_NAME, negative indices, index 0 no-op
  - Unit tests for each function, including special indices

- `rust/src/comm/segue.rs` — Segue state management
  - marker: `@plan PLAN-20260314-COMM.P04`
  - marker: `@requirement DS-REQ-011, SB-REQ-001, SB-REQ-002, SB-REQ-003, SB-REQ-004`
  - `Segue` enum: `Peace`, `Hostile`, `Victory`, `Defeat`
  - `set_segue(segue: Segue)` — applies side effects
  - `get_segue() -> Segue` — reads current state
  - FFI bridge to C `BATTLE_SEGUE` global
  - Unit tests for all segue transitions

### Files to modify

- `rust/src/comm/state.rs`
  - Add `phrase_state: PhraseState` field to `CommState`
  - Add `segue: Segue` field
  - Extend `clear()` to reset phrase state and segue
  - Add accessor methods: `phrase_enabled()`, `disable_phrase()`, `set_segue()`, `get_segue()`
  - marker: `@plan PLAN-20260314-COMM.P04`

- `rust/src/comm/ffi.rs`
  - Add FFI exports:
    - `rust_PhraseEnabled(index: c_int) -> c_int`
    - `rust_DisablePhrase(index: c_int)`
    - `rust_NPCPhrase_cb(index: c_int, callback: Option<extern "C" fn()>)`
    - `rust_NPCPhrase_splice(index: c_int)`
    - `rust_NPCNumber(number: c_int, fmt: *const c_char)`
    - `rust_ConstructResponse(buf: *mut c_char, buf_len: c_uint, response_ref: c_int, fragments: *const c_int, fragment_count: c_uint) -> c_int`
    - `rust_SetSegue(segue: c_uint)`
    - `rust_GetSegue() -> c_uint`
  - marker: `@plan PLAN-20260314-COMM.P04`

- `rust/src/comm/mod.rs`
  - Add: `pub mod phrase_state;`, `pub mod glue;`, `pub mod segue;`

- `rust/src/comm/response.rs`
  - Add method `do_response_phrase_from_table()` — resolves response text from conversation phrases resource when no explicit text provided (for RS-REQ-002)

### Pseudocode traceability
- Uses pseudocode lines: 40-50 (phrase state), 55-74 (NPCPhrase_cb), 80-86 (NPCPhrase_splice), 90-100 (NPCNumber), 105-112 (construct_response), 115-131 (segue)

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] `phrase_state.rs` created with PhraseState struct
- [ ] `glue.rs` created with all 4 glue functions
- [ ] `segue.rs` created with Segue enum
- [ ] CommState extended with phrase_state and segue
- [ ] FFI exports added for all new functions
- [ ] P00.5 phrase-disable audit artifact referenced from this phase
- [ ] Plan/requirement traceability present

## Semantic Verification Checklist (Mandatory)
- [ ] Phrase disable is encounter-local: disable in one encounter, verify clean state in next
- [ ] PHRASE_ENABLED returns true for non-disabled phrases
- [ ] PHRASE_ENABLED returns false for disabled phrases
- [ ] NPCPhrase on a disabled phrase still resolves original text (PS-REQ-004)
- [ ] NPCPhrase(0) is a no-op (DS-REQ-007)
- [ ] NPCPhrase with GLOBAL_PLAYER_NAME returns commander name (DS-REQ-006)
- [ ] NPCPhrase with negative index returns alliance name (DS-REQ-006)
- [ ] NPCPhrase_splice requests no page break / current-phrase append semantics (full completion in P06)
- [ ] construct_response concatenates fragments in order (DS-REQ-010)
- [ ] setSegue(Peace) sets BATTLE_SEGUE=0
- [ ] setSegue(Hostile) sets BATTLE_SEGUE=1
- [ ] setSegue(Victory) sets BATTLE_SEGUE=1 and instantVictory
- [ ] setSegue(Defeat) sets crew sentinel and triggers restart check
- [ ] If the P00.5 audit found any violating script path, tests cover the selected compatibility preservation path

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/comm/phrase_state.rs rust/src/comm/glue.rs rust/src/comm/segue.rs
```

## Success Criteria
- [ ] All PS-REQ, DS-REQ-005–007, DS-REQ-010–012, SB-REQ behaviors demonstrated in tests
- [ ] DS-REQ-008/009 API shape and queuing hooks are in place, with full track semantics completed in P06
- [ ] Verification commands pass

## Failure Recovery
- rollback: `git restore rust/src/comm/`
- blocking: Trackplayer integration semantics are completed in P06; P04 must not claim those semantics prematurely

## Phase Completion Marker
Create: `project-plans/20260311/comm/.completed/P04.md`
