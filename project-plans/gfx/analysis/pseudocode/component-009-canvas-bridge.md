# Pseudocode: Component 009 — Canvas FFI Bridge

Plan ID: `PLAN-20260223-GFX-FULL-PORT`
Requirements: REQ-CANVAS-010..100, REQ-CANVAS-120..150,
              REQ-COMPAT-050, REQ-COMPAT-060, REQ-GUARD-070, REQ-GUARD-080,
              REQ-FFI-030, REQ-FFI-040

---

## 009A: SurfaceCanvas Adapter for Screen Canvases

> During DCQ flush, screen surfaces are wrapped as SurfaceCanvas instances
> so Rust drawing functions can operate on them. This is the critical
> zero-copy bridge between C's SDL_Surface and Rust's PixelCanvas trait.
> Reference: technical.md §8.4.1, §8.7

```
 1: FUNCTION create_screen_canvas(screen_index: usize) -> Result<SurfaceCanvas<'_>, GraphicsError>
 2:   // --- Obtain surface pointer ---
 3:   state ← get_gfx_state()
 4:   IF state IS None THEN
 5:     RETURN Err(GraphicsError::NotInitialized)
 6:   END IF
 7:
 8:   IF screen_index >= TFB_GFX_NUMSCREENS THEN
 9:     RETURN Err(GraphicsError::InvalidScreen)
10:   END IF
11:
12:   surface_ptr ← state.surfaces[screen_index]
13:   IF surface_ptr IS null THEN
14:     RETURN Err(GraphicsError::NullSurface)
15:   END IF
16:
17:   // --- Lock and wrap ---
18:   // SAFETY: surface from rust_gfx_init, not freed, single-threaded
19:   lock ← LockedSurface::new(surface_ptr)                   // SDL_LockSurface called
20:   canvas ← lock.as_canvas()?                                // validates format, dims, pitch
21:   RETURN Ok(canvas)
22:   // NOTE: LockedSurface must outlive SurfaceCanvas.
23:   // In practice both are scoped to the DCQ flush block.
24: END FUNCTION
```

## 009B: Canvas Drawing FFI Exports

> These functions are exported as `#[no_mangle] pub extern "C" fn` and
> replace the C functions in `canvas.c` and `primitives.c` when
> `USE_RUST_GFX` is defined.
> Reference: technical.md §8.4, functional.md §13

