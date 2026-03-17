# Phase 07: System-Box Compositing + ReinitVideo

## Phase ID
`PLAN-20260314-GRAPHICS.P07`

## Prerequisites
- Required: Phase P06 completed
- Verify: DCQ flush parity tests pass
- Expected files from previous phase: Modified `dcqueue.rs`

## Requirements Implemented (Expanded)

### REQ-RL-012: System-box visibility through fades
**Requirement text**: When the system box is active during presentation, the subsystem shall re-composite the designated main-screen subregion after fade overlay and before final present.

Behavior contract:
- GIVEN: A fade overlay is active (fade_amount != 255) and a system box region is defined
- WHEN: `TFB_SwapBuffers` compositing sequence executes
- THEN: The main screen is re-composited at full opacity within the system-box rectangle, after the fade overlay, making system UI visible through fades

Why it matters:
- The system menu/progress box must remain readable during screen fades
- Without re-composite, the system box would be obscured by the fade overlay

### REQ-RL-011: Reinitialization
**Requirement text**: When the video subsystem is reinitialized, the subsystem shall either restore a valid rendering backend or report failure. On failure, attempt reversion. On double failure, terminate.

Behavior contract:
- GIVEN: A `ReinitVideo` command is dispatched during flush
- WHEN: The handler executes
- THEN: The video backend is torn down and re-created with new parameters
- THEN: On failure, the previous configuration is restored
- THEN: On double failure, the process exits

### REQ-RL-001 / REQ-INT-002 event-sensitive implication
Because reinit can replace SDL backend state, this phase must identify the event-pump ownership boundary that P10 will later revalidate on the migrated path.

## Phase-scoping note

This phase intentionally keeps two concerns together because both are still open, but they are treated as separate subtracks with separate verification:
- **P07-A:** presentation-path orchestration for system-box visibility
- **P07-B:** lifecycle/DCQ orchestration for `ReinitVideo`

Neither subtrack is considered fully validated until it is rechecked again after P09 C-bridge wiring, because the real migrated path still crosses the C orchestration boundary. Event-pump continuity after reinit is explicitly handed off to P10 rather than left implicit.

## Implementation Tasks

### Task 1 (P07-A): System-box compositing support in the real orchestration path

#### Analysis
The system-box re-composite is orchestrated by C code in `sdl_common.c`'s `TFB_SwapBuffers`. The sequence is:
1. `preprocess()`
2. `screen(MAIN, 255, NULL)`
3. `screen(TRANSITION, ...)` if transitioning
4. `color(...)` if fading
5. **`screen(MAIN, 255, &system_box_rect)` if system box active** ← this is the gap
6. `postprocess()`

The Rust `screen()` function already handles clipped compositing (rect parameter). The gap is in the C orchestration layer, not in Rust `postprocess`.

#### File: `sc2/src/libs/graphics/sdl/sdl_common.c`
- In `TFB_SwapBuffers()` (around line 275-330): verify or add the system-box `screen()` call after `color()` under `USE_RUST_GFX`
- If the call already exists, confirm ordering and data flow, then document that no code change was needed
- marker: `@plan PLAN-20260314-GRAPHICS.P07`
- marker: `@requirement REQ-RL-012`

#### File: `rust/src/graphics/ffi.rs`
- Verify `rust_gfx_screen()` correctly handles the re-composite case (full opacity, clipped rect, after fade)
- No changes expected unless actual clipped-compositing incompatibility is found
- marker: `@requirement REQ-RL-012`

### Task 2 (P07-B): ReinitVideo handler implementation

#### File: `rust/src/graphics/dcqueue.rs`
- In `handle_command()` for `DrawCommand::ReinitVideo { driver, flags, width, height }`:
  - Save current configuration (driver, flags, dimensions)
  - Call internal Rust init/uninit helpers, not speculative ad hoc extern-only calls
  - Identify and preserve the event-pump ownership boundary that will need post-reinit verification in P10
  - On init failure: attempt reversion with saved config
  - On reversion failure: call `std::process::exit(1)`
  - On success: update/rebind any DCQ state tied to surfaces/resources
- marker: `@plan PLAN-20260314-GRAPHICS.P07`
- marker: `@requirement REQ-RL-011`

#### File: `rust/src/graphics/ffi.rs`
- If `rust_gfx_uninit()` / `rust_gfx_init()` are extern wrappers only, factor the shared logic into internal helpers with the exact same semantics
- Resolve and document the real event-pump lifecycle owner at the same helper boundary so P10 can verify init → process_events → reinit → process_events → uninit behavior concretely
- The plan should reference the real helper boundary once identified rather than leaving the integration point vague
- marker: `@requirement REQ-RL-011, REQ-RL-001`

### Pseudocode traceability
- Uses pseudocode lines: PC-07 (190-202), PC-08 (210-220)

## TDD Test Plan

### Tests to add

1. `test_screen_with_clip_rect` — verify `rust_gfx_screen()` handles non-null clip rect (may already exist in ffi.rs)
2. `test_reinit_video_handler_saves_config` — push ReinitVideo, verify old config preserved
3. `test_reinit_video_invalid_params` — push ReinitVideo with bad params, verify reversion attempted
4. `test_reinit_video_double_failure_path` — verify irrecoverable failure exits through the documented path using an isolatable test strategy if feasible
5. `test_reinit_video_preserves_event_state_contract_metadata` — verify the reinit path updates the identified event-pump owner/handle consistently enough for later P10 behavioral verification

Note: Full `ReinitVideo` semantic verification is difficult in unit tests alone. P09/P10/P12 must revalidate the behavior again through the migrated orchestration path.

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] System-box compositing call present in C `TFB_SwapBuffers` sequence (or existing call verified with evidence)
- [ ] ReinitVideo handler in `handle_command()` is non-trivial (not just a log)
- [ ] Reversion logic present
- [ ] Double-failure exit present
- [ ] Internal helper boundary for init/uninit is concrete, not speculative
- [ ] Event-pump ownership boundary is explicitly identified for post-reinit verification in P10
- [ ] Plan/requirement markers present

## Semantic Verification Checklist (Mandatory)
- [ ] System-box call happens after fade overlay in compositing sequence
- [ ] ReinitVideo handler calls uninit then init with new params via the actual internal helper path
- [ ] Reversion attempts init with saved config on failure
- [ ] Double failure calls `process::exit(1)`
- [ ] Event-pump lifecycle implications are concretely identified, not left implicit
- [ ] P09/P10/P12 revalidate both behaviors through the actual migrated path
- [ ] All existing tests pass

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/graphics/dcqueue.rs rust/src/graphics/ffi.rs sc2/src/libs/graphics/sdl/sdl_common.c | grep -i "reinit\|system\|event"
```

## Success Criteria
- [ ] REQ-RL-012: System-box compositing path verified in the correct orchestration layer
- [ ] REQ-RL-011: ReinitVideo handler with reversion and exit behavior
- [ ] Event-pump lifecycle owner concretely identified for migrated-path revalidation
- [ ] Verification commands pass

## Failure Recovery
- Rollback: `git checkout -- rust/src/graphics/dcqueue.rs rust/src/graphics/ffi.rs sc2/src/libs/graphics/sdl/sdl_common.c`

## Phase Completion Marker
Create: `project-plans/20260311/graphics/.completed/P07.md`
