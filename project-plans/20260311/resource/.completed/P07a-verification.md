# P07a Verification: SaveResourceIndex Filtering

## Verdict
REJECT

## Scope Verified
- `/Users/acoliver/projects/uqm/project-plans/20260311/resource/.completed/P07.md`
- `/Users/acoliver/projects/uqm/project-plans/20260311/resource/plan/07-save-index-filtering.md`
- `/Users/acoliver/projects/uqm/rust/src/resource/ffi_bridge.rs`
- `/Users/acoliver/projects/uqm/rust/src/resource/dispatch.rs`

## Findings

### 1. SaveResourceIndex skips entries without to_string_fun
**PASS**

In `rust/src/resource/ffi_bridge.rs`, `SaveResourceIndex` now looks up the handler by `desc.type_handler_key` and uses:

```rust
let to_string_fn = match handler.and_then(|h| h.to_string) {
    Some(f) => f,
    None => continue,
};
```

This satisfies the requirement that entries without `to_string_fun` are skipped rather than emitted with a fallback format.

### 2. Uses type_handler_key for handler lookup (not res_type)
**PASS**

`SaveResourceIndex` performs lookup with:

```rust
let handler = state.dispatch.type_registry.lookup(&desc.type_handler_key);
```

This is correct for `UNKNOWNRES` cases where `res_type` and `type_handler_key` differ.

### 3. Tests cover filtering of heap types and UNKNOWNRES
**PASS, but only at helper level**

`ffi_bridge.rs` includes helper-based tests covering:
- heap type filtering without `toString`
- `UNKNOWNRES` filtering
- value types with `toString`
- root filtering
- combined root + filtering behavior

The helper `get_saveable_entries` mirrors the production filtering logic and also uses `type_handler_key` plus `continue` on missing `to_string`.

### 4. Root filtering still works
**PASS**

Both production code and helper tests apply prefix filtering, and test coverage includes root-filter behavior.

## Rejection Reason
The phase plan explicitly requires at least one test to exercise `SaveResourceIndex` itself through the real FFI/UIO write path:

> "At least one test in this phase must exercise `SaveResourceIndex` itself through the actual FFI/UIO-writing path, using a real temp file or a test UIO shim. Dispatch-layer serialization helpers may be used as supplemental unit tests, but they are not sufficient on their own."

Current implementation does **not** satisfy that requirement. The added coverage is exclusively helper-level (`get_saveable_entries`) inside `ffi_bridge.rs`, and there is no real-path `SaveResourceIndex` test present.

Because the implementation meets the code-change requirements but fails the phase's mandatory verification/test requirement, this phase cannot be accepted as complete.

## Requested Test Command Output
```text
test threading::tests::test_condvar_wait_timeout ... ok
test threading::tests::test_condvar_broadcast ... ok

test result: ok. 1598 passed; 0 failed; 5 ignored; 0 measured; 0 filtered out; finished in 0.11s
```

## Final Decision
REJECT
