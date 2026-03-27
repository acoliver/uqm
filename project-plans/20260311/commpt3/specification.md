# Communication Subsystem Production Parity (Part 3) — Specification

Plan ID: `PLAN-20260325-COMMPT3`

## 1. Problem Statement

Parts 1 and 2 of the comm port assembled the Rust module structure, C bridge
wrappers, HailAlien encounter orchestration, input bridging, NPC phrase
emission, and C-side rendering bridges. The code compiles, links, and 268
tests pass. However, runtime testing with `USE_RUST_COMM=on` reveals five
categories of parity failures:

1. **Null handle passthrough**: `set_colormap()` at `talk_segue.rs:1003` passes
   `std::ptr::null_mut()` to `c_SetColorMap` instead of the real
   `CommData.AlienColorMap` handle. `play_alien_music()` at `talk_segue.rs:945`
   passes `std::ptr::null_mut()` to `c_PlayMusic` instead of the real
   `CommData.AlienSong` handle. Both have "for now" markers.

2. **Subtitle rendering disconnection**: The three subtitle bridge functions in
   `rust_comm.c:562-576` (`c_ClearSubtitles`, `c_CheckSubtitles`,
   `c_RedrawSubtitles`) route to Rust FFI (`rust_ClearSubtitles` etc.) instead
   of to the C drawing primitives. The Rust `SubtitleDisplay` model updates
   internal state but has no access to the C drawing surface. Subtitles never
   appear on screen.

3. **DoCommunication response dispatch**: `rust_DoCommunication` at
   `ffi.rs:715-752` has a convoluted response path. `do_communication()` at
   line 723 internally calls `player_response_input()`, then the FFI wrapper at
   lines 732-747 calls `player_response_input()` again, consuming the input
   twice per frame. The lock-drop-before-callback pattern at lines 742-746 is
   fragile: it depends on detecting the Selected state via a second call.

4. **Conversation summary model unused**: `rust_ShowConversationSummary` at
   `ffi.rs:860-889` uses the Rust `SummaryView` pagination model with "abort
   not yet wired" and "input handling is not yet implemented" comments. However,
   the actual cancel-key path in `player_response_input` at
   `talk_segue.rs:816-827` correctly calls `c_SelectConversationSummary`
   directly. The Rust model is dead code in production.

5. **Stale "for now" markers**: Three production code markers remain:
   - `talk_segue.rs:1002`: "pass null for now" (colormap)
   - `ffi.rs:879`: "abort not yet wired"
   - `ffi.rs:881`: "input handling is not yet implemented"

## 2. Architectural Boundaries

### 2.1 What Rust Owns (unchanged from pt2)

- `CommState` — dialogue state machine, response system, phrase state, segue
- `talk_segue.rs` — DoCommunication / AlienTalkSegue / PlayerResponseInput
  state machine logic (decides WHAT to do each frame)
- `ffi.rs` — FFI boundary, lock discipline, callback dispatch
- `hail.rs` — HailAlien encounter orchestration sequence

### 2.2 What C Owns (unchanged from pt2)

- All screen rendering (font drawing, rectangle fills, stamps)
- Trackplayer (SpliceTrack, PlayTrack, GetTrackSubtitle, subtitle history)
- DoInput frame loop and timing
- Resource loading/destruction
- 27 race scripts
- CommData static instance
- Subtitle state (`SubtitleText`, `clear_subtitles`, `add_text` — all static
  in comm.c)

### 2.3 Boundary Invariant: Rust Decides, C Renders

Rust drives the state machine and decides what action each frame requires.
C performs all actual rendering and resource management. Rust never draws
pixels or reads the frame buffer. C never makes dialogue-tree decisions.

## 3. Data Contracts

### 3.1 Colormap Handle

- **Source**: `CommData.AlienColorMap` (set at `hail.rs:209` via
  `c_SetCommDataAlienColorMap`)
- **Access**: Direct from `CommData` in C — no Rust-side accessor needed
- **Consumer**: `SetColorMap(GetColorMapAddress(cmap))` in C
- **New bridge**: `c_SetColorMapFromCommData()` in `rust_comm.c` — calls
  `SetColorMap(GetColorMapAddress(CommData.AlienColorMap))` directly. Zero
  parameters. Returns void. No-op if `CommData.AlienColorMap == 0`.

