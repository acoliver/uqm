# Phase 21a: C Bridge Wiring — Verification

## Phase ID
`PLAN-20260224-RES-SWAP.P21a`

## Prerequisites
- Required: Phase 21 completed

## Verification Checklist

### Build verification
- [ ] `USE_RUST_RESOURCE=1`: builds, links, no undefined/duplicate symbols
- [ ] `USE_RUST_RESOURCE=0`: builds, links, no errors

### Symbol verification
```bash
# Verify Rust provides all symbols when enabled
nm -g rust/target/release/libuqm_rust.a 2>/dev/null | grep -c "T _\?res_"
# Expected: ~25 res_* symbols

# Verify no duplicate definitions
cd sc2 && ./build.sh uqm 2>&1 | grep -c "multiple definition"
# Expected: 0
```

### Runtime verification
- [ ] Game launches to main menu
- [ ] Config values readable (e.g., fullscreen, sfxvol)
- [ ] Content resources load (graphics/fonts render)
- [ ] Settings menu reads and writes correctly
- [ ] Save config + reload = same values

### Regression verification
- [ ] Build and run with `USE_RUST_RESOURCE=0` — identical behavior

## Gate Decision
- [ ] PASS: proceed to P22
- [ ] FAIL: fix wiring
