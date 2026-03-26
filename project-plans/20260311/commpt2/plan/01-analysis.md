# Phase 01: Analysis

## Phase ID
`PLAN-20260326-COMMPT2.P01`

## Prerequisites
- Required: Phase 00.5 (Preflight Verification) completed
- All existing comm tests pass
- Both build modes compile

## Purpose

Map every stub, gap, and deferred implementation to its resolution. Trace the
complete call paths. Document resource lifecycles. Identify all integration
touchpoints that must be wired.

---

## 1. Stub Inventory

### 1.1 FFI Stubs (Rust → C, 4 stubs in `rust/src/comm/ffi.rs`)

| Stub | Location | Current Behavior | Resolution |
|------|----------|-----------------|------------|
| `rust_HailAlien()` | ffi.rs:869–875 | Empty body (no-op) | P07: Full encounter orchestration in new `hail.rs` |
| `rust_NPCPhrase_cb(index, cb)` | ffi.rs:882–886 | Discards args (no-op) | P04: Resolve phrase, splice track, register callback |
| `rust_NPCPhrase_splice(index)` | ffi.rs:890–893 | Discards arg (no-op) | P04: Resolve phrase, splice track (no callback) |
| `rust_ProcessCommAnimations` | ffi.rs (if exists) | May delegate to C | P07: Verify wired in encounter loop |

### 1.2 Input Stubs (6 functions in `rust/src/comm/talk_segue.rs`)

| Stub | Location | Current Behavior | Resolution |
|------|----------|-----------------|------------|
| `check_select_input()` | talk_segue.rs:504–515 | Returns `false` (non-test) | P03: Call `c_GetPulsedMenuKey(0)` |
| `check_cancel_input()` | talk_segue.rs:487–502 | Returns `false` (non-test) | P03: Call `c_GetPulsedMenuKey(3)` |
| `check_up_input()` | talk_segue.rs:543–554 | Returns `false` (non-test) | P03: Call `c_GetPulsedMenuKey(1)` |
| `check_down_input()` | talk_segue.rs:556–567 | Returns `false` (non-test) | P03: Call `c_GetPulsedMenuKey(2)` |
| `check_left_input()` | talk_segue.rs:517–528 | Returns `false` (non-test) | P03: Call `c_GetPulsedMenuKey(4)` |
| `check_right_input()` | talk_segue.rs:530–541 | Returns `false` (non-test) | P03: Call `c_GetPulsedMenuKey(5)` |

### 1.3 C Rendering Stubs (3 stubs in `sc2/src/uqm/rust_comm.c`)

| Stub | Location | Current Behavior | Resolution |
|------|----------|-----------------|------------|
| `c_FeedbackPlayerPhrase(text)` | rust_comm.c:665–670 | Discards text (no-op) | P05: Call C text rendering |
| `c_RefreshResponses(top, num, cur)` | rust_comm.c:673–681 | Discards args (no-op) | P05: Call C response list rendering |
| `c_SelectConversationSummary()` | rust_comm.c:684–687 | No-op | P05: Call C summary overlay |

### 1.4 Transition Animation Stub (1 in `rust/src/comm/talk_segue.rs`)

| Stub | Location | Current Behavior | Resolution |
|------|----------|-----------------|------------|
| `has_transition_anim()` | talk_segue.rs:828–839 | Returns `false` (non-test) | P03: Call `c_HasTransitionAnim()` or check LOCDATA |

---

## 2. Call-Path Traces

### 2.1 HailAlien Flow (C → Rust → C round-trip)