### 3.2 Music Handle

- **Source**: `CommData.AlienSong` (set at `hail.rs:224` via
  `c_SetCommDataAlienSong`)
- **Access**: Direct from `CommData` in C
- **Consumer**: `PlayMusic(song, TRUE, 1)` in C
- **New bridge**: `c_PlayAlienMusic()` in `rust_comm.c` — calls
  `PlayMusic(CommData.AlienSong, TRUE, 1)` directly. Zero parameters.
  Returns void. No-op if `CommData.AlienSong == 0`.

### 3.3 Subtitle State

- **Source**: C trackplayer `GetTrackSubtitle()` (in `libs/sound/trackplayer.c`)
- **Rendering**: C `SubtitleText` struct + `add_text()` + `font_DrawText()`
  (all static in `comm.c`)
- **Bridge fix**: Subtitle functions in `rust_comm.c` must route to C
  implementations, NOT back to Rust FFI. Since `SubtitleText`, `clear_subtitles`,
  `last_subtitle`, and `add_text` are all static in `comm.c`, the C-side
  subtitle implementations MUST live in `comm.c` inside the
  `#ifdef USE_RUST_COMM` block (section starting at line 1715). The bridge
  functions in `rust_comm.c` then call these `comm.c` implementations.

### 3.4 Response Callback Dispatch

- **Input**: `(callback_fn: extern "C" fn(u32), response_ref: u32)` extracted
  from `select_response`
- **Precondition**: COMM_STATE write lock released before callback invocation
- **C callback re-entry**: The callback will re-enter Rust through
  `rust_NPCPhrase_cb`, `rust_DoResponsePhrase`, `rust_SetSegue`,
  `rust_DisablePhrase` — each acquiring its own `COMM_STATE.write()` lock
- **Contract**: `parking_lot::RwLock` does NOT support recursive write locking;
  the outer lock MUST be dropped before any callback invocation

## 4. Integration Points

### 4.1 Colormap Integration (fix)

```
AlienTalkSegue (first call, intro setup)
  └─ set_colormap()                      [talk_segue.rs:997-1009]
       └─ c_SetColorMapFromCommData()    [NEW bridge in rust_comm.c]
            └─ SetColorMap(GetColorMapAddress(CommData.AlienColorMap))  [C]
```

**Replaces**: `c_SetColorMap(std::ptr::null_mut())` at `talk_segue.rs:1003`

### 4.2 Music Integration (fix)

```
AlienTalkSegue (first call, intro setup)
  └─ play_alien_music()                  [talk_segue.rs:939-951]
       └─ c_PlayAlienMusic()             [NEW bridge in rust_comm.c]
            └─ PlayMusic(CommData.AlienSong, TRUE, 1)  [C]
```

**Replaces**: `c_PlayMusic(std::ptr::null_mut(), 1, 1)` at `talk_segue.rs:945`

### 4.3 Subtitle Display Integration (fix)

**Current** (broken circular routing):
```
talk_segue.rs → c_CheckSubtitles [rust_comm.c:568]
                 → rust_CheckSubtitles [ffi.rs:833]
                   → SubtitleDisplay.check_subtitle [Rust model, no rendering]
```

**Fixed**:
```
talk_segue.rs → c_CheckSubtitles [rust_comm.c]
                 → comm_CheckSubtitles [comm.c, #ifdef USE_RUST_COMM]
                   → GetTrackSubtitle + compare + update SubtitleText  [C]

talk_segue.rs → c_RedrawSubtitles [rust_comm.c]
                 → comm_RedrawSubtitles [comm.c, #ifdef USE_RUST_COMM]
                   → add_text(1, &SubtitleText)  [C]

talk_segue.rs → c_ClearSubtitles [rust_comm.c]
                 → comm_ClearSubtitles [comm.c, #ifdef USE_RUST_COMM]
                   → clear_subtitles=TRUE, SubtitleText.pStr=NULL  [C]
```

### 4.4 DoCommunication Response Dispatch (fix)

```
DoInput frame
  └─ rust_do_communication_cb [rust_comm.c]
       └─ rust_DoCommunication [ffi.rs]
            ├─ [abort/load] → return 0
            ├─ [talking] do_communication → alien_talk_segue → return 1
            ├─ [responses, Continue] do_communication → return 1
            ├─ [responses, Selected] do_communication detected Selected:
            │    extract (fn, ref) from select_response
            │    DROP state
            │    fn(ref) → C race script → re-enters Rust FFI
            │    return 1
            └─ [no responses, Done] → return 0
```

