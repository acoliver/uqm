# Plan: Graphics Subsystem Gap Closure

Plan ID: PLAN-20260314-GRAPHICS
Generated: 2026-03-14
Total Phases: 17 (P00.5 through P11, each with verification sub-phase)
Requirements: REQ-RL-*, REQ-DQ-*, REQ-CAN-*, REQ-IMG-*, REQ-FONT-*, REQ-CMAP-*, REQ-FADE-*, REQ-SCAL-*, REQ-TRANS-*, REQ-ERR-*, REQ-OWN-*, REQ-INT-*

## Context

The graphics subsystem already has active Rust-side behavioral ownership for SDL initialization, presentation compositing (`preprocess`/`screen`/`color`/`postprocess`), surface creation, backend-vtable integration, and SDL event collection forwarding under `USE_RUST_GFX`. However, draw-command ingress, canvas pixel-coherence synchronization, several colormap/control paths, and other migration-sensitive bridge points are not yet fully redirected from the C entry points to the Rust implementation.

This plan closes the remaining gaps between the current Rust implementation and full specification parity. It does NOT reimagine the architecture -- it fixes concrete, identified deficiencies.

## Governing Documents

The authoritative source documents for this plan are:
- `project-plans/20260311/graphics/requirements.md`
- `project-plans/20260311/graphics/specification.md`

This plan does not use a `spec/requirements/` subdirectory. Any template expectation for `spec/requirements` is intentionally normalized here to the actual repository layout above, and all later references in this plan should be interpreted against those exact files.

## Gap Summary

| # | Gap | Severity | Requirements |
|---|-----|----------|-------------|
| G1 | Canvas pixel sync missing -- `SurfaceCanvas` creates a fresh `Canvas::new_rgba()` instead of importing existing surface pixels; synchronization points beyond destroy-time writeback are not defined | Critical | REQ-CAN-006, REQ-INT-006, REQ-INT-007 |
| G2 | `postprocess` still contains upload/scale/copy fallback that should be present-only | High | REQ-RL-004, REQ-INT-002, REQ-INT-010 |
| G3 | Scanline effect not implemented in Rust postprocess | Medium | REQ-SCAL-006 |
| G4 | DCQ FFI missing push functions: `FilledImage`, `FontChar`, `SetMipmap`, `DeleteData`, `Callback` | High | REQ-DQ-001, REQ-INT-001, REQ-INT-003 |
| G5 | `rust_dcq_push_drawimage` missing scale/colormap/drawmode parameters | High | REQ-DQ-005, REQ-IMG-003, REQ-IMG-004, REQ-INT-008 |
| G6 | `rust_dcq_push_setpalette` is a stub (Callback log, not a real SetPalette command) | Medium | REQ-DQ-001, REQ-CMAP-003 |
| G7 | DCQ `ReinitVideo` handler is a no-op log, no actual reinit | Medium | REQ-RL-011, REQ-INT-002 |
| G8 | Canvas/DCQ/Colormap bridges have zero C call sites -- not wired from the real draw/control entry points; Rust-local verification before bridge wiring is provisional until revalidated through real C integration | Critical | REQ-INT-001, REQ-INT-004, REQ-INT-008 |
| G9 | System-box re-composite not implemented in presentation path | Medium | REQ-RL-012 |
| G10 | Bounding-box tracking during flush not implemented | Low | REQ-DQ-012 |
| G11 | `TFB_Image` rotation (`TFB_DrawImage_New_Rotated`) not implemented at the object/ABI boundary; plan must preserve image lifecycle, hotspot, and derived-field behavior, not just rotate pixels | Medium | REQ-IMG-007, REQ-IMG-008, REQ-OWN-002 |
| G12 | DCQ flush doesn't broadcast `RenderingCond`, doesn't handle empty-queue fade/transition redraw, and does not explicitly verify the complementary idle/no-redraw return path | High | REQ-DQ-006, REQ-DQ-007, REQ-RL-008, REQ-RL-009, REQ-INT-009 |
| G13 | `SetPalette` DCQ command variant missing from `DrawCommand` enum | Medium | REQ-DQ-001 |
| G14 | Queue batch visibility/nesting semantics are not explicitly analyzed or verified for the Rust migration path | High | REQ-DQ-003, REQ-DQ-004 |
| G15 | Deferred free ordering and image metadata synchronization are not explicitly validated for the C→Rust migration path | High | REQ-OWN-006, REQ-OWN-007 |
| G16 | Event-pump lifecycle and SDL event forwarding are not explicitly revalidated through init/reinit/uninit on the migrated path | High | REQ-RL-001, REQ-RL-011, REQ-INT-001, REQ-INT-002 |
| G17 | Backup files (`tfb_draw.rs.bak`, `tfb_draw.rs.bak3`) pollute source tree | Cleanup | N/A |

