# Phase 01: Analysis

## Phase ID
`PLAN-20260314-COMM.P01`

## Prerequisites
- Required: Phase 0.5 (Preflight) completed and PASS

## Purpose
Detailed gap analysis against specification and requirements. Maps every requirement to current implementation status and identifies work needed.

## 1. Entity and State Model

### 1.1 CommData / LOCDATA

**Current state**: Rust `CommData` in `types.rs` has only 10 fields (3 callback addresses, 3 graphics handles, `num_animations`, `ambient_flags`, `transition_time`).

**Required state**: Full LOCDATA parity per spec §3.1 — 26+ fields including resource IDs (AlienFrameRes, AlienFontRes, AlienColorMapRes, AlienSongRes, AlienAltSongRes, ConversationPhrasesRes), text layout (AlienTextFColor, AlienTextBColor, AlienTextBaseline, AlienTextWidth, AlienTextAlign, AlienTextValign), animation descriptors (AlienAmbientArray[20], AlienTransitionDesc, AlienTalkDesc), number speech table pointer, loaded resource handles, and song flags.

**Work**: Expand `CommData` to include all LOCDATA fields. Create FFI accessors to read `LOCDATA*` fields from C. No binary layout match needed (spec §3.2).

**Requirements**: EC-REQ-003, EC-REQ-007, DS-REQ-004, SC-REQ-003

### 1.2 Phrase Enable/Disable State

**Current state**: No phrase state tracking exists in Rust. `PHRASE_ENABLED` / `DISABLE_PHRASE` macros in C directly mutate the string table.

**Required state**: Encounter-local bitset tracking disabled phrases. `PHRASE_ENABLED(p)` → check bitset. `DISABLE_PHRASE(p)` → set bit. Per spec §11.4/§11A, this is a narrower semantic model — disabling does not alter phrase text.

**Work**: Add `disabled_phrases: HashSet<i32>` (or `BitVec`) to `CommState`. Add FFI exports `rust_PhraseEnabled(index) -> bool` and `rust_DisablePhrase(index)`. Rewrite C macros to call these instead of string-table mutation. Treat the narrower semantic model as gated by the mandatory all-27-script audit, with a fallback compatibility branch if any violating path is found.

**Requirements**: PS-REQ-001–007, DS-REQ-012, SC-REQ-002, CV-REQ-015

### 1.3 Segue State

**Current state**: No segue state in Rust. `setSegue`/`getSegue` are C functions in `commglue.c:315-355`.

**Required state**: Rust-owned segue state with FFI accessors. Side effects (instant victory, defeat) must be applied.

**Work**: Add `Segue` enum and `current_segue` to `CommState`. Implement `setSegue`/`getSegue` in Rust. Wire side effects per spec §9.1.

**Requirements**: DS-REQ-011, SB-REQ-001–006

### 1.4 Response Callback Type

**Current state**: `ResponseEntry.response_func` stores `Option<usize>` (raw address). FFI `rust_DoResponsePhrase` accepts `Option<extern "C" fn()>`. `rust_ExecuteResponse` calls callback as `fn()`.

**Required state**: Callback type must be `extern "C" fn(u32)` — receives RESPONSE_REF as argument. Per spec §5.4, §14.4, RS-REQ-011.

**Work**: Change `ResponseEntry.response_func` to `Option<extern "C" fn(u32)>`. Update FFI signatures. Fix `rust_ExecuteResponse` to pass `response_ref` to callback.

**Requirements**: RS-REQ-011, RS-REQ-012, CV-REQ-008

## 2. Integration Touchpoints

### 2.1 Trackplayer Integration

**Current state**: Rust `TrackManager` is a self-contained synthetic timeline with position-based playback. No integration with C trackplayer (`SpliceTrack`, `PlayTrack`, etc.). No phrase-level completion callbacks.

