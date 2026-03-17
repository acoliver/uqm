# Phase 10: Integration Testing + Cleanup

## Phase ID
`PLAN-20260314-GRAPHICS.P10`

## Prerequisites
- Required: Phase P09 completed
- Verify: C bridge wiring builds and links
- Expected: All required Rust FFI functions tested, C `USE_RUST_GFX` guards in place

## Requirements Implemented (Expanded)

### REQ-INT-006: Transition-source compatibility
**Requirement text**: When UQM saves transition source imagery from the main screen and later uses it for transition rendering, the subsystem shall preserve the visible behavior of that workflow.

### REQ-INT-007: Extra-screen workflow compatibility
**Requirement text**: When UQM uses the extra screen as an off-screen staging or copy surface, the subsystem shall preserve the visible results of copying to and from that screen.

### REQ-INT-008: Context-driven draw compatibility
**Requirement text**: When higher-level UQM graphics/context code supplies draw mode, color, clipping, font, scale, or target-screen state, the subsystem shall honor that state.

### REQ-INT-009: Synchronization compatibility
**Requirement text**: When UQM waits for rendering completion through existing synchronization mechanisms, the subsystem shall preserve those synchronization points.

### REQ-DQ-003 / REQ-DQ-004 / REQ-OWN-006
This phase adds migrated-path integration coverage for batching, nested batching, and deferred destruction ordering after the real C bridge is live.

### REQ-SCAL-006
This phase adds deterministic semantic verification for scanline output on the migrated path, not only structural proof that scanline code exists.

## Implementation Tasks

### Task 1: Integration test suite

#### File: `rust/tests/graphics_integration.rs` (new or expanded)
Create integration tests that exercise the migrated C→Rust pipeline and migration-sensitive requirements:

1. **Lifecycle integration**: `rust_gfx_init` → `rust_dcq_init` → `rust_cmap_init` → operations → `rust_cmap_uninit` → `rust_dcq_uninit` → `rust_gfx_uninit`
2. **DCQ roundtrip**: init → push Line → push Rect → push Image → flush → verify no crash and queue drains correctly
3. **Canvas roundtrip**: create surface → `rust_canvas_from_surface` → draw → flush/sync → verify surface pixels changed → destroy
4. **Colormap roundtrip**: init → set colormap → get colormap → verify data matches → uninit
5. **Batch semantics**: batch → push commands → flush → verify not visible yet → unbatch → verify visibility
6. **Nested batch semantics**: batch → push A → batch → push B → unbatch → flush → verify still hidden → unbatch → flush → verify A then B become visible
7. **Screen targeting**: set_screen(1) → push → verify command targets Extra → set_screen(0) → push → verify targets Main
8. **Transition capture timing**: flush main-screen pixels → capture transition source → enqueue more unflushed work → verify captured content matches already-flushed state only
9. **Transition stability**: after capture, mutate main screen further → verify transition screen remains stable for the effect duration
10. **Extra-screen workflow**: copy to/from extra screen and verify visible results
11. **Context-driven state propagation**: verify draw mode, clipping, font/backing image, scale, colormap, and target-screen state survive the bridge and affect output correctly
12. **Synchronization compatibility**: verify migrated flush path still releases the established completion wait/signal mechanism
13. **Deferred destruction ordering**: enqueue uses followed by deletion and verify prior uses complete before free
14. **Scanline semantic check**: render a deterministic known frame with scanlines enabled and verify alternating-line dimming through framebuffer/image sampling rather than only structural assertions
15. **Idle/no-redraw migrated-path check**: verify empty queue + no fade/transition returns without visible-output change on the real integrated path
16. **Reinit/system-box migrated-path checks**: where safely automatable, verify the integrated path still honors reinit semantics and system-box compositing ordering

### Task 2: Remove backup files (G16)

#### Files to delete
- `rust/src/graphics/tfb_draw.rs.bak` (if exists)
- `rust/src/graphics/tfb_draw.rs.bak3` (if exists)
- Any other `.bak*` files in the graphics directory

```bash
find rust/src/graphics/ -name "*.bak*" -delete
```

### Task 3: Verify canvas API contract rather than inventing a new mode feature

#### File: `rust/src/graphics/canvas_ffi.rs`
- Verify that `rust_canvas_draw_rect` and `rust_canvas_fill_rect` remain aligned with the actual C header/caller contract
- Only change signatures if concrete mismatch evidence remains after P08/P09

