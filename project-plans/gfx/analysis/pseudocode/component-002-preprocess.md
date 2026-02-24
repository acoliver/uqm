# Pseudocode: Component 002 — Preprocess

Plan ID: `PLAN-20260223-GFX-FULL-PORT`
Requirements: REQ-PRE-010..050

---

## 002A: rust_gfx_preprocess

```
 1: FUNCTION rust_gfx_preprocess(force_redraw, transition_amount, fade_amount)
 2:   state ← get_gfx_state()
 3:   IF state IS None THEN
 4:     RETURN                                              // REQ-PRE-050
 5:   END IF
 6:
 7:   // Parameters are informational only — REQ-PRE-030, REQ-PRE-040
 8:   // force_redraw, transition_amount, fade_amount unused
 9:
10:   state.canvas.set_blend_mode(BlendMode::None)            // REQ-PRE-010
11:   state.canvas.set_draw_color(Color::RGBA(0, 0, 0, 255))// REQ-PRE-020
12:   state.canvas.clear()                                   // REQ-PRE-020, REQ-PRE-030
15: END FUNCTION
```

### Validation Points
- Line 3–5: Uninitialized guard (REQ-PRE-050)

### Error Handling
- None beyond the initialization guard — clear cannot fail

### Ordering Constraints
- blend_mode MUST be set before clear (line 12 before 14) — REQ-PRE-010

### Integration Boundaries
- Called by C `Rust_Preprocess` wrapper → `TFB_SwapBuffers` step 1
- No surface access, no texture creation
- Establishes clean renderer state for subsequent ScreenLayer/ColorLayer

### Side Effects
- Renderer target is cleared to black
- Renderer blend mode is set to None
