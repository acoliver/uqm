# Pseudocode: Component 007 — Surface Access & Auxiliary Functions

Plan ID: `PLAN-20260223-GFX-VTABLE-FIX`
Requirements: REQ-SURF-010..070, REQ-AUX-010..060, REQ-ERR-011..014

---

## 007A: Surface Accessors

> These are already implemented and working. Documented for completeness.

```
 1: FUNCTION rust_gfx_get_screen_surface(screen) -> *mut SDL_Surface
 2:   IF screen < 0 OR screen >= TFB_GFX_NUMSCREENS THEN
 3:     RETURN null                                         // REQ-SURF-020
 4:   END IF
 5:   state ← get_gfx_state()
 6:   IF state IS None THEN RETURN null                     // REQ-SURF-030, REQ-ERR-011
 7:   RETURN state.surfaces[screen]                         // REQ-SURF-010
 8: END FUNCTION

 9: FUNCTION rust_gfx_get_sdl_screen() -> *mut SDL_Surface
10:   RETURN rust_gfx_get_screen_surface(0)                 // REQ-SURF-060
11: END FUNCTION

12: FUNCTION rust_gfx_get_transition_screen() -> *mut SDL_Surface
13:   RETURN rust_gfx_get_screen_surface(2)                 // REQ-SURF-070
14: END FUNCTION

15: FUNCTION rust_gfx_get_format_conv_surf() -> *mut SDL_Surface
16:   state ← get_gfx_state()
17:   IF state IS None THEN RETURN null                     // REQ-SURF-030, REQ-ERR-011
18:   RETURN state.format_conv_surf                         // REQ-SURF-050
19: END FUNCTION
```

## 007B: Auxiliary Functions

> These are already implemented. Documented for traceability.

```
20: FUNCTION rust_gfx_process_events() -> c_int
21:   state ← get_gfx_state()
22:   IF state IS None THEN RETURN 0                        // REQ-ERR-013
23:   FOR event IN state.event_pump.poll_iter():
24:     IF event IS Quit THEN RETURN 1                      // REQ-AUX-010
25:   END FOR
26:   RETURN 0
27: END FUNCTION

28: FUNCTION rust_gfx_toggle_fullscreen() -> c_int
29:   state ← get_gfx_state()
30:   IF state IS None THEN RETURN -1                       // REQ-ERR-014
31:   state.fullscreen ← NOT state.fullscreen
32:   RETURN IF state.fullscreen THEN 1 ELSE 0              // REQ-AUX-020
33: END FUNCTION

34: FUNCTION rust_gfx_is_fullscreen() -> c_int
35:   state ← get_gfx_state()
36:   IF state IS None THEN RETURN 0                        // REQ-ERR-013
37:   RETURN IF state.fullscreen THEN 1 ELSE 0              // REQ-AUX-030
38: END FUNCTION

39: FUNCTION rust_gfx_set_gamma(gamma: f32) -> c_int
40:   // REQ-AUX-040, REQ-AUX-041: unsupported, return -1
41:   IF get_gfx_state() IS None THEN RETURN -1             // REQ-ERR-014
42:   RETURN -1
43: END FUNCTION

44: FUNCTION rust_gfx_get_width() -> c_int
45:   IF get_gfx_state() IS None THEN RETURN 0              // REQ-ERR-013
46:   RETURN 320                                            // REQ-AUX-050
47: END FUNCTION

48: FUNCTION rust_gfx_get_height() -> c_int
49:   IF get_gfx_state() IS None THEN RETURN 0              // REQ-ERR-013
50:   RETURN 240                                            // REQ-AUX-050
51: END FUNCTION
```

### Notes

Surface accessors and auxiliary functions are already working in the
current codebase. They are included in this plan for traceability and
to verify they are not broken by the vtable fix refactoring.