```
 1: // --- draw_line ---                                        // REQ-CANVAS-010, REQ-CANVAS-090
 2: #[no_mangle]
 3: FUNCTION rust_canvas_draw_line(
 4:     canvas_ptr: *mut c_void,                                // TFB_Canvas (opaque)
 5:     x1: c_int, y1: c_int, x2: c_int, y2: c_int,
 6:     color: FfiColor, draw_mode: FfiDrawMode
 7: )
 8:   // --- Validation ---
 9:   IF canvas_ptr IS null THEN RETURN                         // silent no-op
10:
11:   // --- Resolve canvas ---
12:   canvas ← resolve_canvas(canvas_ptr)                       // see 009E
13:   IF canvas IS Err THEN RETURN
14:
15:   rust_color ← Color::from(color)
16:   // --- Dispatch to generic draw_line ---
17:   result ← draw_line(&mut canvas, x1, y1, x2, y2, rust_color)
18:   IF result IS Err THEN
19:     // Log once, do not spam
20:     log_once("rust_canvas_draw_line failed")
21:   END IF
22: END FUNCTION
23:
24: // --- draw_rect (outlined) ---                              // REQ-CANVAS-020
25: #[no_mangle]
26: FUNCTION rust_canvas_draw_rect(
27:     canvas_ptr: *mut c_void,
28:     rect: *const FfiRect,
29:     color: FfiColor, draw_mode: FfiDrawMode
30: )
31:   IF canvas_ptr IS null THEN RETURN
32:   IF rect IS null THEN RETURN
33:
34:   canvas ← resolve_canvas(canvas_ptr)
35:   IF canvas IS Err THEN RETURN
36:
37:   r ← unsafe { &*rect }
38:   rust_color ← Color::from(color)
39:   result ← draw_rect(&mut canvas, r.x, r.y, r.w, r.h, rust_color)
40:   IF result IS Err THEN log_once("rust_canvas_draw_rect failed")
41: END FUNCTION
42:
43: // --- fill_rect ---                                         // REQ-CANVAS-030, REQ-CANVAS-100
44: #[no_mangle]
45: FUNCTION rust_canvas_fill_rect(
46:     canvas_ptr: *mut c_void,
47:     rect: *const FfiRect,
48:     color: FfiColor, draw_mode: FfiDrawMode
49: )
50:   IF canvas_ptr IS null THEN RETURN
51:   IF rect IS null THEN RETURN
52:
53:   canvas ← resolve_canvas(canvas_ptr)
54:   IF canvas IS Err THEN RETURN
55:
56:   r ← unsafe { &*rect }
57:   // --- Early-exit for entirely-outside rect ---            // REQ-CANVAS-100
58:   IF r.x >= canvas.width() OR r.y >= canvas.height() THEN RETURN
59:   IF r.x + r.w <= 0 OR r.y + r.h <= 0 THEN RETURN
60:
61:   rust_color ← Color::from(color)
62:   result ← fill_rect(&mut canvas, r.x, r.y, r.w, r.h, rust_color)
63:   IF result IS Err THEN log_once("rust_canvas_fill_rect failed")
64: END FUNCTION
65:
66: // --- draw_image ---                                        // REQ-CANVAS-040
67: #[no_mangle]
68: FUNCTION rust_canvas_draw_image(
69:     canvas_ptr: *mut c_void,
70:     img_ptr: *mut FfiTfbImage,
71:     x: c_int, y: c_int,
72:     scale: c_int, scalemode: c_int,
73:     colormap: *mut c_void,
74:     draw_mode: FfiDrawMode
75: )
76:   IF canvas_ptr IS null OR img_ptr IS null THEN RETURN
77:
78:   canvas ← resolve_canvas(canvas_ptr)
79:   IF canvas IS Err THEN RETURN
80:
81:   image ← resolve_image(img_ptr)                            // see 009E
82:   IF image IS None THEN RETURN
83:
84:   // --- Convert scale/scalemode ---
85:   rust_scale ← if scale == 0 then TFB_SCALE_NEAREST else scale
86:
87:   result ← draw_scaled_image(&mut canvas, &image, x, y,
88:                               rust_scale, scalemode, draw_mode.into())
89:   IF result IS Err THEN log_once("rust_canvas_draw_image failed")
90: END FUNCTION
91:
92: // --- draw_filled_image ---                                 // REQ-CANVAS-050
93: #[no_mangle]
94: FUNCTION rust_canvas_draw_filled_image(
95:     canvas_ptr: *mut c_void,
96:     img_ptr: *mut FfiTfbImage,
97:     x: c_int, y: c_int,
98:     scale: c_int, scalemode: c_int,
99:     color: FfiColor,
100:    draw_mode: FfiDrawMode
101:)
102:  IF canvas_ptr IS null OR img_ptr IS null THEN RETURN
103:
104:  canvas ← resolve_canvas(canvas_ptr)
105:  IF canvas IS Err THEN RETURN
106:
107:  image ← resolve_image(img_ptr)
108:  IF image IS None THEN RETURN
109:
110:  rust_color ← Color::from(color)
111:  result ← draw_filled_image(&mut canvas, &image, x, y,
112:                              scale, rust_color, draw_mode.into())
113:  IF result IS Err THEN log_once("rust_canvas_draw_filled_image failed")
114: END FUNCTION
115:
116: // --- draw_fontchar ---                                     // REQ-CANVAS-060
117: #[no_mangle]
118: FUNCTION rust_canvas_draw_fontchar(
119:    canvas_ptr: *mut c_void,
120:    char_ptr: *mut FfiTfbChar,
121:    backing_ptr: *mut FfiTfbImage,
122:    x: c_int, y: c_int,
123:    draw_mode: FfiDrawMode
124:)
125:  IF canvas_ptr IS null OR char_ptr IS null THEN RETURN
126:
127:  canvas ← resolve_canvas(canvas_ptr)
128:  IF canvas IS Err THEN RETURN
129:
130:  tf_char ← resolve_fontchar(char_ptr)
131:  IF tf_char IS None THEN RETURN
132:
133:  // backing_ptr may be null (not all font chars have backing images)
134:  backing ← IF backing_ptr IS NOT null THEN resolve_image(backing_ptr) ELSE None
135:
136:  result ← draw_fontchar(&mut canvas, &tf_char, backing.as_ref(),
137:                          x, y, draw_mode.into())
138:  IF result IS Err THEN log_once("rust_canvas_draw_fontchar failed")
139: END FUNCTION
140:
141: // --- copy_canvas ---                                       // REQ-CANVAS-070
142: #[no_mangle]
143: FUNCTION rust_canvas_copy_rect(
144:    dst_ptr: *mut c_void,
145:    src_ptr: *mut c_void,
146:    rect: *const FfiRect,
147:    dest_x: c_int, dest_y: c_int
148:)
149:  IF dst_ptr IS null OR src_ptr IS null THEN RETURN
150:
151:  dst_canvas ← resolve_canvas(dst_ptr)
152:  IF dst_canvas IS Err THEN RETURN
153:
154:  src_canvas ← resolve_canvas_readonly(src_ptr)
155:  IF src_canvas IS Err THEN RETURN
156:
157:  src_rect ← IF rect IS NOT null THEN
158:    r ← unsafe { &*rect }
159:    Some(Rect { x: r.x, y: r.y, w: r.w, h: r.h })
160:  ELSE
161:    None
162:  END IF
163:
164:  result ← copy_canvas(&mut dst_canvas, &src_canvas, dest_x, dest_y, src_rect)
165:  IF result IS Err THEN log_once("rust_canvas_copy_rect failed")
166: END FUNCTION
167:
168: // --- scissor enable ---                                    // REQ-CANVAS-080
169: #[no_mangle]
170: FUNCTION rust_canvas_set_scissor(
171:    canvas_ptr: *mut c_void,
172:    rect: *const FfiRect
173:)
174:  IF canvas_ptr IS null THEN RETURN
175:
176:  canvas ← resolve_canvas(canvas_ptr)
177:  IF canvas IS Err THEN RETURN
178:
179:  IF rect IS NOT null THEN
180:    r ← unsafe { &*rect }
181:    canvas.set_scissor(r.x, r.y, r.w, r.h)
182:    canvas.enable_scissor()
183:  ELSE
184:    canvas.disable_scissor()
185:  END IF
186: END FUNCTION
```

