# Phase 02: Pseudocode

## Phase ID
`PLAN-20260326-COMMPT2.P02`

## Prerequisites
- Required: Phase 01a (Analysis Verification) completed
- Analysis artifacts in `01-analysis.md` are verified

## Purpose

Provide numbered, algorithmic pseudocode for all implementation phases.
Implementation phases (P03–P08) MUST reference these line ranges.

---

## Pseudocode A: Input Bridge Wiring (P03)

```text
A01: MODULE talk_segue::c_bridge
A02:   DECLARE extern "C" fn c_GetPulsedMenuKey(key_index: c_int) -> c_int
A03:
A04: CONST KEY_MENU_SELECT: c_int = 0
A05: CONST KEY_MENU_CANCEL: c_int = 3
A06: CONST KEY_MENU_UP: c_int = 1
A07: CONST KEY_MENU_DOWN: c_int = 2
A08: CONST KEY_MENU_LEFT: c_int = 4
A09: CONST KEY_MENU_RIGHT: c_int = 5
A10:
A11: FUNCTION check_select_input(state: &CommState) -> bool
A12:   #[cfg(not(test))]:
A13:     RETURN unsafe { c_bridge::c_GetPulsedMenuKey(KEY_MENU_SELECT) != 0 }
A14:   #[cfg(test)]:
A15:     RETURN state.test_input_select()
A16:
A17: FUNCTION check_cancel_input(state: &CommState) -> bool
A18:   #[cfg(not(test))]:
A19:     RETURN unsafe { c_bridge::c_GetPulsedMenuKey(KEY_MENU_CANCEL) != 0 }
A20:   #[cfg(test)]:
A21:     RETURN state.test_input_cancel()
A22:
A23: FUNCTION check_up_input(state: &CommState) -> bool
A24:   #[cfg(not(test))]:
A25:     RETURN unsafe { c_bridge::c_GetPulsedMenuKey(KEY_MENU_UP) != 0 }
A26:   #[cfg(test)]:
A27:     RETURN state.test_input_up()
A28:
A29: FUNCTION check_down_input(state: &CommState) -> bool
A30:   #[cfg(not(test))]:
A31:     RETURN unsafe { c_bridge::c_GetPulsedMenuKey(KEY_MENU_DOWN) != 0 }
A32:   #[cfg(test)]:
A33:     RETURN state.test_input_down()
A34:
A35: FUNCTION check_left_input(state: &CommState) -> bool
A36:   #[cfg(not(test))]:
A37:     RETURN unsafe { c_bridge::c_GetPulsedMenuKey(KEY_MENU_LEFT) != 0 }
A38:   #[cfg(test)]:
A39:     RETURN state.test_input_left()
A40:
A41: FUNCTION check_right_input(state: &CommState) -> bool
A42:   #[cfg(not(test))]:
A43:     RETURN unsafe { c_bridge::c_GetPulsedMenuKey(KEY_MENU_RIGHT) != 0 }
A44:   #[cfg(test)]:
A45:     RETURN state.test_input_right()
```

## Pseudocode B: Transition Animation Check (P03)

```text
B01: FUNCTION has_transition_anim(state: &CommState) -> bool
B02:   #[cfg(not(test))]:
B03:     RETURN unsafe { c_bridge::c_HasTransitionAnim() != 0 }
B04:   #[cfg(test)]:
B05:     RETURN state.animations().has_transition_anim()
```

## Pseudocode C: NPC Phrase Implementation (P04)