## Authoritative DCQ Inventory

The draw-command queue command inventory is exactly 16 commands, matching specification §5.1:
1. `Line`
2. `Rect`
3. `Image`
4. `FilledImage`
5. `FontChar`
6. `Copy`
7. `CopyToImage`
8. `SetMipmap`
9. `DeleteImage`
10. `DeleteData`
11. `SendSignal`
12. `ReinitVideo`
13. `SetPalette`
14. `ScissorEnable`
15. `ScissorDisable`
16. `Callback`

`batch`, `unbatch`, and `set_screen` are queue control operations, not queue commands. They affect enqueue visibility and destination state but are not part of the 16-command inventory.

## Phase Structure

| Phase | Title | Gaps Addressed | Est. LoC |
|-------|-------|---------------|----------|
| P00.5 | Preflight Verification | -- | 0 |
| P01 | Analysis | -- | 0 |
| P01a | Analysis Verification | -- | 0 |
| P02 | Pseudocode | -- | 0 |
| P02a | Pseudocode Verification | -- | 0 |
| P03 | Canvas Pixel Sync | G1 | ~170 |
| P03a | Canvas Pixel Sync Verification | -- | 0 |
| P04 | Postprocess Cleanup + Scanlines | G2, G3 | ~120 |
| P04a | Postprocess + Scanlines Verification | -- | 0 |
| P05 | DCQ Command Completeness | G4, G5, G6, G13 | ~350 |
| P05a | DCQ Command Completeness Verification | -- | 0 |
| P06 | DCQ Flush + Queue Semantics Parity | G10, G12, G14, G15 | ~180 |
| P06a | DCQ Flush + Queue Semantics Verification | -- | 0 |
| P07 | System-Box Compositing + ReinitVideo | G7, G9 | ~160 |
| P07a | System-Box + ReinitVideo Verification | -- | 0 |
| P08 | Image Rotation + Canvas Mode | G11 | ~120 |
| P08a | Image Rotation Verification | -- | 0 |
| P09 | C-Side Bridge Wiring + Revalidation | G8 | ~220 (C) |
| P09a | C-Side Bridge Wiring Verification | -- | 0 |
| P10 | Event-Pump Lifecycle + Forwarding Revalidation | G16 | ~70 |
| P10a | Event-Pump Verification | -- | 0 |
| P11 | Integration Testing + Cleanup | G17 | ~90 |
| P11a | Integration Testing Verification | -- | 0 |
| P12 | End-to-End Verification | All | 0 |

Total estimated new/modified LoC: ~1380 (Rust/C combined, mostly Rust) + ~220 (C bridge-focused)

## Execution Order

```
P00.5 -> P01 -> P01a -> P02 -> P02a
       -> P03 -> P03a -> P04 -> P04a
       -> P05 -> P05a -> P06 -> P06a
       -> P07 -> P07a -> P08 -> P08a
       -> P09 -> P09a -> P10 -> P10a
       -> P11 -> P11a -> P12
```

Each phase MUST be completed and verified before the next begins. No skipping.

## Critical Reminders

Before implementing any phase:
1. Preflight verification is complete (Phase 0.5)
2. Integration points are explicitly listed
3. TDD cycle is defined per slice
4. Lint/test/coverage gates are declared
5. Rust-local verification before P09 is provisional for migration-sensitive behaviors and MUST be revalidated after C bridge wiring lands
6. Source-document references must use the exact governing document paths listed above
7. Every `DONE / REVALIDATE` requirement claim must point to a concrete downstream phase task or checklist item

