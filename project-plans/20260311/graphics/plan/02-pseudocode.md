# Phase 02: Pseudocode

## Phase ID
`PLAN-20260314-GRAPHICS.P02`

## Prerequisites
- Required: Phase P01 completed
- Verify: Analysis document covers all gaps

---

## PC-01: Canvas Pixel Sync + Synchronization Points (G1)

```text
01: FUNCTION rust_canvas_from_surface(surface: *mut SDL_Surface) -> *mut SurfaceCanvas
02:   VALIDATE surface not null, pixels not null, dimensions > 0
03:   READ surface.w, surface.h, surface.pitch
04:   CREATE canvas = Canvas::new_rgba(w, h)
05:   IMPORT surface pixels into canvas:
06:     FOR each row 0..h:
07:       src_row = surface.pixels + row * surface.pitch
08:       FOR each pixel 0..w:
09:         READ [X, B, G, R] from src_row (RGBX8888 little-endian)
10:         WRITE [R, G, B, 255] to canvas pixel buffer
11:   RETURN Box::into_raw(SurfaceCanvas { surface, canvas, w, h, dirty: false })
12:
13: FUNCTION rust_canvas_flush(sc: *mut SurfaceCanvas) -> c_int
14:   VALIDATE sc not null
15:   IF sc.dirty is false: RETURN 0
16:   EXPORT canvas pixels back to surface:
17:     FOR each row 0..h:
18:       dst_row = surface.pixels + row * surface.pitch
19:       FOR each pixel 0..w:
20:         READ [R, G, B, A] from canvas pixel buffer
21:         WRITE [0, B, G, R] to dst_row (RGBX8888, discard alpha)
22:   SET sc.dirty = false
23:   RETURN 0
24:
25: BEFORE presentation compositing reads a surface-backed canvas:
26:   CALL rust_canvas_flush() or equivalent synchronization hook
27:
28: BEFORE transition capture reads main-screen pixels:
29:   CALL rust_canvas_flush() or equivalent synchronization hook
30:
31: BEFORE interop readback / screen-to-image copy reads pixels:
32:   CALL rust_canvas_flush() or equivalent synchronization hook
33:
34: FUNCTION rust_canvas_destroy(sc: *mut SurfaceCanvas)
35:   VALIDATE sc not null
36:   CALL rust_canvas_flush(sc)
37:   DROP Box::from_raw(sc)
```

---

## PC-02: Postprocess Cleanup + Scanlines (G2, G3)

```text
40: FUNCTION rust_gfx_postprocess()
41:   GET state from global singleton
42:   IF state is None: RETURN
43:   IF SCANLINES flag is set in state.flags:
44:     CALL apply_scanlines(state)
45:   CALL state.canvas.present()
46:
47: FUNCTION apply_scanlines(state)
48:   SET renderer draw color to (0, 0, 0, scanline_alpha)
49:   SET blend mode to Blend
50:   FOR y in (0..logical_height).step_by(2):
51:     DRAW filled rect at (0, y, logical_width, 1)
52:   RESET blend mode to None
```

---

## PC-03: Missing DCQ Push Functions (G4)

```text
60: FUNCTION rust_dcq_push_filledimage(image_id, x, y, color, scale, scale_mode, draw_mode)
61:   GET state from DCQ singleton
62:   GET dest from current_screen
63:   CREATE DrawCommand::FilledImage { image: ImageRef(image_id), x, y, color, scale, scale_mode, draw_mode, dest }
64:   PUSH command to queue
65:   RETURN 0 or -1
66:
67: FUNCTION rust_dcq_push_fontchar(fontchar_data, pitch, w, h, hs_x, hs_y,
68:                                  disp_w, disp_h, backing_image_id, x, y, color, draw_mode)
69:   GET state from DCQ singleton
70:   GET dest from current_screen
71:   CREATE FontCharRef from raw data pointer, dimensions, hotspot, display extent
72:   CREATE backing = if backing_image_id != 0 { Some(ImageRef(backing_image_id)) } else { None }
73:   CREATE DrawCommand::FontChar { fontchar, backing, x, y, draw_mode, dest }
74:   PUSH command to queue
75:   RETURN 0 or -1
76:
77: FUNCTION rust_dcq_push_setmipmap(image_id, mipmap_id, hot_x, hot_y)
78:   GET state from DCQ singleton
79:   CREATE DrawCommand::SetMipmap { image: ImageRef(image_id), mipmap: ImageRef(mipmap_id), hotx, hoty }
80:   PUSH command to queue
81:   RETURN 0 or -1
82:
83: FUNCTION rust_dcq_push_deletedata(data_ptr)
84:   GET state from DCQ singleton
85:   CREATE DrawCommand::DeleteData { data: data_ptr as u64 }
86:   PUSH command to queue
87:   RETURN 0 or -1
88:
89: FUNCTION rust_dcq_push_callback(fn_ptr, arg)
90:   GET state from DCQ singleton
91:   WRAP fn_ptr as Rust-callable function
92:   CREATE DrawCommand::Callback { callback: wrapped_fn, arg }
93:   PUSH command to queue
94:   RETURN 0 or -1
```

