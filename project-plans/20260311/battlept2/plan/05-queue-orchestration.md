# Phase 05: Queue Orchestration + Zoom/Camera

## Phase ID
`PLAN-20260320-BATTLEPT2.P05`

## Prerequisites
- Required: Phase 04a (ProcessCollisions Verification) completed with PASS
- Expected files: `process_loop.rs` with P03 + P04 functions
- Expected artifacts: pre_process, post_process, process_collisions verified

## Requirements Implemented (Expanded)

### REQ: Top-level frame dispatch (battle/requirements.md §Top-level frame dispatch)
**Requirement text**: RedrawQueue executes: set status context → PreProcessQueue → PostProcessQueue → update sounds → set space context → conditional render → flush sounds.

Behavior contract:
- GIVEN: A battle frame is being processed
- WHEN: RedrawQueue is called
- THEN: Steps execute in exact order; simulation always runs; rendering conditionally skipped

### REQ: PreProcessQueue (battle/requirements.md §PreProcessQueue)
**Requirement text**: Iterate head-to-tail, call PreProcess per element, run collision detection against successors, track player-ship positions for camera.

Behavior contract:
- GIVEN: A display list with elements
- WHEN: PreProcessQueue executes
- THEN: Each element is preprocessed, collisions checked forward, ship positions recorded for zoom/camera

### REQ: PostProcessQueue — newly-added cascading (battle/requirements.md §Newly-added element cascading)
**Requirement text**: Elements lacking PRE_PROCESS flag enter inner loop: PreProcess + collision vs entire list from head. Cascading continues until no new elements. Scroll offsets zeroed after inner loop.

Behavior contract:
- GIVEN: A newly-added element (no PRE_PROCESS flag) encountered during PostProcessQueue
- WHEN: The cascading inner loop runs
- THEN: The element gets PreProcess + collision vs full list; further spawned elements also processed; scroll offsets zeroed

### REQ: PostProcessQueue — scroll offset application (battle/requirements.md §Scroll offset application)
**Requirement text**: PRE_PROCESS set + POST_PROCESS not set → apply scroll offsets. Both set → zero offsets (already adjusted).

Behavior contract:
- GIVEN: An element with PRE_PROCESS set, POST_PROCESS not set
- WHEN: PostProcessQueue applies coordinates
- THEN: Camera scroll offsets are applied to element coordinates

### REQ: PostProcessQueue — element removal and rendering (battle/requirements.md §Element removal and rendering setup)
**Requirement text**: DISAPPEARING elements removed. Surviving elements: world→screen coords, zoom-appropriate sprite, postprocess callback, insert display prim.

Behavior contract:
- GIVEN: A DISAPPEARING element in PostProcessQueue
- WHEN: PostProcessQueue processes it
- THEN: Element is removed from display list and deallocated

### REQ: Zoom calculation (battle/requirements.md §Zoom calculation)
**Requirement text**: Discrete zoom: 3 levels with hysteresis. Continuous zoom: smooth interpolation clamped to max.

Behavior contract:
- GIVEN: Two ships at distance D in step zoom mode
- WHEN: CalcReduction runs
- THEN: One of 3 discrete zoom levels selected with hysteresis preventing oscillation

### REQ: Camera calculation (battle/requirements.md §Camera calculation)
**Requirement text**: Camera origin = midpoint between ships. Single ship: clamped scroll speed. Zoom change: recalculate origin.

Behavior contract:
- GIVEN: Two active player ships
- WHEN: CalcView runs
- THEN: Camera centered on midpoint; scroll delta computed; view state updated

### REQ: World-to-screen conversion (battle/requirements.md §World-to-screen coordinate conversion)
**Requirement text**: Discrete: subtract origin, shift by reduction. Continuous: subtract origin, shift left by precision, divide by factor.

Behavior contract:
- GIVEN: A world coordinate and current zoom level
- WHEN: CalcDisplayCoord runs
- THEN: Correct screen coordinate produced for current zoom mode

