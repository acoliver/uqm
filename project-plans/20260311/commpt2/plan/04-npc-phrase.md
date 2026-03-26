# Phase 04: NPC Phrase Implementation

## Phase ID
`PLAN-20260326-COMMPT2.P04`

## Prerequisites
- Required: Phase 03a (Input Bridge Verification) completed
- `c_get_conversation_phrase` exists in rust_comm.c (line 283)
- `c_SpliceTrack` exists in rust_comm.c (line 320)
- `rust_NPCPhrase_cb` and `rust_NPCPhrase_splice` stubs exist in ffi.rs (lines 882, 890)
- All existing comm tests pass

## Requirements Implemented (Expanded)

### REQ-NP-001: rust_NPCPhrase_cb resolves and splices with all C branches
**Requirement text**: `rust_NPCPhrase_cb(index, callback)` SHALL implement all branches from C `NPCPhrase_cb` (commglue.c:36–97): index==0 early return, GLOBAL_PLAYER_NAME, GLOBAL_SHIP_NAME, negative-index alliance-name variants, and normal positive-index phrase lookup with audio clip and timestamp.

Behavior contract:
- GIVEN: An active encounter with loaded ConversationPhrases
- WHEN: `rust_NPCPhrase_cb(phrase_index, Some(callback))` is called from a race script
- THEN: The implementation handles ALL of these branches exactly as C does:
  1. **index == 0**: Early return (no SpliceTrack call)
  2. **GLOBAL_PLAYER_NAME**: Use `GLOBAL_SIS(CommanderName)` as text, pClip=NULL, pTimeStamp=NULL
  3. **GLOBAL_SHIP_NAME**: Use `GLOBAL_SIS(ShipName)` as text, pClip=NULL, pTimeStamp=NULL
  4. **index < 0 (negative)**: Alliance name variant — offset by GLOBAL_ALLIANCE_NAME, index into string table at `(index-1) + GET_GAME_STATE(NEW_ALLIANCE_NAME)`, if alliance name state==3 append CommanderName. pClip=NULL, pTimeStamp=NULL
  5. **index > 0 (normal)**: Look up `SetAbsStringTableIndex(ConversationPhrases, index-1)`, extract text via `GetStringAddress`, audio clip via `GetStringSoundClip`, and timestamp via `GetStringTimeStamp`
  6. **All non-zero paths**: Call `SpliceTrack(pClip, pStr, pTimeStamp, cb)`

Why it matters:
- This is the primary mechanism for NPC speech — without it, aliens are silent. Missing any branch causes specific dialogue lines (player name, ship name, alliance references) to be silent or crash.

### REQ-NP-002: rust_NPCPhrase_splice works without callback
**Requirement text**: `rust_NPCPhrase_splice(index)` SHALL resolve and splice a phrase without a completion callback.

Behavior contract:
- GIVEN: An active encounter with loaded ConversationPhrases
- WHEN: `rust_NPCPhrase_splice(phrase_index)` is called
- THEN: The phrase is resolved and spliced identically to `rust_NPCPhrase_cb` but with `cb = NULL`

Why it matters:
- Many race scripts use the simple `NPCPhrase(index)` macro which omits the callback

### REQ-NP-003: Phrase resolution uses c_get_conversation_phrase
**Requirement text**: Phrase resolution SHALL use `c_get_conversation_phrase(phrases, index)` to obtain text from the C string table.

Behavior contract:
- GIVEN: ConversationPhrases handle is valid and index > 0
- WHEN: Phrase resolution is attempted
- THEN: `c_get_conversation_phrase(phrases_handle, index)` is called, returning a pointer to the phrase text

Why it matters:
- The C string table is the authoritative source of phrase text; Rust must not duplicate it

### REQ-NP-004: NPCPhrase updates conversation summary
**Requirement text**: NPCPhrase emission SHALL update the conversation summary/history with each emitted phrase.

