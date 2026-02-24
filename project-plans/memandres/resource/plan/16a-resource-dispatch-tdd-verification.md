# Phase 16a: Resource Dispatch — TDD Verification

## Phase ID
`PLAN-20260224-RES-SWAP.P16a`

## Prerequisites
- Required: Phase 16 completed

## Verification Checklist
- [ ] All dispatch tests compile
- [ ] Tests fail with stubs (RED)
- [ ] Refcount lifecycle fully tested (get→incr, free→decr, detach→zero)
- [ ] Error paths tested (null key, undefined key, non-heap, not loaded)

## Gate Decision
- [ ] PASS: proceed to P17
- [ ] FAIL: fix tests
