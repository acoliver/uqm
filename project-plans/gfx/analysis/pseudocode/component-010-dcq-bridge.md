# Pseudocode: Component 010 — DCQ FFI Bridge

Plan ID: `PLAN-20260223-GFX-FULL-PORT`
Requirements: REQ-DCQ-010..190, REQ-DCQ-130..170,
              REQ-COMPAT-010, REQ-COMPAT-080, REQ-GUARD-010, REQ-GUARD-020,
              REQ-GUARD-120, REQ-FFI-030, REQ-FFI-040

---

## 010A: Global DCQ Singleton

> Single DrawCommandQueue instance shared across all FFI and Rust callers.
> Reference: technical.md §8.8, REQ-DCQ-150

```
 1: STATIC GLOBAL_DCQ: OnceLock<DrawCommandQueue> = OnceLock::new()
 2:
 3: FUNCTION global_dcq() -> &'static DrawCommandQueue
 4:   RETURN GLOBAL_DCQ.get_or_init(|| {
 5:     context ← Arc::new(RwLock::new(RenderContext::new()))
 6:     config ← DcqConfig::standard()                          // max_commands, livelock_max
 7:     DrawCommandQueue::with_config(config, context)
 8:   })
 9: END FUNCTION
10:
11: // --- Integration with rust_gfx_init ---
12: // Init ensures the DCQ exists before any commands are enqueued.
13: // The OnceLock lazy init handles this, but explicit init is preferred:
14: FUNCTION rust_dcq_init()
15:   global_dcq()                                               // force initialization
16: END FUNCTION
```

## 010B: DCQ Enqueue FFI Exports (15 Commands)

> Each export converts C types → Rust types and pushes to the Rust DCQ.
> These replace TFB_DrawScreen_* functions in tfb_draw.c.
> Reference: technical.md §8.2, functional.md §12

