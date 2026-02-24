# Pseudocode: Component 008 — PixelCanvas Trait, LockedCanvas, PixelFormat

Plan ID: `PLAN-20260223-GFX-FULL-PORT`
Requirements: REQ-CANVAS-150, REQ-CANVAS-120, REQ-CANVAS-130,
              REQ-CANVAS-110, REQ-COMPAT-050

---

## 008A: PixelFormat Unified Enum

> Resolves the CanvasFormat vs PixmapFormat type mismatch. Both existing
> format types convert into PixelFormat via From impls. Drawing functions
> handle only one format enum.
> Reference: technical.md §8.4.0

```
 1: ENUM PixelFormat
 2:   Rgba32    // 4 bytes per pixel, [R, G, B, A] order
 3:   Rgbx32    // 4 bytes per pixel, [R, G, B, X] order (alpha ignored)
 4:   Indexed8  // 1 byte per pixel, palette-indexed
 5: END ENUM
 6:
 7: FUNCTION PixelFormat::bytes_per_pixel(self) -> usize
 8:   MATCH self
 9:     Rgba32  → RETURN 4
10:     Rgbx32  → RETURN 4
11:     Indexed8 → RETURN 1
12:   END MATCH
13: END FUNCTION
14:
15: // --- From conversions ---                                // REQ-CANVAS-150
16: IMPL From<CanvasFormat> FOR PixelFormat
17:   FUNCTION from(cf: CanvasFormat) -> PixelFormat
18:     MATCH cf.kind
19:       Rgba   → RETURN PixelFormat::Rgba32
20:       Rgbx   → RETURN PixelFormat::Rgbx32
21:       Indexed → RETURN PixelFormat::Indexed8
22:     END MATCH
23:   END FUNCTION
24: END IMPL
25:
26: IMPL From<PixmapFormat> FOR PixelFormat
27:   FUNCTION from(pf: PixmapFormat) -> PixelFormat
28:     MATCH pf
29:       PixmapFormat::Rgba32 → RETURN PixelFormat::Rgba32
30:       // Other PixmapFormat variants map as appropriate
31:       _                    → RETURN PixelFormat::Rgba32    // safe default
32:     END MATCH
33:   END FUNCTION
34: END IMPL
```

## 008B: PixelCanvas Trait

> The core abstraction enabling generic drawing functions across
> SurfaceCanvas, LockedCanvas, and any future canvas types.
> Reference: technical.md §8.4.0, REQ-CANVAS-150

```
 1: TRAIT PixelCanvas
 2:   FUNCTION width(&self) -> u32
 3:   FUNCTION height(&self) -> u32
 4:   FUNCTION pitch(&self) -> usize
 5:   FUNCTION pixels(&self) -> &[u8]
 6:   FUNCTION pixels_mut(&mut self) -> &mut [u8]
 7:   FUNCTION format(&self) -> PixelFormat
 8: END TRAIT
 9:
10: // --- Validation: all implementors must satisfy ---
11: // V1: pitch() >= width() * format().bytes_per_pixel()
12: // V2: pixels().len() >= pitch() * height() as usize
13: // V3: pixels_mut().len() == pixels().len()
14: // V4: format() returns a valid PixelFormat variant
```

## 008C: LockedCanvas Adapter

> Holds a MutexGuard on the existing Canvas's CanvasInner, bridging
> the Arc<Mutex<CanvasInner>> design to the PixelCanvas trait.
> Reference: technical.md §8.4.0a

```
 1: STRUCT LockedCanvas<'a>
 2:   guard: MutexGuard<'a, CanvasInner>
 3: END STRUCT
 4:
 5: IMPL PixelCanvas FOR LockedCanvas<'a>
 6:   FUNCTION width(&self) -> u32
 7:     RETURN self.guard.width
 8:   END FUNCTION
 9:
10:   FUNCTION height(&self) -> u32
11:     RETURN self.guard.height
12:   END FUNCTION
13:
14:   FUNCTION pitch(&self) -> usize
15:     RETURN self.guard.pitch
16:   END FUNCTION
17:
18:   FUNCTION pixels(&self) -> &[u8]
19:     RETURN &self.guard.data
20:   END FUNCTION
21:
22:   FUNCTION pixels_mut(&mut self) -> &mut [u8]
23:     RETURN &mut self.guard.data
24:   END FUNCTION
25:
26:   FUNCTION format(&self) -> PixelFormat
27:     RETURN self.guard.format.into()                         // CanvasFormat → PixelFormat
28:   END FUNCTION
29: END IMPL
30:
31: // --- Creation method on Canvas ---
32: IMPL Canvas
33:   FUNCTION lock_pixels(&self) -> LockedCanvas<'_>
34:     guard ← self.inner.lock()                               // Arc<Mutex<CanvasInner>>
35:     IF guard IS Err (poisoned) THEN
36:       guard ← poisoned.into_inner()                         // recover from poison
37:     END IF
38:     RETURN LockedCanvas { guard }
39:   END FUNCTION
40: END IMPL
```

