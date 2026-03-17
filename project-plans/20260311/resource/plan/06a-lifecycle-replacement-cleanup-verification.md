# Phase 06a: Lifecycle & Replacement Cleanup — Verification

## Phase ID
`PLAN-20260314-RESOURCE.P06a`

## Prerequisites
- Phase 06 complete

## Structural Verification Checklist
- [ ] `cleanup_all_entries()` method exists on `ResourceDispatch`
- [ ] `process_resource_desc` has pre-insert replacement logic
- [ ] `UninitResourceSystem` calls `cleanup_all_entries` before `*guard = None`
- [ ] Direct-verification tests for `free_resource`, `detach_resource`, and `remove_resource` are added and compile

## Semantic Verification Checklist
- [ ] `test_process_resource_desc_replacement_calls_free_fun` — PASSES
- [ ] `test_process_resource_desc_replacement_warns_on_refcount` — PASSES
- [ ] `test_process_resource_desc_replacement_value_type_no_free` — PASSES
- [ ] `test_uninit_frees_loaded_heap_resources` — PASSES
- [ ] `test_uninit_skips_value_types` — PASSES
- [ ] `test_uninit_skips_unloaded_heap_entries` — PASSES
- [ ] `test_free_resource_on_value_type_never_calls_free_fun` — PASSES
- [ ] `test_detach_resource_on_value_type_returns_null_without_destructor` — PASSES
- [ ] `test_remove_materialized_heap_entry_frees_and_erases_key` — PASSES
- [ ] `test_remove_value_type_erases_key_without_heap_destructor` — PASSES

## Regression Check
- [ ] All existing tests pass
- [ ] `cargo clippy` clean
- [ ] `cargo fmt --check` clean

## Success Criteria
- [ ] Phase 06 behavior is fully verified
- [ ] Direct coverage exists for REQ-RES-LOAD-007 and REQ-RES-LOAD-008
- [ ] Regression checks pass

## Failure Recovery
- rollback steps: `git checkout -- project-plans/20260311/resource/plan/06a-lifecycle-replacement-cleanup-verification.md`
- blocking issues to resolve before next phase: missing direct test coverage for value-type free/detach or materialized remove behavior

## Gate Decision
- [ ] Phase 06 complete and verified
- [ ] Proceed to Phase 07
