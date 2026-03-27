# Communication Subsystem Production Parity (Part 3) — Requirements

Plan ID: `PLAN-20260325-COMMPT3`

## Purpose

Close remaining runtime parity gaps so that `USE_RUST_COMM=on` is
production-ready and matches C behavior across all 27 race encounters.
Parts 1 and 2 built the Rust module structure, FFI bridge, HailAlien
orchestration, input bridging, NPC phrase emission, and C-side rendering
bridges. This plan addresses five categories of defects discovered during
integration testing.

---

## Defect Summary (grounded in current codebase)

| # | Defect | Evidence |
|---|--------|----------|
| 1 | Null colormap handle | `talk_segue.rs:1003` calls `c_SetColorMap(std::ptr::null_mut())` with comment "pass null for now" |
| 2 | Null music handle | `talk_segue.rs:945` calls `c_PlayMusic(std::ptr::null_mut(), 1, 1)` — song handle is not resolved from CommData |
| 3 | Subtitle rendering disconnected | `rust_comm.c:562-576` routes `c_ClearSubtitles/c_CheckSubtitles/c_RedrawSubtitles` to `rust_ClearSubtitles/rust_CheckSubtitles/rust_RedrawSubtitles` — these Rust functions update a model that never renders to screen |
| 4 | DoCommunication response dispatch | `ffi.rs:732-747` calls `player_response_input` twice (once inside `do_communication`, once in the FFI wrapper) and does not reliably drop `COMM_STATE` write lock before C callback invocation |
| 5 | Stale markers | `talk_segue.rs:1002` ("for now"), `ffi.rs:879-881` ("not yet wired" / "not yet implemented") indicate deferred work in production code paths |

---

## Requirement Families

### CM — ColorMap Application

| ID | EARS Style | Requirement |
|---|---|---|
| REQ-CM-001 | Event-Driven | WHEN `set_colormap` is called during `AlienTalkSegue` intro setup, the system SHALL call a C bridge function that executes `SetColorMap(GetColorMapAddress(CommData.AlienColorMap))` directly, not pass a null pointer. |
| REQ-CM-002 | Ubiquitous | The colormap bridge function SHALL obtain the handle from `CommData.AlienColorMap` (which was populated during `hail_alien` resource loading at `hail.rs:208-209`) and SHALL be a no-op if the handle is zero. |
| REQ-CM-003 | State-Driven | WHILE the encounter is active, the colormap applied by `c_SetColorMapFromCommData` SHALL reflect the current `CommData.AlienColorMap` value (which may change if color transforms are applied). |

### MU — Music Playback

| ID | EARS Style | Requirement |
|---|---|---|
| REQ-MU-001 | Event-Driven | WHEN `play_alien_music` is called during encounter startup, the system SHALL call a C bridge function that executes `PlayMusic(CommData.AlienSong, TRUE, 1)` directly, not pass a null pointer. |
| REQ-MU-002 | Ubiquitous | The music bridge function SHALL obtain the song handle from `CommData.AlienSong` (populated during `hail_alien` resource loading at `hail.rs:211-224`) and SHALL be a no-op if the handle is zero. |
| REQ-MU-003 | Event-Driven | WHEN the encounter enters the first `AlienTalkSegue` call, music SHALL already be playing at background volume, matching C's `FadeMusic(BACKGROUND_VOL, 0)` call order at `talk_segue.rs:319`. |

### SD — Subtitle Display Integration