## 008D: SurfaceCanvas (PixelCanvas Implementation)

> SurfaceCanvas wraps borrowed SDL_Surface pixel data. Unlike LockedCanvas,
> it implements PixelCanvas directly with no Mutex overhead.
> Reference: technical.md §8.4.1, §8.7

```
 1: STRUCT SurfaceCanvas<'a>
 2:   pixels: &'a mut [u8]                                     // borrowed surface->pixels
 3:   width: u32
 4:   height: u32
 5:   pitch: usize
 6:   format: PixelFormat                                       // always Rgbx32 for screens
 7:   scissor: ScissorRect
 8: END STRUCT
 9:
10: IMPL SurfaceCanvas<'a>
11:   // --- Construction (unsafe) ---
12:   FUNCTION from_locked(lock: &'a mut LockedSurface) -> Result<Self, GraphicsError>
13:     surf ← lock.surface
14:     // SAFETY: lock.surface is a valid SDL_Surface held under SDL_LockSurface
15:
16:     // --- Format validation ---                             // REQ-CANVAS-120
17:     IF surf.format.format != SDL_PIXELFORMAT_RGBX8888 THEN
18:       RETURN Err(GraphicsError::UnsupportedFormat)           // technical §8.7.5
19:     END IF
20:
21:     // --- Dimension validation ---
22:     IF surf.w <= 0 OR surf.h <= 0 THEN
23:       RETURN Err(GraphicsError::InvalidDimensions)
24:     END IF
25:
26:     // --- Pitch validation ---
27:     IF surf.pitch < surf.w * 4 THEN
28:       RETURN Err(GraphicsError::InvalidPitch)
29:     END IF
30:
31:     // --- Construct pixel slice ---
32:     pixel_len ← (surf.pitch AS usize) * (surf.h AS usize)
33:     IF pixel_len > isize::MAX AS usize THEN
34:       RETURN Err(GraphicsError::BufferTooLarge)
35:     END IF
36:
37:     pixels ← unsafe {
38:       // SAFETY: (a) surface locked via SDL_LockSurface,
39:       //         (b) pixel_len validated, (c) single-threaded per REQ-THR-010
40:       std::slice::from_raw_parts_mut(surf.pixels AS *mut u8, pixel_len)
41:     }
42:
43:     RETURN Ok(SurfaceCanvas {
44:       pixels,
45:       width: surf.w AS u32,
46:       height: surf.h AS u32,
47:       pitch: surf.pitch AS usize,
48:       format: PixelFormat::Rgbx32,
49:       scissor: ScissorRect::disabled(),
50:     })
51:   END FUNCTION
52:
53:   // --- Row-level access (technical §8.7.9) ---
54:   FUNCTION row_mut(&mut self, y: u32) -> &mut [u8]
55:     ASSERT y < self.height
56:     offset ← (y AS usize) * self.pitch
57:     row_bytes ← (self.width AS usize) * self.format.bytes_per_pixel()
58:     RETURN &mut self.pixels[offset .. offset + row_bytes]
59:   END FUNCTION
60: END IMPL
61:
62: IMPL PixelCanvas FOR SurfaceCanvas<'a>
63:   FUNCTION width(&self) -> u32    { RETURN self.width }
64:   FUNCTION height(&self) -> u32   { RETURN self.height }
65:   FUNCTION pitch(&self) -> usize  { RETURN self.pitch }
66:   FUNCTION pixels(&self) -> &[u8] { RETURN &self.pixels }
67:   FUNCTION pixels_mut(&mut self) -> &mut [u8] { RETURN &mut self.pixels }
68:   FUNCTION format(&self) -> PixelFormat { RETURN self.format }
69: END IMPL
```

## 008E: LockedSurface RAII Guard

> Ensures SDL_UnlockSurface is called when the guard is dropped.
> Reference: technical.md §8.7.2

