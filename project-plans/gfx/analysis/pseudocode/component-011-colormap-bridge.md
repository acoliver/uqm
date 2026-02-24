# Pseudocode: Component 011 — Colormap FFI Bridge

Plan ID: `PLAN-20260223-GFX-FULL-PORT`
Requirements: REQ-CMAP-010..080, REQ-GUARD-100, REQ-GUARD-150,
              REQ-FFI-030, REQ-FFI-040, REQ-COMPAT-060

---

## 011A: Global ColorMapManager Singleton

> The Rust ColorMapManager replaces the C colormap pool in cmap.c.
> Must be accessible from both FFI and Rust-native callers.
> Reference: functional.md §14, technical.md §8.6

```
 1: STATIC GLOBAL_CMAP: OnceLock<Mutex<ColorMapManager>> = OnceLock::new()
 2:
 3: FUNCTION global_cmap() -> &'static Mutex<ColorMapManager>
 4:   RETURN GLOBAL_CMAP.get_or_init(|| {
 5:     Mutex::new(ColorMapManager::new())
 6:   })
 7: END FUNCTION
 8:
 9: // --- FadeController is separate (manages fade_amount global) ---
10: STATIC GLOBAL_FADE: OnceLock<Mutex<FadeController>> = OnceLock::new()
11:
12: FUNCTION global_fade() -> &'static Mutex<FadeController>
13:   RETURN GLOBAL_FADE.get_or_init(|| {
14:     Mutex::new(FadeController::new())
15:   })
16: END FUNCTION
```

## 011B: InitColorMaps FFI Export

> Called once during game startup. Initializes the colormap pool.
> Reference: REQ-CMAP-010, functional.md §14.1

```
 1: #[no_mangle]
 2: FUNCTION rust_cmap_init()                                    // REQ-CMAP-010
 3:   mgr ← global_cmap().lock()
 4:   IF mgr IS Err (poisoned) THEN
 5:     mgr ← poisoned.into_inner()                             // recover from poison
 6:   END IF
 7:
 8:   // --- Initialize pool ---
 9:   mgr.init_pool(MAX_COLORMAPS + SPARE_COLORMAPS)            // 250 + 20 = 270
10:   // MAX_COLORMAPS = 250, SPARE_COLORMAPS = 20              // matches C cmap.c
11:
12:   // --- Initialize fade controller ---
13:   fade ← global_fade().lock()
14:   IF fade IS Err (poisoned) THEN
15:     fade ← poisoned.into_inner()
16:   END IF
17:   fade.reset()
18:   // fade_amount starts at FADE_NORMAL_INTENSITY (255)       // fully visible
19: END FUNCTION
```

## 011C: SetColorMap FFI Export

> Called when game code changes the active colormap on a drawable.
> Reference: REQ-CMAP-020, functional.md §14.1

```
 1: #[no_mangle]
 2: FUNCTION rust_cmap_set(cmap_ptr: *const c_void)             // REQ-CMAP-020
 3:   IF cmap_ptr IS null THEN RETURN
 4:
 5:   mgr ← global_cmap().lock()
 6:   IF mgr IS Err (poisoned) THEN
 7:     mgr ← poisoned.into_inner()
 8:   END IF
 9:
10:   // --- Decode colormap data from C pointer ---
11:   // SAFETY: cmap_ptr is a valid COLORMAPPTR from C game code
12:   // C format: COLORMAPPTR is a packed byte array encoding colormap type + data
13:   cmap_data ← unsafe {
14:     // Decode header: first 2 bytes are colormap type identifier
15:     let header = std::slice::from_raw_parts(cmap_ptr AS *const u8, 2)
16:     (header[0], header[1])
17:   }
18:
19:   // --- Apply colormap ---
20:   MATCH cmap_data
21:     (COLORMAP_TYPE_PALETTE, _) →
22:       // Full palette replacement
23:       palette ← decode_palette(cmap_ptr)
24:       mgr.set_palette(palette)
25:
26:     (COLORMAP_TYPE_XFORM, xform_index) →
27:       // Colormap transform (used for color cycling, etc.)
28:       xform ← decode_xform(cmap_ptr)
29:       mgr.set_xform(xform_index AS usize, xform)
30:
31:     _ →
32:       LOG_ONCE "Unknown colormap type: {cmap_data.0}"
33:   END MATCH
34: END FUNCTION
```

## 011D: FadeScreen FFI Export

