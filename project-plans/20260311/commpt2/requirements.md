# Communication Subsystem Completion (Part 2) — Requirements

Plan ID: `PLAN-20260326-COMMPT2`

## Purpose

Complete the Rust communication subsystem so that `USE_RUST_COMM=on` produces
a fully functional game with working alien conversations.  The first comm plan
(PLAN-20260314-COMM) built component modules (animation, track, encounter,
response, subtitle, talk_segue) and wired C-side guards, but left critical
runtime integration stubs that cause the game to silently skip all dialogue
when the Rust path is active.

This plan closes those gaps.

---

## Requirement Families

### HL — HailAlien Loop (encounter orchestration)

| ID | Requirement |
|---|---|
| REQ-HL-001 | `rust_HailAlien` SHALL execute the full encounter loop: load resources, set up animation/text contexts, call init_encounter_func, run DoInput-equivalent loop, call post/uninit_encounter_func, free resources. |
| REQ-HL-002 | The encounter loop SHALL load alien portrait (AlienFrame), font (AlienFont), colormap (AlienColorMap), song (AlienSong with alt-song fallback), and conversation phrases (ConversationPhrases) using C bridge resource functions. |
| REQ-HL-003 | The encounter loop SHALL create and manage the AnimContext and TextCacheContext graphics contexts matching C HailAlien behavior. |
| REQ-HL-004 | The encounter loop SHALL invoke the LOCDATA init_encounter_func, then enter the dialogue input loop, then call post_encounter_func (if not aborted), then call uninit_encounter_func unconditionally. |
| REQ-HL-005 | The encounter loop SHALL clean up all loaded resources on exit regardless of exit path (normal, abort, load). Cleanup SHALL: (a) restore SpaceContext and saved font before destruction, (b) use `DestroyX(ReleaseX(handle))` for captured resources (AlienFrame, AlienColorMap, ConversationPhrases, TextCacheFrame), (c) use direct `DestroyX(handle)` for non-captured resources (AlienFont, AlienSong, PlayerFont), (d) follow exact C destruction order: ConversationPhrases → AlienSong → AlienColorMap → AlienFont → AlienFrame → TextCacheContext → TextCacheFrame → PlayerFont, (e) clear `CommData.ConversationPhrasesRes` and `CommData.ConversationPhrases` to 0, and (f) clear `pCurInputState` to NULL. |
| REQ-HL-006 | The encounter loop SHALL set `LastActivity |= CHECK_LOAD` before calling init_encounter_func to prevent spurious input. |
| REQ-HL-007 | The encounter loop SHALL draw the SIS frame, SIS message, SIS title, and SIS comm window matching C behavior for both WON_LAST_BATTLE and normal encounter contexts. |

### IP — Input Polling Bridge

| ID | Requirement |
|---|---|
| REQ-IP-001 | `check_select_input` SHALL return the actual state of `PulsedInputState.menu[KEY_MENU_SELECT]` via the C bridge, not hardcoded false. |
| REQ-IP-002 | `check_cancel_input` SHALL return the actual state of `PulsedInputState.menu[KEY_MENU_CANCEL]` via the C bridge. |
| REQ-IP-003 | `check_up_input` SHALL return the actual state of `PulsedInputState.menu[KEY_MENU_UP]` via the C bridge. |
| REQ-IP-004 | `check_down_input` SHALL return the actual state of `PulsedInputState.menu[KEY_MENU_DOWN]` via the C bridge. |
| REQ-IP-005 | `check_left_input` SHALL return the actual state of `PulsedInputState.menu[KEY_MENU_LEFT]` via the C bridge. |
| REQ-IP-006 | `check_right_input` SHALL return the actual state of `PulsedInputState.menu[KEY_MENU_RIGHT]` via the C bridge. |
| REQ-IP-007 | Input bridge functions SHALL be called from the Rust encounter loop via the existing `c_GetPulsedMenuKey(key_index)` C wrapper already present in rust_comm.c. |
| REQ-IP-008 | Test mode (`#[cfg(test)]`) SHALL continue to use test-driven input simulation, not the C bridge. |

