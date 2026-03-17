# Phase 15: C-Side Bridge & Build Toggle

## Phase ID
`PLAN-20260314-CAMPAIGN.P15`

## Prerequisites
- Required: Phase 14a completed
- Expected files: All Rust campaign modules complete and tested
- Dependency: Build system (`sc2/build/unix/build.config`)
- Dependency: validated seam inventory from P01 and accessor model from P03.5

## Requirements Implemented (Expanded)

### Build Toggle (project convention)
Following existing pattern (`USE_RUST_CLOCK`, `USE_RUST_STATE`, `USE_RUST_COMM`), add `USE_RUST_CAMPAIGN` toggle.

### FFI Bridge
All Rust campaign functions callable from C must be exposed via `extern "C"` exports, and all C functions called by Rust campaign code must have FFI declarations.

### C-Side Guards
C bodies for replaced functionality must be guarded behind `#ifndef USE_RUST_CAMPAIGN` **only after** the seam inventory confirms the replacement mode and exact signature.

### Mandatory Runtime-Owner Wiring
`sc2/src/uqm/restart.c` and `sc2/src/uqm/starcon.c` are not optional examples in this phase. Once P01/P03.5 validate the concrete seams, P15 must wire the start/load owner and the live campaign-loop owner so `USE_RUST_CAMPAIGN` actually switches runtime control to Rust.

## Implementation Tasks

### Bridge policy

P15 must consume the validated seam inventory from P01/P03.5. No export/import/guard is allowed here unless it maps back to:
- source file
- exact function or call site
- validated replacement mode
- validated signature
- caller/callee ownership notes

Unvalidated speculative names from earlier drafts are intentionally removed from this phase.

### Mandatory owner-seam obligations

In addition to general bridge work, P15 must include validated runtime-owner wiring for:
- `sc2/src/uqm/restart.c` — the new-game/load-game/restart start-flow owner seam that chooses the Rust start/load path when `USE_RUST_CAMPAIGN` is enabled
- `sc2/src/uqm/starcon.c` — the top-level campaign-loop/kernal-lifecycle owner seam that invokes Rust campaign dispatch when `USE_RUST_CAMPAIGN` is enabled

These seams may be implemented as function-body guards, wrapper calls, or narrowly-scoped call-site substitution depending on the validated inventory, but they are mandatory deliverables. This phase is incomplete if Rust modules exist but the live runtime still never enters them.

### Files to create

- `rust/src/campaign/ffi.rs` — FFI exports for campaign subsystem
  - marker: `@plan PLAN-20260314-CAMPAIGN.P15`
  - Export only the validated Rust bridge functions from the seam inventory
  - Each export must carry a comment/doc tag pointing back to its validated seam row ID from P01
  - Exports must include the validated wrapper(s) needed for the mandatory owner seams in `restart.c` and `starcon.c`
  - Additional exports may include validated wrappers for:
    - event registration / event dispatch seam(s)
    - save/load seam(s)
    - encounter seam(s)
    - starbase seam(s)
    - hyperspace transition/menu seam(s)
    - canonical export seam(s)
  - FFI imports (C functions called by Rust) limited to validated lower-boundary seams such as:
    - communication/dialogue entrypoints
    - battle entrypoint
    - solar-system exploration entrypoint
    - graphics/audio/input helpers only if the seam inventory proves campaign ownership requires them
  - Safety wrappers for all FFI calls with proper error handling

- `sc2/src/uqm/campaign_rust.h` — Rust campaign FFI declarations for C
  - marker: `@plan PLAN-20260314-CAMPAIGN.P15`
  - Declarations only for validated `rust_*` functions
  - Include guard: `#ifdef USE_RUST_CAMPAIGN`

- `sc2/src/uqm/campaign_rust.c` — Rust campaign bridge (thin C wrapper)
  - marker: `@plan PLAN-20260314-CAMPAIGN.P15`
  - Any C-side adaptation needed (struct conversion, context setup)
  - No new campaign policy logic beyond validated bridge adaptation

### Files to modify

