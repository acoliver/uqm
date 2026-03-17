# Phase 11a: Integration Testing Verification

## Phase ID
`PLAN-20260314-GRAPHICS.P11a`

## Prerequisites
- Required: Phase P11 completed

## Structural Verification
- [ ] Integration test file exists with the planned migrated-path coverage
- [ ] Backup files are removed
- [ ] Scanline semantic verification is present
- [ ] Event-lifecycle integration verification is present
- [ ] No new plan-phase TODO/FIXME placeholders remain in production code

## Semantic Verification
- [ ] Transition, extra-screen, batching, synchronization, deferred free, and context-state behaviors are covered on the migrated path
- [ ] Idle/no-redraw behavior is covered on the migrated path
- [ ] Reinit/system-box behavior is covered to the extent safely automatable
- [ ] Event path remains covered after init and reinit in the integrated harness
- [ ] All planned integration tests pass

## Gate Decision
- [ ] PASS: proceed to P12 end-to-end verification
- [ ] FAIL: revise plan before proceeding