## 009C: Canvas Lifecycle FFI Exports

> C code creates and destroys canvases during resource loading and
> drawable management. These FFI exports replace canvas.c lifecycle functions.
> Reference: REQ-CANVAS-140

```
 1: // --- New_TrueColorCanvas ---                               // REQ-CANVAS-140
 2: #[no_mangle]
 3: FUNCTION rust_canvas_new_truecolor(
 4:     width: c_int, height: c_int, format: c_int
 5: ) -> *mut c_void
 6:   // --- Validation ---
 7:   IF width <= 0 OR height <= 0 THEN
 8:     RETURN null
 9:   END IF
10:
11:   pixel_fmt ← MATCH format
12:     0 → CanvasFormat::rgba()                                // 32bpp RGBA
13:     1 → CanvasFormat::rgbx()                                // 32bpp RGBX
14:     _ → RETURN null                                          // unknown format
15:   END MATCH
16:
17:   canvas ← Canvas::new(width AS u32, height AS u32, pixel_fmt)
18:   boxed ← Box::new(canvas)
19:   RETURN Box::into_raw(boxed) AS *mut c_void                // C owns the pointer
20: END FUNCTION
21:
22: // --- Canvas_Delete ---                                      // REQ-CANVAS-140
23: #[no_mangle]
24: FUNCTION rust_canvas_delete(canvas_ptr: *mut c_void)
25:   IF canvas_ptr IS null THEN RETURN
26:   // SAFETY: pointer was created by rust_canvas_new_truecolor (Box::into_raw)
27:   unsafe { drop(Box::from_raw(canvas_ptr AS *mut Canvas)) }
28: END FUNCTION
```

