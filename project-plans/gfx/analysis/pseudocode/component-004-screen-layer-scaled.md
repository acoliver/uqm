# Pseudocode: Component 004 — ScreenLayer (Scaled Path)

Plan ID: `PLAN-20260223-GFX-VTABLE-FIX`
Requirements: REQ-SCALE-010..070, REQ-SCALE-025, REQ-SCALE-055,
              REQ-SCR-070, REQ-SCR-075, REQ-NP-025

---

## 004A: screen_layer_scaled

> Called from `rust_gfx_screen` (component-003, line 39) when
> `state.scaled_buffers[screen].is_some()`.

```
 1: FUNCTION screen_layer_scaled(state, screen, alpha, sdl2_rect, surf, pitch)
 2:   // --- Mutable state access ---
 3:   // state is obtained via: LET state = &mut *GFX_STATE.get()
 4:   // All mutations (scaled_buffers, log flags) go through this single
 5:   // mutable reference. The caller (rust_gfx_screen) obtains the reference
 6:   // once and passes it down — no re-borrowing of the global.
 7:
 8:   // --- Determine scale factor ---
 3:   scale_factor ← IF (state.flags & bit8) != 0 THEN 3   // xBRZ 3×
 4:                   ELSE IF (state.flags & bit9) != 0 THEN 4  // xBRZ 4×
 5:                   ELSE 2                                 // HQ2x default
 6:   using_xbrz ← (state.flags & (bit8 | bit9)) != 0
 7:
 8:   tex_w ← SCREEN_WIDTH * scale_factor                   // REQ-SCALE-040
 9:   tex_h ← SCREEN_HEIGHT * scale_factor
10:
11:   // --- Create scaled texture ---
12:   texture_creator ← state.canvas.texture_creator()
13:   texture ← texture_creator.create_texture_streaming(
14:               RGBX8888, tex_w, tex_h)
15:   IF texture IS Err THEN RETURN                         // REQ-SCR-130
16:
17:   buffer ← state.scaled_buffers[screen].as_mut()
18:   IF buffer IS None THEN RETURN                         // defensive
19:
 20:   // --- Read source pixels ---
 21:   src_width ← SCREEN_WIDTH AS usize
 22:   src_height ← SCREEN_HEIGHT AS usize
 23:
 24:   // --- Pitch/size validation (before from_raw_parts) ---  // REQ-SCR-165
 25:   VALIDATE surf.pitch >= SCREEN_WIDTH * 4                   // minimum stride for 32bpp
 26:   VALIDATE surf.pitch * src_height <= isize::MAX            // from_raw_parts safety
 27:   VALIDATE surf.pitch * src_height > 0                      // non-empty buffer
 28:   IF validation fails THEN RETURN                           // skip this layer
 29:
 30:   src_size ← pitch * src_height
 31:   src_bytes ← unsafe {
 32:     // SAFETY: surface from rust_gfx_init, single-threaded,
 33:     //         pitch/size validated above
 34:     std::slice::from_raw_parts(surf.pixels AS *const u8, src_size)
 35:   }
28:
29:   // --- Convert RGBX8888 → RGBA for scaler input ---     // REQ-SCALE-010
30:   pixmap ← Pixmap::new(1, SCREEN_WIDTH, SCREEN_HEIGHT, Rgba32)
31:   dst_bytes ← pixmap.data_mut()
32:
33:   FOR y IN 0..src_height:
34:     src_row ← src_bytes[y * pitch .. y * pitch + src_width * 4]
35:     dst_row ← dst_bytes[y * src_width * 4 .. (y+1) * src_width * 4]
36:     FOR x IN 0..src_width:
37:       s ← src_row[x*4 .. x*4+4]
38:       d ← dst_row[x*4 .. x*4+4]
39:       // RGBX8888 memory [X,B,G,R] → RGBA [R,G,B,A]     // REQ-SCALE-060
40:       d[0] ← s[3]   // R
41:       d[1] ← s[2]   // G
42:       d[2] ← s[1]   // B
43:       d[3] ← 0xFF   // A (opaque)
44:     END FOR
45:   END FOR
46:
47:   // --- Run scaler ---                                   // REQ-SCALE-020
48:   IF using_xbrz THEN
49:     scaled_bytes ← xbrz::scale_rgba(dst_bytes, src_width, src_height, scale_factor)
50:     // Log once
51:     IF NOT state.xbrz_logged THEN
52:       LOG "xBRZ scaler active ({scale_factor}x)"
53:       state.xbrz_logged ← true
54:     END IF
55:   ELSE
56:     params ← ScaleParams::new(512, RustScaleMode::Hq2x)
57:     scaled_result ← state.hq2x.scale(&pixmap, params)
58:     IF scaled_result IS Err THEN RETURN
59:     scaled_bytes ← scaled_result.data()
60:     // Log once
61:     IF NOT state.hq2x_logged THEN
62:       LOG "HQ2x scaler active"
63:       state.hq2x_logged ← true
64:     END IF
65:   END IF
66:
67:   // --- Convert RGBA → RGBX8888 for texture upload ---   // REQ-SCALE-030
68:   dst_width ← src_width * scale_factor
69:   dst_height ← src_height * scale_factor
70:   dst_stride ← dst_width * 4
71:
72:   FOR y IN 0..dst_height:
73:     src_row ← scaled_bytes[y * dst_stride .. (y+1) * dst_stride]
74:     dst_row ← buffer[y * dst_stride .. (y+1) * dst_stride]
75:     FOR x IN 0..dst_width:
76:       s ← src_row[x*4 .. x*4+4]
77:       d ← dst_row[x*4 .. x*4+4]
78:       // RGBA [R,G,B,A] → RGBX8888 memory [X,B,G,R]     // REQ-SCALE-070
79:       d[0] ← 0xFF   // X (padding)
80:       d[1] ← s[2]   // B
81:       d[2] ← s[1]   // G
82:       d[3] ← s[0]   // R
83:     END FOR
84:   END FOR
85:
86:   // --- Upload scaled data to texture ---
87:   result ← texture.update(None, buffer, dst_stride)
88:   IF result IS Err THEN RETURN                           // REQ-ERR-065
89:
90:   // --- Set blend mode and alpha ---
91:   IF alpha == 255 THEN
92:     texture.set_blend_mode(BlendMode::None)
93:   ELSE
94:     texture.set_blend_mode(BlendMode::Blend)
95:     texture.set_alpha_mod(alpha)
96:   END IF
97:
98:   // --- Compute source rect (scaled coordinates) ---     // REQ-SCALE-050
99:   src_rect ← IF sdl2_rect IS Some(r) THEN
100:    // REQ-SCALE-055: no overflow (max 1280 within i32)
101:    Some(Rect::new(
102:      r.x * scale_factor AS i32,
103:      r.y * scale_factor AS i32,
104:      r.width * scale_factor,
105:      r.height * scale_factor))
106:  ELSE
107:    None
108:  END IF
109:
 110:  // --- Render (dst rect in logical coordinates) ---     // REQ-WIN-030
 111:  IF state.canvas.copy(&texture, src_rect, sdl2_rect) fails THEN
 112:    log_once("canvas.copy failed in scaled screen layer")  // REQ-ERR-060
 113:    RETURN
 114:  END IF
 115:
 116:  // Texture dropped here — REQ-NP-025
 117: END FUNCTION
```

