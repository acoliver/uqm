# Phase 08: Image Rotation + Canvas Draw Mode Verification

## Phase ID
`PLAN-20260314-GRAPHICS.P08`

## Prerequisites
- Required: Phase P07 completed
- Verify: System-box and ReinitVideo implementations verified
- Expected files from previous phase: Modified `dcqueue.rs`, potentially `sdl_common.c`

## Requirements Implemented (Expanded)

### REQ-IMG-007: Rotation compatibility
**Requirement text**: When externally visible APIs request a rotated image object, the subsystem shall return an image object whose rendered orientation matches the requested rotation semantics.

Behavior contract:
- GIVEN: A source image exists
- WHEN: `TFB_DrawImage_New_Rotated(img, angle)` is called
- THEN: A new image object is created through the real ABI-visible creation path
- THEN: Its rendered pixels match the requested rotation
- THEN: Its hotspot and extent semantics remain externally compatible
- THEN: Its destruction path releases any derived resources correctly

### REQ-IMG-008: Hot-spot compatibility
The rotated image object must preserve externally visible hotspot behavior used by positioning and scaling logic.

### REQ-OWN-002: No external free obligation for subsystem-owned resources
Any rotated/derived image resources created in this phase must remain owned and freed through the graphics subsystem's normal destruction APIs.

### REQ-CAN-005: Primitive support (draw_rect mode parameter)
The earlier plan treated canvas draw-rect mode as a gap, but the review showed this was not a substantive standalone planning issue. This phase therefore limits canvas work to verifying that `draw_rect` / `fill_rect` exposure already matches the existing C header/caller contract and only changes it if concrete mismatch evidence is found.

## Implementation Tasks

### Task 1: Identify the real rotated-image integration boundary

#### Files to inspect/modify
- The current C implementation site for `TFB_DrawImage_New_Rotated`
- The Rust image lifecycle implementation handling `TFB_Image` creation/destruction
- `sc2/src/libs/graphics/sdl/rust_gfx.h` only if a concrete export is required by the actual caller surface

Required analysis outputs before implementation:
- Where the current API allocates and returns the rotated `TFB_Image*`
- Which fields must be initialized on the returned object (`NormalImg`, `extent`, hotspots, cache fields, dirty state, mutex, etc.)
- Which destruction path frees the resulting derived resources

### Task 2: Implement pixel rotation helper(s)

#### File: `rust/src/graphics/tfb_draw.rs` (or a new helper module if local organization warrants it)
- Add `create_rotated_canvas(src: &Canvas, angle_degrees: f64) -> Canvas`:
  - Compute rotated dimensions from source extent and angle
  - Allocate destination canvas at rotated dimensions
  - Use inverse rotation matrix for pixel sampling (nearest-neighbor unless the current C contract proves otherwise)
  - Fill uncovered regions with transparent pixels if the canvas format requires it
- marker: `@plan PLAN-20260314-GRAPHICS.P08`
- marker: `@requirement REQ-IMG-007`

### Task 3: Implement rotated-image object creation through the real lifecycle boundary

#### File(s): real image lifecycle boundary identified in Task 1
- Use the rotated canvas helper to build the new image content
- Allocate/initialize the rotated `TFB_Image` through the existing ABI-compatible path
- Preserve/recompute:
  - hotspot(s)
  - extent
  - dirty / cache state
  - mutex / synchronization fields
  - ownership of any derived canvases
- Ensure `TFB_DrawImage_Delete` or equivalent destruction path releases all derived resources
- marker: `@requirement REQ-IMG-007, REQ-IMG-008, REQ-OWN-002`

### Task 4: Verify canvas draw-rect/fill-rect contract only if needed

#### File: `rust/src/graphics/canvas_ffi.rs`
- Verify that `rust_canvas_draw_rect` and `rust_canvas_fill_rect` already match the existing C header/caller contract
- Only change signatures if the actual C declaration/caller proves a mismatch exists

#### File: `sc2/src/libs/graphics/sdl/rust_gfx.h`
- Verify `rust_canvas_draw_rect` and `rust_canvas_fill_rect` declarations are complete and aligned with Rust exports

### Pseudocode traceability
- Uses pseudocode lines: PC-09, lines 230-253

## TDD Test Plan

### Tests to add

#### Pixel rotation tests (in `tfb_draw.rs` or helper module)
1. `test_rotate_90_degrees` — 2x3 image → 3x2 rotated, verify pixel positions
2. `test_rotate_180_degrees` — image flipped, verify pixel content
3. `test_rotate_0_degrees` — identity rotation, output matches input
4. `test_rotate_45_degrees` — verify rotated dimensions are correct
5. `test_rotate_transparent_fill` — uncovered regions are transparent when required by format

#### Object-level rotation tests
6. `test_new_rotated_image_preserves_hotspot_contract` — verify rotated hotspot / origin behavior
7. `test_new_rotated_image_initializes_required_fields` — verify extent, cache, mutex, and lifecycle fields are set
8. `test_rotated_image_delete_releases_derived_resources` — verify destruction path owns the derived object correctly

#### Canvas contract verification
9. `test_canvas_draw_rect_contract_matches_header` — only if a signature/behavior mismatch was found

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] Real `TFB_DrawImage_New_Rotated` integration boundary identified
- [ ] `create_rotated_canvas` function exists with inverse-mapping rotation
- [ ] Rotated image object creation path uses the real lifecycle boundary, not a speculative ID-only export
- [ ] Ownership/destruction path for rotated images is explicit
- [ ] Canvas draw_rect/fill_rect contract verified against the header/caller surface
- [ ] 8+ rotation/object tests added

## Semantic Verification Checklist (Mandatory)
- [ ] 90-degree rotation produces correct pixel layout
- [ ] 180-degree rotation produces correct pixel layout
- [ ] Identity rotation preserves pixels exactly
- [ ] Non-axis-aligned rotation (45 degrees) produces correct expanded dimensions
- [ ] Transparent fill for uncovered pixels where required
- [ ] Rotated image hotspot/origin behavior remains externally compatible
- [ ] Rotated image destruction releases all subsystem-owned derived resources
- [ ] All existing canvas/image tests pass

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/graphics/ sc2/src/libs/graphics/ --include="*.rs" --include="*.c" --include="*.h" | grep -i rotat
```

## Success Criteria
- [ ] REQ-IMG-007: Rotated image object compatibility demonstrated
- [ ] REQ-IMG-008 and REQ-OWN-002 obligations covered for rotated images
- [ ] Verification commands pass

## Failure Recovery
- Rollback: `git checkout -- rust/src/graphics/ sc2/src/libs/graphics/`

## Phase Completion Marker
Create: `project-plans/20260311/graphics/.completed/P08.md`