## 009D: Lock/Unlock Protocol for DCQ Flush

> The DCQ flush wraps screen surfaces in SurfaceCanvas for the duration
> of command processing. This is the per-flush lifecycle.
> Reference: technical.md §8.7.1, §8.7.2

```
 1: FUNCTION flush_with_surface_canvas(screen_index: usize,
 2:     commands: &[DrawCommand])
 3:   state ← get_gfx_state()
 4:   IF state IS None THEN RETURN
 5:
 6:   surface_ptr ← state.surfaces[screen_index]
 7:   IF surface_ptr IS null THEN RETURN
 8:
 9:   // --- Lock/unlock bracket ---                             // technical §8.7.2
10:   {
11:     lock ← LockedSurface::new(surface_ptr)                  // SDL_LockSurface
12:     canvas_result ← lock.as_canvas()
13:     IF canvas_result IS Err THEN
14:       // lock dropped → SDL_UnlockSurface
15:       RETURN
16:     END IF
17:     canvas ← canvas_result.unwrap()
18:
19:     // --- Dispatch all commands ---
20:     FOR cmd IN commands:
21:       dispatch_draw_command(&mut canvas, cmd)                // see component-010
22:     END FOR
23:
24:     // canvas dropped (borrows from lock)
25:     // lock dropped → SDL_UnlockSurface                     // RAII cleanup
26:   }
27: END FUNCTION
28:
29: // --- Self-blit safety ---                                   // technical §8.7.8
30: // When a draw command copies within the same surface:
31: FUNCTION self_blit_safe<C: PixelCanvas>(
32:     canvas: &mut C,
33:     src_rect: Rect, dst_rect: Rect
34: )
35:   // Allocate temp buffer for source region
36:   buf_size ← src_rect.w * src_rect.h * canvas.format().bytes_per_pixel()
37:   temp ← vec![0u8; buf_size]
38:
39:   // Copy source → temp
40:   FOR y IN 0..src_rect.h:
41:     src_offset ← ((src_rect.y + y) AS usize) * canvas.pitch()
42:                   + (src_rect.x AS usize) * canvas.format().bytes_per_pixel()
43:     row_len ← (src_rect.w AS usize) * canvas.format().bytes_per_pixel()
44:     temp[y * row_len .. (y+1) * row_len]
45:       .copy_from_slice(&canvas.pixels()[src_offset .. src_offset + row_len])
46:   END FOR
47:
48:   // Copy temp → destination
49:   FOR y IN 0..dst_rect.h:
50:     dst_offset ← ((dst_rect.y + y) AS usize) * canvas.pitch()
51:                   + (dst_rect.x AS usize) * canvas.format().bytes_per_pixel()
52:     row_len ← (src_rect.w AS usize) * canvas.format().bytes_per_pixel()
53:     canvas.pixels_mut()[dst_offset .. dst_offset + row_len]
54:       .copy_from_slice(&temp[y * row_len .. (y+1) * row_len])
55:   END FOR
56: END FUNCTION
```

## 009E: Canvas Resolution Helpers

> Resolves opaque C `void*` canvas pointers to PixelCanvas implementors.
> During DCQ flush, screen canvases are SurfaceCanvas. For owned canvases
> (images, offscreen), they are LockedCanvas from Box<Canvas>.

