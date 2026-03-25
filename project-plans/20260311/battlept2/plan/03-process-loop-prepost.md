# Phase 03: Process Loop — PreProcess + PostProcess

## Phase ID
`PLAN-20260320-BATTLEPT2.P03`

## Prerequisites
- Required: Phase 02a (Pseudocode Verification) completed with PASS
- Verify: Pseudocode files exist in `analysis/pseudocode/`
- Expected files from previous phase: `analysis/pseudocode/process-loop.md`

## Requirements Implemented (Expanded)

### REQ: Element lifecycle (battle/requirements.md §Element lifecycle)
**Requirement text**: When an element's life span reaches zero during preprocessing, the battle engine shall invoke the element's death callback. When a death callback sets the DISAPPEARING flag, the element shall be removed during postprocess cleanup. When a death callback extends life span and clears DISAPPEARING, the element shall remain active.

Behavior contract:
- GIVEN: An element with life_span == 0 in PreProcess
- WHEN: PreProcess executes for that element
- THEN: Untarget is called, DISAPPEARING is set, and death_func is invoked

Why it matters:
- Drives the entire death/explosion/new-ship callback chain

### REQ: Element lifecycle flag transitions (battle/requirements.md §Element lifecycle flag transitions)
**Requirement text**: PreProcess shall set PRE_PROCESS and clear POST_PROCESS + COLLISION. PostProcess shall set POST_PROCESS and clear PRE_PROCESS, CHANGING, APPEARING. Asymmetric DEFY_PHYSICS clearing: COLLISION set → clear COLLISION keep DEFY; no COLLISION → clear DEFY.

Behavior contract:
- GIVEN: An element entering PreProcess
- WHEN: PreProcess completes
- THEN: PRE_PROCESS is set, POST_PROCESS and COLLISION are cleared

- GIVEN: An element entering PostProcess with COLLISION set
- WHEN: PostProcess asymmetric clearing executes
- THEN: COLLISION is cleared but DEFY_PHYSICS is retained

- GIVEN: An element entering PostProcess without COLLISION
- WHEN: PostProcess asymmetric clearing executes
- THEN: DEFY_PHYSICS is cleared

### REQ: PreProcess per-element (battle/requirements.md §PreProcess per-element)
**Requirement text**: APPEARING handling — PLAYER_SHIP clears APPEARING in local copy only (actual flags retain APPEARING, callback can detect first-frame). Non-PLAYER_SHIP with APPEARING skips preprocess callback (only intersection geometry initialized). Velocity stepping via Bresenham accumulation. FINITE_LIFE decrement after velocity. CHANGING + collidable reinit intersection frame.

Behavior contract:
- GIVEN: A PLAYER_SHIP element with APPEARING flag
- WHEN: PreProcess executes
- THEN: APPEARING is cleared in local copy only; preprocess_func IS called; element's actual state_flags still have APPEARING

- GIVEN: A non-PLAYER_SHIP element with APPEARING flag
- WHEN: PreProcess executes
- THEN: preprocess_func is NOT called; only intersection init runs

- GIVEN: An element with !IGNORE_VELOCITY
- WHEN: PreProcess velocity stepping runs
- THEN: Phase 1 get_next_components() is called to compute next position

### REQ: Element allocation and deallocation (battle/requirements.md §Display list management)
**Requirement text**: AllocElement allocates an element and a display primitive, binding them. FreeElement returns both to their respective free lists. SetUpElement initializes an allocated element's fields.

Behavior contract:
- GIVEN: The display list has capacity
- WHEN: AllocElement is called
- THEN: An element and display primitive are allocated and bound together

- GIVEN: An active element
- WHEN: FreeElement is called
- THEN: The element and its display primitive are returned to free lists

### REQ: Element removal and untargeting (battle/requirements.md §Element lifecycle)
**Requirement text**: When an element is removed, all other elements' tracking targets pointing to it shall be cleared. RemoveElement removes sound and the element from the display queue.

