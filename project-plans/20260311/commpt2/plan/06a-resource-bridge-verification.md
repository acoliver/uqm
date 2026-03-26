# Phase 06a: Resource Bridge Verification

## Phase ID
`PLAN-20260326-COMMPT2.P06a`

## Prerequisites
- Required: Phase 06 (Resource Bridge) completed
- Phase completion marker exists: `project-plans/20260311/commpt2/.completed/P06.md`

## Structural Verification Checklist

- [ ] All 5 Load bridges exist and compile (LoadGraphic, LoadFont, LoadColorMap, LoadMusic, LoadStringTable)
- [ ] All 3 Capture bridges exist (CaptureDrawable, CaptureColorMap, CaptureStringTable)
- [ ] All 3 Release bridges exist (ReleaseDrawable, ReleaseColorMap, ReleaseStringTable)
- [ ] All context management bridges exist (7+: Create, Destroy, Set, SetFGFrame, SetClipRect, ClearClipRect, SetBGColor, SetFont)
- [ ] All drawable management bridges exist (4: Create, SetFrameTransparentColor, Clear, GetFrameRect)
- [ ] Batching bridges exist (2: BatchGraphics, UnbatchGraphics)
- [ ] Transition bridges exist (2: SetTransitionSource, ScreenTransition)
- [ ] SIS drawing bridges exist (3: DrawSISFrame, DrawSISMessage, DrawSISTitle)
- [ ] DoInput bridge exists
- [ ] CommData accessor bridges exist (~15+)
- [ ] Encounter function call bridges exist (3: init, post, uninit)
- [ ] Screen/SpaceContext accessors exist
- [ ] Game state accessors exist (IsStarbaseConversation, GetPlanetName, CheckLoad, etc.)
- [ ] SIS dimension accessors exist (ScreenWidth, ScreenHeight, SliderY, SliderHeight, Origin)
- [ ] All new functions declared in `rust_comm.h`
- [ ] All functions within `#ifdef USE_RUST_COMM` guard
- [ ] `@plan` and `@requirement` markers present

## Semantic Verification Checklist

- [ ] Load bridges use `(RESOURCE)` cast for resource ID parameter
- [ ] Capture/Release bridges use correct C handle types
- [ ] Destroy bridges were already verified in P00.5 (they existed previously)
- [ ] Context Set returns old context (needed for save/restore pattern)
- [ ] Font Set returns old font (needed for save/restore pattern)
- [ ] SetContextClipRect builds RECT struct correctly
- [ ] SetContextBackGroundColor builds color correctly with BUILD_COLOR/MAKE_RGB15
- [ ] GetFrameRect extracts all 4 fields (corner.x, corner.y, extent.width, extent.height)
- [ ] DoInput correctly casts state pointer to INPUT_STATE_DESC*
- [ ] CommData getters return correct field values (not aliased fields)
- [ ] CommData setters modify correct fields with correct type casts
- [ ] Encounter function callers dereference function pointer correctly (use `(*ptr)()` syntax)
- [ ] NULL handling: SetTransitionSource(0) passes NULL, not literal 0 cast to FRAME
- [ ] DrawSISMessage handles NULL msg (calls DrawSISMessage(NULL) to show default)
- [ ] No mixing of handle types (FRAME vs DRAWABLE vs FONT etc.)
- [ ] uintptr_t used consistently as opaque handle type between Rust and C

## Verification Commands

```bash
# C build — both modes
# (project-specific build with USE_RUST_COMM=on)
# (project-specific build with USE_RUST_COMM=off)

# Count new bridge functions
grep -c "^c_\|^void$\|^uintptr_t$\|^int$\|^unsigned$\|^const " sc2/src/uqm/rust_comm.c
# Should show significant increase from pre-P06

# Verify header completeness
diff <(grep "^[a-z_]*(" sc2/src/uqm/rust_comm.c | sed 's/(.*//' | sort) \
     <(grep "c_" sc2/src/uqm/rust_comm.h | grep -o "c_[A-Za-z_]*" | sort) | head -20
# Check for any functions in .c without .h declaration

# No duplicate symbols at link time
# (verified by successful build)

# No undefined symbols
# (verified by successful build and link)

# Rust tests still pass
cargo test --workspace --all-features
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings

# Deferred implementation check
grep -n "Stub\|TODO\|FIXME\|placeholder" sc2/src/uqm/rust_comm.c | grep -v "^[0-9]*:.*//.*" | grep -v "^[0-9]*:.*/\*.*\*/"
# Filter for non-comment matches only
```

## Release-before-Destroy Enforcement Rule

**HARD FAIL**: Captured resource types (Drawable via CaptureDrawable, ColorMap via CaptureColorMap, StringTable via CaptureStringTable) MUST be released before destroy in the HailAlien cleanup path. The correct pattern is:

```c
DestroyDrawable(ReleaseDrawable(frame));      // NOT DestroyDrawable(frame)
DestroyColorMap(ReleaseColorMap(cmap));        // NOT DestroyColorMap(cmap)
DestroyStringTable(ReleaseStringTable(table)); // NOT DestroyStringTable(table)
```

Non-captured resources (Font via LoadFont, Music via LoadMusic) use direct Destroy:
```c
DestroyFont(font);    // OK — Font is not captured
DestroyMusic(song);   // OK — Music is not captured
```

- [ ] All Release bridges (ReleaseDrawable, ReleaseColorMap, ReleaseStringTable) exist and compile
- [ ] P07 hail.rs cleanup code uses `c_DestroyX(c_ReleaseX(handle))` for captured types
- [ ] P07 hail.rs cleanup code uses direct `c_DestroyX(handle)` for non-captured types
- [ ] No captured resource is destroyed without a prior Release call

## Pass/Fail Gate Criteria

**PASS if**:
- All structural checks pass (all bridges exist)
- All semantic checks pass (correct types, casts, return values)
- Release-before-Destroy enforcement passes for all captured types
- Both build modes compile and link without errors
- No duplicate or undefined symbols
- Rust tests still pass
- No deferred implementation markers in new code

**FAIL if**:
- Any bridge function is missing
- Any bridge function has wrong type signature
- Captured resource destroyed without Release (reference count leak)
- Build fails in either mode
- Link errors (duplicate/undefined symbols)
- Any Rust test regression
- Type confusion between handle types
