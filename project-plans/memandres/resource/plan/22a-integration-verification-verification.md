# Phase 22a: Integration Verification — Verification

## Phase ID
`PLAN-20260224-RES-SWAP.P22a`

## Prerequisites
- Required: Phase 22 completed

## Verification Checklist

### Full system verification
- [ ] `cargo fmt --all --check` passes
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings` passes
- [ ] `cargo test --workspace --all-features` passes
- [ ] `cd sc2 && ./build.sh uqm` succeeds with USE_RUST_RESOURCE=1

### Runtime verification
- [ ] Game launches to main menu
- [ ] All content resources load (graphics, fonts, sounds, music)
- [ ] Settings menu reads and writes config values
- [ ] Config file saved/reloaded correctly
- [ ] Addon packs load when specified
- [ ] Save/load game works (resource system participates in state)

### Regression verification
- [ ] No deferred patterns in resource code: `grep -RIn "TODO\|FIXME\|HACK\|todo!\|unimplemented!" rust/src/resource/`
- [ ] No new clippy warnings
- [ ] All pre-existing tests still pass

### Rollback verification
- [ ] Setting `USE_RUST_RESOURCE=0` and rebuilding restores C behavior
- [ ] Game works identically on C path

## Gate Decision
- [ ] PASS: Resource migration complete
- [ ] FAIL: List issues and remediation plan

## Phase Completion Marker
Create: `project-plans/memandres/resource/.completed/P22a.md`