| ID | EARS Style | Requirement |
|---|---|---|
| REQ-SD-001 | Ubiquitous | The `c_ClearSubtitles`, `c_CheckSubtitles`, and `c_RedrawSubtitles` bridge functions in `rust_comm.c` SHALL delegate to C-side drawing primitives for actual screen rendering; they SHALL NOT route back to Rust FFI functions. |
| REQ-SD-002 | Event-Driven | WHEN `c_ClearSubtitles` is called, it SHALL set `clear_subtitles = TRUE`, `last_subtitle = NULL`, `SubtitleText.pStr = NULL`, and `SubtitleText.CharCount = 0` — matching the C implementation at `comm.c:1661-1667`. |
| REQ-SD-003 | Event-Driven | WHEN `c_CheckSubtitles` is called, it SHALL read the current subtitle from `GetTrackSubtitle()`, compare against `SubtitleText`, and update `SubtitleText` and `clear_subtitles` accordingly — matching `comm.c:1670-1701`. |
| REQ-SD-004 | Event-Driven | WHEN `c_RedrawSubtitles` is called, it SHALL draw the current `SubtitleText` to screen using `add_text(1, &t)` if `optSubtitles` is true and `SubtitleText.pStr` is non-null — matching `comm.c:1646-1657`. |
| REQ-SD-005 | Unwanted | The Rust `SubtitleDisplay` model SHALL NOT independently render subtitle text. The C trackplayer (`GetTrackSubtitle()`) is the sole source of truth for subtitle content. Rust `rust_ClearSubtitles/rust_CheckSubtitles/rust_RedrawSubtitles` FFI functions remain for test use only. |

### CS — Conversation Summary (C-Side Authority)

| ID | EARS Style | Requirement |
|---|---|---|
| REQ-CS-001 | Event-Driven | WHEN the player presses Cancel during response selection, the system SHALL invoke `c_SelectConversationSummary()` which runs the C-side summary paging loop. This is already correctly wired at `talk_segue.rs:816-827`. |
| REQ-CS-002 | Ubiquitous | The `rust_ShowConversationSummary` FFI function SHALL delegate to `c_SelectConversationSummary()` in production builds (non-test). The Rust `SummaryView` model SHALL only execute under `#[cfg(test)]`. |
| REQ-CS-003 | Unwanted | The `rust_ShowConversationSummary` production path SHALL NOT use the Rust `SummaryView` pagination loop, because it does not interact with C's `DoInput` or draw to screen. |

### RL — Response-Callback Lock Discipline

| ID | EARS Style | Requirement |
|---|---|---|
| REQ-RL-001 | Ubiquitous | When a response callback is dispatched from `rust_DoCommunication`, the `COMM_STATE` write lock SHALL be released before the callback is invoked. |
| REQ-RL-002 | Event-Driven | WHEN `rust_DoCommunication` detects that the player has confirmed a response (via `do_communication` → `player_response_input` returning `Selected`), it SHALL: (1) call `select_response` to perform pre-callback work while holding the lock, (2) extract the `(callback_fn, response_ref)` tuple, (3) drop the `COMM_STATE` write guard, (4) invoke `callback_fn(response_ref)`, (5) return 1 (Continue). |
| REQ-RL-003 | Unwanted | The response-callback dispatch path SHALL NOT hold the COMM_STATE write lock while executing a C callback, because C callbacks call Rust FFI functions (`rust_NPCPhrase_cb`, `rust_DoResponsePhrase`, `rust_SetSegue`, `rust_DisablePhrase`) that acquire the same lock — holding it during callback creates a deadlock since `parking_lot::RwLock` does not support recursive write locking. |
| REQ-RL-004 | Event-Driven | WHEN `select_response` in `talk_segue.rs` needs to execute pre-callback work (clear responses, stop track, clear subtitles, fade music, feedback player phrase), it SHALL perform all that work while holding the lock, then return the `(callback_fn, response_ref)` tuple for the caller to dispatch outside the lock. |

### DC — DoCommunication State Machine Correctness

| ID | EARS Style | Requirement |
|---|---|---|
| REQ-DC-001 | Event-Driven | WHEN `rust_DoCommunication` is invoked by DoInput each frame, it SHALL execute exactly one iteration of the state machine: either an `alien_talk_segue` step (if `talking_finished == false`) or a player-response-input step (if `talking_finished == true` and responses exist) or a last-replay exit (if `talking_finished == true` and no responses). |
| REQ-DC-002 | State-Driven | WHILE the alien is talking (`talking_finished == false`), `rust_DoCommunication` SHALL NOT process player response input and SHALL NOT call `player_response_input`. |
| REQ-DC-003 | State-Driven | WHILE the alien has finished talking AND responses are registered, `rust_DoCommunication` SHALL process exactly one frame of player response input (select/cancel/up/down/left/right handling). |
| REQ-DC-004 | Event-Driven | WHEN `rust_DoCommunication` determines the encounter is done (no responses after talking finished, and last-replay complete), it SHALL return 0 (FALSE) to end the DoInput loop. |
| REQ-DC-005 | Event-Driven | WHEN `CHECK_ABORT` or `CHECK_LOAD` is set, `rust_DoCommunication` SHALL return 0 to exit the DoInput loop immediately. |

