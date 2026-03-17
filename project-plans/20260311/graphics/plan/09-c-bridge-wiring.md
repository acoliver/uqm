# Phase 09: C-Side Bridge Wiring + Revalidation

## Phase ID
`PLAN-20260314-GRAPHICS.P09`

## Prerequisites
- Required: Phase P08 completed
- Verify: All Rust-side implementations (P03-P08) pass tests
- Expected: All required `rust_dcq_push_*`, `rust_cmap_*` FFI functions exist and are tested

## Requirements Implemented (Expanded)

### REQ-INT-001: Existing API compatibility
**Requirement text**: The subsystem shall preserve the externally visible behavior required by the existing UQM graphics API surface.

Behavior contract:
- GIVEN: C game code calls `TFB_DrawScreen_Line`, `TFB_DrawScreen_Image`, etc.
- WHEN: `USE_RUST_GFX` is defined
- THEN: These calls are redirected to `rust_dcq_push_*` functions without losing caller-visible state

### REQ-INT-002: Backend-vtable compatibility
**Requirement text**: The subsystem shall provide behaviorally compatible backend implementations.

Behavior contract:
- GIVEN: `USE_RUST_GFX` is defined
- WHEN: `TFB_FlushGraphics` is called
- THEN: It invokes `rust_dcq_flush()` instead of the C flush loop

### REQ-INT-003: FFI symbol compatibility
**Requirement text**: Where integration depends on named exported FFI entry points, the subsystem shall preserve those entry points.

### REQ-INT-004: Build-flag behavioral responsibility
**Requirement text**: When the build enables the Rust graphics path, the subsystem shall assume behavioral ownership of all externally visible graphics behavior.

### Revalidation requirements carried by this phase
This phase is where Rust-local work becomes migration-trustworthy. After the real C bridge is wired, revalidate:
- REQ-DQ-003 / REQ-DQ-004 batch and nested batch semantics
- REQ-RL-011 reinit behavior on the real lifecycle path
- REQ-RL-012 system-box compositing on the real orchestration path
- REQ-INT-006 transition-source workflow compatibility
- REQ-INT-007 extra-screen workflow compatibility
- REQ-INT-008 context-driven draw compatibility
- REQ-INT-009 synchronization compatibility
- REQ-OWN-006 deferred free ordering on the migrated path
- REQ-OWN-007 image synchronization obligations at the migrated ABI boundary

## Implementation Tasks

### Task 1: Wire TFB_DrawScreen_* functions to Rust DCQ

#### File: `sc2/src/libs/graphics/tfb_draw.c`
For each `TFB_DrawScreen_*` function, add `USE_RUST_GFX` guard that calls the corresponding Rust push function while preserving the exact caller-visible state already available in C.

Required functions to wire include the full externally used set, not just a subset:

| C Function | Rust FFI | Required forwarding details |
|-----------|----------|-----------------------------|
| `TFB_DrawScreen_Line` | `rust_dcq_push_drawline` | preserve packed color and destination screen/state |
| `TFB_DrawScreen_Rect` | `rust_dcq_push_drawrect` / `rust_dcq_push_fillrect` | preserve outline/fill behavior and draw mode |
| `TFB_DrawScreen_Image` | `rust_dcq_push_drawimage` | preserve scale, scale_mode, colormap, draw_mode, target screen |
| `TFB_DrawScreen_FilledImage` | `rust_dcq_push_filledimage` | preserve fill color, scale, mode, target screen |
| `TFB_DrawScreen_FontChar` | `rust_dcq_push_fontchar` | preserve glyph metrics, backing image, draw mode, target screen |
| `TFB_DrawScreen_Copy` | `rust_dcq_push_copy` | preserve source/dest screen and clipping |
| `TFB_DrawScreen_CopyToImage` | `rust_dcq_push_copytoimage` | preserve source rect and screen |
| `TFB_DrawScreen_SetMipmap` | `rust_dcq_push_setmipmap` | preserve hotspot parameters |
| `TFB_DrawScreen_DeleteImage` | `rust_dcq_push_deleteimage` | preserve identity of object being released |
| `TFB_DrawScreen_DeleteData` | `rust_dcq_push_deletedata` | preserve pointer payload |
| `TFB_DrawScreen_WaitSignal` | `rust_dcq_push_waitsignal` | preserve synchronization object |

Pattern for each forwarding function:
```c
#ifdef USE_RUST_GFX
    /* gather exact caller-visible state from current context */
    /* call Rust FFI with the real parameters, not defaults */
    return;
#else
    /* existing C enqueue code */
#endif
```

marker: `@plan PLAN-20260314-GRAPHICS.P09`
marker: `@requirement REQ-INT-001, REQ-INT-003, REQ-INT-008`

### Task 2: Wire TFB_FlushGraphics to Rust DCQ flush