---

## PC-04: DrawImage Expanded Parameters / Context Propagation (G5)

```text
100: FUNCTION rust_dcq_push_drawimage(image_id, x, y, scale, scale_mode, colormap_index, draw_mode)
101:   GET state from DCQ singleton
102:   GET dest from current_screen
103:   LET cmap = if colormap_index >= 0 { Some(ColorMapRef(colormap_index)) } else { None }
104:   LET mode = DrawMode::from(draw_mode)
105:   LET smode = ScaleMode::from(scale_mode)
106:   CREATE DrawCommand::Image { image: ImageRef(image_id), x, y, scale, scale_mode: smode, colormap: cmap, draw_mode: mode, dest }
107:   PUSH command to queue
108:   RETURN 0 or -1
109:
110: IN C bridge forwarding path:
111:   EXTRACT current draw mode, color/colormap, clip/font/scale/screen state from caller-visible context
112:   PASS exact values to Rust FFI instead of hardcoding defaults
```

---

## PC-05: SetPalette Command Variant (G6, G13)

```text
120: ADD to DrawCommand enum:
121:   SetPalette { colormap_id: u32 }
122:
123: IN handle_command():
124:   MATCH DrawCommand::SetPalette { colormap_id }:
125:     ACCESS render_context
126:     SET active colormap to colormap_id
127:     UPDATE any dependent colormap version / cache invalidation state required by the real render context
128:
129: FUNCTION rust_dcq_push_setpalette(colormap_id)
130:   GET state from DCQ singleton
131:   CREATE DrawCommand::SetPalette { colormap_id }
132:   PUSH command to queue
133:   RETURN 0 or -1
```

---

## PC-06: DCQ Flush + Queue Semantics Parity (G10, G12, G14, G15)

```text
140: FUNCTION process_commands()
141:   IF queue is empty:
142:     IF fade_active OR transition_active:
143:       CALL swap_buffers(REDRAW_FADING)
144:       BROADCAST rendering_cond if required by integration contract
145:     RETURN Ok
146:   RESET bounding_box to empty
147:   WHILE queue has commands visible at current batch depth:
148:     POP next visible command in FIFO order
149:     IF command targets Screen::Main:
150:       EXPAND bounding_box by command's affected region
151:     CALL handle_command(command)
152:     IF queue.len() > force_break_size AND livelock_count > threshold:
153:       ACQUIRE DCQ lock or equivalent producer-blocking path
154:       PROCESS remaining visible commands
155:       RELEASE producer-blocking path
156:   CALL swap_buffers(REDRAW_NO)
157:   BROADCAST rendering_cond
158:   RESET bounding_box
159:   RETURN Ok
160:
161: FUNCTION batch()
162:   INCREMENT batch_depth
163:
164: FUNCTION unbatch()
165:   VALIDATE batch_depth > 0
166:   DECREMENT batch_depth
167:   ONLY when batch_depth reaches 0 do previously hidden commands become visible to flush
168:
169: VERIFY nested batching:
170:   batch(); push(A); batch(); push(B); unbatch(); flush() => neither A nor B visible yet
171:   unbatch(); flush() => A then B become visible in FIFO order
172:
173: VERIFY deferred free ordering:
174:   push(draw image X); push(copy image X); push(delete image X)
175:   flush() => delete executes only after all prior uses of X complete
176:
177: VERIFY image synchronization obligations:
178:   any externally visible concurrent image metadata read uses per-image mutex or equivalent ABI-compatible synchronization
```

---

## PC-07: System-Box Compositing in C Orchestration Path (G9)