```
 1: // --- [0] Line ---                                          // REQ-DCQ-010
 2: #[no_mangle]
 3: FUNCTION rust_dcq_push_drawline(
 4:     x1: c_int, y1: c_int, x2: c_int, y2: c_int,
 5:     color: FfiColor, draw_mode: FfiDrawMode, dest: c_int
 6: )
 7:   cmd ← DrawCommand::Line {
 8:     x1, y1, x2, y2,
 9:     color: Color::from(color),
10:     draw_mode: DrawMode::from(draw_mode),
11:     dest: Screen::from_c(dest),
12:   }
13:   global_dcq().push(cmd)                                     // blocks if full (REQ-DCQ-180)
14: END FUNCTION
15:
16: // --- [1] Rectangle ---                                     // REQ-DCQ-020
17: #[no_mangle]
18: FUNCTION rust_dcq_push_drawrect(
19:     rect: *const FfiRect,
20:     color: FfiColor, draw_mode: FfiDrawMode, dest: c_int
21: )
22:   IF rect IS null THEN RETURN                                // defensive
23:   r ← unsafe { &*rect }
24:   cmd ← DrawCommand::Rect {
25:     rect: Rect { corner: Point { x: r.x, y: r.y },
26:                   extent: Extent { width: r.w, height: r.h } },
27:     color: Color::from(color),
28:     draw_mode: DrawMode::from(draw_mode),
29:     dest: Screen::from_c(dest),
30:   }
31:   global_dcq().push(cmd)
32: END FUNCTION
33:
34: // --- [2] Image ---                                         // REQ-DCQ-030
35: #[no_mangle]
36: FUNCTION rust_dcq_push_drawimage(
37:     img: *mut FfiTfbImage, x: c_int, y: c_int,
38:     scale: c_int, scalemode: c_int, colormap: *mut c_void,
39:     draw_mode: FfiDrawMode, dest: c_int
40: )
41:   IF img IS null THEN RETURN
42:   image_ref ← ImageRef::from_raw(img)                       // REQ-COMPAT-090
43:   cmap_ref ← IF colormap IS NOT null THEN
44:     Some(ColorMapRef::from_raw(colormap))
45:   ELSE None END IF
46:
47:   cmd ← DrawCommand::Image {
48:     image: image_ref,
49:     x, y, scale, scalemode,
50:     colormap: cmap_ref,
51:     draw_mode: DrawMode::from(draw_mode),
52:     dest: Screen::from_c(dest),
53:   }
54:   global_dcq().push(cmd)
55: END FUNCTION
56:
57: // --- [3] FilledImage ---                                   // REQ-DCQ-040
58: #[no_mangle]
59: FUNCTION rust_dcq_push_drawfilledimage(
60:     img: *mut FfiTfbImage, x: c_int, y: c_int,
61:     scale: c_int, scalemode: c_int,
62:     color: FfiColor, draw_mode: FfiDrawMode, dest: c_int
63: )
64:   IF img IS null THEN RETURN
65:   image_ref ← ImageRef::from_raw(img)
66:   cmd ← DrawCommand::FilledImage {
67:     image: image_ref,
68:     x, y, scale, scalemode,
69:     color: Color::from(color),
70:     draw_mode: DrawMode::from(draw_mode),
71:     dest: Screen::from_c(dest),
72:   }
73:   global_dcq().push(cmd)
74: END FUNCTION
75:
76: // --- [4] FontChar ---                                      // REQ-DCQ-050
77: #[no_mangle]
78: FUNCTION rust_dcq_push_drawfontchar(
79:     char_ptr: *mut FfiTfbChar,
80:     backing_ptr: *mut FfiTfbImage,
81:     x: c_int, y: c_int,
82:     draw_mode: FfiDrawMode, dest: c_int
83: )
84:   IF char_ptr IS null THEN RETURN
85:   char_ref ← FontCharRef::from_raw(char_ptr)
86:   backing_ref ← IF backing_ptr IS NOT null THEN
87:     Some(ImageRef::from_raw(backing_ptr))
88:   ELSE None END IF
89:
90:   cmd ← DrawCommand::FontChar {
91:     font_char: char_ref,
92:     backing: backing_ref,
93:     x, y,
94:     draw_mode: DrawMode::from(draw_mode),
95:     dest: Screen::from_c(dest),
96:   }
97:   global_dcq().push(cmd)
98: END FUNCTION
99:
100: // --- [5] Copy ---                                          // REQ-DCQ-060
101: #[no_mangle]
102: FUNCTION rust_dcq_push_copy(
103:     rect: *const FfiRect,
104:     src_screen: c_int, dst_screen: c_int
105: )
106:   IF rect IS null THEN RETURN
107:   r ← unsafe { &*rect }
108:   cmd ← DrawCommand::Copy {
109:     rect: Rect::from_ffi(r),
110:     src: Screen::from_c(src_screen),
111:     dest: Screen::from_c(dst_screen),
112:   }
113:   global_dcq().push(cmd)
114: END FUNCTION
115:
116: // --- [6] CopyToImage ---                                   // REQ-DCQ-070
117: #[no_mangle]
118: FUNCTION rust_dcq_push_copytoimage(
119:     img: *mut FfiTfbImage,
120:     rect: *const FfiRect,
121:     src_screen: c_int
122: )
123:   IF img IS null OR rect IS null THEN RETURN
124:   image_ref ← ImageRef::from_raw(img)
125:   r ← unsafe { &*rect }
126:   cmd ← DrawCommand::CopyToImage {
127:     image: image_ref,
128:     rect: Rect::from_ffi(r),
129:     src: Screen::from_c(src_screen),
130:   }
131:   global_dcq().push(cmd)
132: END FUNCTION
133:
134: // --- [7] ScissorEnable ---                                 // REQ-CANVAS-080
135: #[no_mangle]
136: FUNCTION rust_dcq_push_scissor_enable(
137:     rect: *const FfiRect, dest: c_int
138: )
139:   IF rect IS null THEN RETURN
140:   r ← unsafe { &*rect }
141:   cmd ← DrawCommand::ScissorEnable {
142:     rect: Rect::from_ffi(r),
143:     dest: Screen::from_c(dest),
144:   }
145:   global_dcq().push(cmd)
146: END FUNCTION
147:
148: // --- [8] ScissorDisable ---
149: #[no_mangle]
150: FUNCTION rust_dcq_push_scissor_disable(dest: c_int)
151:   cmd ← DrawCommand::ScissorDisable {
152:     dest: Screen::from_c(dest),
153:   }
154:   global_dcq().push(cmd)
155: END FUNCTION
156:
157: // --- [9] SetMipmap ---                                     // REQ-DCQ-080
158: #[no_mangle]
159: FUNCTION rust_dcq_push_setmipmap(
160:     img: *mut FfiTfbImage,
161:     mipmap: *mut FfiTfbImage,
162:     hs_x: c_int, hs_y: c_int
163: )
164:   IF img IS null THEN RETURN
165:   cmd ← DrawCommand::SetMipmap {
166:     image: ImageRef::from_raw(img),
167:     mipmap: IF mipmap IS NOT null THEN Some(ImageRef::from_raw(mipmap)) ELSE None,
168:     hot_spot: HotSpot { x: hs_x, y: hs_y },
169:   }
170:   global_dcq().push(cmd)
171: END FUNCTION
172:
173: // --- [10] DeleteImage ---                                  // REQ-DCQ-090
174: #[no_mangle]
175: FUNCTION rust_dcq_push_deleteimage(img: *mut FfiTfbImage)
176:   IF img IS null THEN RETURN
177:   cmd ← DrawCommand::DeleteImage {
178:     image: ImageRef::from_raw(img),
179:   }
180:   global_dcq().push(cmd)
181: END FUNCTION
182:
183: // --- [11] DeleteData ---
184: #[no_mangle]
185: FUNCTION rust_dcq_push_deletedata(data: *mut c_void)
186:   IF data IS null THEN RETURN
187:   cmd ← DrawCommand::DeleteData {
188:     data: DataRef::from_raw(data),
189:   }
190:   global_dcq().push(cmd)
191: END FUNCTION
192:
193: // --- [12] SendSignal (WaitForSignal) ---                   // REQ-DCQ-100
194: #[no_mangle]
195: FUNCTION rust_dcq_push_waitsignal()
196:   signal ← Arc::new(AtomicBool::new(false))
197:   signal_clone ← Arc::clone(&signal)
198:
199:   cmd ← DrawCommand::SendSignal { signal: signal_clone }
200:   global_dcq().push(cmd)
201:
202:   // --- Block until consumer sets the signal ---
203:   WHILE NOT signal.load(Ordering::Acquire):
204:     std::thread::yield_now()
205:   END WHILE
206: END FUNCTION
207:
208: // --- [13] ReinitVideo ---                                  // REQ-DCQ-110
209: #[no_mangle]
210: FUNCTION rust_dcq_push_reinitvideo(
211:     driver: c_int, flags: c_int,
212:     width: c_int, height: c_int
213: )
214:   cmd ← DrawCommand::ReinitVideo {
215:     driver, flags, width, height,
216:   }
217:   global_dcq().push(cmd)
218: END FUNCTION
219:
220: // --- [14] Callback ---                                     // REQ-DCQ-120
221: #[no_mangle]
222: FUNCTION rust_dcq_push_callback(
223:     func: extern "C" fn(*mut c_void),
224:     arg: *mut c_void
225: )
226:   cmd ← DrawCommand::Callback {
227:     func: CallbackFn::from_raw(func),
228:     arg: CallbackArg::from_raw(arg),
229:   }
230:   global_dcq().push(cmd)
231: END FUNCTION
```

