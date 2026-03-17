# Phase 15a: C-Side Bridge & Build Toggle Verification

## Phase ID
`PLAN-20260314-CAMPAIGN.P15a`

## Prerequisites
- Required: Phase 15 completed

## Verification Commands

```bash
# Rust-side gates
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features

# C-only build (toggle off)
cd sc2 && ./build.sh uqm

# Rust-bridged build (toggle on)
cd sc2 && USE_RUST_CAMPAIGN=1 ./build.sh uqm
```

## Structural Verification Checklist
- [ ] FFI exports match C declarations
- [ ] Build succeeds both ways
- [ ] No symbol conflicts

## Semantic Verification Checklist
- [ ] C-only build: game works identically to baseline
- [ ] Rust-bridged build: links successfully
- [ ] All C guards comprehensive (no unguarded conflicts)

## Gate Decision
- [ ] PASS: proceed to Phase 16
- [ ] FAIL: fix issues, re-verify

## Phase Completion Marker
Create: `project-plans/20260311/campaign-gameplay/.completed/P15a.md`