### TS — Talk Segue Parity

| ID | EARS Style | Requirement |
|---|---|---|
| REQ-TS-001 | Event-Driven | WHEN `alien_talk_segue` is called for the first time in an encounter (`first_talk_call == false`), it SHALL execute the intro sequence: `init_speech_graphics`, `set_colormap` (with real handle), `draw_alien_frame`, `update_speech_graphics`, `comm_intro_transition`, `play_alien_music` (with real handle), `set_music_background_vol`, `init_comm_animations`, `clear_last_activity_load_flag` — matching `talk_segue.rs:309-322`. |
| REQ-TS-002 | Event-Driven | WHEN a track is playing and no abort has occurred, `talk_segue` SHALL call `c_CheckSubtitles` (via the subtitle bridge), `c_UpdateAnimations`, and `c_UpdateSpeechGraphics` each frame. |
| REQ-TS-003 | Event-Driven | WHEN the talking animation should start (track is playing and `want_talking_anim && have_talking_anim`), `talk_segue` SHALL call `set_run_talking_anim`. WHEN the track finishes, it SHALL call `set_stop_talking_anim`. |
| REQ-TS-004 | Event-Driven | WHEN `talk_segue` detects track completion (`ts.ended == true`), `alien_talk_segue` SHALL set `talking_finished = true` and fade music to foreground. |

### SM — Stale Marker Elimination

| ID | EARS Style | Requirement |
|---|---|---|
| REQ-SM-001 | Ubiquitous | After this plan completes, zero instances of `for now`, `TODO`, `FIXME`, `HACK`, `placeholder`, `stub`, `not yet implemented`, or `not yet wired` SHALL remain in production code paths within `rust/src/comm/*.rs` and `sc2/src/uqm/rust_comm.c`. |
| REQ-SM-002 | Optional | IF a marker exists in a `#[cfg(test)]` block, a doc comment describing design rationale (not deferred work), or a reference to existing C stubs (e.g. "the USE_RUST_COMM stubs"), it MAY remain. |

### E2E — End-to-End Production Parity

| ID | EARS Style | Requirement |
|---|---|---|
| REQ-E2E-001 | Ubiquitous | With `USE_RUST_COMM=on`, entering any alien conversation SHALL display the alien portrait with correct colormap, play background music, render subtitle text synchronized with speech audio, and present player response options. |
| REQ-E2E-002 | Ubiquitous | With `USE_RUST_COMM=on`, the conversation summary (Cancel key) SHALL display paginated subtitle history with `...more...` indicators, matching C's per-page layout. |
| REQ-E2E-003 | Ubiquitous | With `USE_RUST_COMM=on`, response selection SHALL highlight the current response, scroll through overflow responses, wrap at list boundaries, and dispatch the correct callback with the correct `RESPONSE_REF` argument. |
| REQ-E2E-004 | Ubiquitous | With `USE_RUST_COMM=on`, replay (Left key during response selection) SHALL replay the last committed NPC phrase's audio without re-firing callbacks. |
| REQ-E2E-005 | Ubiquitous | Both `USE_RUST_COMM=on` and `USE_RUST_COMM=off` builds SHALL compile, link, and run without errors. |
| REQ-E2E-006 | Ubiquitous | All existing 268 comm tests SHALL continue to pass with zero regressions. |
| REQ-E2E-007 | Unwanted | The communication subsystem SHALL NOT deadlock under any encounter flow (including rapid response selection, skip, abort, load, and summary access). |
