# Phase 10: Dead Code Removal Preflight + Removal

## Phase ID
`PLAN-20260314-RESOURCE.P10`

## Prerequisites
- Required: Phase 09/09a completed
- Verify previous phase markers/artifacts exist
- Expected files from previous phase: `09-minor-fixes.md`, `09a-minor-fixes-verification.md`
- Phase 0.5 artifact must already record the build-system status of `sc2/src/libs/resource/rust_resource.h` and `rust_resource.c`
- All gap-closure implementation work is done
- All tests pass

## Requirements Implemented (Expanded)

### REQ-RES-INT-006: Single authoritative runtime path
**Requirement text**: The resource subsystem shall expose one authoritative runtime behavior for the public resource ABI so that consumers do not observe divergent semantics based solely on whether the implementation is backed by one internal module stack or another.

Behavior contract:
- GIVEN: The authoritative runtime path is `ffi_bridge.rs` → `dispatch.rs` → `type_registry.rs`
- WHEN: A developer reads the codebase and the engine is built with `USE_RUST_RESOURCE`
- THEN: there is one authoritative runtime path, and any removal of alternate code has been proven not to break the actual build/integration path

Why it matters:
- Dead-code cleanup must not remove files that still participate in build/linkage.

## Implementation Tasks

### Step 1: prove-unused analysis (mandatory before removal)

Before removing any file, verify it is NOT imported by any authoritative module and is not an active build/link dependency.

```bash
# Check if any authoritative file imports from the dead modules
grep -n "use.*resource::ffi[^_]" rust/src/resource/ffi_bridge.rs rust/src/resource/dispatch.rs rust/src/resource/type_registry.rs rust/src/resource/propfile.rs rust/src/resource/ffi_types.rs rust/src/resource/resource_type.rs
grep -n "use.*resource_system" rust/src/resource/ffi_bridge.rs rust/src/resource/dispatch.rs
grep -n "use.*resource::loader" rust/src/resource/ffi_bridge.rs rust/src/resource/dispatch.rs
grep -n "use.*resource::cache" rust/src/resource/ffi_bridge.rs rust/src/resource/dispatch.rs
grep -n "use.*resource::index[^_]" rust/src/resource/ffi_bridge.rs rust/src/resource/dispatch.rs
grep -n "use.*config_api" rust/src/resource/ffi_bridge.rs rust/src/resource/dispatch.rs
```

Also verify `stringbank.rs` — determine if it's used by any authoritative module or if it should also be removed.

For the C-side wrappers:
- verify whether `sc2/src/libs/resource/rust_resource.h` is included by any compiled source when `USE_RUST_RESOURCE` is enabled
- verify whether `sc2/src/libs/resource/rust_resource.c` is compiled or linked in the active build
- record exact build-system evidence before any change

### Step 2: Rust-side removal (only after Step 1 passes)

#### Files to remove

| File | Reason |
|------|--------|
| `rust/src/resource/ffi.rs` | Non-authoritative FFI layer (exports `rust_init_resource_system`, `rust_load_index`, etc. — none called by C) |
| `rust/src/resource/resource_system.rs` | Alternate `ResourceSystem` with `PropertyFile`/`Arc<ResourceValue>` — not the active path |
| `rust/src/resource/loader.rs` | Loader abstraction for the non-authoritative path |
| `rust/src/resource/cache.rs` | Cache abstraction for the non-authoritative path |
| `rust/src/resource/index.rs` | Alternate index representation |
| `rust/src/resource/config_api.rs` | Alternate config API (Rust-native, not C-ABI) |

#### File to modify

##### `rust/src/resource/mod.rs`
Remove the `mod` declarations for the deleted files:
```rust
// Remove these lines:
// mod ffi;
// mod resource_system;
// mod loader;
// mod cache;
// mod index;
// mod config_api;
```

Keep:
```rust
mod dispatch;
mod ffi_bridge;
mod ffi_types;
mod propfile;
mod resource_type;
mod type_registry;
// Keep stringbank only if used by authoritative modules
mod stringbank;  // verify first
mod tests;
```

### Step 3: C-side wrapper disposition (conditional)

#### `sc2/src/libs/resource/rust_resource.h` and `rust_resource.c`
These C files declare and wrap the non-authoritative `rust_init_resource_system`, `rust_load_index`, etc. exports.

- If Step 1 proves they are not compiled and not included on the active `USE_RUST_RESOURCE` path, removal or leave-in-place-as-unused cleanup may proceed.
- If they are compiled or included conditionally, convert this phase into a minimal safe change: guard them appropriately or defer their removal until after Phase 11 integration verification confirms they are unnecessary.
- Do not remove these files on assumption alone.

### Step 4: post-removal confirmation
- run the standard Rust verification suite
- confirm no dangling imports remain
- carry the build/linkage evidence forward into Phase 11 for final confirmation before declaring cleanup complete

### Pseudocode traceability
- N/A (removal phase, no pseudocode needed)

## Verification Commands

```bash
# Verify removal doesn't break compilation
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features

# Verify no dangling imports
grep -rn "resource::ffi[^_]" rust/src/
grep -rn "resource_system" rust/src/
grep -rn "resource::loader" rust/src/
grep -rn "resource::cache" rust/src/
grep -rn "resource::index" rust/src/
grep -rn "config_api" rust/src/
```

## Structural Verification Checklist
- [ ] prove-unused/build-dependency analysis recorded before removal
- [ ] 6 Rust files removed from `rust/src/resource/` only after analysis passes
- [ ] `mod.rs` updated to remove dead module declarations
- [ ] No dangling imports anywhere in the crate
- [ ] C-side `rust_resource.h`/`rust_resource.c` either proven unused, safely guarded, or explicitly deferred pending Phase 11 confirmation

## Semantic Verification Checklist (Mandatory)
- [ ] All remaining tests pass
- [ ] `cargo test --workspace --all-features` — clean
- [ ] `cargo clippy` — clean
- [ ] Full engine build either already proven safe for this removal or final cleanup is clearly deferred until Phase 11 confirmation
- [ ] No tests were lost that tested authoritative behavior (if any tests in removed files tested `ffi_bridge`/`dispatch` behavior, move them to `tests.rs` first)
- [ ] Integration points validated end-to-end for cleanup safety before final removal is considered complete

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/resource/
```

## Success Criteria
- [ ] Rust-side dead modules removed safely
- [ ] Compilation succeeds
- [ ] All tests pass
- [ ] Single authoritative module stack remains
- [ ] Any C-side wrapper cleanup is backed by build-system evidence, not assumption

## Failure Recovery
- rollback steps: `git checkout -- rust/src/resource/`
- blocking issues to resolve before next phase: active build/link dependency on `rust_resource.h` / `rust_resource.c`

## Phase Completion Marker
Create: `project-plans/20260311/resource/.completed/P10.md`

Contents:
- phase ID
- timestamp
- files changed
- tests added/updated
- verification outputs
- semantic verification summary
- build-system evidence for any removal/defer decision
