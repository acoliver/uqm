# Phase 11: FFI Bridge & C Wiring

## Phase ID
`PLAN-20260314-SUPERMELEE.P11`

## Prerequisites
- Required: Phase 10a completed and passed
- Compatibility-audit decisions recorded
- Scoped SuperMelee setup, selection, and netplay-boundary modules available for wiring

## Purpose

Wire the scoped Rust SuperMelee implementation into the existing C boundary without expanding into out-of-scope battle-engine porting. This phase covers only SuperMelee-owned entry points and integration seams for setup/menu, persistence, selection, and battle handoff boundaries.

## Implementation Tasks

### Files to create or complete

- `rust/src/supermelee/c_bridge.rs`
  - marker: `@plan PLAN-20260314-SUPERMELEE.P11`
  - Implement audited imported C signatures needed by the scoped setup/handoff boundary

- `rust/src/supermelee/setup/ffi.rs`
  - marker: `@plan PLAN-20260314-SUPERMELEE.P11`
  - Implement Rust-exported FFI entry points for:
    - SuperMelee entry/menu lifecycle
    - team load/save integration hooks
    - battle-facing initial/next combatant selection hooks
    - scoped netplay-boundary hooks as required by the audited boundary

### C-side wiring to update

- `sc2/src/uqm/supermelee/melee.c`
- `sc2/src/uqm/supermelee/meleesetup.c`
- `sc2/src/uqm/supermelee/loadmele.c`
- `sc2/src/uqm/supermelee/buildpick.c`
- `sc2/src/uqm/supermelee/pickmele.c`

For each touched C file:
- redirect only the scoped SuperMelee-owned entry points to Rust,
- preserve guards/fallbacks required by the existing build strategy,
- do not absorb generic battle-engine ownership into this phase.

### Tests / checks to create

- Rust-side FFI smoke tests where feasible for ABI-safe marshaling boundaries
- build-path verification proving the scoped C↔Rust SuperMelee path compiles under the project's actual integration build

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

If the project has a known C/Rust integration build command for SuperMelee wiring, run that exact audited command in verification for this phase.

## Structural Verification Checklist
- [ ] `c_bridge.rs` and `setup/ffi.rs` exist and are scoped to SuperMelee-owned boundaries
- [ ] C-side wiring targets only `sc2/src/uqm/supermelee/*` ownership points listed in the overview/tracker
- [ ] No new plan work claims ownership of `battle.c`, `process.c`, `collide.c`, `ship.c`, `intel.c`, or `tactrans.c`

## Semantic Verification Checklist (Mandatory)
- [ ] Exported/imported signatures are audited against actual headers before being treated as exact
- [ ] FFI wiring preserves the battle-facing combatant contract rather than weakening it
- [ ] C-side redirection is limited to scoped SuperMelee entry points
- [ ] Netplay-boundary hooks remain at the setup/selection boundary and do not redefine transport/protocol behavior

## Success Criteria
- [ ] The missing FFI/C wiring implementation phase identified in review now exists concretely in the plan body
- [ ] The plan schedules real Rust/C bridge work before later E2E and signoff phases

## Failure Recovery
- rollback: `git checkout -- rust/src/supermelee/c_bridge.rs rust/src/supermelee/setup/ffi.rs sc2/src/uqm/supermelee/`

## Phase Completion Marker
Create: `project-plans/20260311/supermelee/.completed/P11.md`