Behavior contract:
- GIVEN: Elements A and B where A.hTarget == B's handle
- WHEN: RemoveElement(B) is called
- THEN: A.hTarget is cleared to null/zero, B is removed from disp_q

## Implementation Tasks

### Commit 1 (rename-only)
- Rename `rust/src/battle/process_types.rs` → `rust/src/battle/process_loop.rs`
- Update `rust/src/battle/mod.rs` to reference `process_loop` instead of `process_types`
- marker: `@plan PLAN-20260320-BATTLEPT2.P03`
- **NO logic changes** — only file rename and import path updates
- This ensures `git log --follow` tracks history correctly

### Commit 2+: Files to modify

- `rust/src/battle/process_loop.rs` — Add behavioral logic
  - marker: `@plan PLAN-20260320-BATTLEPT2.P03`
  - marker: `@requirement REQ-ELEMENT-LIFECYCLE, REQ-FLAG-TRANSITIONS, REQ-PREPROCESS`
  - Contents to add:
    - `pub fn pre_process(element: &mut Element) -> PreProcessResult` — Per-element preprocessing matching process.c:180-270. Handles life_span==0 death, APPEARING (PLAYER_SHIP vs non-PLAYER_SHIP), preprocess_func callback dispatch, CHANGING intersection reinit, velocity stepping via Phase 1 get_next_components(), FINITE_LIFE decrement, collidable intersection end-point init. Sets PRE_PROCESS, clears POST_PROCESS + COLLISION.
    - `pub fn post_process(element: &mut Element)` — Per-element postprocessing matching process.c:274-285. Calls postprocess_func, copies next→current via Phase 1 commit_state(), reinits intersection points. Sets POST_PROCESS, clears PRE_PROCESS + CHANGING + APPEARING.
    - `pub fn alloc_element() -> Option<ElementHandle>` — Allocates element + display primitive matching process.c:143-155. Uses Phase 1 display_list alloc + display primitive free list.
    - `pub fn free_element(handle: ElementHandle)` — Deallocates element + display primitive matching process.c:160-177. Returns both to free lists.
    - `pub fn setup_element(element: &mut Element, setup: &ElementSetup)` — Field initialization matching process.c:156-177.
    - `pub fn untarget(dying_handle: ElementHandle)` — Clears hTarget references matching process.c:93-115. Iterates all elements.
    - `pub fn remove_element(handle: ElementHandle)` — Removes sound + Untarget + remove from queue matching process.c:118-140.
    - Helper: asymmetric flag-clearing logic for PostProcessQueue's per-element pass

### Files to modify
- `rust/src/battle/mod.rs`
  - Update module declaration from `process_types` to `process_loop`
  - Update any re-exports
  - marker: `@plan PLAN-20260320-BATTLEPT2.P03`

### C reference functions ported

| C Function | C File | C Lines | Rust Function | Rust Module |
|-----------|--------|---------|---------------|-------------|
| `PreProcess()` | process.c | :180-270 | `pre_process()` | `process_loop.rs` |
| `PostProcess()` | process.c | :274-285 | `post_process()` | `process_loop.rs` |
| `AllocElement()` | process.c | :143-155 | `alloc_element()` | `process_loop.rs` |
| `FreeElement()` | process.c | :160-177 | `free_element()` | `process_loop.rs` |
| `SetUpElement()` | process.c | :156-177 | `setup_element()` | `process_loop.rs` |
| `Untarget()` | process.c | :93-115 | `untarget()` | `process_loop.rs` |
| `RemoveElement()` | process.c | :118-140 | `remove_element()` | `process_loop.rs` |

### C branches to handle