```
 1: STRUCT LockedSurface<'a>
 2:   surface: &'a mut SDL_Surface
 3: END STRUCT
 4:
 5: IMPL LockedSurface<'a>
 6:   FUNCTION new(surface_ptr: *mut SDL_Surface) -> Self
 7:     // --- Validation ---
 8:     ASSERT surface_ptr IS NOT null
 9:     unsafe { SDL_LockSurface(surface_ptr) }
10:     RETURN LockedSurface { surface: unsafe { &mut *surface_ptr } }
11:   END FUNCTION
12:
13:   FUNCTION as_canvas(&mut self) -> Result<SurfaceCanvas<'_>, GraphicsError>
14:     RETURN SurfaceCanvas::from_locked(self)
15:   END FUNCTION
16: END IMPL
17:
18: IMPL Drop FOR LockedSurface<'_>
19:   FUNCTION drop(&mut self)
20:     unsafe { SDL_UnlockSurface(self.surface AS *mut SDL_Surface) }
21:   END FUNCTION
22: END IMPL
```

## 008F: Drawing Functions Become Generic

> Existing draw_line, fill_rect, etc. in tfb_draw.rs change from
> `fn draw_line(canvas: &mut Canvas, ...)` to generic form.
> Reference: technical.md §8.4.0, REQ-CANVAS-150

```
 1: // --- BEFORE (current signatures) ---
 2: // fn draw_line(canvas: &mut Canvas, x1: i32, y1: i32,
 3: //     x2: i32, y2: i32, color: Color) -> Result<(), CanvasError>
 4:
 5: // --- AFTER (generic over PixelCanvas) ---                 // REQ-CANVAS-150
 6: FUNCTION draw_line<C: PixelCanvas>(
 7:     canvas: &mut C, x1: i32, y1: i32,
 8:     x2: i32, y2: i32, color: Color
 9: ) -> Result<(), CanvasError>
10:   // Implementation unchanged — already operates on pixel slices
11:   // canvas.pixels_mut(), canvas.width(), canvas.pitch(), canvas.format()
12:   // replace direct field access
13: END FUNCTION
14:
15: FUNCTION fill_rect<C: PixelCanvas>(
16:     canvas: &mut C, x: i32, y: i32, w: i32, h: i32, color: Color
17: ) -> Result<(), CanvasError>
18:   // Implementation unchanged — pixel-level logic is format-agnostic
19: END FUNCTION
20:
21: FUNCTION copy_canvas<C: PixelCanvas, S: PixelCanvas>(
22:     dst: &mut C, src: &S, dst_x: i32, dst_y: i32,
23:     src_rect: Option<Rect>
24: ) -> Result<(), CanvasError>
25:   // Format mismatch check uses PixelFormat instead of CanvasFormat
26:   IF dst.format() != src.format() THEN
27:     RETURN Err(CanvasError::FormatMismatch)
28:   END IF
29:   // ... rest unchanged
30: END FUNCTION
```

### Validation Points
- 008A line 8–12: bytes_per_pixel returns correct BPP for each format
- 008C line 35–37: Poison recovery for MutexGuard
- 008D line 17–18: Format validation at construction (RGBX8888 only)
- 008D line 22–24: Dimension validation (positive w, h)
- 008D line 27–29: Pitch >= width * 4
- 008D line 33–35: pixel_len <= isize::MAX (from_raw_parts safety)
- 008E line 8: Null pointer assertion before lock

### Error Handling
- SurfaceCanvas::from_locked returns Result — construction can fail
- LockedCanvas creation: recovers from mutex poison
- Drawing functions return Result<(), CanvasError> (unchanged)
- LockedSurface::new asserts non-null (programming error = panic)

### Ordering Constraints
- LockedSurface MUST be created before SurfaceCanvas (SDL_LockSurface first)
- SurfaceCanvas MUST be dropped before LockedSurface (borrow scoping)
- LockedSurface Drop calls SDL_UnlockSurface (automatic via RAII)
- LockedCanvas guard MUST be held for entire draw operation, NOT per-pixel

### Integration Boundaries
- PixelCanvas trait defined in tfb_draw.rs (or new pixel_canvas.rs module)
- LockedCanvas defined alongside Canvas in tfb_draw.rs
- SurfaceCanvas defined in new surface_canvas.rs module
- LockedSurface defined in surface_canvas.rs (uses SDL FFI)
- Drawing functions in tfb_draw.rs become generic `<C: PixelCanvas>`
- Existing Canvas type remains unchanged; 40+ existing tests unaffected  // REQ-CANVAS-150
- SurfaceCanvas is !Send + !Sync (raw pointer field)                    // technical §8.7.6

### Side Effects
- SDL_LockSurface/SDL_UnlockSurface bracket the flush scope
- SurfaceCanvas pixel writes immediately visible to presentation layer   // REQ-CANVAS-130
- No copies: drawing goes directly into SDL_Surface->pixels
