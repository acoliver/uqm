# Phase 09: Minor Fixes — Implementation

## Phase ID
`PLAN-20260314-RESOURCE.P09`

## Prerequisites
- Required: Phase 08/08a completed
- Verify previous phase markers/artifacts exist
- Expected files from previous phase: `08-res-openresfile-sentinel.md`, `08a-res-openresfile-sentinel-verification.md`

## Requirements Implemented (Expanded)

### REQ-RES-TYPE-004: Type count visibility (GAP-8)
**Requirement text**: When a consumer queries the number of registered resource types through the established API, the resource subsystem shall report a count that reflects the registrations currently active in the authoritative type registry.

Behavior contract:
- GIVEN: 12 type handlers registered (5 built-in + 7 downstream)
- WHEN: `CountResourceTypes()` is called
- THEN: Returns `12` as `u32` (not `u16`)

Why it matters:
- The exported ABI must match the specified width.

### REQ-RES-FILE-006: Raw resource data compatibility (GAP-10)
**Requirement text**: When a caller requests raw resource bytes through the established raw-data helper, the resource subsystem shall read and validate the legacy 4-byte prefix, reject non-uncompressed prefixes, and return length - 4 payload bytes.

Behavior contract:
- GIVEN: `GetResourceData` is read by maintainers or reviewers alongside its implementation
- WHEN: the doc comment describes the helper
- THEN: it accurately reflects the real prefix-reading and payload behavior already implemented by code

Why it matters:
- The remaining work in this phase is an ABI fix plus a documentation parity fix; file-backed load guard work is completed earlier in Phase 08.

## Implementation Tasks

### Files to modify

#### 1. `rust/src/resource/ffi_bridge.rs` — `CountResourceTypes` return type (GAP-8)

**Location:** `CountResourceTypes` function (~line 664)

**Change:** Change return type from `u16` to `u32`.

```rust
// @plan PLAN-20260314-RESOURCE.P09
// @requirement REQ-RES-TYPE-004
#[no_mangle]
pub extern "C" fn CountResourceTypes() -> u32 {
    // ... existing logic, change cast to u32
}
```

Also verify corresponding C declaration in `reslib.h` expects `DWORD` or `unsigned int` (32-bit).

#### 2. `rust/src/resource/ffi_bridge.rs` — `GetResourceData` doc comment fix (GAP-10)

**Location:** `GetResourceData` function (~lines 1160-1163)

**Change:** Fix the misleading doc comment. Current comment says something about "seek back 4 bytes". Change to accurately describe the behavior: reads 4-byte prefix, if prefix is `~0` (uncompressed), reads remaining `length - 4` bytes.

```rust
/// @plan PLAN-20260314-RESOURCE.P09
/// @requirement REQ-RES-FILE-006
/// Reads raw resource data from a file.
///
/// Reads the 4-byte legacy prefix from the provided stream.
/// If the prefix is `0xFFFFFFFF` (uncompressed marker), allocates and reads
/// the remaining `length - 4` payload bytes. Returns null on any failure
/// or if the prefix indicates a compressed resource (not supported).
```

### Tests to add

1. **`test_count_resource_types_returns_u32`**
   - Register more than 256 types (or verify return value fits u32)
   - In practice: just verify the function compiles with u32 return and returns correct count
   - marker: `@plan PLAN-20260314-RESOURCE.P09`
   - marker: `@requirement REQ-RES-TYPE-004`

### Pseudocode traceability
- Uses pseudocode lines: PC-9 (230-233)

## Verification Commands

```bash
# Structural gate
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] `CountResourceTypes` returns `u32`
- [ ] `GetResourceData` doc comment is accurate
- [ ] Plan/requirement traceability present in all changes
- [ ] Tests compile and run

## Semantic Verification Checklist (Mandatory)
- [ ] `CountResourceTypes` compiles and returns correct count
- [ ] Doc comment accurately describes the code's actual behavior
- [ ] No regressions in existing tests
- [ ] Integration points validated end-to-end for the exported type-count ABI

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/resource/ffi_bridge.rs
```

## Success Criteria
- [ ] All changes compile
- [ ] All tests pass
- [ ] Verification commands pass

## Failure Recovery
- rollback steps: `git checkout -- rust/src/resource/ffi_bridge.rs`
- blocking issues to resolve before next phase: header/ABI mismatch for `CountResourceTypes`

## Phase Completion Marker
Create: `project-plans/20260311/resource/.completed/P09.md`

Contents:
- phase ID
- timestamp
- files changed
- tests added/updated
- verification outputs
- semantic verification summary