## 010C: DCQ Flush and Command Dispatch

> The flush function processes all pending DCQ commands. Called from
> TFB_FlushGraphics in place of the C command loop.
> Reference: technical.md §8.3, functional.md §12.5

```
 1: // --- Flush entry point ---                                 // REQ-DCQ-130
 2: #[no_mangle]
 3: FUNCTION rust_dcq_flush_graphics()
 4:   dcq ← global_dcq()
 5:   result ← dcq.process_commands()
 6:   IF result IS Err THEN
 7:     LOG "DCQ flush error: {result}"
 8:   END IF
 9: END FUNCTION
10:
11: // --- Command dispatch (9 drawing arms) ---                 // REQ-DCQ-140
12: // This is the internal handle_command match that dispatches
13: // each command to the appropriate drawing function.
14: // The "9 arms" refers to the 9 drawing/canvas commands (types 0-8).
15: // Types 9-14 are lifecycle/control commands handled separately.
16:
17: FUNCTION handle_command(cmd: DrawCommand, context: &mut RenderContext)
18:   MATCH cmd
19:     // --- Drawing commands (operate on canvas) ---
20:
21:     DrawCommand::Line { x1, y1, x2, y2, color, draw_mode, dest } →
22:       canvas ← get_screen_canvas(context, dest)              // SurfaceCanvas or LockedCanvas
23:       IF canvas IS Err THEN RETURN                            // REQ-COMPAT-080
24:       draw_line(&mut canvas, x1, y1, x2, y2, color)         // component-008/009
25:
26:     DrawCommand::Rect { rect, color, draw_mode, dest } →
27:       canvas ← get_screen_canvas(context, dest)
28:       IF canvas IS Err THEN RETURN
29:       // draw_mode.kind determines outline vs fill
30:       IF draw_mode.kind == DrawKind::Replace THEN
31:         fill_rect(&mut canvas, rect.x(), rect.y(),
32:                   rect.width(), rect.height(), color)
33:       ELSE
34:         draw_rect(&mut canvas, rect.x(), rect.y(),
35:                   rect.width(), rect.height(), color)
36:       END IF
37:
38:     DrawCommand::Image { image, x, y, scale, scalemode, colormap,
39:                          draw_mode, dest } →
40:       canvas ← get_screen_canvas(context, dest)
41:       IF canvas IS Err THEN RETURN
42:       img ← context.get_image(image)                        // REQ-GFXLOAD-040
43:       IF img IS None THEN
44:         LOG_ONCE "Image not found in registry: {image}"
45:         RETURN                                               // REQ-COMPAT-080
46:       END IF
47:       draw_scaled_image(&mut canvas, img, x, y, scale, scalemode,
48:                         draw_mode)
49:
50:     DrawCommand::FilledImage { image, x, y, scale, scalemode,
51:                                color, draw_mode, dest } →
52:       canvas ← get_screen_canvas(context, dest)
53:       IF canvas IS Err THEN RETURN
54:       img ← context.get_image(image)
55:       IF img IS None THEN RETURN
56:       draw_filled_image(&mut canvas, img, x, y, scale, color, draw_mode)
57:
58:     DrawCommand::FontChar { font_char, backing, x, y,
59:                             draw_mode, dest } →
60:       canvas ← get_screen_canvas(context, dest)
61:       IF canvas IS Err THEN RETURN
62:       tf_char ← context.get_fontchar(font_char)
63:       IF tf_char IS None THEN RETURN
64:       backing_img ← IF backing IS Some(ref) THEN context.get_image(ref) ELSE None
65:       draw_fontchar(&mut canvas, tf_char, backing_img, x, y, draw_mode)
66:
67:     DrawCommand::Copy { rect, src, dest } →
68:       // --- Self-blit check ---                              // technical §8.7.8
69:       IF src == dest THEN
70:         canvas ← get_screen_canvas(context, dest)
71:         IF canvas IS Err THEN RETURN
72:         self_blit_safe(&mut canvas, rect.to_src(), rect.to_dst())
73:       ELSE
74:         src_canvas ← get_screen_canvas_readonly(context, src)
75:         dst_canvas ← get_screen_canvas(context, dest)
76:         IF src_canvas IS Err OR dst_canvas IS Err THEN RETURN
77:         copy_canvas(&mut dst_canvas, &src_canvas, rect)
78:       END IF
79:
80:     DrawCommand::CopyToImage { image, rect, src } →
81:       src_canvas ← get_screen_canvas_readonly(context, src)
82:       IF src_canvas IS Err THEN RETURN
83:       img ← context.get_image_mut(image)
84:       IF img IS None THEN RETURN
85:       img_canvas ← img.lock_primary_canvas()                // LockedCanvas for image
86:       copy_canvas(&mut img_canvas, &src_canvas, rect)
87:
88:     DrawCommand::ScissorEnable { rect, dest } →
89:       canvas ← get_screen_canvas(context, dest)
90:       IF canvas IS Err THEN RETURN
91:       canvas.set_scissor(rect.x(), rect.y(), rect.width(), rect.height())
92:       canvas.enable_scissor()
93:
94:     DrawCommand::ScissorDisable { dest } →
95:       canvas ← get_screen_canvas(context, dest)
96:       IF canvas IS Err THEN RETURN
97:       canvas.disable_scissor()
98:
99:     // --- Lifecycle/control commands (no canvas ops) ---
100:
101:    DrawCommand::SetMipmap { image, mipmap, hot_spot } →
102:      img ← context.get_image_mut(image)
103:      IF img IS None THEN RETURN
104:      img.set_mipmap(mipmap, hot_spot)
105:
106:    DrawCommand::DeleteImage { image } →
107:      context.unregister_image(image)
108:
109:    DrawCommand::DeleteData { data } →
110:      // Free the associated data
111:      data.free()
112:
113:    DrawCommand::SendSignal { signal } →
114:      signal.store(true, Ordering::Release)                   // wake waiting thread
115:
116:    DrawCommand::ReinitVideo { driver, flags, width, height } →
117:      rust_gfx_uninit()
118:      result ← rust_gfx_init(driver, flags, ptr::null(), width, height)
119:      IF result != 0 THEN
120:        LOG "Video reinit failed"
121:      END IF
122:      // TFB_SwapBuffers called after reinit per functional §12.1
123:
124:    DrawCommand::Callback { func, arg } →
125:      // SAFETY: func is a C function pointer, arg is a C void*
126:      // Both provided by C caller and valid for this call
127:      unsafe { func.call(arg.as_ptr()) }
128:
129:    _ →
130:      LOG_ONCE "Unimplemented command type"                   // REQ-COMPAT-080
131:      // Skip unknown command, do not crash
132:  END MATCH
133: END FUNCTION
```