Behavior contract:
- GIVEN: A phrase has been successfully resolved
- WHEN: The phrase is spliced for playback
- THEN: The phrase text is appended to the conversation summary history

Why it matters:
- The conversation summary (accessible via cancel key) must contain all NPC dialogue

## Implementation Tasks

### Files to modify

#### `rust/src/comm/ffi.rs`

- **Implement `rust_NPCPhrase_cb`** (replacing stub at line 882–886)
  - Match C `NPCPhrase_cb` (commglue.c:36–97) branch-for-branch:
  - **Branch 1 — index == 0**: Early return, do nothing
  - **Branch 2 — GLOBAL_PLAYER_NAME**: Get commander name via `c_GetCommanderName()`, set pClip=NULL, pTimeStamp=NULL
  - **Branch 3 — GLOBAL_SHIP_NAME**: Get ship name via `c_GetShipName()`, set pClip=NULL, pTimeStamp=NULL
  - **Branch 4 — index < 0 (negative)**: Alliance name variant:
    - Compute offset: `index -= GLOBAL_ALLIANCE_NAME`
    - Get alliance name state: `i = c_GetGameState_NewAllianceName()`
    - Look up string at `SetAbsStringTableIndex(ConversationPhrases, (index-1) + i)`
    - If `i == 3`, append CommanderName to the string
    - Set pClip=NULL, pTimeStamp=NULL
  - **Branch 5 — index > 0 (normal)**: Look up `SetAbsStringTableIndex(ConversationPhrases, index-1)`:
    - Get text via `GetStringAddress`
    - Get audio clip via `GetStringSoundClip`
    - Get timestamp via `GetStringTimeStamp`
  - **All non-zero paths**: Call `c_SpliceTrack(pClip, pStr, pTimeStamp, cb)`
  - Update conversation summary with the phrase text
  - marker: `@plan PLAN-20260326-COMMPT2.P04`
  - marker: `@requirement REQ-NP-001`

- **Implement `rust_NPCPhrase_splice`** (replacing stub at line 890–893)
  - Delegate to `rust_NPCPhrase_cb(index, None)`
  - marker: `@plan PLAN-20260326-COMMPT2.P04`
  - marker: `@requirement REQ-NP-002`

- **Add extern "C" declarations** for C bridge functions used:
  - `c_get_conversation_phrase(phrases: *const c_void, index: c_int) -> *const c_uchar` — normal phrase text lookup
  - `c_SpliceTrack(filespec: *const c_char, textspec: *const c_char, timestamp: *const c_char, cb: Option<unsafe extern "C" fn()>)` — audio splice
  - `c_GetStringSoundClip(phrases: *const c_void, index: c_int) -> *const c_void` — audio clip for phrase
  - `c_GetStringTimeStamp(phrases: *const c_void, index: c_int) -> *const c_void` — timestamp for phrase
  - `c_GetCommanderName() -> *const c_char` — GLOBAL_SIS(CommanderName)
  - `c_GetShipName() -> *const c_char` — GLOBAL_SIS(ShipName)
  - `c_GetGameState_NewAllianceName() -> c_int` — GET_GAME_STATE(NEW_ALLIANCE_NAME)
  - marker: `@plan PLAN-20260326-COMMPT2.P04`
  - marker: `@requirement REQ-NP-003`

#### `rust/src/comm/state.rs` (if needed)
- **Add accessor for ConversationPhrases handle** if not already present
  - `conversation_phrases_handle(&self) -> *const c_void`
  - marker: `@plan PLAN-20260326-COMMPT2.P04`

#### `rust/src/comm/summary.rs` (if needed)
- **Add method to append NPC phrase to summary**
  - `append_npc_phrase(&mut self, text: &str)`
  - marker: `@plan PLAN-20260326-COMMPT2.P04`
  - marker: `@requirement REQ-NP-004`

### Pseudocode traceability
- Uses pseudocode lines: C01–C19 (NPC Phrase implementation)

## Verification Commands