### NP — NPC Phrase Emission

| ID | Requirement |
|---|---|
| REQ-NP-001 | `rust_NPCPhrase_cb(index, callback)` SHALL resolve the phrase text from ConversationPhrases, splice the audio track via `c_SpliceTrack`, and register the completion callback. |
| REQ-NP-002 | `rust_NPCPhrase_splice(index)` SHALL resolve and splice a phrase without a completion callback. |
| REQ-NP-003 | Phrase resolution SHALL use `c_get_conversation_phrase(phrases, index)` to obtain text from the C string table. |
| REQ-NP-004 | NPCPhrase emission SHALL update the conversation summary/history with each emitted phrase. |

### RB — Rendering Bridges (C-side display delegation)

| ID | Requirement |
|---|---|
| REQ-RB-001 | `c_FeedbackPlayerPhrase(text)` SHALL render the player's selected response text in the subtitle area by calling the appropriate C drawing functions. |
| REQ-RB-002 | `c_RefreshResponses(top, num_responses, cur_response)` SHALL render the response list in the SIS comm window using C's text rendering. |
| REQ-RB-003 | `c_SelectConversationSummary()` SHALL display the conversation summary overlay using C's drawing functions. |
| REQ-RB-004 | The rendering bridges SHALL use the same graphics contexts, fonts, and colors as C HailAlien sets up. |

### AT — Animation and Transition Integration

| ID | Requirement |
|---|---|
| REQ-AT-001 | `has_transition_anim` SHALL check the actual LOCDATA transition descriptor (NumFrames > 0), not return hardcoded false. |
| REQ-AT-002 | Animation processing during the encounter loop SHALL call `rust_ProcessCommAnimations` which delegates to the existing Rust animation engine. |
| REQ-AT-003 | The intro/transition animation sequence SHALL play when entering a conversation, matching C CommIntroTransition behavior. |

### DI — DoInput Integration

| ID | Requirement |
|---|---|
| REQ-DI-001 | The Rust encounter loop SHALL integrate with C's `DoInput` framework or provide an equivalent frame-driven input loop. |
| REQ-DI-002 | The input loop SHALL call `c_DoInput(encounter_state)` or implement equivalent per-frame dispatch: graphics batch, callback invocation, sleep-thread timing. |
| REQ-DI-003 | The loop SHALL respect `CHECK_ABORT` and `CHECK_LOAD` activity flags to exit cleanly. |
| REQ-DI-004 | Frame timing SHALL match C's `ONE_SECOND / COMM_ANIM_RATE` cadence. |

### CS — C-Side Stub Completion

| ID | Requirement |
|---|---|
| REQ-CS-001 | All `P11: Stub` markers in rust_comm.c SHALL be replaced with working implementations. |
| REQ-CS-002 | `c_FeedbackPlayerPhrase`, `c_RefreshResponses`, `c_SelectConversationSummary` SHALL reimplement the rendering logic in `rust_comm.c` by calling the same C drawing primitives (SetContext, DrawText, etc.), since the original C functions are static and compiled out under `USE_RUST_COMM`. |
| REQ-CS-003 | No `TODO`, `FIXME`, `Stub`, `placeholder`, or `for now` markers SHALL remain in production code paths after completion. |

### E2E — End-to-End Verification

| ID | Requirement |
|---|---|
| REQ-E2E-001 | With `USE_RUST_COMM=on`, entering a conversation SHALL display the alien portrait, play speech audio, show subtitles, and present response options. |
| REQ-E2E-002 | Selecting a response SHALL invoke the response callback and advance the conversation. |
| REQ-E2E-003 | The conversation SHALL complete normally with proper resource cleanup. |
| REQ-E2E-004 | All 27 race encounters SHALL work identically to C-only mode. |
| REQ-E2E-005 | Both `USE_RUST_COMM=on` and `USE_RUST_COMM=off` builds SHALL compile, link, and run correctly. |
| REQ-E2E-006 | No regression in the existing 267+ comm tests. |