> Initiates a screen fade (to black, white, or color).
> Sets fade_amount that the presentation layer reads.
> Reference: REQ-CMAP-030, REQ-CMAP-050, functional.md §14.4

```
 1: #[no_mangle]
 2: FUNCTION rust_cmap_fade_screen(                              // REQ-CMAP-030
 3:     fade_type: c_int, seconds: c_int
 4: )
 5:   fade ← global_fade().lock()
 6:   IF fade IS Err (poisoned) THEN
 7:     fade ← poisoned.into_inner()
 8:   END IF
 9:
10:   // --- Validate fade_type ---
11:   // C defines: FADE_NO_INTENSITY = 0, FADE_NORMAL_INTENSITY = 255,
12:   //            FADE_FULL_INTENSITY = 510
13:   // fade_type encodes target intensity
14:   IF fade_type < 0 OR fade_type > FADE_FULL_INTENSITY THEN
15:     LOG "Invalid fade_type: {fade_type}"
16:     RETURN
17:   END IF
18:
19:   // --- Validate seconds ---
20:   IF seconds < 0 THEN
21:     LOG "Invalid fade seconds: {seconds}"
22:     RETURN
23:   END IF
24:
25:   // --- Start fade ---
26:   fade.start_fade(fade_type, seconds)                       // REQ-CMAP-050
27:   // fade_amount will be updated on each step
28:   // Range: 0 (no intensity) → 255 (normal) → 510 (full/white)
29:
30:   // --- Immediate mode (seconds == 0) ---
31:   IF seconds == 0 THEN
32:     fade.set_immediate(fade_type)
33:     // Update the global fade_amount immediately
34:     update_fade_amount_global(fade.current_amount())
35:   END IF
36: END FUNCTION
37:
38: // --- Update C-visible fade_amount global ---
39: FUNCTION update_fade_amount_global(amount: i32)
40:   // The C presentation layer reads `fade_amount` from a C global variable
41:   // in gfx_common.c. We must update it so TFB_SwapBuffers uses the
42:   // correct value.
43:   extern "C" { static mut fade_amount: c_int; }
44:   unsafe { fade_amount = amount; }                           // REQ-CMAP-050
45: END FUNCTION
```

## 011E: FlushFadeXForms FFI Export

> Called from the main loop to step all active fade/colormap transforms.
> Reference: REQ-CMAP-040, REQ-CMAP-060, functional.md §14.1

```
 1: #[no_mangle]
 2: FUNCTION rust_cmap_flush_xforms()                            // REQ-CMAP-040
 3:   fade ← global_fade().lock()
 4:   IF fade IS Err (poisoned) THEN
 5:     fade ← poisoned.into_inner()
 6:   END IF
 7:
 8:   // --- Step fade controller ---
 9:   IF fade.is_active() THEN
10:     fade.step()                                              // advance fade_amount
11:     update_fade_amount_global(fade.current_amount())
12:   END IF
13:
14:   // --- Step colormap transforms ---
15:   mgr ← global_cmap().lock()
16:   IF mgr IS Err (poisoned) THEN
17:     mgr ← poisoned.into_inner()
18:   END IF
19:
20:   // Process up to MAX_XFORMS active transforms              // REQ-CMAP-060
21:   FOR i IN 0..MAX_XFORMS:
22:     xform ← mgr.get_xform(i)
23:     IF xform IS None OR NOT xform.is_active() THEN CONTINUE
24:
25:     // --- Step this transform ---
26:     xform.step()                                             // interpolate palette
27:
28:     IF xform.is_complete() THEN
29:       xform.deactivate()
30:     END IF
31:   END FOR
32: END FUNCTION
```

## 011F: XFormColorMap_step FFI Export

> Single step of a specific colormap transform. Called per-frame
> for palette cycling effects.
> Reference: functional.md §14.1

```
 1: #[no_mangle]
 2: FUNCTION rust_cmap_xform_step() -> c_int
 3:   mgr ← global_cmap().lock()
 4:   IF mgr IS Err (poisoned) THEN
 5:     mgr ← poisoned.into_inner()
 6:   END IF
 7:
 8:   active_count ← 0
 9:   FOR i IN 0..MAX_XFORMS:                                   // REQ-CMAP-060
10:     xform ← mgr.get_xform(i)
11:     IF xform IS None OR NOT xform.is_active() THEN CONTINUE
12:     xform.step()
13:     IF xform.is_active() THEN
14:       active_count ← active_count + 1
15:     END IF
16:   END FOR
17:
18:   RETURN active_count                                        // 0 = all transforms complete
19: END FUNCTION
```

