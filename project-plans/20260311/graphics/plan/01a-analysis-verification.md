# Phase 01a: Analysis Verification

## Phase ID
`PLAN-20260314-GRAPHICS.P01a`

## Prerequisites
- Required: Phase P01 completed

## Structural Verification
- [ ] All 17 gaps (G1-G17) are documented
- [ ] Each substantive gap references specific file paths and concrete integration boundaries
- [ ] Each gap references specific REQ-* identifiers
- [ ] Integration touchpoints table is complete
- [ ] Old code removal list is complete

## Semantic Verification
- [ ] Every normative requirement from requirements.md is traceable to at least one gap or marked as already satisfied with evidence
- [ ] No gap proposes reimplementation of working functionality without migration-boundary justification
- [ ] Data flow diagrams for pixel sync (G1) cover import/export and all required synchronization points
- [ ] DCQ command inventory is normalized to the spec §5.1 complete list of 16 commands, with queue control operations explicitly excluded from that count
- [ ] Queue batching and nested batching (G14) are explicitly analyzed
- [ ] Ownership-sensitive obligations (G15) are explicitly analyzed
- [ ] Event-pump lifecycle/forwarding (G16) is explicitly analyzed with init/reinit/uninit revalidation scope
- [ ] C wiring gap (G8) distinguishes required bridge wiring from out-of-scope speculative canvas.c work
- [ ] REQ-RL-009 idle/no-redraw behavior is explicitly traced as a first-class requirement, not only implied by REQ-DQ-007 handling
- [ ] Every `DONE / REVALIDATE` matrix row names at least one concrete downstream phase where verification occurs

## Requirements Coverage Matrix

