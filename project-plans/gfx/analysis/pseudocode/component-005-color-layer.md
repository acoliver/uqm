# Pseudocode: Component 005 — ColorLayer

Plan ID: `PLAN-20260223-GFX-FULL-PORT`
Requirements: REQ-CLR-010..070, REQ-CLR-055, REQ-FFI-010

---

## 005A: rust_gfx_color

```
 1: FUNCTION rust_gfx_color(r, g, b, a, rect)
 2:   state ← get_gfx_state()
 3:   IF state IS None THEN RETURN                          // REQ-CLR-060
 4:
 5:   // --- Rect validation ---
 6:   IF rect IS NOT null THEN
 7:     // SAFETY: null checked, valid per REQ-ASM-040 — REQ-FFI-010
 8:     r_val ← unsafe { &*rect }
 9:     IF r_val.w < 0 OR r_val.h < 0 THEN RETURN          // REQ-CLR-055
10:   END IF
11:
12:   // --- Set blend mode ---
13:   // NOTE: blend mode MUST be set BEFORE draw color
14:   IF a == 255 THEN
15:     state.canvas.set_blend_mode(BlendMode::None)        // REQ-CLR-020
16:   ELSE
17:     state.canvas.set_blend_mode(BlendMode::Blend)       // REQ-CLR-030
18:   END IF
19:
20:   // --- Set draw color ---
 21:   state.canvas.set_draw_color(Color::RGBA(r, g, b, a)) // REQ-CLR-010
 22:
 23:   // --- Fill ---
 24:   LET fill_result ← IF rect IS null THEN
 25:     state.canvas.fill_rect(None)                        // REQ-CLR-040
 26:   ELSE
 27:     r_val ← unsafe { &*rect }
 28:     sdl2_rect ← Rect::new(r_val.x, r_val.y, r_val.w AS u32, r_val.h AS u32)
 29:     state.canvas.fill_rect(Some(sdl2_rect))             // REQ-CLR-050
 30:   END IF
 31:
 32:   IF fill_result fails THEN
 33:     log_once("canvas.fill_rect failed in color layer")   // REQ-ERR-060
 34:     RETURN
 35:   END IF
 36: END FUNCTION
```

### Validation Points
- Line 3: Uninitialized guard (REQ-CLR-060)
- Line 9: Negative rect dimension check (REQ-CLR-055)

### Error Handling
- Uninitialized: silent return
- Negative rect: silent return
- fill_rect failure: log_once and return (REQ-ERR-060)

### Ordering Constraints
- blend_mode MUST be set before draw_color (line 14–18 before 21)
- draw_color MUST be set before fill_rect (line 21 before 24–30)
- Per requirements note: set blend mode → set draw color → fill rectangle

### Integration Boundaries
- Called by C `Rust_ColorLayer` wrapper → `TFB_SwapBuffers` step 4
- Only called when fade_amount != 255
- fade_amount < 255: color(0,0,0, 255-fade_amount, NULL) — fade to black
- fade_amount > 255: color(255,255,255, fade_amount-255, NULL) — fade to white
- a ranges 0–255 (REQ-CLR-070, no clamping needed)

### Side Effects
- Modifies renderer target (fills a colored rectangle)
- Renderer blend mode changed
- Renderer draw color changed