#### File: `sc2/src/libs/graphics/dcqueue.c`
In `TFB_FlushGraphics()`:
```c
#ifdef USE_RUST_GFX
    rust_dcq_flush();
    return;
#else
    /* existing C flush loop */
#endif
```

Also preserve any synchronization or bookkeeping contract the C path performed around flush.

marker: `@requirement REQ-INT-002, REQ-INT-009`

### Task 3: Wire DCQ batch/unbatch/set_screen through the actual C call sites

#### File(s): wherever `TFB_BatchGraphics`, `TFB_UnbatchGraphics`, `TFB_SetScreen` are actually defined
```c
#ifdef USE_RUST_GFX
    rust_dcq_batch();     /* or unbatch/set_screen */
    return;
#else
    /* existing C code */
#endif
```

This task is mandatory because batch/nested batch semantics cannot be considered migration-complete until the real C entry points drive the Rust queue.

### Task 4: Wire colormap lifecycle at the real graphics lifecycle boundary

#### File: actual lifecycle owner (likely `sc2/src/libs/graphics/sdl/sdl_common.c`, but verify)
In `TFB_InitGraphics()` / `TFB_UninitGraphics()` or the real equivalents under `USE_RUST_GFX`:
- Add `rust_cmap_init()` / `rust_dcq_init()`
- Add `rust_dcq_uninit()` / `rust_cmap_uninit()`

Do not assume `gfx_common.c` is the right site without verifying the current lifecycle owner.

### Task 5: Wire real colormap operations

#### File: `sc2/src/libs/graphics/cmap.c` or the actual operation sites
For key colormap operations (`FadeScreen`, `GetFadeAmount`, `XFormColorMap_step`, etc.), redirect to Rust under `USE_RUST_GFX` only where those operations are part of the migrated behavior boundary.

### Task 6: Enumerate and wire remaining deferred control-path ingress points

This task closes the gap between draw ingress migration and full single-ingress ownership for deferred control operations.

#### Required control-path inventory (must be explicit)
Identify the exact file/function for each active C-side source of deferred control work under `USE_RUST_GFX`, then either wire it to Rust or document with call-path evidence why no change is needed:

- palette selection / colormap activation path(s)
- callback enqueue path(s)
- signal / wait submission path(s)
- screen-state control path(s) not already covered by Task 3
- any remaining deferred queue-control entry points reachable from the graphics API surface

For each entry, record:
- concrete file + function name
- corresponding Rust export
- caller-visible state that must be preserved
- whether the path enqueues deferred work, performs immediate control, or is unreachable under `USE_RUST_GFX`

This inventory must be complete enough to support REQ-DQ-001 single-ingress claims, not just the draw wrappers in `tfb_draw.c`.

### Task 7: Do **not** wire `canvas.c` unless call-site analysis proves it is necessary

#### File: `sc2/src/libs/graphics/sdl/canvas.c`
Default disposition: no changes.

Only modify this file if analysis proves there are still active C call sites in the `USE_RUST_GFX` path that bypass the Rust DCQ/canvas ownership transfer and therefore require an additional bridge.

If no such call sites exist, explicitly record that `canvas.c` is out of scope for this plan because DCQ-level migration already covers the required boundary.

### Task 8: Include `rust_gfx.h` in all modified C files

Ensure `#include "rust_gfx.h"` (or equivalent path) is present in each modified `.c` file under `USE_RUST_GFX` guard.

### Task 9: ABI/export parity audit before and after wiring

#### Files: `sc2/src/libs/graphics/sdl/rust_gfx.h` plus the Rust FFI export modules
Perform an explicit symbol/export audit that covers the full migrated graphics boundary, not only DCQ push functions.

Required audit scope:
- `rust_dcq_push_*`
- `rust_dcq_*` lifecycle / control exports
- `rust_canvas_*`
- `rust_cmap_*`
- backend-vtable / presentation-facing exports used by C under `USE_RUST_GFX`
- `rust_gfx_process_events` and any event-path exports used by the live graphics lifecycle

For every symbol declared in `rust_gfx.h`, confirm there is a matching Rust `#[no_mangle] extern "C"` export with ABI-compatible signature. Any mismatch found here must be fixed before final migrated-path verification proceeds.

### Task 10: Revalidate migration-sensitive semantics through the real bridge

After wiring, add or run tests covering:
- transition-source capture timing and stability
- extra-screen copy workflows
- context-driven state propagation (draw mode, color/colormap, clipping, font, scale, target screen)
- batch and nested batch visibility
- deferred free ordering
- image metadata synchronization at the ABI boundary
- flush completion synchronization
- idle/no-redraw behavior on the migrated flush path
- reinit behavior through the real lifecycle entry path
- system-box compositing through the real presentation orchestration path
- event-processing call reachability and lifecycle continuity handoff into P10

### Pseudocode traceability
- Uses pseudocode lines: PC-10, lines 260-292

## Build Integration Verification

After wiring, verify the project builds with `USE_RUST_GFX` enabled:
```bash
cd sc2 && ./build.sh uqm
```

