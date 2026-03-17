# Phase 10: Response UI & Speech Graphics

## Phase ID
`PLAN-20260314-COMM.P10`

## Prerequisites
- Required: Phase 09a completed
- Expected: Talk segue, main loop, response system, oscilloscope model, and minimum response/subtitle/summary-entry display seams already functional from P09

## Requirements Implemented (Expanded)

### RS-REQ-006–009: Response rendering
**Requirement text**: Render available responses in comm window, selected response distinct, scroll indicators for overflow, up/down cycles selection.

Behavior contract:
- GIVEN: 5 responses registered, visible area fits 3
- WHEN: Selection is on response 4
- THEN: Responses 3-5 visible, down arrow shown, response 4 highlighted, others dimmed

### AO-REQ-011–015: Speech visualization
**Requirement text**: Initialize oscilloscope and playback slider. Update oscilloscope with audio samples during playback. Slider shows play/seek/stop states. Clear on stop/exit.

### SS-REQ-007–008: Subtitle display rendering
**Requirement text**: Poll subtitle, update display when changed, clear stale text.

### RS-REQ-015, SS-REQ-013–016: Conversation summary
**Requirement text**: Cancel during response selection opens summary. Navigate pages. Carry text across page boundaries.

## Implementation Tasks

### Phase boundary for this file

P10 no longer owns the first implementation of main-loop-visible feedback/summary-entry/subtitle behavior. Those minimum seams land in P09 because the main loop depends on them. P10 completes the rendering stack by adding:

- full response-list drawing and overflow indicators
- polished feedback/response rendering alignment with the C implementation
- oscilloscope rendering fidelity and full slider icon rendering
- full subtitle rendering/cache behavior
- full summary-page rendering, pagination, and navigation

### Files to create

- `rust/src/comm/speech_graphics.rs` — Oscilloscope and slider rendering
  - marker: `@plan PLAN-20260314-COMM.P10`
  - marker: `@requirement AO-REQ-011 through AO-REQ-015`

  **Structures:**

  - `SpeechGraphics` — manages oscilloscope frame, slider icon states
    - `init()` — create oscilloscope display frame, load slider icons
    - `update_oscilloscope(osc: &Oscilloscope)` — render waveform into oscilloscope region
    - `update_slider(state: SliderState)` — show play/stop/seek/rewind icon
    - `clear()` — reset to idle

  **Rendering approach:**
  - Oscilloscope: draw vertical lines at each x, height = osc.get_y(x)
  - Slider: stamp appropriate icon at slider position
  - Uses graphics subsystem FFI for drawing (SetContextForeGroundColor, DrawLine, etc.)

- `rust/src/comm/response_ui.rs` — Response rendering and scrolling
  - marker: `@plan PLAN-20260314-COMM.P10`
  - marker: `@requirement RS-REQ-006 through RS-REQ-009`

  **Functions:**

  - `draw_responses(responses: &ResponseSystem, top: usize, player_font: u32, window: &Rect)`
    - Draw each visible response text with player font
    - Selected response: foreground color (highlight)
    - Other responses: dimmed color
    - If top > 0: draw up arrow indicator
    - If more responses below visible area: draw down arrow indicator

  - `calculate_visible_range(response_count: usize, selected: usize, visible_count: usize) -> (usize, usize)`
    - Returns (top, bottom) range for visible responses
    - Scrolls to keep selected response visible

  - `draw_feedback_text(text: &str, player_font: u32, window: &Rect)`
    - Render the player's chosen response text in comm window

- `rust/src/comm/subtitle_display.rs` — Subtitle rendering
  - marker: `@plan PLAN-20260314-COMM.P10`
  - marker: `@requirement SS-REQ-007, SS-REQ-008, AO-REQ-010`

  **Functions:**

  - `update_subtitle_display(current_text: Option<&str>, prev_text: Option<&str>, comm_data: &CommData)`
    - If text changed: mark for redraw
    - Clear previous subtitle area
    - Render new text with alien font, foreground/background colors, baseline, alignment from CommData
    - Use text-cache context (offscreen pixmap) for overlay rendering

  - `clear_subtitle_display()`
    - Clear text-cache context, mark subtitle area for redraw

### Files to modify

- `rust/src/comm/summary.rs`
  - Extend the P06 summary model/pagination module with summary-view rendering/input behavior
  - marker: `@plan PLAN-20260314-COMM.P10`
  - marker: `@requirement RS-REQ-015, SS-REQ-013 through SS-REQ-016`

  **Functions added in this phase:**

  - `show_conversation_summary(window: &Rect) -> CommResult<SummaryResult>`
    - Enumerate subtitle history via the existing P06 trackplayer-backed summary model
    - Paginate text with word wrapping to window width
    - Carry overflow text across page boundaries (SS-REQ-015)
    - Navigation: Select/Cancel/Right advances page; end of pages exits
    - Returns to response selection

  - `render_summary_page(page: &Page, window: &Rect)`
    - Draw page contents using comm-window rendering conventions

- `rust/src/comm/mod.rs`
  - Add `pub mod speech_graphics;`
  - Add `pub mod response_ui;`
  - Add `pub mod subtitle_display;`
  - Ensure existing `pub mod summary;` remains the single summary module boundary
  - marker: `@plan PLAN-20260314-COMM.P10`