## Definition of Done

1. All `cargo test --workspace --all-features` pass
2. All `cargo clippy --workspace --all-targets --all-features -- -D warnings` pass
3. `cargo fmt --all --check` passes
4. Game boots with `USE_RUST_GFX=1` and renders correctly
5. Canvas pixel coherence verified at all synchronization points: presentation, transition capture, and interop readback
6. All 16 DCQ command types are pushable through FFI and dispatchable during flush
7. Batch and nested batch visibility semantics verified through the migrated C→Rust path
8. Deferred destruction ordering and image metadata synchronization obligations verified through the migrated path
9. Transition capture timing, extra-screen workflows, context-driven draw state propagation, and event forwarding/lifecycle behavior verified end-to-end
10. Scanline effect visible when enabled
11. System box remains visible through fades
12. Empty-queue redraw and idle/no-redraw behavior both verified against REQ-RL-008 and REQ-RL-009
13. No backup files or placeholder stubs remain in implementation code

## Plan Files

```
plan/
  00-overview.md                          (this file)
  00a-preflight-verification.md           P00.5: toolchain/dep/type/call-path checks
  01-analysis.md                          P01: detailed gap analysis for G1-G17
  01a-analysis-verification.md            P01a: requirements coverage matrix
  02-pseudocode.md                        P02: pseudocode for all implementations
  02a-pseudocode-verification.md          P02a: pseudocode completeness checklist
  03-canvas-pixel-sync.md                 P03: import/export surface pixels + synchronization points (G1)
  03a-canvas-pixel-sync-verification.md   P03a
  04-postprocess-scanlines.md             P04: strip postprocess fallback + add scanlines (G2, G3)
  04a-postprocess-scanlines-verification.md  P04a
  05-dcq-command-completeness.md          P05: missing push functions + drawimage params + setpalette (G4, G5, G6, G13)
  05a-dcq-command-completeness-verification.md  P05a
  06-dcq-flush-parity.md                  P06: bbox, livelock blocking, flush signal, batching, deferred frees, image sync, idle/no-redraw semantics (G10, G12, G14, G15)
  06a-dcq-flush-parity-verification.md    P06a
  07-system-box-reinit.md                 P07: system-box compositing + ReinitVideo handler (G7, G9)
  07a-system-box-reinit-verification.md   P07a
  08-image-rotation-canvas-mode.md        P08: image rotation ABI/lifecycle compatibility (G11)
  08a-image-rotation-canvas-mode-verification.md  P08a
  09-c-bridge-wiring.md                   P09: wire C draw/control entry points to Rust DCQ and revalidate migration-sensitive semantics (G8)
  09a-c-bridge-wiring-verification.md     P09a
  10-event-pump-lifecycle.md              P10: event-pump lifecycle/forwarding verification across init/reinit/uninit (G16)
  10a-event-pump-lifecycle-verification.md P10a
  11-integration-testing.md               P11: migrated-path integration tests, scanline semantic checks, and cleanup (G17)
  11a-integration-testing-verification.md P11a
  12-e2e-verification.md                  P12: full end-to-end verification
```

## Deferred Items

The following items are explicitly out of scope for this plan (per specification/requirements delegation):

- **REQ-INT-005, REQ-INT-012**: Asset loading parity. Loader behavior is delegated to C code parity per specification S12.5. The C loading path (`gfxload.c`, `png2sdl.c`, `sdluio.c`) remains authoritative.
- **HQxx / xBRZ scaler internals**: Scaler buffer allocation exists; scaler algorithm implementation uses the `resize` crate. Detailed scaler algorithm parity is a future concern.
- **OpenGL backend**: The `sdl.rs` module has an OpenGL driver stub. Full OpenGL backend is out of scope.
- **Multi-threaded DCQ producers beyond the current UQM threading contract**: The plan preserves current synchronization/forward-progress guarantees but does not expand the engine threading model.
