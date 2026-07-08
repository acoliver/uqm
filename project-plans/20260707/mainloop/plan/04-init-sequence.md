# Phase 04: Startup Verification (Revised — C owns startup)

## Phase ID
`PLAN-20260707-MAINLOOP.P04`

## Prerequisites
- Phase 03 complete (Rust activity types + FFI externs)

## Purpose
**C `main()` owns the full startup sequence** (`uqm.c:283-452`): option
parsing, config, addons, NETPLAY, graphics, input — all ~50 side effects.
Rust does NOT replicate or wrap this. This phase verifies that the
existing C startup path works unchanged when `USE_RUST_MAINLOOP=0`
and that the C symbols Rust needs are present.

**This is a verification gate, not an implementation phase.**

## Requirements Verified

### REQ-ML-002: Init Sequence Orchestration
The init sequence runs in C `main()` exactly as before. Rust calls only
Starcon2Main-specific init (audio, kernel, splash) from inside
`rust_game_loop()`.

---

## Verification

```bash
# 1. Build with USE_RUST_MAINLOOP=0 (original path)
cd sc2 && ./build.sh uqm

# 2. Verify startup symbols Rust needs are exported
nm sc2/uqm | grep -E 'initAudio|LoadKernel|StartGame|InitGameStructures|InitGameClock|AddInitialGameEvents|SetPlayerInputAll'

# 3. Boot test (original C path)
cd sc2 && ./uqm --help
```

## Checklist
- [ ] Binary builds with original Starcon2Main path
- [ ] All Starcon2Main-specific init symbols present in binary
- [ ] `--help` prints version output (proves C main() + startup works)
- [ ] No startup wrapper exists (`grep uqm_rust_safe_startup` returns nothing)

## Success Criteria
- [ ] REQ-ML-002: C startup path verified working unchanged

## Phase Completion Marker
Create: `project-plans/20260707/mainloop/.completed/P04.md`