## 010D: Batch/Unbatch FFI Exports

> Reference: REQ-DCQ-160, REQ-DCQ-170, functional.md §12.5

```
 1: #[no_mangle]
 2: FUNCTION rust_dcq_batch_graphics()                           // REQ-DCQ-160
 3:   global_dcq().batch()
 4: END FUNCTION
 5:
 6: #[no_mangle]
 7: FUNCTION rust_dcq_unbatch_graphics()                         // REQ-DCQ-170
 8:   global_dcq().unbatch()
 9: END FUNCTION
```

## 010E: Screen Canvas Resolution for DCQ

> Maps Screen enum to the correct canvas type during command dispatch.

```
 1: FUNCTION get_screen_canvas(context: &mut RenderContext,
 2:     screen: Screen) -> Result<impl PixelCanvas, GraphicsError>
 3:   state ← get_gfx_state()
 4:   IF state IS None THEN
 5:     RETURN Err(GraphicsError::NotInitialized)
 6:   END IF
 7:
 8:   surface_ptr ← state.surfaces[screen.index()]
 9:   IF surface_ptr IS null THEN
10:     RETURN Err(GraphicsError::NullSurface)
11:   END IF
12:
13:   // SAFETY: surface locked at start of flush scope (component-009D)
14:   // Surface remains valid for duration of flush
15:   canvas ← unsafe { SurfaceCanvas::from_raw_locked(surface_ptr) }
16:   RETURN Ok(canvas)
17: END FUNCTION
```

