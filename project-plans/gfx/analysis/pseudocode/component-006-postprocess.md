# Pseudocode: Component 006 — Postprocess

Plan ID: `PLAN-20260223-GFX-VTABLE-FIX`
Requirements: REQ-POST-010..030, REQ-INV-010

---

## 006A: rust_gfx_postprocess

```
 1: FUNCTION rust_gfx_postprocess()
 2:   state ← get_gfx_state()
 3:   IF state IS None THEN RETURN                          // REQ-POST-030
 4:
 5:   // REQ-POST-010: Present the composed frame
 6:   // REQ-POST-020: NO texture creation, NO surface upload, NO state.canvas.copy
 7:   // REQ-INV-010: All compositing done by ScreenLayer
 8:   state.canvas.present()
 9: END FUNCTION
```

### Validation Points
- Line 3: Uninitialized guard

### Error Handling
- present() does not return an error in sdl2 crate API

### Ordering Constraints
- MUST be called AFTER all ScreenLayer and ColorLayer calls for this frame
- MUST NOT create textures or call state.canvas.copy (REQ-POST-020)

### Integration Boundaries
- Called by C `Rust_Postprocess` wrapper → `TFB_SwapBuffers` step 6 (always last)
- The renderer contains the fully composited frame from prior vtable calls

### Side Effects
- Frame is presented to the display (becomes visible)
- Renderer is ready for next frame's Preprocess

## 006B: rust_gfx_upload_transition_screen

```
 1: FUNCTION rust_gfx_upload_transition_screen()
 2:   // REQ-UTS-010: No-op because ScreenLayer always uploads full surface
 3:   // REQ-UTS-030: Safe when uninitialized (no state access needed)
 4:   // INVARIANT (REQ-UTS-020): If ScreenLayer adds dirty tracking,
 5:   //   this must set dirty flag for TFB_SCREEN_TRANSITION
 6:   RETURN
 7: END FUNCTION
```
