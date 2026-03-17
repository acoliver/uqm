# Phase 07: SaveResourceIndex Filtering — TDD + Implementation

## Phase ID
`PLAN-20260314-RESOURCE.P07`

## Prerequisites
- Required: Phase 06/06a completed
- Verify previous phase markers/artifacts exist
- Expected files from previous phase: `06-lifecycle-replacement-cleanup.md`, `06a-lifecycle-replacement-cleanup-verification.md`
- Preflight artifact available describing whether `SaveResourceIndex` can be exercised through a real or shimmed UIO/temp-file path

## Requirements Implemented (Expanded)

### REQ-RES-IDX-005: Save compatibility
**Requirement text**: When a caller saves a resource index through the established API, the resource subsystem shall iterate the global map, apply root/prefix filtering, and emit only entries whose current type handler has a serialization function (toString), producing a representation compatible with subsequent loading. The sole emission gate beyond root matching is toString presence; the subsystem shall not apply class-based or category-based filtering beyond this check.

Behavior contract:
- GIVEN: Global map contains: STRING entry "config.name", INT32 entry "config.sfxvol", GFXRES entry "comm.foo.gfx" (no toString), UNKNOWNRES entry "addon.bar" (no toString)
- WHEN: `SaveResourceIndex(dir, "uqm.cfg", "config.", 1)` is called
- THEN: Output contains "name = STRING:Player\n" and "sfxvol = INT32:128\n" — does NOT contain any GFXRES or UNKNOWNRES entries

Why it matters:
- Save correctness must be proven on the real write path, not only via a simulated iterator.

### REQ-RES-UNK-002: Unknown type save behavior
**Requirement text**: When a save operation encounters an entry stored as the built-in unknown type, the resource subsystem shall skip that entry because the unknown type has no serialization function (toString).

Behavior contract:
- GIVEN: The map contains UNKNOWNRES entries alongside serializable config values
- WHEN: `SaveResourceIndex` runs
- THEN: UNKNOWNRES entries are omitted entirely from the emitted file

Why it matters:
- Unknown entries must not bleed into saved config output.

## Implementation Tasks

### TDD: Tests to add

At least one test in this phase must exercise `SaveResourceIndex` itself through the actual FFI/UIO-writing path, using a real temp file or a test UIO shim. Dispatch-layer serialization helpers may be used as supplemental unit tests, but they are not sufficient on their own.

1. **`test_save_resource_index_skips_entries_without_to_string_real_path`**
   - Populate authoritative state with: a STRING entry (has toString), a heap-type entry (no toString registered)
   - Invoke `SaveResourceIndex` itself against a temp or shimmed UIO file
   - Read the emitted file contents back through the same test harness
   - Assert: only the STRING entry is written
   - marker: `@plan PLAN-20260314-RESOURCE.P07`
   - marker: `@requirement REQ-RES-IDX-005`

2. **`test_save_resource_index_skips_unknownres_entries_real_path`**
   - Populate authoritative state with an UNKNOWNRES entry plus at least one serializable value entry
   - Invoke `SaveResourceIndex` itself
   - Assert: UNKNOWNRES is not present in the output file
   - marker: `@plan PLAN-20260314-RESOURCE.P07`
   - marker: `@requirement REQ-RES-UNK-002`

3. **`test_save_emits_all_value_types_with_to_string`**
   - Populate with STRING, INT32, BOOLEAN, COLOR entries (all have toString)
   - Invoke the real save path if feasible; otherwise use the closest helper plus one real-path save test from items 1–2
   - Assert: all four are emitted
   - marker: `@plan PLAN-20260314-RESOURCE.P07`
   - marker: `@requirement REQ-RES-IDX-005`

4. **`test_save_respects_root_filter_and_strip_root_real_path`**
   - Populate with "config.name" and "addon.bar" STRING entries
   - Save with root="config." and `strip_root = 1`
   - Assert: only the config entry is emitted and the emitted key is stripped correctly
   - marker: `@plan PLAN-20260314-RESOURCE.P07`
   - marker: `@requirement REQ-RES-IDX-005`

Dispatch-layer simulation may still be added as a helper-level test, but it must be documented as supplemental coverage only.

### Implementation: Modify `rust/src/resource/ffi_bridge.rs`

**Function:** `SaveResourceIndex` (~lines 396–461)

**Change:** Where the code currently falls through to `format!("{}:{}", desc.res_type, desc.fname)` when `to_string_fun` is None, change to `continue` (skip the entry).

```rust
// @plan PLAN-20260314-RESOURCE.P07
// @requirement REQ-RES-IDX-005
let handler = state.dispatch.type_registry.lookup(&desc.type_handler_key);
let to_string_fun = match handler.and_then(|h| h.to_string_fun) {
    Some(f) => f,
    None => continue,  // Skip entries without toString
};
// ... call to_string_fun and write the line ...
```

### Pseudocode traceability
- Uses pseudocode lines: PC-6 (150-169)

## Verification Commands

```bash
# Structural gate
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features

# Targeted tests
cargo test --lib -- resource::tests::test_save_resource_index_skips_entries_without_to_string_real_path
cargo test --lib -- resource::tests::test_save_resource_index_skips_unknownres_entries_real_path
cargo test --lib -- resource::tests::test_save_respects_root_filter_and_strip_root_real_path
```

## Structural Verification Checklist
- [ ] `SaveResourceIndex` modified — `continue` when handler has no `to_string_fun`
- [ ] No fallback `format!` path remains for entries without `toString`
- [ ] At least one targeted test exercises `SaveResourceIndex` itself through the real write path
- [ ] Plan/requirement traceability present

## Semantic Verification Checklist (Mandatory)
- [ ] Entries without toString are skipped (not emitted with fallback format)
- [ ] UNKNOWNRES entries are skipped
- [ ] Heap types without toString are skipped (GFXRES, SNDRES, etc.)
- [ ] Value types with toString (STRING, INT32, BOOLEAN, COLOR) are emitted
- [ ] Root filtering still works correctly
- [ ] strip_root still works correctly on the actual save path
- [ ] Integration points validated end-to-end for `SaveResourceIndex` output

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/resource/ffi_bridge.rs
```

## Success Criteria
- [ ] All new tests pass
- [ ] Real-path `SaveResourceIndex` behavior is demonstrated
- [ ] All existing tests pass
- [ ] Verification commands pass

## Failure Recovery
- rollback steps: `git checkout -- rust/src/resource/ffi_bridge.rs`
- blocking issues to resolve before next phase: inability to exercise the real save path through UIO/temp-file test harness

## Phase Completion Marker
Create: `project-plans/20260311/resource/.completed/P07.md`

Contents:
- phase ID
- timestamp
- files changed
- tests added/updated
- verification outputs
- semantic verification summary