```text
C01: FUNCTION rust_NPCPhrase_cb(index: c_int, cb: Option<extern "C" fn()>)
C02:   ACQUIRE write lock on COMM_STATE
C03:   LET phrases = state.conversation_phrases_handle()
C04:   IF phrases is null OR index <= 0
C05:     LOG warning "invalid phrase index {index}"
C06:     RETURN
C07:   LET text_ptr = c_get_conversation_phrase(phrases, index)
C08:   IF text_ptr is null
C09:     LOG warning "phrase {index} not found"
C10:     RETURN
C11:   LET text = CStr::from_ptr(text_ptr)
C12:   LET phrase_file = resolve_phrase_file(index) // track file for this phrase
C13:   LET timestamp = resolve_phrase_timestamp(index) // timing data
C14:   CALL c_SpliceTrack(phrase_file, text_ptr, timestamp, cb)
C15:   APPEND text to conversation summary
C16:   UPDATE phrase_state with current index
C17:
C18: FUNCTION rust_NPCPhrase_splice(index: c_int)
C19:   CALL rust_NPCPhrase_cb(index, None)
```

## Pseudocode D: C Rendering Bridges (P05)

```text
D01: FUNCTION c_FeedbackPlayerPhrase(text: const char*)
D02:   // Save/restore context around rendering
D03:   LET old_context = SetContext(SpaceContext)
D04:   LET old_font = SetContextFont(PlayerFont)
D05:   // Clear the player response area
D06:   SET rect to player text area (below slider)
D07:   DrawFilledRectangle(rect) with COMM_PLAYER_BACKGROUND_COLOR
D08:   // Draw the player's phrase text
D09:   IF text is not NULL and not empty
D10:     SET text_rect position
D11:     font_DrawText(text)  // using C's text rendering
D12:   RESTORE old_font
D13:   RESTORE old_context
D14:
D15: FUNCTION c_RefreshResponses(top, num_responses, cur_response)
D16:   LET old_context = SetContext(SpaceContext)
D17:   // Clear response area
D18:   SET rect to response list area
D19:   DrawFilledRectangle(rect) with background
D20:   // Draw each visible response
D21:   FOR i = top TO top + visible_count
D22:     IF i == cur_response
D23:       SET highlight color
D24:     ELSE
D25:       SET normal color
D26:     LET response_text = get_response_text(i)
D27:     font_DrawText(response_text) at response_y[i - top]
D28:   RESTORE old_context
D29:
D30: FUNCTION c_SelectConversationSummary()
D31:   // Show conversation history overlay
D32:   LET old_context = SetContext(SpaceContext)
D33:   BatchGraphics()
D34:   // Draw summary background
D35:   DrawFilledRectangle(comm_window_rect) with summary_bg
D36:   // Draw each summary line
D37:   FOR EACH line IN conversation_summary
D38:     font_DrawText(line) at line_y
D39:   UnbatchGraphics()
D40:   RESTORE old_context
```

## Pseudocode E: Resource Bridge Wrappers (P06)