- `rust/src/comm/state.rs`
  - Add `speech_graphics: SpeechGraphics` field
  - Add `prev_subtitle: Option<String>` for change detection
  - Add `top_response: usize` for scroll position
  - marker: `@plan PLAN-20260314-COMM.P10`

- `rust/src/comm/ffi.rs`
  - Add `rust_DrawResponses()` export
  - Add `rust_UpdateSpeechGraphics()` export
  - Add `rust_UpdateSubtitleDisplay()` export
  - Add `rust_ShowConversationSummary() -> c_int` export
  - marker: `@plan PLAN-20260314-COMM.P10`

### External C functions needed via FFI

```rust
extern "C" {
    // Drawing
    fn c_SetContextForeGroundColor(color: u32);
    fn c_SetContextFont(font: u32);
    fn c_DrawText(text: *const c_char, x: i32, y: i32);
    fn c_DrawLine(x1: i32, y1: i32, x2: i32, y2: i32);
    fn c_DrawFilledRect(x: i32, y: i32, w: i32, h: i32);
    fn c_SetContext(ctx: u32);
    fn c_GetSISCommWindow() -> Rect;
}
```

### Concrete seam ownership and source-path mapping

- response-rendering behavior is anchored to existing C implementations in `sc2/src/uqm/comm.c`:
  - `RefreshResponses()` for list drawing, top-of-window scrolling, and arrow indicators
  - `FeedbackPlayerPhrase()` for feedback text presentation
- subtitle-rendering behavior is anchored to existing C implementations in `sc2/src/uqm/comm.c`:
  - `ClearSubtitles()`
  - `CheckSubtitles()`
  - `RedrawSubtitles()`
  - subtitle-cache context creation within `HailAlien()`
- speech-graphics behavior is anchored to existing C implementations in `sc2/src/uqm/comm.c` and rendering support modules already used there:
  - `InitSpeechGraphics()`
  - `UpdateSpeechGraphics()`
  - oscilloscope drawing in `RadarContext`
- animation-processing linkage remains anchored to `sc2/src/uqm/commanim.c` (`ProcessCommAnimations`) for parity checks while Rust rendering takes ownership in Rust mode

### Module-boundary note

The plan now uses a single summary module boundary:
- `summary.rs` owns summary data access through trackplayer enumeration, pagination, and summary-view behavior.
- P06 establishes the model/enumeration seam.
- P09 establishes summary entry from the main loop.
- P10 adds rendering and full summary-view behavior on top of that same module.

No separate `conversation_summary.rs` module is introduced.

### Intermediate build invariants (must hold after P10/P10a)

- Rust is authoritative for response rendering, subtitle rendering, speech graphics, and summary rendering in Rust mode.
- C fallback mode remains untouched except for bridge wrappers/guards.
- Mixed mode remains valid only if these rendering modules improve completeness/polish without changing the loop/callback ownership established in P09.

### Pseudocode traceability
- Uses pseudocode lines: 270-300 (response UI and conversation summary)

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] `speech_graphics.rs` created with oscilloscope and slider rendering
- [ ] `response_ui.rs` created with response drawing and scrolling
- [ ] `subtitle_display.rs` created with text cache rendering
- [ ] `summary.rs` extended with summary-view pagination and navigation
- [ ] All modules registered in `mod.rs`
- [ ] FFI exports for all rendering functions
- [ ] Graphics FFI extern declarations present
- [ ] Every wrapper or parity seam in this phase names its concrete C source file/path

## Semantic Verification Checklist (Mandatory)
- [ ] Test: response rendering highlights selected, dims others
- [ ] Test: scroll indicators appear when responses overflow
- [ ] Test: scroll range tracks selection (keeps selected visible)
- [ ] Test: oscilloscope rendering produces valid pixel values
- [ ] Test: slider state changes icon correctly
- [ ] Test: subtitle change detection works (same text → no redraw)
- [ ] Test: subtitle clear removes stale text
- [ ] Test: summary pagination respects line width
- [ ] Test: summary carries overflow across pages (SS-REQ-015)
- [ ] Test: summary shows all history entries in order
- [ ] Test: summary navigation (advance page, exit at end)
- [ ] Test: summary disabled during final battle (CV-REQ-006)
- [ ] Test: P10 refines rather than introduces the P09-visible feedback/subtitle/summary-entry behaviors

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/comm/speech_graphics.rs rust/src/comm/response_ui.rs rust/src/comm/subtitle_display.rs rust/src/comm/summary.rs
```

## Success Criteria
- [ ] All rendering functions produce correct output
- [ ] UI matches C implementation behavior
- [ ] Summary ownership boundary is coherent (single module)
- [ ] P10 completes polish/completeness without hiding unresolved P09 loop dependencies
- [ ] All tests pass

## Failure Recovery
- rollback: `git restore rust/src/comm/speech_graphics.rs rust/src/comm/response_ui.rs rust/src/comm/subtitle_display.rs rust/src/comm/summary.rs`
- blocking: Graphics FFI wrappers must exist

## Phase Completion Marker
Create: `project-plans/20260311/comm/.completed/P10.md`