**Required state**: Comm must integrate with the authoritative trackplayer per spec §6.1. Two possible approaches:
1. If trackplayer is C-owned: Rust comm calls through FFI to C trackplayer functions and uses trackplayer-owned subtitle enumeration, replay-target semantics, and pending-completion APIs as the source of truth.
2. If trackplayer is Rust-owned: Rust comm calls Rust trackplayer directly, but the same ownership boundary still applies — trackplayer, not comm, owns pending completion, subtitle history enumeration, replay-target updates, and phrase advancement semantics.

The current Rust `TrackManager` must therefore be redesigned into a thin integration layer rather than a shadow owner of queue/history/commit state.

**Critical APIs needed**:
- `SpliceTrack(clip, text, timestamps, callback)` — queue a phrase
- `SpliceMultiTrack(tracks[], text)` — queue multi-clip phrase
- `PlayTrack()` / `StopTrack()` / `JumpTrack()` — playback control
- `GetTrackSubtitle()` — current subtitle
- `GetFirstTrackSubtitle()` / `GetNextTrackSubtitle()` / `GetTrackSubtitleText()` — history enumeration
- `PlayingTrack()` — status query
- `PollPendingTrackCompletion()` / `CommitTrackAdvancement()` — phrase completion handoff

**Requirements**: TP-REQ-001–013, SS-REQ-001–017, IN-REQ-001–003

### 2.2 Graphics Integration

**Current state**: No graphics integration in Rust comm. Animation frame drawing, subtitle rendering, response rendering, oscilloscope drawing — all absent.

**Required state**: Comm calls graphics FFI for all rendering. Graphics subsystem is partially ported (Rust backend, C drawing primitives).

**Key integration points**:
- `DrawStamp` / `BatchGraphics` / `UnbatchGraphics` for animation frames
- `font_DrawText` / `SetContextFont` / `SetContextForeGroundColor` for subtitles
- `CreateContext` / `SetContext` / `SetContextClipRect` for context management
- `LoadGraphic` / `LoadFont` / `LoadColorMap` / `LoadMusic` for resource loading
- `CaptureDrawable` / `DestroyDrawable` / `ReleaseDrawable` for lifecycle

**Requirements**: IN-REQ-005, IN-REQ-006, AO-REQ-010

### 2.3 Game State Integration

**Current state**: No game state access from Rust comm.

**Required state**: Rust needs to read `GLOBAL(CurrentActivity)`, `GLOBAL_SIS()`, `LastActivity` for encounter flow and `RaceCommunication()` context selection. Scripts access `GET_GAME_STATE`/`SET_GAME_STATE` directly (these remain C macros, not comm-owned).

**Requirements**: IN-REQ-007, IN-REQ-008

### 2.4 Input System Integration

**Current state**: No input handling in Rust comm.

**Required state**: Response navigation (up/down/select/cancel), seek controls, menu sounds. Uses `DoInput` loop pattern.

**Requirements**: IN-REQ-010

### 2.5 Encounter Flow Integration

**Current state**: No encounter flow integration in Rust comm.

**Required state**: Calls to `BuildBattle`, `EncounterBattle`, `InitEncounter`, `UninitEncounter`, `StartSphereTracking`, and the saved-game SIS display refresh path used during encounter setup ordering.

**Requirements**: IN-REQ-009, EC-REQ-011–012

### 2.6 Public Entry-Point Ownership

**Current state**: Planning and implementation focus almost entirely on `InitCommunication()`. `RaceCommunication()` remains effectively C-owned / unspecified.

**Required state**: Both public entry points are Rust-owned in `USE_RUST_COMM` mode. `RaceCommunication()` selects encounter context from game state (hyperspace, interplanetary, last-battle, etc.), performs any required saved-game display update step, resolves the `CONVERSATION` variant, then delegates to `InitCommunication()`.

**Work**: Add a Rust-owned entry-point module or extend `encounter.rs` so `RaceCommunication()` is explicitly implemented and wired through the C bridge only as a thin wrapper in Rust mode. Preserve the C fallback path unchanged when `USE_RUST_COMM` is off.

**Requirements**: EC-REQ-001, IN-REQ-012

## 3. Old Code to Replace/Remove