```bash
# Structural gate
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features

# Verify stubs are replaced
grep -n "P11: Stub\|P11: Track\|let _ = (index\|let _ = index" rust/src/comm/ffi.rs
# Should not match in rust_NPCPhrase_cb or rust_NPCPhrase_splice

# Verify phrase resolution calls exist
grep -n "c_get_conversation_phrase\|c_SpliceTrack" rust/src/comm/ffi.rs

# C build verification
# (project-specific USE_RUST_COMM=on build)
```

## Structural Verification Checklist
- [ ] `rust_NPCPhrase_cb` body is no longer a stub
- [ ] `rust_NPCPhrase_splice` body is no longer a stub
- [ ] C bridge extern declarations added for `c_get_conversation_phrase` and `c_SpliceTrack`
- [ ] ConversationPhrases handle accessor available
- [ ] Conversation summary update integrated
- [ ] `@plan` and `@requirement` markers present
- [ ] All existing tests compile and pass

## Semantic Verification Checklist (Mandatory)
- [ ] **index==0 branch**: Early return with no SpliceTrack call
- [ ] **GLOBAL_PLAYER_NAME branch**: Uses commander name, NULL clip/timestamp
- [ ] **GLOBAL_SHIP_NAME branch**: Uses ship name, NULL clip/timestamp
- [ ] **Negative index branch**: Correctly computes alliance name variant with GLOBAL_ALLIANCE_NAME offset and NEW_ALLIANCE_NAME game state; appends CommanderName when state==3
- [ ] **Normal positive index branch**: Calls `c_get_conversation_phrase` for text, `GetStringSoundClip` for audio clip, `GetStringTimeStamp` for timestamp
- [ ] `rust_NPCPhrase_cb` calls `c_SpliceTrack(pClip, pStr, pTimeStamp, cb)` for all non-zero paths
- [ ] `rust_NPCPhrase_cb` passes the callback parameter through to `c_SpliceTrack`
- [ ] `rust_NPCPhrase_splice` delegates to `rust_NPCPhrase_cb` with `None` callback
- [ ] Null phrase pointer from `c_get_conversation_phrase` is handled (log + early return)
- [ ] Conversation summary is updated on successful phrase emission
- [ ] No memory leaks — phrase text pointer is not freed by Rust (owned by C string table)
- [ ] Feature is reachable: `commglue.c` calls `rust_NPCPhrase_cb` under USE_RUST_COMM guard

## Deferred Implementation Detection (Mandatory)

```bash
# Reject if these appear in the NPCPhrase functions:
grep -A10 "rust_NPCPhrase_cb\|rust_NPCPhrase_splice" rust/src/comm/ffi.rs | grep -i "stub\|todo\|fixme\|placeholder\|for now"
```

## Success Criteria
- [ ] `rust_NPCPhrase_cb` resolves phrases and splices tracks
- [ ] `rust_NPCPhrase_splice` works as convenience wrapper
- [ ] Conversation summary updated on each phrase
- [ ] All 267+ comm tests pass
- [ ] Both build modes compile
- [ ] `cargo fmt`, `cargo clippy`, `cargo test` all pass

## Failure Recovery
- Rollback: `git checkout -- rust/src/comm/ffi.rs rust/src/comm/state.rs rust/src/comm/summary.rs`
- If `c_get_conversation_phrase` signature doesn't match: check rust_comm.c:283 and rust_comm.h
- If ConversationPhrases handle is not accessible: add accessor to CommState or use C bridge
- Blocking: `c_SpliceTrack` must exist and have correct signature

## Phase Completion Marker
Create: `project-plans/20260311/commpt2/.completed/P04.md`

Contents:
- Phase ID: `PLAN-20260326-COMMPT2.P04`
- Timestamp
- Files changed: `rust/src/comm/ffi.rs`, possibly `state.rs`, `summary.rs`
- Tests added/updated: (list)
- Verification outputs
- Semantic verification summary
