# Phase 05: res_GetString Parity — TDD + Implementation

## Phase ID
`PLAN-20260314-RESOURCE.P05`

## Prerequisites
- Required: Phase 04/04a completed
- Expected files: value-type dispatch fixes in `dispatch.rs` and `ffi_bridge.rs`

## Requirements Implemented (Expanded)

### REQ-RES-CONF-003: String get semantics
**Requirement text**: When a caller requests a string value through the established string getter, the resource subsystem shall return the configured string value only for entries whose type is the string value type, whose value is non-null, and that satisfy the string-value contract. For missing keys, keys of non-string type, or keys with null values, the subsystem shall return a pointer to an empty string (not a null pointer).

Behavior contract:
- GIVEN: Key "config.sfxvol" is an INT32 entry with value 128
- WHEN: `res_GetString("config.sfxvol")` is called
- THEN: Returns pointer to "" (empty string), NOT "128"

- GIVEN: Key "config.name" is a STRING entry with value "Player"
- WHEN: `res_GetString("config.name")` is called
- THEN: Returns pointer to "Player"

- GIVEN: Key "nonexistent" does not exist
- WHEN: `res_GetString("nonexistent")` is called
- THEN: Returns pointer to "" (empty string), NOT null

Why it matters: C callers do `strlen(res_GetString(key))` — null return crashes them.

### REQ-RES-ERR-003: Missing-or-type-mismatch getter behavior
**Requirement text**: When a typed getter is applied to a missing key or a key of an incompatible type, the resource subsystem shall preserve the established externally visible result behavior for that getter so that existing consumers continue to handle defaults and fallback logic correctly.

## Implementation Tasks

### TDD: Tests to add in `rust/src/resource/tests.rs` or `ffi_bridge.rs` test block

1. **`test_res_get_string_returns_empty_for_missing_key`**
   - Call `res_GetString` with a key that doesn't exist
   - Assert: returned pointer is non-null
   - Assert: pointed-to string is empty ("")
   - marker: `@plan PLAN-20260314-RESOURCE.P05`
   - marker: `@requirement REQ-RES-CONF-003`

2. **`test_res_get_string_returns_empty_for_integer_entry`**
   - Load an INT32 entry, then call `res_GetString` on that key
   - Assert: returns pointer to "" (not the integer's descriptor string)
   - marker: `@plan PLAN-20260314-RESOURCE.P05`
   - marker: `@requirement REQ-RES-CONF-003`

3. **`test_res_get_string_returns_empty_for_boolean_entry`**
   - Load a BOOLEAN entry, call `res_GetString` on that key
   - Assert: returns pointer to ""
   - marker: `@plan PLAN-20260314-RESOURCE.P05`
   - marker: `@requirement REQ-RES-CONF-003`

4. **`test_res_get_string_returns_value_for_string_entry`**
   - Load a STRING entry with value "hello", call `res_GetString`
   - Assert: returns pointer to "hello"
   - marker: `@plan PLAN-20260314-RESOURCE.P05`
   - marker: `@requirement REQ-RES-CONF-003`

5. **`test_res_get_string_returns_empty_for_null_key`**
   - Call `res_GetString(null)`
   - Assert: returns pointer to "" (not null)
   - marker: `@plan PLAN-20260314-RESOURCE.P05`
   - marker: `@requirement REQ-RES-ERR-003`

6. **`test_res_get_string_returns_empty_for_unknownres_entry`**
   - Load an UNKNOWNRES entry, call `res_GetString`
   - Assert: returns pointer to "" (type mismatch: UNKNOWNRES ≠ STRING)
   - marker: `@plan PLAN-20260314-RESOURCE.P05`
   - marker: `@requirement REQ-RES-UNK-003`

### Implementation: Modify `rust/src/resource/ffi_bridge.rs`

**File:** `rust/src/resource/ffi_bridge.rs`
**Function:** `res_GetString` (~lines 786–817)

**Change:**
```rust
// @plan PLAN-20260314-RESOURCE.P05
// @requirement REQ-RES-CONF-003
#[no_mangle]
pub extern "C" fn res_GetString(key: *const c_char) -> *const c_char {
    static EMPTY: &[u8] = b"\0";

    if key.is_null() {
        return EMPTY.as_ptr() as *const c_char;
    }

    let key_str = match unsafe { CStr::from_ptr(key) }.to_str() {
        Ok(s) => s,
        Err(_) => return EMPTY.as_ptr() as *const c_char,
    };

    let guard = match RESOURCE_STATE.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };

    let state = match guard.as_ref() {
        Some(s) => s,
        None => return EMPTY.as_ptr() as *const c_char,
    };

    let entry = match state.dispatch.entries.get(key_str) {
        Some(e) => e,
        None => return EMPTY.as_ptr() as *const c_char,
    };

    // Type check: must be STRING
    if entry.res_type != "STRING" {
        return EMPTY.as_ptr() as *const c_char;
    }

    // Value check: str_ptr must be non-null
    unsafe {
        if entry.data.str_ptr.is_null() {
            return EMPTY.as_ptr() as *const c_char;
        }
        entry.data.str_ptr
    }
}
```

### Pseudocode traceability
- Uses pseudocode lines: PC-4 (90-112)

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features

# Targeted tests
cargo test --lib -- resource::tests::test_res_get_string_returns_empty_for_missing_key
cargo test --lib -- resource::tests::test_res_get_string_returns_empty_for_integer_entry
cargo test --lib -- resource::tests::test_res_get_string_returns_value_for_string_entry
cargo test --lib -- resource::tests::test_res_get_string_returns_empty_for_null_key
```

## Structural Verification Checklist
- [ ] 6 new tests added
- [ ] `res_GetString` modified with STRING type check
- [ ] Static empty string defined (not heap-allocated)
- [ ] Plan/requirement traceability present

## Semantic Verification Checklist (Mandatory)
- [ ] Missing key → returns "" (non-null)
- [ ] INT32 key → returns ""
- [ ] BOOLEAN key → returns ""
- [ ] STRING key with value → returns value
- [ ] Null key → returns ""
- [ ] UNKNOWNRES key → returns ""
- [ ] No crashes, no null pointer returns
- [ ] Existing `res_GetString` callers unaffected (they receive same or safer behavior)

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/resource/ffi_bridge.rs
```

## Success Criteria
- [ ] All 6 new tests pass
- [ ] All existing tests pass
- [ ] `res_GetString` never returns null

## Failure Recovery
- rollback: `git checkout -- rust/src/resource/ffi_bridge.rs`
- blocking: if `dispatch.entries` is not accessible from test context, adjust test to use FFI integration test

## Phase Completion Marker
Create: `project-plans/20260311/resource/.completed/P05.md`