Verify link: all required `rust_dcq_push_*`, `rust_cmap_*`, `rust_canvas_*`, and other actually used symbols resolve.

## TDD / Integration Test Plan

C-side wiring is verified primarily through integration tests in this phase and later end-to-end verification.

1. Verify all required FFI symbols exist in the compiled Rust library.
2. Add targeted migrated-path tests that confirm the real C wrappers preserve state/ordering semantics, not just link successfully.
3. Use exact symbol-name verification, not only counts.
4. Verify that every deferred control-path ingress identified in Task 6 is either wired, proven unreachable, or intentionally out of scope with evidence.
5. Confirm the event-processing export path used under `USE_RUST_GFX` remains wired after any lifecycle-owner changes.

Example symbol verification:
```bash
grep 'rust_dcq_push_' sc2/src/libs/graphics/sdl/rust_gfx.h | sed 's/.*\(rust_dcq_push_[a-z_]*\).*/\1/' | sort -u > /tmp/declared_dcq.txt
nm -gU rust/target/release/libuqm_rust.a | grep ' _rust_dcq_push_' | sed 's/.*_\(rust_dcq_push_[a-z_]*\)$/\1/' | sort -u > /tmp/exported_dcq.txt
diff /tmp/declared_dcq.txt /tmp/exported_dcq.txt
```

## Verification Commands

```bash
# Rust side
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features

# C side (build)
cd sc2 && ./build.sh uqm
```

## Structural Verification Checklist
- [ ] All relevant `TFB_DrawScreen_*` functions have `USE_RUST_GFX` guards
- [ ] `TFB_FlushGraphics` has `USE_RUST_GFX` guard redirecting to `rust_dcq_flush`
- [ ] Batch/unbatch/set_screen have `USE_RUST_GFX` guards at the real C entry points
- [ ] Colormap init/uninit wired at the real lifecycle boundary
- [ ] Key colormap operations have `USE_RUST_GFX` guards where required
- [ ] All active deferred control-path ingress points are explicitly inventoried with concrete file/function ownership
- [ ] `canvas.c` either remains untouched with explicit justification, or is modified only with documented call-site evidence
- [ ] `rust_gfx.h` included in all modified C files
- [ ] Full `rust_gfx.h` ↔ Rust export audit completed for DCQ, canvas, colormap, backend-facing symbols, and event-path exports
- [ ] Project builds successfully with `USE_RUST_GFX` enabled

## Semantic Verification Checklist (Mandatory)
- [ ] C draw calls reach Rust push functions with no loss of caller-visible state
- [ ] C flush calls reach Rust `process_commands`
- [ ] Color packing (C Color struct → u32) matches Rust unpacking
- [ ] Screen index mapping (C SCREEN_TYPE → Rust Screen) is correct
- [ ] Batch semantics hold through the actual C entry points
- [ ] Nested batch semantics hold through the actual C entry points
- [ ] Transition capture reads the correct already-flushed pixels after wiring
- [ ] Extra-screen copy workflows produce correct visible results after wiring
- [ ] Context-driven state propagation is preserved after wiring
- [ ] Flush completion synchronization matches the established UQM path
- [ ] Idle/no-redraw behavior holds on the migrated path when no visible update is required
- [ ] ReinitVideo behavior is revalidated through the real lifecycle/orchestration path
- [ ] System-box compositing is revalidated through the real presentation orchestration path
- [ ] Deferred free ordering still holds on the migrated path
- [ ] Image metadata synchronization obligations hold at the migrated ABI boundary
- [ ] Event-processing call path remains reachable after bridge/lifecycle wiring and is ready for dedicated P10 verification
- [ ] Colormap lifecycle matches: init before use, uninit on shutdown
- [ ] No deferred control-path ingress remains partially on the C side without documented justification
- [ ] No double-free or leak of colormap/DCQ state
- [ ] Full project compiles and links

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" sc2/src/libs/graphics/ --include="*.c" --include="*.h" | head -20
```

## Success Criteria
- [ ] All required C draw/control functions redirect to Rust under `USE_RUST_GFX`
- [ ] Flush redirects to Rust
- [ ] Colormap lifecycle wired at the correct boundary
- [ ] Deferred control-path ingress inventory is complete and acted on
- [ ] Full `rust_gfx.h` ↔ Rust export parity audit passes
- [ ] Project builds and links successfully
- [ ] Migration-sensitive semantics are revalidated through the real bridge path
- [ ] Verification commands pass

## Failure Recovery
- Rollback: `git checkout -- sc2/src/libs/graphics/`
- Blocking: Link errors → verify exact symbol names match between `rust_gfx.h` and Rust `#[no_mangle]` exports
- Blocking: Type mismatch → verify C types match Rust FFI parameter types and actual caller surface

## Phase Completion Marker
Create: `project-plans/20260311/graphics/.completed/P09.md`