```
C: InitCommunication()
  └─ if (status == HAIL)
       └─ #ifdef USE_RUST_COMM → rust_HailAlien()    [comm.c:1458]
            └─ Rust: hail_alien()                      [NEW: hail.rs]
                 │
                 ├─ 1. Init encounter state
                 │     └─ CommState::init_encounter()
                 │
                 ├─ 2. Load resources (7 C bridge calls)
                 │     ├─ c_LoadFont(PLAYER_FONT)
                 │     ├─ c_CaptureDrawable(c_LoadGraphic(AlienFrameRes))
                 │     ├─ c_LoadFont(AlienFontRes)
                 │     ├─ c_CaptureColorMap(c_LoadColorMap(AlienColorMapRes))
                 │     ├─ c_LoadMusic(AlienAltSongRes or AlienSongRes)
                 │     └─ c_CaptureStringTable(c_LoadStringTable(ConversationPhrasesRes))
                 │
                 ├─ 3. Setup TextCacheContext
                 │     ├─ c_CreateContext("TextCacheContext")
                 │     ├─ c_CaptureDrawable(c_CreateDrawable(...))
                 │     ├─ c_SetContext(TextCacheContext)
                 │     ├─ c_SetContextFGFrame(TextCacheFrame)
                 │     ├─ c_SetContextBackGroundColor(TextBack)
                 │     ├─ c_ClearDrawable()
                 │     └─ c_SetFrameTransparentColor(TextCacheFrame, TextBack)
                 │
                 ├─ 4. Setup AnimContext
                 │     ├─ c_CreateContext("AnimContext")
                 │     ├─ c_SetContext(AnimContext)
                 │     ├─ c_SetContextFGFrame(Screen)
                 │     ├─ c_GetFrameRect(AlienFrame) → CommWndRect
                 │     ├─ c_SetTransitionSource(NULL)
                 │     └─ c_BatchGraphics()
                 │
                 ├─ 5. Draw SIS UI
                 │     ├─ if WON_LAST_BATTLE: c_SetContextClipRect(r)
                 │     └─ else:
                 │          ├─ c_SetContextClipRect(SIS_ORG)
                 │          ├─ c_DrawSISFrame()
                 │          ├─ c_DrawSISMessage(msg)
                 │          ├─ c_DrawSISTitle(title)
                 │          └─ c_DrawSISComWindow()
                 │
                 ├─ 6. Set CHECK_LOAD flag
                 │     └─ c_SetLastActivityCheckLoad()
                 │
                 ├─ 7. Call init_encounter_func
                 │     └─ c_call_init_encounter_func()
                 │
                 ├─ 8. Run encounter loop
                 │     └─ c_DoInput(&ES, FALSE)
                 │          └─ per-frame:
                 │               ├─ DoCommunication callback
                 │               │    ├─ check_*_input() → c_GetPulsedMenuKey()
                 │               │    ├─ talk_segue() / alien_talk_segue()
                 │               │    ├─ player_response_input()
                 │               │    └─ c_UpdateAnimations()
                 │               └─ SleepThread timing
                 │
                 ├─ 9. Post-encounter
                 │     ├─ if not aborted: c_call_post_encounter_func()
                 │     └─ c_call_uninit_encounter_func()
                 │
                 └─ 10. Cleanup (reverse order)
                       ├─ c_DestroyStringTable(phrases)
                       ├─ c_DestroyMusic(song)
                       ├─ c_DestroyColorMap(colormap)
                       ├─ c_DestroyFont(alien_font)
                       ├─ c_DestroyDrawable(alien_frame)
                       ├─ c_DestroyContext(TextCacheContext)
                       ├─ c_DestroyDrawable(text_cache_frame)
                       ├─ c_DestroyFont(player_font)
                       └─ Clear CommData fields
```

### 2.2 Input Bridge Call Path

```
Rust: DoCommunication (via c_DoInput callback)
  └─ player_response_input(&mut state)
       ├─ check_select_input(&state)
       │    └─ #[cfg(not(test))]: c_GetPulsedMenuKey(KEY_MENU_SELECT=0)
       ├─ check_cancel_input(&state)
       │    └─ #[cfg(not(test))]: c_GetPulsedMenuKey(KEY_MENU_CANCEL=3)
       ├─ check_up_input(&state)
       │    └─ #[cfg(not(test))]: c_GetPulsedMenuKey(KEY_MENU_UP=1)
       ├─ check_down_input(&state)
       │    └─ #[cfg(not(test))]: c_GetPulsedMenuKey(KEY_MENU_DOWN=2)
       ├─ check_left_input(&state)
       │    └─ #[cfg(not(test))]: c_GetPulsedMenuKey(KEY_MENU_LEFT=4)
       └─ check_right_input(&state)
            └─ #[cfg(not(test))]: c_GetPulsedMenuKey(KEY_MENU_RIGHT=5)
```

### 2.3 NPCPhrase Call Path

```
C: race_script_func (e.g., arilou_comm.c)
  └─ NPCPhrase(phrase_index)
       └─ #ifdef USE_RUST_COMM:
            rust_NPCPhrase_cb(index, NULL)   [commglue.c guard]
            └─ Rust ffi.rs:
                 ├─ c_get_conversation_phrase(phrases, index) → text
                 ├─ c_SpliceTrack(file, text, timestamp, cb)
                 └─ update conversation summary
```

### 2.4 Rendering Bridge Call Path

```
Rust: player selects response
  └─ feedback_player_phrase(text)
       └─ c_FeedbackPlayerPhrase(text)          [Rust FFI → C]
            └─ C: render text in subtitle area   [P05 implementation]

Rust: response list changes
  └─ refresh_responses(top, num, cur)
       └─ c_RefreshResponses(top, num, cur)      [Rust FFI → C]
            └─ C: render response list            [P05 implementation]

Rust: user presses cancel for summary
  └─ select_conversation_summary()
       └─ c_SelectConversationSummary()           [Rust FFI → C]
            └─ C: display summary overlay         [P05 implementation]
```

---

## 3. Resource Lifecycle Map

