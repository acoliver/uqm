# Phase 03a: Colormap + Music Bridge Stub Verification

## Phase ID
`PLAN-20260325-COMMPT3.P03a`

## Prerequisites
- Required: Phase P03 completed
- Expected artifacts: Modified `talk_segue.rs`, `rust_comm.c`, `rust_comm.h` with stubs

## Verification Commands

```bash
# Build gate
cd rust && cargo check --workspace --all-features
cd rust && cargo fmt --all --check
cd rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
cd rust && cargo test --workspace --all-features

# Verify stubs exist in C
grep -n "c_SetColorMapFromCommData" sc2/src/uqm/rust_comm.c
grep -n "c_PlayAlienMusic" sc2/src/uqm/rust_comm.c

# Verify declarations in header
grep "c_SetColorMapFromCommData\|c_PlayAlienMusic" sc2/src/uqm/rust_comm.h

# Verify Rust wiring
grep "c_SetColorMapFromCommData\|c_PlayAlienMusic" rust/src/comm/talk_segue.rs

# Verify null_mut gone from production paths
grep -n "null_mut" rust/src/comm/talk_segue.rs | grep -v test | grep -v "cfg(test)" | grep -v "///"
```

## Structural Verification Checklist
- [ ] `c_SetColorMapFromCommData()` exists in `rust_comm.c` with empty/stub body
- [ ] `c_PlayAlienMusic()` exists in `rust_comm.c` with empty/stub body
- [ ] Both declared in `rust_comm.h`
- [ ] Extern declarations in `talk_segue.rs` for both new functions
- [ ] `set_colormap()` calls `c_SetColorMapFromCommData()` — no `null_mut`
- [ ] `play_alien_music()` calls `c_PlayAlienMusic()` — no `null_mut`
- [ ] Both build modes compile and link

## Semantic Verification Checklist (Mandatory)
- [ ] Stubs have NO functional behavior (empty bodies or `/* stub */` only)
- [ ] Call sites are rewired — old null-passthrough pattern replaced
- [ ] Old extern declarations removed (or justified if kept for other callers)
- [ ] All 268+ tests pass (stubs are no-ops, matching previous null-passthrough)

## Semantic Negative-Proof Gate (Mandatory)
The following negative proofs confirm the stubs are genuinely non-functional:
- [ ] C stubs `c_SetColorMapFromCommData` and `c_PlayAlienMusic` do NOT call
  `SetColorMap`, `GetColorMapAddress`, or `PlayMusic` (verified by reading function bodies)
- [ ] Intentionally adding a `return 1;` or any side-effect to a stub body
  would not cause any test to fail (confirms TDD phase P04 is needed)

## Phase Completion Marker
Create: `project-plans/20260311/commpt3/.completed/P03a.md`
