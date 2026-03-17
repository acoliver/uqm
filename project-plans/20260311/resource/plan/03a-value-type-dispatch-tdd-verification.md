# Phase 03a: Value-Type Dispatch Fix — TDD Verification

## Phase ID
`PLAN-20260314-RESOURCE.P03a`

## Prerequisites
- Phase 03 complete

## Structural Verification Checklist
- [ ] All 6 test functions added to `rust/src/resource/dispatch.rs`
- [ ] Each test has `@plan` and `@requirement` markers
- [ ] Tests compile without errors
- [ ] No production code modified

## Semantic Verification Checklist
- [ ] `test_unknownres_registered_as_value_type` — verifies UNKNOWNRES has loadFun and no freeFun
- [ ] `test_process_resource_desc_unknown_type_stores_as_value` — verifies unregistered types become UNKNOWNRES with str_ptr set
- [ ] `test_get_resource_value_type_string_returns_str_ptr` — verifies STRING entries return str_ptr through get_resource
- [ ] `test_get_resource_value_type_int_returns_num_as_ptr` — verifies INT32 entries return num as pointer
- [ ] `test_get_resource_unknownres_returns_str_ptr` — verifies UNKNOWNRES returns descriptor string pointer
- [ ] `test_get_resource_heap_type_still_lazy_loads` — verifies heap types still trigger loadFun

## TDD Red Phase Confirmation
- [ ] Tests 2-5 FAIL against current code (confirming the gap exists)
- [ ] Test 6 (heap type) PASSES against current code (no regression)
- [ ] Test 1 FAILS (UNKNOWNRES loadFun is currently None)

## Gate Decision
- [ ] All tests compile and expected failures confirmed
- [ ] Proceed to Phase 04 (implementation)
