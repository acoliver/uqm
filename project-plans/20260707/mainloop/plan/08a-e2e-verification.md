# Phase 08a: P08 Verification

## Phase ID
`PLAN-20260707-MAINLOOP.P08a`

## Verifies
P08 (End-to-End Integration)

## Requirements Verified
All REQ-ML-* (holistic)

## Verification Gate
```bash
# Build with USE_RUST_MAINLOOP=1
cd sc2 && ./build.sh uqm

# Symbol verification
nm sc2/uqm | grep rust_game_loop
nm sc2/uqm | grep rust_dispatch_activity

# Full boot test (background process, NOT --help which exits before init)
cd sc2
./uqm -o -f &
PID=$!
sleep 10
kill "$PID"
wait "$PID" || true

# Verify no crash markers in log
grep -c "NULL frame\|SIGSEGV\|panic" sc2/uqm_test.log 2>/dev/null || echo 0

# Anti-fraud gate
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/mainloop/
```

## Requirements Holistic Checklist

| REQ | Evidence Required | Verified |
|-----|-------------------|----------|
| REQ-ML-001 | `rust_game_loop` symbol in binary, full boot reaches splash | [ ] |
| REQ-ML-002 | Binary boots (C main() startup succeeded) | [ ] |
| REQ-ML-003 | Activity round-trip test passes both directions | [ ] |
| REQ-ML-004 | All state machine dispatch branches pass | [ ] |
| REQ-ML-005 | Full boundary round-trip test passes | [ ] |
| REQ-ML-007 | Outer (StartGame) + inner (loop-until-CHECK_ABORT) verified | [ ] |
| REQ-ML-008 | Game-kernel cleanup runs, MainExited set, C main() tears down | [ ] |
| REQ-ML-009 | `rust_dispatch_activity` symbol present | [ ] |
| REQ-ML-010 | Named game-state accessors round-trip (no byte offsets) | [ ] |

## Decision
- [ ] PASS → plan complete
- [ ] FAIL → remediate identified phases