### Validation Points
- 010A line 4–8: OnceLock lazy init with standard config
- 010B: Every enqueue function checks null pointers before unsafe deref
- 010B line 203: Signal wait loop with Acquire ordering
- 010C line 6–8: Flush error handling
- 010C line 23, 43–46: Missing canvas/image → log and skip (REQ-COMPAT-080)
- 010C line 69: Self-blit detection for same-screen copy
- 010C line 129–131: Unknown command type → log and skip
- 010E line 4–6: State initialization check

### Error Handling
- Null C pointers: silent return on enqueue (no crash)                   // REQ-FFI-030
- Missing image/font: LOG_ONCE + skip command                            // REQ-COMPAT-080
- Canvas construction failure: skip command, continue with next
- Unknown command type: log_once + skip (graceful degradation)
- Flush error: log but do not crash
- Signal wait: spin loop (matches C semantics)                           // REQ-DCQ-180

### Ordering Constraints
- global_dcq() MUST be initialized before any push() call
- push() MUST convert C types → Rust types BEFORE pushing                // REQ-COMPAT-060
- flush MUST be called from main/graphics thread only                    // REQ-THR-010
- ReinitVideo dispatch MUST call uninit before init (line 117–118)
- SendSignal MUST store(true) AFTER all preceding commands processed
- Batch/Unbatch pair: batch BEFORE a sequence, unbatch AFTER              // REQ-DCQ-160/170
- EXACTLY ONE DCQ active at a time (C or Rust, never both)               // REQ-COMPAT-010

### Integration Boundaries
- Exported: 15 enqueue functions + flush + batch + unbatch = 18 symbols
- Replaces: tfb_draw.c (493 lines, REQ-GUARD-020), dcqueue.c command loop
  (REQ-GUARD-010), gfx_common.c Batch/UnbatchGraphics (REQ-GUARD-120)
- Uses: DrawCommandQueue (dcqueue.rs), RenderContext (render_context.rs)
- Uses: PixelCanvas trait and SurfaceCanvas (component-008, component-009)
- Called from: C game code (tfb_prim.c, frame.c, font.c, widgets.c)
  and C flush (TFB_FlushGraphics in dcqueue.c)

### Side Effects
- Push: adds command to queue (may block if full)
- Flush: processes all pending commands, modifies screen surface pixels
- SendSignal: wakes waiting thread via AtomicBool
- ReinitVideo: destroys and recreates entire graphics state
- Callback: executes arbitrary C function pointer
- Batch: suppresses automatic flush
- Unbatch: triggers deferred flush