### Validation Points
- Line 15: Texture creation failure
- Line 18: Scaled buffer existence check
- Lines 25–28: Pitch/size validation (minimum stride, isize::MAX, non-zero)
- Line 58: HQ2x scaling failure
- Line 88: Texture update failure
- Line 111: canvas.copy failure (log_once and return)

### Error Handling
- All failures: silent return (missing layer, no crash)
- canvas.copy failure: log_once diagnostic, then return
- No per-frame logging (one-time log for scaler activation and copy failure)

### Ordering Constraints
- RGBX→RGBA conversion (line 29–45) MUST precede scaler call (line 47–65)
- Scaler call MUST precede RGBA→RGBX conversion (line 67–84)
- RGBA→RGBX conversion MUST precede texture upload (line 87)
- Texture update MUST succeed before state.canvas.copy (REQ-ERR-065)

### Integration Boundaries
- Called from component-003 (rust_gfx_screen) scaled path
- Uses state.scaled_buffers[screen] as output buffer
- Uses state.hq2x (Hq2xScaler) or xbrz::scale_rgba
- Uses crate::graphics::pixmap::Pixmap as intermediate

### Side Effects
- Modifies state.scaled_buffers[screen] contents
- Modifies renderer target (composites scaled texture)
- Modifies state.hq2x_logged / state.xbrz_logged (one-time)
rust_gfx_screen) scaled path
- Uses state.scaled_buffers[screen] as output buffer
- Uses state.hq2x (Hq2xScaler) or xbrz::scale_rgba
- Uses crate::graphics::pixmap::Pixmap as intermediate

### Side Effects
- Modifies state.scaled_buffers[screen] contents
- Modifies renderer target (composites scaled texture)
- Modifies state.hq2x_logged / state.xbrz_logged (one-time)
