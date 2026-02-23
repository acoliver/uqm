# Phase 13a: Error Handling Hardening — Verification

## Phase ID
`PLAN-20260223-GFX-VTABLE-FIX.P13a`

## Prerequisites
- Required: Phase P13 completed
- Expected artifacts: Error handling tests and hardening

## Verification Commands

```bash
cd rust && cargo fmt --all --check
cd rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
cd rust && cargo test --workspace --all-features

# Verify NO deferred patterns in entire file
grep -c "TODO\|FIXME\|HACK\|todo!\|unimplemented!\|for now\|will be implemented\|placeholder" rust/src/graphics/ffi.rs | xargs test 0 -eq && echo "CLEAN" || echo "FAIL"

# Verify no unwrap/expect in production code (outside #[cfg(test)])
# This is a heuristic — manual review recommended
grep -n "\.unwrap()\|\.expect(" rust/src/graphics/ffi.rs | grep -v "#\[cfg(test)\]" | grep -v "mod tests" || echo "CHECK: may need manual review of unwrap() usage"

# Verify panic boundary policy — every extern "C" fn has PANIC-FREE or catch_unwind
grep -c 'pub extern "C" fn' rust/src/graphics/ffi.rs
grep -c 'PANIC-FREE\|catch_unwind' rust/src/graphics/ffi.rs
# Second count should be >= first count (one annotation per extern fn)
```

## Structural Verification Checklist
- [ ] REQ-INIT-095 guard present in rust_gfx_init (added in P03, verified here)
- [ ] `rust_gfx_upload_transition_screen` has proper documentation
- [ ] All error handling tests present
- [ ] No deferred patterns in ffi.rs
- [ ] Every `#[no_mangle] pub extern "C" fn` has panic-safety annotation (REQ-FFI-030)

## Semantic Verification Checklist (Mandatory)
- [ ] Every FFI function has an uninitialized-state test (REQ-ERR-010)
- [ ] Out-of-sequence calls verified safe (REQ-SEQ-070)
- [ ] Repeated calls verified safe (REQ-INV-040/050)
- [ ] Surface accessors verified to return null when uninitialized (REQ-ERR-011)
- [ ] No `unwrap()`/`expect()` in production FFI paths
- [ ] Panic boundary policy satisfied for all extern "C" functions (REQ-FFI-030)
- [ ] No per-frame logging in vtable validation paths (REQ-ERR-030)
- [ ] Init failure paths log diagnostics (REQ-ERR-040)
- [ ] All cargo gates pass

## Comprehensive Test Count

```bash
# Count total test functions in ffi.rs
grep -c "#\[test\]" rust/src/graphics/ffi.rs
# Expected: >= 20 tests total across all phases
```

## Success Criteria
- [ ] Zero deferred patterns in ffi.rs
- [ ] All structural and semantic checks pass
- [ ] All tests pass
- [ ] File is ready for integration verification

## Phase Completion Marker
Create: `project-plans/gfx/.completed/P13a.md`
