# Phase 05a: Queue Orchestration + Zoom/Camera Verification

## Phase ID
`PLAN-20260320-BATTLEPT2.P05a`

## Prerequisites
- Required: Phase 05 (Queue Orchestration + Zoom/Camera) completed
- Expected artifacts: 9 functions in `process_loop.rs`

## Structural Verification Checklist
- [ ] All 9 functions present: redraw_queue, pre_process_queue, post_process_queue, calc_reduction, calc_view, init_display_list, insert_prim, calc_display_coord, init_kernel
- [ ] Plan/requirement traceability markers present
- [ ] No new module files created

## Semantic Verification Checklist (Mandatory — Most Important)

### PreProcessQueue equivalence with C (process.c:640-765)
- [ ] Iterates head-to-tail via GetHeadElement/GetSuccElement
- [ ] For each element: calls PreProcess if not yet preprocessed (PRE_PROCESS not set)
- [ ] For collidable elements: runs ProcessCollisions vs successor list
- [ ] Tracks player ship positions (up to 2 ships) for zoom/camera
- [ ] After iteration: calls CalcReduction from ship separation, CalcView for camera
- [ ] Returns ViewState + scroll deltas

### PostProcessQueue equivalence with C (process.c:767-1000)
- [ ] **Asymmetric flag clearing**: COLLISION set → clear COLLISION keep DEFY; else → clear DEFY
- [ ] **Newly-added cascading**: element without PRE_PROCESS → inner loop from element to tail → PreProcess + collisions vs HEAD → continues for new tail elements → scroll offsets zeroed after loop
- [ ] **Scroll offset application**: PRE_PROCESS+!POST_PROCESS → apply camera scroll; both set → zero scroll
- [ ] **DISAPPEARING removal**: remove_element called, element deallocated
- [ ] **Coordinate transform**: world→screen via CalcDisplayCoord with current reduction
- [ ] **Zoom-level sprite**: selects from frame array [0]=big/[1]=med/[2]=sml based on reduction level
- [ ] **Trilinear mipmap**: set up for smooth zoom transitions when applicable
- [ ] **Line primitives**: both endpoints transformed, wrap-around handled
- [ ] **PostProcess call**: postprocess_func invoked, commit_state(), insert into render list
- [ ] **InsertPrim**: sorted rendering-order insertion by display position

### Cascading correctness (critical)
- [ ] Test: element A's preprocess_func spawns element B; B is PreProcessed and collision-checked in the same frame
- [ ] Test: element B's preprocess_func spawns element C; cascading continues to C
- [ ] Test: scroll offsets are zeroed only after cascading completes
- [ ] Test: cascading only triggers for elements without PRE_PROCESS (not for already-processed)

### CalcReduction equivalence with C (process.c:287-370)
- [ ] **Step mode**: exactly 3 levels (0, 1, 2) based on distance thresholds
- [ ] **Step hysteresis**: zoom-in threshold < zoom-out threshold for each level transition, preventing oscillation
- [ ] **Continuous mode**: smooth linear interpolation with fractional precision
- [ ] **Continuous clamping**: result clamped at MAX_ZOOM_OUT (value 4 in fractional representation)

### CalcView equivalence with C (CALC_ZOOM_STUFF)
- [ ] **Midpoint camera**: origin = average of two ship positions
- [ ] **Single-ship clamping**: when ships_alive <= 1, scroll speed clamped to ORG_JUMP per-frame max
- [ ] **VIEW_STABLE**: no change from previous frame
- [ ] **VIEW_SCROLL**: camera moved but zoom unchanged
- [ ] **VIEW_CHANGE**: zoom level changed (triggers origin recalculation)

### RedrawQueue equivalence with C (process.c:1001-1109)
- [ ] Sequence: SetContext(StatusContext) → PreProcessQueue → PostProcessQueue → UpdateSoundPositions → SetContext(SpaceContext) → conditional render → FlushSounds
- [ ] **Simulation always runs**: PreProcessQueue + PostProcessQueue execute regardless of skip state
- [ ] **Rendering conditional**: BatchGraphics/ClearDrawable/draw/UnbatchGraphics skipped when max-speed

## Branch-Parity Verification
- [ ] Max-speed rendering skip: simulation executes, rendering skipped — test verifies both paths

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/battle/process_loop.rs
```

## Pass/Fail Gate Criteria
- **PASS:** All 9 functions implemented with correct C behavioral equivalence. Cascading verified. Zoom hysteresis verified. Camera calculation verified. Simulation-always-rendering-conditional verified. No TODO/FIXME/HACK.
- **FAIL:** Cascading doesn't chain through spawned elements. Scroll offsets applied incorrectly. Zoom oscillates between levels. Single-ship camera not clamped. Rendering not conditional on max-speed.
