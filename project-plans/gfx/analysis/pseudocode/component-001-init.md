# Pseudocode: Component 001 — Initialization & Teardown

Plan ID: `PLAN-20260223-GFX-VTABLE-FIX`
Requirements: REQ-INIT-010..100, REQ-UNINIT-010..030, REQ-INIT-095, REQ-INIT-096, REQ-INIT-097

---

## 001A: rust_gfx_init

> Note: Init is largely already implemented. This pseudocode documents the
> existing behavior plus the REQ-INIT-095 (already-initialized guard) that
> must be verified present.

```
 1: FUNCTION rust_gfx_init(driver, flags, renderer, width, height) -> c_int
 2:   IF get_gfx_state() IS Some THEN
 3:     LOG "already initialized"
 4:     RETURN -1                                         // REQ-INIT-095
 5:   END IF
 6:
 7:   LOG diagnostic: flags, width, height
 8:
 9:   fullscreen ← (flags & 0x01) != 0                   // REQ-INIT-050
10:
11:   sdl_context ← sdl2::init()                          // REQ-INIT-010
12:   IF sdl_context IS Err THEN LOG error; RETURN -1
13:
14:   video ← sdl_context.video()                         // REQ-INIT-010
15:   IF video IS Err THEN LOG error; RETURN -1
16:
17:   window ← video.window(title, width, height)         // REQ-INIT-010
18:   IF fullscreen THEN window.fullscreen()               // REQ-INIT-050
19:   IF window IS Err THEN LOG error; RETURN -1
20:
21:   canvas ← window.into_canvas().software()            // REQ-INIT-020
22:   IF canvas IS Err THEN LOG error; RETURN -1
23:
24:   SET hint "SDL_HINT_RENDER_SCALE_QUALITY" = "0"      // REQ-INIT-020, REQ-NP-040
25:
26:   // canvas is a local here (not yet stored in state)
27:   canvas.set_logical_size(320, 240)                    // REQ-INIT-020, REQ-WIN-010
28:   IF set_logical_size IS Err THEN LOG error; RETURN -1
28:
29:   event_pump ← sdl_context.event_pump()               // REQ-INIT-010
30:   IF event_pump IS Err THEN LOG error; RETURN -1
31:
32:   // Create screen surfaces — REQ-INIT-030
33:   surfaces ← [null; 3]
34:   FOR i IN 0..3:
35:     surfaces[i] ← SDL_CreateRGBSurface(0, 320, 240, 32,
36:                      R_MASK, G_MASK, B_MASK, A_MASK_SCREEN)
37:     IF surfaces[i] IS null THEN
38:       FOR j IN 0..i:                                   // REQ-INIT-097
39:         SDL_FreeSurface(surfaces[j])
40:       END FOR
41:       LOG error; RETURN -1
42:     END IF
43:   END FOR
44:
45:   // Create format conversion surface — REQ-INIT-040
46:   // NOTE: 1×1 rather than 0×0 — some SDL2 backends reject 0×0 surfaces.
47:   // The surface is only used as a format template, so dimensions are irrelevant.
48:   format_conv_surf ← SDL_CreateRGBSurface(0, 1, 1, 32,
49:                        R_MASK, G_MASK, B_MASK, A_MASK_ALPHA)
48:   IF format_conv_surf IS null THEN
49:     FOR i IN 0..3: SDL_FreeSurface(surfaces[i])        // REQ-INIT-097
50:     LOG error; RETURN -1
51:   END IF
52:
53:   // Allocate scaling buffers — REQ-INIT-055, REQ-INIT-060, REQ-INIT-070
54:   scale_any ← flags & (bits 3..9)
55:   use_soft_scaler ← scale_any != 0 AND (flags & bit3) == 0
56:   IF use_soft_scaler THEN
57:     scale_factor ← IF (flags & bit8) != 0 THEN 3      // REQ-INIT-070
58:                     ELSE IF (flags & bit9) != 0 THEN 4
59:                     ELSE 2
60:     buffer_size ← 320 * scale_factor * 240 * scale_factor * 4
61:     FOR i IN 0..3:
62:       scaled_buffers[i] ← vec![0u8; buffer_size]        // REQ-INIT-060
63:     END FOR
64:   END IF
65:
66:   state ← RustGraphicsState { all fields }
67:   set_gfx_state(Some(state))
68:
69:   LOG "Success"
70:   RETURN 0                                              // REQ-INIT-080
71: END FUNCTION
```

## 001B: rust_gfx_uninit

```
 1: FUNCTION rust_gfx_uninit()
 2:   state_opt ← RUST_GFX.take()
 3:   IF state_opt IS None THEN
 4:     RETURN                                              // REQ-UNINIT-030
 5:   END IF
 6:
 7:   state ← state_opt.unwrap()
 8:
 9:   // Free scaling buffers first — REQ-UNINIT-020 step 1
10:   FOR i IN 0..3:
11:     state.scaled_buffers[i] ← None
12:   END FOR
13:
14:   // Free surfaces — REQ-UNINIT-020 step 2
15:   FOR i IN 0..3:
16:     IF state.surfaces[i] IS NOT null THEN
17:       SDL_FreeSurface(state.surfaces[i])                // REQ-UNINIT-010
18:       state.surfaces[i] ← null
19:     END IF
20:   END FOR
21:   IF state.format_conv_surf IS NOT null THEN
22:     SDL_FreeSurface(state.format_conv_surf)
23:     state.format_conv_surf ← null
24:   END IF
25:
26:   // Drop in dependency order — REQ-UNINIT-020 steps 3-5
27:   drop(state.canvas)
28:   drop(state.video)
29:   drop(state.sdl_context)
30: END FUNCTION
```