## Implementation Tasks

### Files to modify

- `rust/src/battle/process_loop.rs` — Add queue orchestration + zoom/camera
  - marker: `@plan PLAN-20260320-BATTLEPT2.P05`
  - marker: `@requirement REQ-FRAME-DISPATCH, REQ-PREPROCESS-QUEUE, REQ-POSTPROCESS-QUEUE, REQ-ZOOM, REQ-CAMERA`
  - Contents to add:
    - `pub fn redraw_queue(force_redraw: bool)` — Top-level frame matching process.c:1012-1061 RedrawQueue(BOOLEAN clear). Full dispatch sequence:
      1. SetContext(StatusContext)
      2. PreProcessQueue → (view_state, scroll_x, scroll_y)
      3. PostProcessQueue(view_state, scroll_x, scroll_y)
      4. if optStereoSFX: UpdateSoundPositions()
      5. SetContext(SpaceContext)
      6. **Normal render path** (process.c:1027-1053): if SUPER_MELEE OR !(CHECK_ABORT|CHECK_LOAD):
         - skip_frames = HIBYTE(nth_frame); if skip_frames != 0xFF AND (skip_frames == 0 OR frame counter expired):
           - nth_frame += skip_frames
           - if clear: ClearDrawable()
           - if continuous zoom: CALC_ZOOM_STUFF → SetGraphicScale(scale)
           - DrawBatch(DisplayArray, DisplayLinks, 0)
           - SetGraphicScale(0)
         - FlushSounds()
      7. **Abort/check path** (process.c:1054-1058): else (CHECK_ABORT or CHECK_LOAD active, not SUPER_MELEE):
         - ProcessSound((SOUND)~0, NULL) — flush pending sound queue
         - FlushSounds()
      8. DisplayLinks = MakeLinks(END_OF_LIST, END_OF_LIST) — reset render list
      
      Note: BatchGraphics/UnbatchGraphics are NOT in RedrawQueue — they are in DoBattle (battle.c:314/327). RedrawQueue is called between BatchGraphics and UnbatchGraphics. The `inHQSpace()` check affects DoBattle's first_time flag (battle.c:469: `bs->first_time = inHQSpace()`), which controls whether ScreenTransition is applied on the first frame. There is no inHQSpace branch within RedrawQueue itself — `inHQSpace()` appears in process.c only at lines 311 and 315, both within CalcView(), not RedrawQueue.
      
      Note: ProcessSound((SOUND)~0, NULL) does NOT appear in the normal render path. In the normal path, sounds are flushed only via FlushSounds() (process.c:1052). ProcessSound((SOUND)~0, NULL) appears solely in the abort/check path (process.c:1056) to drain the pending sound queue when rendering is being skipped due to CHECK_ABORT/CHECK_LOAD. This is distinct from the ProcessSound calls within individual element preprocess/postprocess callbacks which queue sounds during simulation.
    - `fn pre_process_queue() -> (ViewState, i32, i32)` — matching process.c:640-765. Head-to-tail iteration, PreProcess per element, ProcessCollisions vs successors, ship position tracking, CalcView/CalcReduction for zoom/camera. Returns view_state + scroll deltas.
    - `fn post_process_queue(scroll_x: i32, scroll_y: i32)` — matching process.c:767-1000. Iterates elements: asymmetric flag clearing, newly-added cascading (inner loop with PreProcess + collisions vs head), scroll offset application, DISAPPEARING removal, world→screen via CalcDisplayCoord, zoom-appropriate sprite selection, postprocess callback, InsertPrim for render list. Trilinear mipmap setup.
    - `fn calc_reduction(ship_distance: i32) -> i32` — matching process.c:287-370. Step mode (3 levels + hysteresis) and continuous mode (smooth + MAX_ZOOM_OUT clamp).
    - `fn calc_view(ship0: Point, ship1: Point, ships_alive: u8) -> (ViewState, i32, i32)` — matching process.c CALC_ZOOM_STUFF macro. Midpoint camera, single-ship clamping (ORG_JUMP), VIEW_STABLE/VIEW_SCROLL/VIEW_CHANGE states.
    - `pub fn init_display_list()` — matching process.c:631-637. Empties active list, rebuilds free chain.
    - `fn insert_prim(prim_index: usize, element: &Element)` — matching process.c InsertPrim. Sorted insertion into DisplayLinks by display position.
    - `fn calc_display_coord(world_x: i32, world_y: i32, reduction: i32) -> (i32, i32)` — World→screen coordinate conversion for current zoom mode.
    - `pub fn init_kernel()` — Graphics kernel initialization matching process.c InitKernel.