```
 1: FUNCTION resolve_canvas(ptr: *mut c_void) -> Result<impl PixelCanvas, GraphicsError>
 2:   // --- Check if ptr matches a known screen surface ---
 3:   state ← get_gfx_state()
 4:   IF state IS Some THEN
 5:     FOR i IN 0..TFB_GFX_NUMSCREENS:
 6:       IF ptr == state.surfaces[i] AS *mut c_void THEN
 7:         // This is a screen surface → create SurfaceCanvas
 8:         // (Assumes surface is already locked within flush scope)
 9:         RETURN Ok(SurfaceCanvas::from_raw_locked(ptr AS *mut SDL_Surface))
10:       END IF
11:     END FOR
12:   END IF
13:
14:   // --- Otherwise, ptr is a Box<Canvas> from rust_canvas_new_truecolor ---
15:   // SAFETY: caller guarantees ptr was created by rust_canvas_new_truecolor
16:   canvas_ref ← unsafe { &mut *(ptr AS *mut Canvas) }
17:   locked ← canvas_ref.lock_pixels()
18:   RETURN Ok(locked)                                          // LockedCanvas
19: END FUNCTION
20:
21: FUNCTION resolve_image(img_ptr: *mut FfiTfbImage) -> Option<&TFImage>
22:   // Lookup in RenderContext registry
23:   context ← global_render_context()
24:   image_ref ← ImageRef::from_raw(img_ptr)
25:   RETURN context.get_image(image_ref)                       // REQ-GFXLOAD-040
26: END FUNCTION
27:
28: FUNCTION resolve_fontchar(char_ptr: *mut FfiTfbChar) -> Option<&TFChar>
29:   // Lookup in RenderContext font registry
30:   context ← global_render_context()
31:   RETURN context.get_fontchar(char_ptr)
32: END FUNCTION
```

### Validation Points
- 009A line 4–6: State initialization check
- 009A line 8–10: Screen index range check
- 009A line 13–15: Null surface pointer check
- 009B line 9, 31, 51, 76, 102, 125, 149, 174: Null pointer guards on all FFI entries
- 009C line 7–9: Non-positive dimension rejection
- 009C line 14: Unknown format rejection
- 009D line 12–16: Surface canvas construction failure handling
- 009E line 4–12: Screen surface identity check before resolve

### Error Handling
- All FFI functions: null pointer → silent return (no crash)              // REQ-FFI-030
- Canvas construction failure: log_once, return (no crash)
- Draw function errors: log_once, continue (frame missing element)
- Box::from_raw in delete: null guard prevents double-free

### Ordering Constraints
- Screen surfaces MUST be locked (LockedSurface) before SurfaceCanvas creation
- SurfaceCanvas MUST be dropped before LockedSurface (RAII scoping)
- All FFI exports MUST use #[no_mangle] extern "C"                       // REQ-FFI-040
- resolve_canvas MUST check screen surfaces FIRST (common fast path)
- Self-blit: source MUST be copied to temp BEFORE destination write       // technical §8.7.8

### Integration Boundaries
- Exported from Rust library as #[no_mangle] symbols
- Replaces: canvas.c (2,176 lines), primitives.c (633 lines)            // REQ-GUARD-070, REQ-GUARD-080
- Called from: C DCQ dispatch (dcqueue.c) via TFB_DrawCanvas_* wrappers
- Also called from: Rust DCQ dispatch (component-010)
- Uses: PixelCanvas trait (component-008), RenderContext (render_context.rs)
- Thread affinity: main/graphics thread only                              // REQ-THR-010

### Side Effects
- Pixel writes to SurfaceCanvas are immediately visible to presentation   // REQ-CANVAS-130
- Canvas lifecycle (new/delete) allocates/frees heap memory
- LockedSurface calls SDL_LockSurface/SDL_UnlockSurface