```text
E01: // C-side bridge functions in rust_comm.c
E02:
E03: FUNCTION c_LoadGraphic(res: unsigned int) -> uintptr_t
E04:   RETURN (uintptr_t) LoadGraphic((RESOURCE)res)
E05:
E06: FUNCTION c_LoadFont(res: unsigned int) -> uintptr_t
E07:   RETURN (uintptr_t) LoadFont((RESOURCE)res)
E08:
E09: FUNCTION c_LoadColorMap(res: unsigned int) -> uintptr_t
E10:   RETURN (uintptr_t) LoadColorMap((RESOURCE)res)
E11:
E12: FUNCTION c_LoadMusic(res: unsigned int) -> uintptr_t
E13:   RETURN (uintptr_t) LoadMusic((RESOURCE)res)
E14:
E15: FUNCTION c_LoadStringTable(res: unsigned int) -> uintptr_t
E16:   RETURN (uintptr_t) LoadStringTable((RESOURCE)res)
E17:
E18: FUNCTION c_CaptureDrawable(handle: uintptr_t) -> uintptr_t
E19:   RETURN (uintptr_t) CaptureDrawable((DRAWABLE)(handle))
E20:
E21: FUNCTION c_CaptureColorMap(handle: uintptr_t) -> uintptr_t
E22:   RETURN (uintptr_t) CaptureColorMap((COLORMAP)(handle))
E23:
E24: FUNCTION c_CaptureStringTable(handle: uintptr_t) -> uintptr_t
E25:   RETURN (uintptr_t) CaptureStringTable((STRING_TABLE)(handle))
E26:
E27: FUNCTION c_ReleaseDrawable(handle: uintptr_t) -> uintptr_t
E28:   RETURN (uintptr_t) ReleaseDrawable((FRAME)(handle))
E29:
E30: FUNCTION c_ReleaseColorMap(handle: uintptr_t) -> uintptr_t
E31:   RETURN (uintptr_t) ReleaseColorMap((COLORMAP)(handle))
E32:
E33: FUNCTION c_ReleaseStringTable(handle: uintptr_t) -> uintptr_t
E34:   RETURN (uintptr_t) ReleaseStringTable((STRING_TABLE)(handle))
E35:
E36: FUNCTION c_CreateContext(name: const char*) -> uintptr_t
E37:   RETURN (uintptr_t) CreateContext(name)
E38:
E39: FUNCTION c_DestroyContext(ctx: uintptr_t)
E40:   DestroyContext((CONTEXT)(ctx))
E41:
E42: FUNCTION c_SetContext(ctx: uintptr_t) -> uintptr_t
E43:   RETURN (uintptr_t) SetContext((CONTEXT)(ctx))
E44:
E45: FUNCTION c_SetContextFGFrame(frame: uintptr_t)
E46:   SetContextFGFrame((FRAME)(frame))
E47:
E48: FUNCTION c_SetContextClipRect(x, y, w, h)
E49:   RECT r = { {x, y}, {w, h} }
E50:   SetContextClipRect(&r)
E51:
E52: FUNCTION c_SetContextBackGroundColor(r, g, b)
E53:   SetContextBackGroundColor(BUILD_COLOR(MAKE_RGB15(r, g, b), 0x00))
E54:
E55: FUNCTION c_CreateDrawable(type, w, h, num_frames) -> uintptr_t
E56:   RETURN (uintptr_t) CreateDrawable(type, w, h, num_frames)
E57:
E58: FUNCTION c_SetFrameTransparentColor(frame, r, g, b)
E59:   SetFrameTransparentColor((FRAME)(frame), BUILD_COLOR(MAKE_RGB15(r, g, b), 0x00))
E60:
E61: FUNCTION c_ClearDrawable()
E62:   ClearDrawable()
E63:
E64: FUNCTION c_BatchGraphics()
E65:   BatchGraphics()
E66:
E67: FUNCTION c_UnbatchGraphics()
E68:   UnbatchGraphics()
E69:
E70: FUNCTION c_SetTransitionSource(src: uintptr_t)
E71:   SetTransitionSource((FRAME)(src))
E72:
E73: FUNCTION c_ScreenTransition(num_frames: int, rect_ptr: void*)
E74:   ScreenTransition(num_frames, (RECT*)rect_ptr)
E75:
E76: FUNCTION c_DrawSISFrame()
E77:   DrawSISFrame()
E78:
E79: FUNCTION c_DrawSISMessage(msg: const char*)
E80:   IF msg is NULL: DrawSISMessage(NULL)
E81:   ELSE: DrawSISMessage((UNICODE*)msg)
E82:
E83: FUNCTION c_DrawSISTitle(title: const char*)
E84:   DrawSISTitle((UNICODE*)title)
E85:
E86: FUNCTION c_DoInput(state_ptr: void*, exclusive: int)
E87:   DoInput((INPUT_STATE_DESC*)state_ptr, exclusive)
E88:
E89: FUNCTION c_GetFrameRect(frame: uintptr_t, x*, y*, w*, h*)
E90:   RECT r; GetFrameRect((FRAME)(frame), &r)
E91:   *x = r.corner.x; *y = r.corner.y
E92:   *w = r.extent.width; *h = r.extent.height
E93:
E94: FUNCTION c_GetScreen() -> uintptr_t
E95:   RETURN (uintptr_t) Screen
E96:
E97: FUNCTION c_SetContextFont(font: uintptr_t) -> uintptr_t
E98:   RETURN (uintptr_t) SetContextFont((FONT)(font))
E99:
E100: FUNCTION c_GetSpaceContext() -> uintptr_t
E101:   RETURN (uintptr_t) SpaceContext
E102:
E103: FUNCTION c_SetLastActivityCheckLoad()
E104:   LastActivity |= CHECK_LOAD
```