```text
190: IN TFB_SwapBuffers sequence (called from C, orchestrated in sdl_common.c):
191:   preprocess(force_redraw, transition_amount, fade_amount)
192:   screen(MAIN, 255, NULL)
193:   IF transition_amount != 255:
194:     screen(TRANSITION, 255 - transition_amount, &clip_rect)
195:   IF fade_amount != 255:
196:     color(fade_r, fade_g, fade_b, fade_alpha, NULL)
197:   IF system_box_active:
198:     screen(MAIN, 255, &system_box_rect)
199:   postprocess()
200:
201: NOTE: system-box sequencing is owned by C orchestration.
202: Rust screen() only needs to preserve clipped compositing semantics.
```

---

## PC-08: ReinitVideo (G7)

```text
210: IN handle_command() for ReinitVideo { driver, flags, width, height }:
211:   SAVE current_driver, current_flags, current_w, current_h
212:   CALL internal_gfx_uninit()
213:   LET result = internal_gfx_init(driver, flags, 0, width, height)
214:   IF result != 0:
215:     LOG "reinit failed, attempting reversion"
216:     LET revert = internal_gfx_init(current_driver, current_flags, 0, current_w, current_h)
217:     IF revert != 0:
218:       LOG "reversion also failed, exiting"
219:       CALL std::process::exit(1)
220:   REBIND any DCQ/render-state references to newly initialized surfaces/resources
```

---

## PC-09: Rotated Image Object Compatibility (G11)

```text
230: IDENTIFY existing C entry point implementing TFB_DrawImage_New_Rotated(img, angle)
231: IDENTIFY real ownership boundary where rotated TFB_Image object is allocated and returned
232:
233: FUNCTION create_rotated_canvas(source_canvas, angle_degrees) -> Result<Canvas>
234:   GET source pixels
235:   COMPUTE rotated dimensions from source extent and angle
236:   CREATE destination canvas at rotated dimensions
237:   FOR each destination pixel (dx, dy):
238:     COMPUTE source (sx, sy) via inverse rotation matrix
239:     IF (sx, sy) within source bounds:
240:       SAMPLE source pixel (nearest neighbor)
241:       WRITE to destination pixel
242:     ELSE:
243:       WRITE transparent pixel
244:   RETURN destination canvas
245:
246: FUNCTION create_rotated_image_object(source_image, angle_degrees) -> Result<TFB_Image-compatible object>
247:   CALL create_rotated_canvas(source_image.NormalImg, angle_degrees)
248:   ALLOCATE new image object through the real ABI-visible lifecycle path
249:   COPY / recompute hotspot, extent, and any derived cache fields required by contract
250:   INITIALIZE ownership fields so destruction APIs free all derived resources correctly
251:   RETURN new rotated image object through existing ABI-compatible caller path
252:
253: ONLY add a new FFI export if analysis proves an existing caller-backed ABI boundary requires it
```

---

## PC-10: C-Side Bridge Wiring + Revalidation (G8)

```text
260: IN sc2/src/libs/graphics/tfb_draw.c:
261:   FOR each TFB_DrawScreen_* function:
262:     ADD #ifdef USE_RUST_GFX
263:       EXTRACT exact existing caller-visible parameters/state
264:       CALL corresponding rust_dcq_push_* function with no semantic loss
265:       RETURN
266:     #else
267:       (existing C implementation)
268:     #endif
269:
270: IN sc2/src/libs/graphics/dcqueue.c:
271:   IN TFB_FlushGraphics():
272:     ADD #ifdef USE_RUST_GFX
273:       CALL rust_dcq_flush()
274:       RETURN
275:     #else
276:       (existing C flush loop)
277:     #endif
278:
279: IN actual graphics lifecycle owner (sdl_common.c and/or real init/uninit site):
280:   UNDER USE_RUST_GFX
281:     CALL rust_cmap_init()
282:     CALL rust_dcq_init()
283:   DURING shutdown
284:     CALL rust_dcq_uninit()
285:     CALL rust_cmap_uninit()
286:
287: DO NOT wire canvas.c by default
288:   ONLY modify canvas.c if analysis proves specific active call sites still bypass Rust DCQ/canvas ownership transfer
289:
290: AFTER C wiring lands:
291:   RE-RUN semantic verification for transition capture, extra-screen workflows, context-driven state propagation,
292:   batch nesting, deferred destruction ordering, and synchronization behavior through the real migrated path
```