| Resource | Variable | Load Call | Store In | Free Call | When |
|----------|----------|-----------|----------|-----------|------|
| Player font | `PlayerFont` | `LoadFont(PLAYER_FONT)` | Local | `DestroyFont(PlayerFont)` | Cleanup |
| Alien frame | `CommData.AlienFrame` | `CaptureDrawable(LoadGraphic(res))` | CommData | `DestroyDrawable(ReleaseDrawable(frame))` | Cleanup |
| Alien font | `CommData.AlienFont` | `LoadFont(res)` | CommData | `DestroyFont(font)` | Cleanup |
| Alien colormap | `CommData.AlienColorMap` | `CaptureColorMap(LoadColorMap(res))` | CommData | `DestroyColorMap(ReleaseColorMap(cmap))` | Cleanup |
| Alien song | `CommData.AlienSong` | `LoadMusic(res)` + alt fallback | CommData | `DestroyMusic(song)` | Cleanup |
| Phrases | `CommData.ConversationPhrases` | `CaptureStringTable(LoadStringTable(res))` | CommData | `DestroyStringTable(ReleaseStringTable(table))` | Cleanup |
| Text cache frame | `TextCacheFrame` | `CaptureDrawable(CreateDrawable(...))` | Local/global | `DestroyDrawable(ReleaseDrawable(frame))` | Cleanup |
| Text cache ctx | `TextCacheContext` | `CreateContext("TextCacheContext")` | Local/global | `DestroyContext(ctx)` | Cleanup |
| Anim ctx | `AnimContext` | `CreateContext("AnimContext")` | Local/global | Implicit (set back to SpaceContext) | Cleanup |

**Critical invariant**: All resources MUST be freed on every exit path (normal, abort, load).
The C code frees them unconditionally after the DoInput loop returns. Rust must match this.

---

## 4. Integration Touchpoints

### 4.1 Existing Callers (C → Rust)

| Caller | File:Line | Calls | Guard |
|--------|-----------|-------|-------|
| `InitCommunication` | comm.c:1458 | `rust_HailAlien()` | `#ifdef USE_RUST_COMM` |
| `NPCPhrase_cb` | commglue.c | `rust_NPCPhrase_cb()` | `#ifdef USE_RUST_COMM` |
| `NPCPhrase` | commglue.c | `rust_NPCPhrase_splice()` | `#ifdef USE_RUST_COMM` |

### 4.2 Existing Callers (Rust → C)

| Caller | File | Calls |
|--------|------|-------|
| `talk_segue.rs` | talk_segue.rs c_bridge | 25+ C bridge functions |
| `ffi.rs` | ffi.rs | Various C bridge wrappers |

### 4.3 New Callers (to be added)

| Caller | File | Will Call |
|--------|------|----------|
| `hail_alien()` | hail.rs (NEW) | `c_LoadGraphic`, `c_LoadFont`, `c_CreateContext`, `c_DrawSISFrame`, `c_DoInput`, etc. |
| `check_*_input()` | talk_segue.rs | `c_GetPulsedMenuKey()` |
| `rust_NPCPhrase_cb()` | ffi.rs | `c_get_conversation_phrase()`, `c_SpliceTrack()` |

### 4.4 Old Behavior Replaced

| What | Where | Old Behavior | New Behavior |
|------|-------|-------------|--------------|
| `rust_HailAlien` | ffi.rs:869 | No-op (skips conversation) | Full encounter loop |
| `check_*_input` | talk_segue.rs:487–567 | Return false | Poll real input |
| `rust_NPCPhrase_cb` | ffi.rs:882 | Discard args | Resolve + splice |
| `c_FeedbackPlayerPhrase` | rust_comm.c:665 | No-op | Render text |
| `c_RefreshResponses` | rust_comm.c:673 | No-op | Render list |
| `c_SelectConversationSummary` | rust_comm.c:684 | No-op | Show overlay |
| `has_transition_anim` | talk_segue.rs:828 | Return false | Check LOCDATA |

---

## 5. Requirements Coverage Matrix

| Requirement | Covered By Phase |
|-------------|-----------------|
| REQ-HL-001 through REQ-HL-007 | P07 (HailAlien) |
| REQ-IP-001 through REQ-IP-008 | P03 (Input Bridge) |
| REQ-NP-001 through REQ-NP-004 | P04 (NPC Phrase) |
| REQ-RB-001 through REQ-RB-004 | P05 (C Rendering Bridges) |
| REQ-AT-001 | P03 (has_transition_anim) |
| REQ-AT-002, REQ-AT-003 | P07 (HailAlien encounter loop) |
| REQ-DI-001 through REQ-DI-004 | P07 (HailAlien DoInput integration) |
| REQ-CS-001 through REQ-CS-003 | P05 (C stubs) + P08 (sweep) |
| REQ-E2E-001 through REQ-E2E-006 | P08 (Integration sweep) |