## Pseudocode F: HailAlien Orchestration (P07)

```text
F01: FUNCTION hail_alien()
F02:   // 1. Initialize encounter state
F03:   LET mut es = EncounterState::new()
F04:   es.input_func = DoCommunication
F05:   c_set_talking_finished(FALSE)
F06:   c_set_cur_input_state(&es)
F07:
F08:   // 2. Load resources
F09:   LET player_font = c_LoadFont(PLAYER_FONT)
F10:   LET alien_frame_res = c_locdata_get_alien_frame_res()
F11:   LET alien_frame_raw = c_LoadGraphic(alien_frame_res)
F12:   LET alien_frame = c_CaptureDrawable(alien_frame_raw)
F13:   c_set_comm_alien_frame(alien_frame)
F14:
F15:   LET alien_font_res = c_locdata_get_alien_font_res()
F16:   LET alien_font = c_LoadFont(alien_font_res)
F17:   c_set_comm_alien_font(alien_font)
F18:
F19:   LET alien_cmap_res = c_locdata_get_alien_colormap_res()
F20:   LET alien_cmap_raw = c_LoadColorMap(alien_cmap_res)
F21:   LET alien_cmap = c_CaptureColorMap(alien_cmap_raw)
F22:   c_set_comm_alien_colormap(alien_cmap)
F23:
F24:   // Alt song fallback
F25:   LET song_flags = c_locdata_get_alien_song_flags()
F26:   LET alt_song_res = c_locdata_get_alien_alt_song_res()
F27:   LET mut song_ref: uintptr_t = 0
F28:   IF (song_flags & LDASF_USE_ALTERNATE) != 0 AND alt_song_res != 0
F29:     song_ref = c_LoadMusic(alt_song_res)
F30:   IF song_ref == 0
F31:     LET song_res = c_locdata_get_alien_song_res()
F32:     song_ref = c_LoadMusic(song_res)
F33:   c_set_comm_alien_song(song_ref)
F34:
F35:   LET phrases_res = c_locdata_get_conversation_phrases_res()
F36:   LET phrases_raw = c_LoadStringTable(phrases_res)
F37:   LET phrases = c_CaptureStringTable(phrases_raw)
F38:   c_set_comm_conversation_phrases(phrases)
F39:
F40:   // 3. Setup subtitle text baseline/alignment from CommData
F41:   c_setup_subtitle_text_from_commdata()
F42:
F43:   // 4. Create TextCacheContext
F44:   LET text_cache_ctx = c_CreateContext("TextCacheContext")
F45:   LET text_cache_frame_raw = c_CreateDrawable(WANT_PIXMAP, SIS_SCREEN_WIDTH,
F46:                                SIS_SCREEN_HEIGHT - SLIDER_Y - SLIDER_HEIGHT + 2, 1)
F47:   LET text_cache_frame = c_CaptureDrawable(text_cache_frame_raw)
F48:   c_SetContext(text_cache_ctx)
F49:   c_SetContextFGFrame(text_cache_frame)
F50:   c_SetContextBackGroundColor(0x00, 0x00, 0x10) // TextBack color key
F51:   c_ClearDrawable()
F52:   c_SetFrameTransparentColor(text_cache_frame, 0x00, 0x00, 0x10)
F53:   c_set_text_cache_context(text_cache_ctx)
F54:   c_set_text_cache_frame(text_cache_frame)
F55:
F56:   // 5. Clear phrase buffer
F57:   c_clear_phrase_buf()
F58:
F59:   // 6. Setup SpaceContext and PlayerFont
F60:   LET space_ctx = c_GetSpaceContext()
F61:   c_SetContext(space_ctx)
F62:   LET old_font = c_SetContextFont(player_font)
F63:
F64:   // 7. Create AnimContext
F65:   LET anim_ctx = c_CreateContext("AnimContext")
F66:   c_SetContext(anim_ctx)
F67:   LET screen = c_GetScreen()
F68:   c_SetContextFGFrame(screen)
F69:   LET (fx, fy, fw, fh) = c_GetFrameRect(alien_frame)
F70:   LET comm_wnd_width = SIS_SCREEN_WIDTH
F71:   LET comm_wnd_height = fh
F72:   c_set_anim_context(anim_ctx)
F73:   c_set_comm_wnd_rect(fw, fh)
F74:
F75:   // 8. Transition and draw
F76:   c_SetTransitionSource(0) // NULL
F77:   c_BatchGraphics()
F78:
F79:   IF c_WonLastBattle() != 0
F80:     // Set clip rect to CommWndRect corner
F81:     c_SetContextClipRect(comm_wnd_x, comm_wnd_y, comm_wnd_width, comm_wnd_height)
F82:   ELSE
F83:     c_SetContextClipRect(SIS_ORG_X, SIS_ORG_Y, comm_wnd_width, comm_wnd_height)
F84:     c_DrawSISFrame()
F85:     // Starbase check
F86:     IF c_is_starbase_conversation()
F87:       c_DrawSISMessage(starbase_commander_text)
F88:       c_DrawSISTitle(starbase_text)
F89:     ELSE
F90:       c_DrawSISMessage(NULL)
F91:       c_DrawSISTitle(c_get_planet_name())
F92:
F93:   c_DrawSISComWindow()
F94:
F95:   // 9. Prevent spurious input
F96:   c_SetLastActivityCheckLoad()
F97:
F98:   // 10. Run encounter
F99:   c_call_init_encounter_func()
F100:  c_DoInput(&es, FALSE)
F101:
F102:  // 11. Post-encounter
F103:  IF NOT c_CheckAbort() AND NOT c_CheckLoad()
F104:    c_call_post_encounter_func()
F105:  c_call_uninit_encounter_func()
F106:
F107:  // 12. Restore context
F108:  c_SetContext(space_ctx)
F109:  c_SetContextFont(old_font)
F110:
F111:  // 13. Cleanup (reverse order)
F112:  c_DestroyStringTable(phrases)
F113:  c_DestroyMusic(song_ref)
F114:  c_DestroyColorMap(alien_cmap)
F115:  c_DestroyFont(alien_font)
F116:  c_DestroyDrawable(alien_frame)
F117:  c_DestroyContext(text_cache_ctx)
F118:  c_DestroyDrawable(text_cache_frame)
F119:  c_DestroyFont(player_font)
F120:
F121:  // 14. Clear CommData fields
F122:  c_clear_comm_conversation_phrases_res()
F123:  c_clear_comm_conversation_phrases()
F124:  c_set_cur_input_state(null)
```

## Pseudocode G: Integration Sweep (P08)

```text
G01: // Deferred implementation detection
G02: RUN grep -RIn "TODO|FIXME|HACK|placeholder|for now|will be implemented|P11: Stub" rust/src/comm/
G03: RUN grep -RIn "TODO|FIXME|HACK|placeholder|for now|P11: Stub" sc2/src/uqm/rust_comm.c
G04: FOR EACH match
G05:   IF in production code (not test, not comment-only)
G06:     FAIL — must be resolved
G07:
G08: // Test verification
G09: RUN cargo test --workspace --all-features
G10: ASSERT all 267+ comm tests pass
G11: ASSERT no new test failures
G12:
G13: // Build verification
G14: RUN build with USE_RUST_COMM=on
G15: RUN build with USE_RUST_COMM=off
G16: ASSERT both compile and link
G17: ASSERT no duplicate symbols
G18: ASSERT no undefined symbols
```