| Requirement | Covered by Gap / Evidence | Status |
|-------------|---------------------------|--------|
| REQ-RL-001 | `rust/src/graphics/gfx_common.rs` init/reinit/uninit state management; revalidate full lifecycle in P09, event-pump ownership/forwarding in P10, integrated behavior in P12 | DONE / REVALIDATE IN P09-P10-P12 |
| REQ-RL-002 | `rust/src/graphics/gfx_common.rs` logical screen model (`Main`, `Extra`, `Transition`) and presentation APIs; revalidate through bridge in P09 and integrated verification in P11-P12 | DONE / REVALIDATE IN P09-P11-P12 |
| REQ-RL-003 | `rust/src/graphics/gfx_common.rs` logical dimensions and validation paths; revalidate visible output in P09 and P12 | DONE / REVALIDATE IN P09-P12 |
| REQ-RL-004 | G2; `rust/src/graphics/gfx_common.rs` presentation sequence remains active but postprocess cleanup must remove duplicated fallback work; verify in P04 and P12 | PARTIAL / PLANNED |
| REQ-RL-005 | `rust/src/graphics/gfx_common.rs` `screen()` compositing of main layer; revalidate final orchestration through P09 and P12 | DONE / REVALIDATE IN P09-P12 |
| REQ-RL-006 | `rust/src/graphics/gfx_common.rs` transition compositing path plus transition-screen support; revalidate capture/presentation timing in P09, P11, and P12 | DONE / REVALIDATE IN P09-P11-P12 |
| REQ-RL-007 | `rust/src/graphics/gfx_common.rs` presentation layer does not composite extra screen by default; revalidate after bridge wiring in P09 and extra-screen tests in P11 | DONE / REVALIDATE IN P09-P11 |
| REQ-RL-008 | G12 empty-queue fade/transition redraw path in `rust/src/graphics/dcqueue.rs`; verify semantics in P06, migrated path in P09/P11, final check in P12 | PLANNED |
| REQ-RL-009 | G12 complementary idle/no-redraw early-return behavior in `rust/src/graphics/dcqueue.rs`; explicit semantic verification required in P06, migrated-path verification in P09/P11, final check in P12 | PLANNED |
| REQ-RL-010 | `rust/src/graphics/gfx_common.rs` teardown/resource clearing and ownership ordering; revalidate full shutdown in P09, P10 event lifecycle checks, and P12 | DONE / REVALIDATE IN P09-P10-P12 |
| REQ-RL-011 | G7 (ReinitVideo) in `rust/src/graphics/dcqueue.rs` plus real helper boundary in `rust/src/graphics/ffi.rs`; verify in P07, migrated path in P09, event-state revalidation in P10, final check in P12 | PLANNED |
| REQ-RL-012 | G9 in `sc2/src/libs/graphics/sdl/sdl_common.c` orchestration, with supporting clip-path verification in `rust/src/graphics/ffi.rs`; verify in P07, revalidate in P09/P11, final check in P12 | PLANNED |
| REQ-DQ-001 | G4, G6, G13 plus P09 real C ingress wiring for deferred draw/control commands; command inventory normalized in P01/P05/P09/P12 | PLANNED |
| REQ-DQ-002 | `rust/src/graphics/dcqueue.rs` queue order and `process_commands()` FIFO dispatch; revalidate after P09 wiring and in P12 | DONE / REVALIDATE IN P09-P12 |
| REQ-DQ-003 | G14 (batch visibility) in `rust/src/graphics/dcqueue.rs`, then migrated-path proof in P06, P09, P11, P12 | PLANNED |
| REQ-DQ-004 | G14 (nested batching) in `rust/src/graphics/dcqueue.rs`, then migrated-path proof in P06, P09, P11, P12 | PLANNED |
| REQ-DQ-005 | G5 (`rust/src/graphics/dcq_ffi.rs` drawimage parameter propagation), plus P09 screen-state wiring from C call sites and P11 migrated tests | PLANNED |
| REQ-DQ-006 | G12 flush completion signal in `rust/src/graphics/dcqueue.rs` and actual bridge-path completion behavior in P06/P09/P11/P12 | PLANNED |
| REQ-DQ-007 | G12 empty-queue fade handling in `rust/src/graphics/dcqueue.rs`; revalidate in P09/P11 through actual `TFB_FlushGraphics` redirect and final check in P12 | PLANNED |
| REQ-DQ-008 | `rust/src/graphics/dcqueue.rs` livelock/backpressure repair in P06 and final verification in P12 | PLANNED |
| REQ-DQ-009 | `rust/src/graphics/dcqueue.rs` callback dispatch path plus G4 missing callback push FFI in `rust/src/graphics/dcq_ffi.rs`; revalidate after P05/P09 wiring and in P12 | PARTIAL / PLANNED |
| REQ-DQ-010 | Existing deferred destruction mechanism in `rust/src/graphics/dcqueue.rs` plus G15 ordering verification in P06, migrated-path verification in P09/P11, final check in P12 | PARTIAL / PLANNED |
| REQ-DQ-011 | Existing copy/copy-to-image handlers in `rust/src/graphics/dcqueue.rs` plus G1/P09/P11 revalidation of current-pixel coherence | PARTIAL / PLANNED |
| REQ-DQ-012 | G10 bounding box tracking in `rust/src/graphics/dcqueue.rs`; verify in P06 and P12 | PLANNED |
| REQ-DQ-013 | Existing signal ordering in `rust/src/graphics/dcqueue.rs` plus G12/P09/P11 synchronization verification through migrated flush path | PARTIAL / PLANNED |
| REQ-CAN-001 | `rust/src/graphics/canvas_ffi.rs` opaque canvas-handle FFI surface | DONE |
| REQ-CAN-002 | `rust/src/graphics/canvas_ffi.rs` canvas creation/wrap extent handling; revalidate after P03/P09 sync changes and P12 | DONE / REVALIDATE IN P03-P09-P12 |
| REQ-CAN-003 | `rust/src/graphics/canvas.rs` / `rust/src/graphics/tfb_draw.rs` scissor/clipping behavior | DONE |
| REQ-CAN-004 | `rust/src/graphics/tfb_draw.rs` bounds clipping for primitives/copies/images/font rendering | DONE |
| REQ-CAN-005 | `rust/src/graphics/tfb_draw.rs` line/rect/fill/copy/image/font primitives | DONE |
| REQ-CAN-006 | G1 (`rust/src/graphics/canvas_ffi.rs` plus concrete read-synchronization hooks in real readback sites); verify in P03, revalidate in P09/P11, final in P12 | PLANNED |
| REQ-IMG-001 | Existing image object lifecycle in `rust/src/graphics/image.rs` / `rust/src/graphics/dcqueue.rs`; revalidate after P09 draw ingress wiring and in P12 | DONE / REVALIDATE IN P09-P12 |
| REQ-IMG-002 | Existing image destruction/cached-resource cleanup in `rust/src/graphics/image.rs`; revalidate with deferred-destroy path in P06/P11/P12 | DONE / REVALIDATE IN P06-P11-P12 |
| REQ-IMG-003 | Existing scale-cache logic in `rust/src/graphics/image.rs` plus G5 propagation of effective scale parameters from C ingress; verify in P05/P09/P11 | PARTIAL / PLANNED |
| REQ-IMG-004 | Existing cache invalidation in `rust/src/graphics/image.rs` plus G5/G6/P09 colormap and draw-state propagation verification | PARTIAL / PLANNED |
| REQ-IMG-005 | Existing mipmap association support in image model plus G4 missing `SetMipmap` push function in `rust/src/graphics/dcq_ffi.rs`; verify in P05/P09/P12 | PARTIAL / PLANNED |
| REQ-IMG-006 | Existing filled-image rendering in `rust/src/graphics/tfb_draw.rs` plus G4 filled-image push wiring; verify in P05/P09/P11 | PARTIAL / PLANNED |
| REQ-IMG-007 | G11 (Rotation object/ABI compatibility); verify in P08 and P12 | PLANNED |
| REQ-IMG-008 | G11 (Hotspot compatibility through rotated-object creation); verify in P08 and P12 | PLANNED |
| REQ-FONT-001 | `rust/src/graphics/tfb_draw.rs` glyph alpha blending | DONE |
| REQ-FONT-002 | `rust/src/graphics/tfb_draw.rs` glyph hotspot/origin handling | DONE |
| REQ-FONT-003 | Existing font/page/glyph metrics path in Rust font modules; revalidate after P09 font-char wiring and in P11/P12 | DONE / REVALIDATE IN P09-P11-P12 |
| REQ-FONT-004 | `rust/src/graphics/tfb_draw.rs` backing-image composition order; revalidate after P09 font-char ingress wiring and in P11/P12 | DONE / REVALIDATE IN P09-P11-P12 |
| REQ-CMAP-001 | Existing Rust colormap store capacity in `rust/src/graphics/colormap.rs`; revalidate lifecycle wiring in P09 and P12 | DONE / REVALIDATE IN P09-P12 |
| REQ-CMAP-002 | Existing 256-entry RGB colormap representation in `rust/src/graphics/colormap.rs` | DONE |
| REQ-CMAP-003 | G6 real `SetPalette` queue command plus P09 real colormap operation wiring, P11 integration tests, P12 final check | PARTIAL / PLANNED |
| REQ-CMAP-004 | Existing colormap version/change tracking in `rust/src/graphics/colormap.rs`; revalidate with migrated callers in P09/P11/P12 | DONE / REVALIDATE IN P09-P11-P12 |
| REQ-CMAP-005 | Existing retention/reference model in `rust/src/graphics/colormap.rs`; revalidate through queue and bridge behavior in P05/P09/P11 | PARTIAL / PLANNED |
| REQ-FADE-001 | Existing fade-intensity model in `rust/src/graphics/gfx_common.rs` / fade-related modules; revalidate migrated path in P09-P12 | DONE / REVALIDATE IN P09-P12 |
| REQ-FADE-002 | Existing fade progression state and update flow in Rust fade/compositing path; revalidate through actual flush/present path in P09-P12 | DONE / REVALIDATE IN P09-P12 |
| REQ-FADE-003 | Existing immediate-completion support in Rust fade handling; revalidate migrated path in P09-P12 | DONE / REVALIDATE IN P09-P12 |
| REQ-FADE-004 | Existing ordering of screen composition then fade color overlay in `rust/src/graphics/gfx_common.rs`; revalidate with real orchestration in P07/P09/P12 | DONE / REVALIDATE IN P07-P09-P12 |
| REQ-FADE-005 | Existing darkening behavior in color/fade compositing; revalidate migrated path in P09-P12 | DONE / REVALIDATE IN P09-P12 |
| REQ-FADE-006 | Existing brightening behavior in color/fade compositing; revalidate migrated path in P09-P12 | DONE / REVALIDATE IN P09-P12 |
| REQ-SCAL-001 | Existing logical-to-physical mapping in `rust/src/graphics/gfx_common.rs` and scaling modules | DONE |
| REQ-SCAL-002 | Existing presentation scaling selection/config support in scaling modules; revalidate with final present path in P09-P12 | DONE / REVALIDATE IN P09-P12 |
| REQ-SCAL-003 | Existing software scaler pipeline in Rust scaling modules; revalidate with cleaned postprocess path in P04/P12 | DONE / REVALIDATE IN P04-P12 |
| REQ-SCAL-004 | Existing sprite-scaling behavior in image drawing path plus G5 migrated-state propagation; verify in P05/P09/P11 | PARTIAL / PLANNED |
| REQ-SCAL-005 | Existing bilinear/trilinear compatibility in scaling/image modules; revalidate through migrated draw path in P09-P12 | DONE / REVALIDATE IN P09-P12 |
| REQ-SCAL-006 | G3 (Scanlines) plus deterministic semantic verification in P11 and final runtime/image-based recheck in P12 | PLANNED |
| REQ-SCAL-007 | Existing alpha-modulated screen compositing in `rust/src/graphics/gfx_common.rs` | DONE |
| REQ-SCAL-008 | Existing rectangular screen compositing/clipping in `rust/src/graphics/gfx_common.rs` | DONE |
| REQ-SCAL-009 | Existing transition-screen compositing path in `rust/src/graphics/gfx_common.rs`; revalidate with real transition-source capture timing in P09/P11/P12 | DONE / REVALIDATE IN P09-P11-P12 |
| REQ-TRANS-001 | G1 synchronization + concrete transition-capture hook identification + P09/P11 migrated-path tests + P12 final verification | PLANNED |
| REQ-TRANS-002 | Existing transition surface model; verify stability after C wiring in P09/P11/P12 | PARTIAL / PLANNED |
| REQ-TRANS-003 | Existing graphics-thread serialization model; revalidate with migrated C path in P09/P11/P12 | PARTIAL / PLANNED |
| REQ-ERR-001 | Existing `catch_unwind` use across FFI exports in `rust/src/graphics/*_ffi.rs` | DONE |
| REQ-ERR-002 | Existing initialized/not-initialized guards in `rust/src/graphics/gfx_common.rs` and FFI wrappers; event path revalidated in P10 | DONE / REVALIDATE IN P10 |
| REQ-ERR-003 | Existing null checks and safe failure paths in FFI wrappers | DONE |
| REQ-ERR-004 | Existing return-convention preservation in Rust FFI boundary; revalidate where new exports are added in P03/P05/P09/P10 | DONE / REVALIDATE IN P03-P05-P09-P10 |
| REQ-ERR-005 | Existing partial-init cleanup in gfx initialization code; revalidate with ReinitVideo helper refactor in P07 and event-state checks in P10 | DONE / REVALIDATE IN P07-P10 |
| REQ-ERR-006 | Existing bridge/logging for graphics failures; revalidate on new failure paths added by P03/P05/P07/P09/P10 | DONE / REVALIDATE IN P03-P05-P07-P09-P10 |
| REQ-ERR-007 | Existing range checks on bounded identifiers; revalidate with new screen/colormap ingress wiring in P05/P09 | DONE / REVALIDATE IN P05-P09 |
| REQ-OWN-001 | Existing backend resource ownership in `rust/src/graphics/gfx_common.rs` and driver state; revalidate lifecycle/event state in P09/P10/P12 | DONE / REVALIDATE IN P09-P10-P12 |
| REQ-OWN-002 | G11 rotated-image object lifecycle compatibility | PLANNED |
| REQ-OWN-003 | G1 surface coherence / access boundary | PLANNED |
| REQ-OWN-004 | Existing destruction ordering in owned graphics teardown; revalidate with ReinitVideo path in P07/P10/P12 | DONE / REVALIDATE IN P07-P10-P12 |
| REQ-OWN-005 | Existing colormap retention obligations plus P05/P09/P11 verification of queue/bridge usage | PARTIAL / PLANNED |
| REQ-OWN-006 | G15 (deferred free ordering); verify in P06/P09/P11/P12 | PLANNED |
| REQ-OWN-007 | G15 (image synchronization obligations); verify in P06/P09/P11/P12 | PLANNED |
| REQ-INT-001 | G4, G5, G8, G11, G16; migrated-path verification in P09-P12 | PLANNED |
| REQ-INT-002 | G2, G7, G9, G12, G8, G16; verify in P04/P07/P09/P10/P12 | PLANNED |
| REQ-INT-003 | G4, G5, G8, G11 plus explicit symbol/export audit before and during P09/P12 | PLANNED |
| REQ-INT-004 | G8 | PLANNED |
| REQ-INT-005, REQ-INT-012 | Deferred (asset loading — prose-delegated to C code parity) | DEFERRED |
| REQ-INT-006 | G1 synchronization + P09/P11 migrated-path transition-capture revalidation + P12 final verification | PLANNED |
| REQ-INT-007 | G1 coherence + P09/P11 extra-screen workflow tests + P12 final verification | PLANNED |
| REQ-INT-008 | G5 parameter propagation + G8 bridge wiring + P09/P11 context-driven tests + P12 final verification | PLANNED |
| REQ-INT-009 | G12 synchronization + P09/P11 migrated-path verification + P12 final verification | PLANNED |
| REQ-INT-010 | G2 plus P09/P12 present-count revalidation | PLANNED |
| REQ-INT-011 | Language-agnostic freedom; no implementation work required | DONE |

## Gate Decision
- [ ] All substantive requirements traced with concrete evidence or planned work
- [ ] No migration-sensitive gaps left undocumented
- [ ] PASS: proceed to pseudocode