### 4.5 Summary Integration (no production wiring change)

The cancel-key path in `player_response_input` at `talk_segue.rs:816-827`
already calls `c_SelectConversationSummary` directly. Only change:
`rust_ShowConversationSummary` in `ffi.rs` is guarded so the production
path delegates to `c_SelectConversationSummary()` and the Rust `SummaryView`
model only runs under `#[cfg(test)]`.

## 5. Error and Edge Case Expectations

| Scenario | Expected Behavior |
|---|---|
| `CommData.AlienColorMap == 0` (load failure) | `c_SetColorMapFromCommData` is a no-op; encounter runs without colormap |
| `CommData.AlienSong == 0` (no music) | `c_PlayAlienMusic` is a no-op; encounter runs silently |
| `GetTrackSubtitle() == NULL` | `CheckSubtitles` sets `CharCount=0`; `RedrawSubtitles` returns immediately |
| Response callback function pointer is NULL | `select_response` returns `None`; no callback dispatched |
| COMM_STATE lock contention during callback | Not possible: lock is dropped before callback; each re-entrant FFI function acquires its own separate lock |
| CHECK_ABORT set during callback | Next `rust_DoCommunication` frame detects abort and returns 0 |
| Multiple rapid Select presses | Only one callback dispatched per `rust_DoCommunication` frame — `do_communication` returns `Continue` with Selected only once |
| Summary with 0 subtitle entries | `c_SelectConversationSummary` shows empty page, returns on any key |

## 6. Non-Functional Requirements

| Requirement | Target |
|---|---|
| Lock hold time | COMM_STATE write lock held for at most one state-machine iteration (~microseconds); released before any C callback |
| Frame timing | `rust_DoCommunication` executes in < 1ms per frame (no blocking I/O) |
| Memory | No new allocations per frame in steady state |
| Build compatibility | Both `USE_RUST_COMM=on` and `=off` compile and link |
| Test stability | 268 existing tests pass; new tests added for fixed behaviors |

## 7. Testability Requirements

| Area | Test Strategy |
|---|---|
| Colormap bridge | Build verification (compiles + links); runtime manual test (colormap visible on alien portrait) |
| Music bridge | Build verification; runtime manual test (music plays during encounter) |
| Subtitle routing | Build verification; C-level function trace in `comm.c`; runtime manual test (subtitles appear) |
| Lock discipline | Rust unit test: simulate select → extract callback info → lock dropped → callback invokes |
| DoCommunication | Rust unit test: state machine transitions (talking → responses → done) |
| Stale markers | Automated `grep` sweep: zero hits in production paths |

## 8. Old Code Replacement/Removal

| Location | Current Code | Replacement |
|---|---|---|
| `talk_segue.rs:997-1009` (`set_colormap`) | `c_SetColorMap(std::ptr::null_mut())` + "for now" comment | `c_SetColorMapFromCommData()` — zero arguments, remove `c_SetColorMap` from extern block |
| `talk_segue.rs:939-951` (`play_alien_music`) | `c_PlayMusic(std::ptr::null_mut(), 1, 1)` | `c_PlayAlienMusic()` — zero arguments, remove `c_PlayMusic` from extern block |
| `rust_comm.c:562-576` | `c_ClearSubtitles → rust_ClearSubtitles`, etc. | Forward to `comm_ClearSubtitles` / `comm_CheckSubtitles` / `comm_RedrawSubtitles` implemented in `comm.c` |
| `ffi.rs:715-752` (`rust_DoCommunication`) | Convoluted double `player_response_input` call | Clean single-pass: `do_communication` → check result → if Selected extract callback → drop lock → invoke callback |
| `ffi.rs:860-889` (`rust_ShowConversationSummary`) | Rust `SummaryView` loop with "not yet wired" comments | `#[cfg(not(test))]` delegates to `c_SelectConversationSummary`; `#[cfg(test)]` retains Rust model |
| `comm.c:1715-1752` | Static variable accessors only | Add `comm_ClearSubtitles`, `comm_CheckSubtitles`, `comm_RedrawSubtitles` implementations |