### C reference functions ported

| C Function | C File | C Lines | Rust Function | Rust Module |
|-----------|--------|---------|---------------|-------------|
| `CalcReduction()` | process.c | :287-370 | `calc_reduction()` | `process_loop.rs` |
| `CalcView()` / CALC_ZOOM_STUFF | process.c | :640-765 | `calc_view()` | `process_loop.rs` |
| `InsertPrim()` | process.c | :855-900 | `insert_prim()` | `process_loop.rs` |
| `CalcDisplayCoord()` | process.c | — | `calc_display_coord()` | `process_loop.rs` |
| `PreProcessQueue()` | process.c | :640-765 | `pre_process_queue()` | `process_loop.rs` |
| `PostProcessQueue()` | process.c | :767-1000 | `post_process_queue()` | `process_loop.rs` |
| `InitDisplayList()` | process.c | :631-637 | `init_display_list()` | `process_loop.rs` |
| `RedrawQueue()` | process.c | :1001-1109 | `redraw_queue()` | `process_loop.rs` |
| `InitKernel()` | process.c | — | `init_kernel()` | `process_loop.rs` |

### C branches to handle

| Branch | Source Sites | Handling |
|--------|-------------|----------|
| Max-speed rendering skip | process.c RedrawQueue :1032-1034 (`skip_frames == (BYTE)~0`) | Simulation always executes; render conditionally skipped. When skip_frames == 0xFF, entire DrawBatch/ClearDrawable/zoom block is skipped |
| Abort/check render bypass | process.c RedrawQueue :1054-1058 | When CHECK_ABORT or CHECK_LOAD and not SUPER_MELEE, DrawBatch is skipped entirely; instead ProcessSound((SOUND)~0, NULL) flushes pending sounds, then FlushSounds |
| `KDEBUG` | process.c:211-213, 276-278, 291-292, 354-356, 638-639, 742-744, 806-808, 980-982 | Debug logging in CalcReduction, CalcView, PreProcessQueue, PostProcessQueue entry/exit. Rust equivalent: `tracing::debug!()` behind a feature flag |
| `optMeleeScale == TFB_SCALE_STEP` vs continuous | process.c: CalcReduction:215-274, CalcView:319-343, PostProcessQueue:809-812, CalcDisplayCoord:788-796, InitDisplayList:990-998, RedrawQueue:1040-1049 | Two zoom modes throughout: step (discrete 3-level with shift) vs continuous (smooth with division). Must handle both paths |
| `optStereoSFX` | process.c:1023-1024, 1096-1105 (UpdateSoundPositions in RedrawQueue, RemoveSoundsForObject in RemoveElement) | Stereo sound position updates conditional on option |
| `optMeleeScale == TFB_SCALE_TRILINEAR` | process.c:938-959 (PostProcessQueue mipmap setup) | Trilinear scaling: sets mipmap from next zoom level frame via TFB_DrawScreen_SetMipmap |
| `NETPLAY_CHECKSUM` | process.c (not in queue funcs; in DoBattle in battle.c) | N/A for P05 queue functions |