| Branch | Source Sites | Handling |
|--------|-------------|----------|
| None directly in function bodies | — | PreProcess/PostProcess themselves do not contain `#ifdef` compile-time branch families |
| Indirect via callbacks | process.c:139-140,155-156,191-192 (death_func, preprocess_func, postprocess_func dispatch) | PreProcess and PostProcess invoke element callback function pointers. The functions stored in these callback slots (e.g., ship_preprocess, ship_postprocess, ship_death, explosion_preprocess, flee_preprocess) may themselves be compiled under `USE_RUST_SHIPS` or `USE_RUST_BATTLE_LOOP` guards. This means behavioral testing of PreProcess/PostProcess must account for which callback implementations are active. When `USE_RUST_SHIPS` is enabled, ship_preprocess/ship_postprocess delegate to Rust; when `USE_RUST_BATTLE_LOOP` is enabled, additional callbacks may be Rust-side. Test scenarios should verify callback dispatch works correctly regardless of which side provides the callback body. |

### Integration points
- Phase 1 `element.rs`: Element struct, ElementFlags, commit_state(), is_collidable()
- Phase 1 `velocity.rs`: get_next_components() for velocity stepping
- Phase 1 `display_list.rs`: alloc/free, iteration, handles
- Phase 1 `battle_types.rs`: coordinate conversion macros
- FFI (future P06): sound removal in RemoveElement calls bridge

### Pseudocode traceability (if impl phase)
- Uses pseudocode lines from `analysis/pseudocode/process-loop.md`: PreProcess, PostProcess, AllocElement, FreeElement, SetUpElement, Untarget, RemoveElement sections

## Verification Commands

```bash
# Structural gate
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] `process_types.rs` renamed to `process_loop.rs` (commit 1 is rename-only)
- [ ] `mod.rs` updated to reference `process_loop`
- [ ] All 7 functions implemented in `process_loop.rs`
- [ ] Phase 1 types used (not redefined)
- [ ] Plan/requirement traceability markers present
- [ ] Tests compile and run

## Semantic Verification Checklist (Mandatory)
- [ ] PreProcess death handling: life_span==0 → Untarget + DISAPPEARING + death_func
- [ ] PreProcess APPEARING: PLAYER_SHIP clears in local copy only; non-PLAYER_SHIP skips callback
- [ ] PreProcess velocity: !IGNORE_VELOCITY → get_next_components() applied
- [ ] PreProcess CHANGING: collidable + CHANGING → reinit intersection frame
- [ ] PreProcess FINITE_LIFE: decrement after velocity stepping
- [ ] PreProcess flags: sets PRE_PROCESS, clears POST_PROCESS + COLLISION
- [ ] PostProcess: calls postprocess_func, commit_state(), reinit intersection, sets POST_PROCESS clears PRE_PROCESS+CHANGING+APPEARING
- [ ] Asymmetric DEFY_PHYSICS clearing verified with test
- [ ] Untarget iterates all elements and clears matching hTarget
- [ ] RemoveElement removes sound, calls Untarget, removes from queue
- [ ] AllocElement/FreeElement pair correctly manages both element and display primitive
- [ ] No placeholder/deferred implementation patterns remain
- [ ] Integration points with Phase 1 types validated

## Deferred Implementation Detection (Mandatory)

```bash
# Reject if these appear in implementation code:
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/battle/process_loop.rs
```

## Success Criteria
- [ ] All 7 functions implemented and tested
- [ ] PreProcess/PostProcess flag transitions match C exactly
- [ ] Velocity stepping produces same results as C
- [ ] All Phase 1 tests (2,151) still pass
- [ ] Verification commands pass
- [ ] Semantic checks pass

## Failure Recovery
- rollback: `git checkout -- rust/src/battle/process_loop.rs rust/src/battle/mod.rs`
- Note: If rename was committed, use `git revert` for the rename commit
- blocking issues: Phase 1 API surface incompatibility discovered during integration

## Phase Completion Marker
Create: `project-plans/20260311/battlept2/.completed/P03.md`

Contents:
- phase ID: PLAN-20260320-BATTLEPT2.P03
- timestamp
- files changed: process_loop.rs (renamed + logic), mod.rs
- tests added/updated
- verification outputs
- semantic verification summary