- `sc2/build/unix/build.config`
  - Add `USE_RUST_CAMPAIGN` toggle (default off initially, on when rust bridge enabled)
  - Add symbol: `SYMBOL_USE_RUST_CAMPAIGN_DEF`
  - Add to export list
  - Add to rust-bridge-enable action

- `sc2/src/uqm/restart.c`
  - Guard or wrap the validated start/load owner seam only
  - Route the validated new-game/load-game/restart entry path to the corresponding `rust_*` export when `USE_RUST_CAMPAIGN` is enabled
  - Preserve C path unchanged when toggle is off

- `sc2/src/uqm/starcon.c`
  - Guard or wrap the validated campaign-loop/kernal-lifecycle owner seam only
  - Route the validated live campaign dispatch path to the corresponding `rust_*` export when `USE_RUST_CAMPAIGN` is enabled
  - Preserve C path unchanged when toggle is off

- Additional validated C source files from the seam inventory only
  - Guard or wrap only the specific validated function bodies/call sites
  - Additional files may include portions of `save.c`, `load.c`, `gameev.c`, `encount.c`, `starbase.c`, `hyper.c`
  - Final additional file list and exact functions are determined by the seam inventory, not assumed here

- `rust/src/campaign/mod.rs`
  - Add `pub mod ffi;`

## Verification Commands

```bash
# With USE_RUST_CAMPAIGN disabled (C path):
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cd sc2 && ./build.sh uqm  # Verify C-only build still works

# With USE_RUST_CAMPAIGN enabled (Rust path):
cd sc2 && USE_RUST_CAMPAIGN=1 ./build.sh uqm  # Verify Rust-bridged build links
```

## Structural Verification Checklist
- [ ] `ffi.rs` created with only seam-inventory-backed exports/imports
- [ ] `campaign_rust.h` and `campaign_rust.c` created
- [ ] `build.config` updated with `USE_RUST_CAMPAIGN` toggle
- [ ] `restart.c` start/load owner seam is wired to validated Rust export(s)
- [ ] `starcon.c` campaign-loop owner seam is wired to validated Rust export(s)
- [ ] Every guarded C body/call site is justified by a validated seam row
- [ ] Build succeeds with toggle OFF (C path)
- [ ] Build succeeds with toggle ON (Rust path)
- [ ] Linker resolves all cross-language symbols

## Semantic Verification Checklist (Mandatory)
- [ ] C-only build: game runs identically to pre-change baseline
- [ ] Rust-bridged build: FFI functions callable from C, correct return types
- [ ] Every `rust_*` export/import in P15 is traceable back to a validated seam in P01
- [ ] No speculative entrypoint ownership assumptions remain (for example, entire-loop ownership versus selected-function bridging must be settled, not mixed)
- [ ] `USE_RUST_CAMPAIGN=1` causes live runtime entry through validated `restart.c` and `starcon.c` owner seams, not only through test-only bridge helpers
- [ ] Start-flow ownership switches to Rust for new-game/load-game paths when toggle is on
- [ ] Main campaign-loop ownership switches to Rust when toggle is on
- [ ] All 27+ race scripts compile without modification
- [ ] No duplicate symbol errors
- [ ] No undefined symbol errors
- [ ] Guards are complete for the validated seams and do not over-guard unrelated C code

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/campaign/ffi.rs
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" sc2/src/uqm/campaign_rust.c
```

## Success Criteria
- [ ] Build toggle works for both paths
- [ ] FFI bridge links correctly
- [ ] C-only path remains functional
- [ ] Replacement boundaries are validated and internally consistent
- [ ] Live runtime reaches Rust through the mandatory `restart.c` and `starcon.c` owner seams

## Failure Recovery
- rollback: `git checkout -- rust/src/campaign/ffi.rs sc2/src/uqm/campaign_rust.h sc2/src/uqm/campaign_rust.c sc2/build/unix/build.config sc2/src/uqm/restart.c sc2/src/uqm/starcon.c sc2/src/uqm/*.c`

## Phase Completion Marker
Create: `project-plans/20260311/campaign-gameplay/.completed/P15.md`