## 011G: GetColorMapAddress FFI Export

> Returns a pointer to colormap data by index.
> Reference: REQ-CMAP-080

```
 1: #[no_mangle]
 2: FUNCTION rust_cmap_get(index: c_int) -> *const c_void       // REQ-CMAP-080
 3:   IF index < 0 OR index >= MAX_COLORMAPS THEN
 4:     RETURN null                                              // out of range
 5:   END IF
 6:
 7:   mgr ← global_cmap().lock()
 8:   IF mgr IS Err (poisoned) THEN
 9:     mgr ← poisoned.into_inner()
10:   END IF
11:
12:   cmap ← mgr.get(index AS usize)
13:   IF cmap IS None THEN
14:     RETURN null
15:   END IF
16:
17:   // Return pointer to the colormap's internal data
18:   // SAFETY: data is owned by ColorMapManager, valid until unregistered
19:   RETURN cmap.as_ptr() AS *const c_void
20: END FUNCTION
```

## 011H: NativePalette Interop

> The palette type storing 256 color entries.
> Reference: REQ-CMAP-070

```
 1: STRUCT NativePalette                                         // REQ-CMAP-070
 2:   entries: [PaletteEntry; NUMBER_OF_PLUTVALS]                // 256 entries
 3: END STRUCT
 4:
 5: STRUCT PaletteEntry
 6:   r: u8
 7:   g: u8
 8:   b: u8
 9:   a: u8
10: END STRUCT
11:
12: CONST NUMBER_OF_PLUTVALS: usize = 256
13: CONST MAX_COLORMAPS: usize = 250
14: CONST SPARE_COLORMAPS: usize = 20
15: CONST MAX_XFORMS: usize = 16
16: CONST FADE_NO_INTENSITY: i32 = 0
17: CONST FADE_NORMAL_INTENSITY: i32 = 255
18: CONST FADE_FULL_INTENSITY: i32 = 510
```

### Validation Points
- 011B line 3–6: Mutex poison recovery
- 011B line 9: Pool size matches C constants (250 + 20)
- 011C line 3: Null pointer check on colormap data
- 011D line 14–17: fade_type range validation (0–510)
- 011D line 20–23: Negative seconds rejection
- 011E line 9: Active check before stepping fade
- 011E line 23: Active check before stepping each xform
- 011F line 9: MAX_XFORMS upper bound on iteration
- 011G line 3–5: Index range check before lookup

### Error Handling
- All FFI functions: null pointer → silent return                        // REQ-FFI-030
- Mutex poison: recover via into_inner() (never panic in FFI)
- Invalid fade_type/seconds: log + return (no crash)
- Out-of-range colormap index: return null
- Unknown colormap type: LOG_ONCE + skip

### Ordering Constraints
- rust_cmap_init MUST be called before any other cmap function
- FadeScreen MUST update fade_amount BEFORE next TFB_SwapBuffers call
- FlushFadeXForms: fade step BEFORE xform steps (fade takes priority)
- GetColorMapAddress: returned pointer valid only until colormap freed
- Mutex lock ordering: always lock GLOBAL_FADE before GLOBAL_CMAP
  (when both needed) to prevent deadlock
- update_fade_amount_global MUST match C global's expected range (0–510)

### Integration Boundaries
- Exported: 7 FFI symbols (init, set, fade_screen, flush_xforms,
  xform_step, get, palette interop)
- Replaces: cmap.c (663 lines, REQ-GUARD-100), palette.c (REQ-GUARD-150)
- Called from: C game code throughout codebase
  (InitColorMaps, SetColorMap, FadeScreen, FlushFadeXForms, etc.)
- Writes to: C global `fade_amount` (read by TFB_SwapBuffers/presentation)
- Uses: ColorMapManager, FadeController, NativePalette (cmap.rs, 774 lines)
- Thread model: game threads call Set/Fade; main thread calls Flush
  → Mutex required (unlike DCQ which is main-thread-only)

### Side Effects
- rust_cmap_init: allocates colormap pool (270 entries)
- rust_cmap_set: modifies active palette/xform state
- rust_cmap_fade_screen: starts fade animation, may set fade_amount immediately
- rust_cmap_flush_xforms: advances fade_amount, steps active transforms
- update_fade_amount_global: writes to C global variable (unsafe)