### Task 4: Review and fix any remaining TODO/FIXME in graphics code introduced by plan phases

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder" rust/src/graphics/ sc2/src/libs/graphics/ --include="*.rs" --include="*.c" --include="*.h" | grep -v "test" | grep -v ".bak"
```

Address each finding:
- If it's in production code from a gap-closure phase: fix it
- If it's a pre-existing TODO outside plan scope: document and defer

## TDD Test Plan

### Tests in `rust/tests/graphics_integration.rs`
1. `test_lifecycle_init_uninit` — full lifecycle without crash
2. `test_dcq_push_and_flush` — push commands, flush, verify queue empty
3. `test_canvas_pixel_roundtrip_integration` — surface → canvas → draw → sync → verify surface
4. `test_colormap_set_get_roundtrip` — set colors, get back, verify match
5. `test_batch_defers_visibility` — batch prevents premature visibility
6. `test_nested_batch_defers_until_outermost` — nested batching preserves invisibility until outermost unbatch
7. `test_screen_targeting` — commands tagged with correct screen
8. `test_transition_capture_uses_flushed_pixels_only` — verifies REQ-INT-006 / REQ-TRANS-001
9. `test_transition_screen_stability_after_capture` — verifies REQ-TRANS-002
10. `test_extra_screen_copy_workflow` — verifies REQ-INT-007
11. `test_context_state_propagation` — verifies REQ-INT-008
12. `test_flush_completion_sync` — verifies REQ-INT-009
13. `test_deferred_delete_after_prior_uses` — verifies REQ-OWN-006
14. `test_scanline_output_dims_alternating_rows` — verifies REQ-SCAL-006 with framebuffer/image sampling on a known frame
15. `test_idle_flush_without_visible_update_preserves_output` — verifies REQ-RL-009 on the migrated path
16. `test_reinit_and_system_box_migrated_path_behavior` — verifies migration-sensitive orchestration behavior as far as safely automatable

### Verify existing tests still pass
17. Run full test suite: `cargo test --workspace --all-features`

## Verification Commands

```bash
# Full Rust verification
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features

# Integration tests specifically
cargo test --test graphics_integration

# No backup files
find rust/src/graphics/ -name "*.bak*" | wc -l
# Expected: 0

# No TODOs in implementation code introduced by this plan
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/graphics/ sc2/src/libs/graphics/ --include="*.rs" --include="*.c" --include="*.h" | grep -v "#\[cfg(test)\]" | grep -v "test_" | wc -l
```

## Structural Verification Checklist
- [ ] Integration test file exists with 16+ tests covering migrated-path semantics
- [ ] Backup files removed
- [ ] Canvas API contract rechecked against the actual header/caller surface
- [ ] Deterministic scanline semantic test exists
- [ ] No new TODOs introduced in implementation code

## Semantic Verification Checklist (Mandatory)
- [ ] Full lifecycle test: init → operations → uninit without crash
- [ ] DCQ commands flow through push → queue → flush → handler
- [ ] Canvas pixel coherence verified end-to-end
- [ ] Colormap data survives set/get roundtrip
- [ ] Batch semantics prevent premature visibility
- [ ] Nested batch semantics preserve invisibility until outermost unbatch
- [ ] Screen targeting correctly tags commands
- [ ] Transition capture uses only already-flushed pixels
- [ ] Transition screen remains stable after capture
- [ ] Extra-screen copy workflow produces correct results
- [ ] Context-driven draw state survives the migrated bridge
- [ ] Existing synchronization points still work
- [ ] Deferred destruction ordering holds after bridge wiring
- [ ] Scanline output is semantically verified with deterministic sampling/image comparison
- [ ] Idle/no-redraw behavior is verified on the migrated path
- [ ] Reinit/system-box migrated-path checks are executed to the extent safely automatable
- [ ] All existing tests pass (no regressions)

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/graphics/ sc2/src/libs/graphics/ --include="*.rs" --include="*.c" --include="*.h" | grep -v test
```

## Success Criteria
- [ ] 16+ integration tests pass
- [ ] 0 backup files remain
- [ ] Migration-sensitive semantics are covered by end-to-end integration tests
- [ ] Scanline behavior is semantically verified before P11
- [ ] No deferred implementation patterns in production code
- [ ] All verification commands pass

## Failure Recovery
- Rollback: `git checkout -- rust/src/graphics/ sc2/src/libs/graphics/ rust/tests/`

## Phase Completion Marker
Create: `project-plans/20260311/graphics/.completed/P10.md`