### Integration points
- P03 `process_loop.rs`: pre_process(), post_process(), remove_element(), alloc_element(), free_element()
- P04 `process_loop.rs`: process_collisions()
- Phase 1 `display_list.rs`: iteration, handles, push_back, remove
- Phase 1 `element.rs`: Element struct, ElementFlags
- Phase 1 `battle_types.rs`: coordinate conversions, WORLD_TO_DISPLAY, DISPLAY_TO_WORLD
- Phase 1 `process_loop.rs` (types): ViewState, ZoomMode, zoom/camera constants
- C FFI (future P06): SetContext, BatchGraphics, UnbatchGraphics, FlushSounds, UpdateSoundPositions, GetFrameIndex, SetGraphicScaleMode, ClearDrawable

### Pseudocode traceability (if impl phase)
- Uses pseudocode lines from `analysis/pseudocode/process-loop.md`: PreProcessQueue, PostProcessQueue, RedrawQueue, InitDisplayList sections
- Uses pseudocode lines from `analysis/pseudocode/zoom-camera.md`: CalcReduction, CalcView sections

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] All 9 functions implemented in `process_loop.rs`
- [ ] No new module files created
- [ ] Plan/requirement traceability markers present
- [ ] Tests compile and run

## Semantic Verification Checklist (Mandatory)
- [ ] PreProcessQueue: head-to-tail, PreProcess per element, collisions vs successors, camera tracking
- [ ] PostProcessQueue newly-added cascading: inner loop from unprocessed element to tail, PreProcess + collisions vs head, continues for spawned elements
- [ ] PostProcessQueue scroll offsets: PRE_PROCESS+!POST_PROCESS → apply; both → zero
- [ ] PostProcessQueue DISAPPEARING removal: element removed and deallocated
- [ ] PostProcessQueue rendering: world→screen, zoom sprite, postprocess callback, InsertPrim
- [ ] CalcReduction step mode: 3 discrete levels with hysteresis thresholds
- [ ] CalcReduction continuous mode: smooth interpolation, MAX_ZOOM_OUT clamping
- [ ] CalcView: midpoint camera, single-ship ORG_JUMP clamping, VIEW_STABLE/SCROLL/CHANGE
- [ ] RedrawQueue: SetContext(StatusContext) → PreProcessQueue → PostProcessQueue → UpdateSoundPositions (if stereo) → SetContext(SpaceContext) → **normal render path** (skip_frames check → ClearDrawable → zoom setup → DrawBatch → FlushSounds) OR **abort path** (ProcessSound((SOUND)~0, NULL) → FlushSounds) → DisplayLinks reset
- [ ] RedrawQueue abort path: when CHECK_ABORT/CHECK_LOAD is set (and not SUPER_MELEE), ProcessSound((SOUND)~0, NULL) flushes pending sounds before FlushSounds (process.c:1054-1058)
- [ ] Simulation always executes (PreProcessQueue + PostProcessQueue); rendering conditionally skipped
- [ ] BatchGraphics/UnbatchGraphics are in DoBattle (battle.c), not in RedrawQueue; inHQSpace affects DoBattle's first_time flag (ScreenTransition), not RedrawQueue's render path
- [ ] InitDisplayList: empties active list, rebuilds free chain
- [ ] InsertPrim: sorted insertion by display position into rendering-order list
- [ ] No placeholder/deferred implementation patterns

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/battle/process_loop.rs
```

## Success Criteria
- [ ] All 9 functions implemented and tested
- [ ] Queue orchestration matches C exactly
- [ ] Zoom calculation matches C for both modes
- [ ] All Phase 1 tests pass
- [ ] Verification commands pass

## Failure Recovery
- rollback: `git checkout -- rust/src/battle/process_loop.rs`
- blocking issues: Coordinate transform precision mismatches, zoom hysteresis threshold values

## Phase Completion Marker
Create: `project-plans/20260311/battlept2/.completed/P05.md`

Contents:
- phase ID: PLAN-20260320-BATTLEPT2.P05
- timestamp
- files changed: process_loop.rs
- tests added/updated
- verification outputs
- semantic verification summary
