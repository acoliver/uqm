# Phase 04a: Value-Type Dispatch Fix — Implementation Verification

## Phase ID
`PLAN-20260314-RESOURCE.P04a`

## Prerequisites
- Phase 04 complete

## Structural Verification Checklist
- [ ] `unknownres_load_fun` is an `extern "C"` function in `ffi_bridge.rs`
- [ ] UNKNOWNRES registration uses `Some(unknownres_load_fun)` not `None`
- [ ] `dispatch.rs:process_resource_desc` — UNKNOWNRES fallback sets `is_value_type = true`
- [ ] `dispatch.rs:process_resource_desc` — value-type loadFun lookup uses `handler_key`
- [ ] `dispatch.rs:get_resource` — value-type path returns union field, not `data.ptr`
- [ ] `dispatch.rs:get_resource` — heap-type path unchanged

## Semantic Verification Checklist
- [ ] `test_unknownres_registered_as_value_type` — PASSES
- [ ] `test_process_resource_desc_unknown_type_stores_as_value` — PASSES
- [ ] `test_get_resource_value_type_string_returns_str_ptr` — PASSES
- [ ] `test_get_resource_value_type_int_returns_num_as_ptr` — PASSES
- [ ] `test_get_resource_unknownres_returns_str_ptr` — PASSES
- [ ] `test_get_resource_heap_type_still_lazy_loads` — PASSES
- [ ] All pre-existing dispatch tests still PASS

## Regression Check
- [ ] `cargo test --workspace --all-features` — all pass
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings` — clean

## Gate Decision
- [ ] Phase 04 complete and verified
- [ ] Proceed to Phase 05
