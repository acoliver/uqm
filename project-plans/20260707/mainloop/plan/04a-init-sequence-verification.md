# Phase 04a: P04 Verification

## Phase ID
`PLAN-20260707-MAINLOOP.P04a`

## Verifies
P04 (Startup Verification — C owns startup)

## Requirements Verified
REQ-ML-002

## Verification Gate
```bash
# Build with USE_RUST_MAINLOOP=0 (original C path)
cd sc2 && ./build.sh uqm

# Verify Starcon2Main-specific init symbols present
nm sc2/uqm | grep -E 'initAudio|LoadKernel|StartGame'

# Verify no Rust startup wrapper exists
! grep -r 'uqm_rust_safe_startup' sc2/src/ rust/src/

# Boot test (original path works)
cd sc2 && ./uqm --help
```

## Checklist
- [ ] Binary builds with USE_RUST_MAINLOOP=0
- [ ] Starcon2Main-specific init symbols present (initAudio, LoadKernel, StartGame)
- [ ] No Rust startup wrapper exists (uqm_rust_safe_startup absent)
- [ ] C main() startup path unchanged (uqm.c:283-452)
- [ ] No `mainloop::init_sequence` module (C owns startup)

## Decision
- [ ] PASS → proceed to P05
- [ ] FAIL → remediate P04 before continuing
