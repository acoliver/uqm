# Pseudocode: Component 003 — ScreenLayer (Unscaled Path)

Plan ID: `PLAN-20260223-GFX-FULL-PORT`
Requirements: REQ-SCR-010..170, REQ-NP-010, REQ-NP-020, REQ-NP-025,
              REQ-FFI-010, REQ-FFI-020, REQ-FFI-050

---

## 003A: rust_gfx_screen (Unscaled Path)

```
 1: FUNCTION rust_gfx_screen(screen, alpha, rect)
 2:   // --- Validation ---
 3:   state ← get_gfx_state()
 4:   IF state IS None THEN RETURN                          // REQ-SCR-140
 5:
 6:   IF screen < 0 OR screen >= TFB_GFX_NUMSCREENS THEN
 7:     RETURN                                              // REQ-SCR-100
 8:   END IF
 9:
10:   IF screen == 1 THEN RETURN                            // REQ-SCR-090 (EXTRA not compositable)
11:
12:   src_surface ← state.surfaces[screen]
13:   IF src_surface IS null THEN RETURN                    // REQ-SCR-110
14:
15:   // --- Rect validation ---
16:   sdl2_rect ← convert_rect(rect)                        // REQ-FFI-010
17:   IF rect IS NOT null THEN
18:     // SAFETY: null checked on line 17, valid per REQ-ASM-040
19:     r ← unsafe { &*rect }
20:     IF r.w < 0 OR r.h < 0 THEN RETURN                  // REQ-SCR-160
21:   END IF
22:
23:   // --- Surface validation ---
24:   surf ← unsafe { &*src_surface }                       // REQ-FFI-020
25:   // SAFETY: surface created by rust_gfx_init, not freed
26:   IF surf.pixels IS null OR surf.pitch <= 0 THEN
27:     RETURN                                              // REQ-SCR-120
28:   END IF
29:
30:   // --- Pitch/size validation ---                        // REQ-SCR-165
31:   pitch ← surf.pitch AS usize
32:   VALIDATE pitch >= SCREEN_WIDTH * 4                     // minimum stride for 32bpp
33:   IF pitch < SCREEN_WIDTH * 4 THEN RETURN
34:
35:   LET pixel_len ← pitch * SCREEN_HEIGHT
36:   VALIDATE pixel_len > 0
37:   IF pixel_len == 0 THEN RETURN
38:
39:   // SAFETY: from_raw_parts requires len <= isize::MAX
40:   VALIDATE pixel_len <= isize::MAX AS usize
41:   IF pixel_len > isize::MAX AS usize THEN RETURN
42:
43:   // --- Determine scaling path ---
44:   use_soft_scaler ← state.scaled_buffers[screen].is_some()
45:   IF use_soft_scaler THEN
46:     CALL screen_layer_scaled(state, screen, alpha, sdl2_rect, surf, pitch)
47:     RETURN                                              // Scaling handled separately (component-004)
48:   END IF
49:
50:   // --- Unscaled path ---
51:   // REQ-SCR-070: Create temporary streaming texture
52:   texture_creator ← state.canvas.texture_creator()
53:   texture ← texture_creator.create_texture_streaming(
54:               RGBX8888, SCREEN_WIDTH, SCREEN_HEIGHT)
55:   IF texture IS Err THEN RETURN                         // REQ-SCR-130
56:
57:   // --- Upload pixel data ---
58:   // REQ-SCR-170, REQ-SCR-075: use pitch as stride
59:   pixel_data ← unsafe {
60:     // SAFETY: (a) surface from rust_gfx_init, (b) C has not freed,
61:     //         (c) single-threaded per REQ-THR-010, (d) pitch > 0,
62:     //         (e) surface dims match expected, (f) pixel_len <= isize::MAX
63:     std::slice::from_raw_parts(surf.pixels AS *const u8,
64:                                 pixel_len)
65:   }
66:   result ← texture.update(None, pixel_data, pitch)
67:   IF result IS Err THEN RETURN                          // REQ-ERR-065
68:
69:   // --- Set blend mode and alpha ---
70:   IF alpha == 255 THEN
71:     texture.set_blend_mode(BlendMode::None)             // REQ-SCR-030
72:   ELSE
73:     texture.set_blend_mode(BlendMode::Blend)            // REQ-SCR-040
74:     texture.set_alpha_mod(alpha)
75:   END IF
76:
77:   // --- Render ---
78:   // REQ-SCR-050: rect NULL → full surface
79:   // REQ-SCR-060: rect non-NULL → clip region (src == dst)
 80:   // REQ-SCR-150: pass rect directly, no coordinate transform
 81:   IF state.canvas.copy(&texture, sdl2_rect, sdl2_rect) fails THEN
 82:     log_once("canvas.copy failed in screen layer")         // REQ-ERR-060
 83:     RETURN
 84:   END IF
 85:
 86:   // Texture dropped here — REQ-NP-020, REQ-NP-025
 87: END FUNCTION
```

## 003B: convert_rect (helper)

```
 1: FUNCTION convert_rect(rect: *const SDL_Rect) -> Option<sdl2::rect::Rect>
 2:   IF rect IS null THEN
 3:     RETURN None
 4:   END IF
 5:
 6:   // SAFETY: null checked above, valid per REQ-ASM-040 — REQ-FFI-010
 7:   r ← unsafe { &*rect }
 8:
 9:   // Negative w/h must be caught by caller before conversion
10:   // w and h are c_int (signed), Rect::new expects u32
11:   RETURN Some(Rect::new(r.x, r.y, r.w AS u32, r.h AS u32))
12: END FUNCTION
```

### Validation Points
- Line 4: Uninitialized guard
- Line 6–8: Screen index range check
- Line 10: Screen 1 (EXTRA) skip
- Line 13: Null surface check
- Line 20: Negative rect dimension check
- Line 26–28: Null pixels / non-positive pitch check
- Line 32–41: Pitch/size validation (minimum stride, pixel_len > 0, isize::MAX)
- Line 55: Texture creation failure
- Line 67: Texture update failure
- Line 81: canvas.copy failure (log_once and return)

### Error Handling
- All validation failures: silent return (no log — REQ-ERR-030)
- Texture failures: silent return (frame missing layer — REQ-ERR-060)
- canvas.copy failure: log_once diagnostic, then return (REQ-ERR-060)

### Ordering Constraints
- Validation MUST precede surface access
- Surface access MUST precede texture creation
- Texture update MUST succeed before state.canvas.copy (REQ-ERR-065)
- Texture MUST be dropped before function returns (REQ-NP-025)

### Integration Boundaries
- Called by C `Rust_ScreenLayer` wrapper → `TFB_SwapBuffers` steps 2, 3, 5
- Reads state.surfaces[screen] (raw pointer to C-created surface)
- Creates/destroys temporary texture on state.canvas
- Uses state.canvas for rendering

### Side Effects
- Modifies renderer target (composites a texture layer)
- Does NOT modify source surface (REQ-SCR-080)
