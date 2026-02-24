# Phase 18a: Init, Index, and UIO Wrappers — Stub Verification

## Phase ID
`PLAN-20260224-RES-SWAP.P18a`

## Prerequisites
- Required: Phase 18 completed

## Verification Checklist
- [ ] All 38 extern "C" function stubs compile
- [ ] Symbol names match reslib.h exactly (check with `nm` or similar)
- [ ] UIO extern declarations present
- [ ] Global state declarations present
- [ ] Existing tests pass

### Symbol verification
```bash
# After building, verify symbols are exported:
nm -g target/release/libuqm_rust.a 2>/dev/null | grep -c "InitResourceSystem\|res_GetResource\|res_GetString"
# Expected: matches for all 3
```

## Gate Decision
- [ ] PASS: proceed to P19
- [ ] FAIL: fix stubs
