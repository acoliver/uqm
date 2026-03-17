# Phase 10a: Response UI & Speech Graphics Verification

## Phase ID
`PLAN-20260314-COMM.P10a`

## Prerequisites
- Required: Phase 10 completed

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] `speech_graphics.rs`, `response_ui.rs`, `subtitle_display.rs`, `summary.rs` all exist
- [ ] All registered in `mod.rs`
- [ ] FFI exports present for rendering operations
- [ ] CommState has speech_graphics, prev_subtitle, top_response fields

## Semantic Verification Checklist

### Response Rendering
- [ ] `test_draw_responses_highlight` — selected response distinct from others
- [ ] `test_draw_responses_scroll_up` — up arrow when top > 0
- [ ] `test_draw_responses_scroll_down` — down arrow when more below
- [ ] `test_visible_range_tracks_selection` — selected always in visible range
- [ ] `test_feedback_text_display` — chosen text rendered after selection

### Oscilloscope & Slider
- [ ] `test_oscilloscope_render` — waveform lines within display bounds
- [ ] `test_slider_play_state` — play icon during playback
- [ ] `test_slider_stop_state` — stop icon after playback ends
- [ ] `test_slider_seek_states` — correct icon during seek operations
- [ ] `test_speech_graphics_clear` — idle state after clear

### Subtitle Display
- [ ] `test_subtitle_change_redraws` — new text triggers redraw
- [ ] `test_subtitle_same_no_redraw` — same text skips redraw
- [ ] `test_subtitle_clear_removes` — clear removes stale text
- [ ] `test_subtitle_alignment` — respects CommData alignment settings
- [ ] `test_subtitle_color` — uses CommData foreground/background colors

### Conversation Summary
- [ ] `test_summary_all_entries` — all history entries present
- [ ] `test_summary_queue_order` — matches queue order
- [ ] `test_summary_pagination` — word wrap at width
- [ ] `test_summary_overflow_carry` — text carries across pages
- [ ] `test_summary_navigate_forward` — Select/Cancel/Right advances
- [ ] `test_summary_exit_end` — exits when no more pages
- [ ] `test_summary_final_battle_disabled` — blocked during final battle

## Deferred Implementation Detection

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/comm/speech_graphics.rs rust/src/comm/response_ui.rs rust/src/comm/subtitle_display.rs rust/src/comm/summary.rs
```

## Phase Completion Marker
Create: `project-plans/20260311/comm/.completed/P10a.md`