### 3.1 C Code to Guard Behind `#ifndef USE_RUST_COMM`

| File | Current Guard | Target Guard |
|------|--------------|--------------|
| `/Users/acoliver/projects/uqm/sc2/src/uqm/comm.c` | Only init/uninit (lines 400-412) | Entire file body except function declarations |
| `/Users/acoliver/projects/uqm/sc2/src/uqm/commglue.c` | None | All function bodies (NPCPhrase_cb, setSegue, init_race, etc.) |
| `/Users/acoliver/projects/uqm/sc2/src/uqm/commanim.c` | None | All function bodies |

### 3.2 C Header Changes

| File | Change |
|------|--------|
| `/Users/acoliver/projects/uqm/sc2/src/uqm/commglue.h` | Rewrite `PHRASE_ENABLED`/`DISABLE_PHRASE` macros to call Rust FFI when `USE_RUST_COMM` |
| `/Users/acoliver/projects/uqm/sc2/src/uqm/rust_comm.h` | Add missing FFI declarations, fix callback signatures |
| `/Users/acoliver/projects/uqm/sc2/src/uqm/rust_comm.c` | Expand wrapper functions for full comm API |

### 3.3 Rust Code to Modify

| File | Change |
|------|--------|
| `rust/src/comm/types.rs` | Expand `CommData` to full LOCDATA parity |
| `rust/src/comm/ffi.rs` | Fix callback signatures, add missing exports |
| `rust/src/comm/response.rs` | Fix callback type to `extern "C" fn(u32)` |
| `rust/src/comm/track.rs` | Redesign into thin trackplayer integration layer |
| `rust/src/comm/animation.rs` | Replace generic model with ANIMATION_DESC-based engine |
| `rust/src/comm/state.rs` | Add phrase state, segue, encounter lifecycle, lock discipline |

### 3.4 New Rust Files

| File | Purpose |
|------|---------|
| `rust/src/comm/glue.rs` | NPCPhrase_cb, NPCPhrase_splice, NPCNumber, construct_response |
| `rust/src/comm/segue.rs` | Segue enum and state management |
| `rust/src/comm/phrase_state.rs` | Phrase enable/disable tracking |
| `rust/src/comm/encounter.rs` | Encounter lifecycle orchestration plus Rust-owned public entry-point routing |
| `rust/src/comm/talk_segue.rs` | Talk segue and DoCommunication main loop |
| `rust/src/comm/summary.rs` | Conversation summary pagination/model over trackplayer enumeration |
| `rust/src/comm/speech_graphics.rs` | Oscilloscope rendering, slider |

## 4. Edge Cases and Error Handling

| Condition | Handling |
|-----------|---------|
| Double init | Return `CommError::AlreadyInitialized` (verify existing behavior explicitly) |
| Operations before init | Return `CommError::NotInitialized` (verify existing behavior explicitly) |
| Response overflow >8 | Silently reject, return false (verify existing behavior explicitly) |
| Null FFI string pointers | Early return / null result (partially implemented) |
| Callback deadlock (re-entrant lock) | Release-and-reacquire pattern before invoking C callbacks |
| Game abort (`CHECK_ABORT`) | Short-circuit dialogue loops |
| Load-game interruption | Detect via `CurrentActivity` and exit cleanly |
| Subtitle pointer lifetime | Copy string before releasing lock |
| Saved-game loaded before encounter | Refresh SIS display before further encounter setup steps |
| Phrase-disable audit finds violating script | Preserve legacy NUL-mutation behavior for that path or block rollout until corrected |

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Success Criteria
- [ ] All requirements mapped to gaps
- [ ] All gaps have identified remediation approach
- [ ] Integration points explicitly listed with file paths
- [ ] `RaceCommunication()` ownership and routing explicitly covered
- [ ] Required all-27-script phrase-disable audit is treated as a gate, not a spot check
- [ ] Existing-behavior assumptions are converted into explicit verification obligations
- [ ] Old code replacement plan explicit
- [ ] Edge cases documented
