# Phase 13: Type Registration — TDD

## Phase ID
`PLAN-20260224-RES-SWAP.P13`

## Prerequisites
- Required: Phase 12a (Type Registration Stub Verification) completed

## Requirements Implemented (Expanded)

### REQ-RES-014-017: Type Registration — Tests
### REQ-RES-004: 14 Types Registered in Order

## Implementation Tasks

### Tests to write

#### Type registration tests
```
test_install_type_stores_under_sys_prefix
  InstallResTypeVectors("TESTTYPE", some_load, some_free, some_tostring)
  Lookup "sys.TESTTYPE" → found, has correct handlers

test_install_type_returns_true_on_success
  Result = InstallResTypeVectors("NEWTYPE", ...)
  Assert result == true

test_install_type_with_null_fns
  InstallResTypeVectors("VALTYPE", some_load, NULL, some_tostring)
  Lookup → free_fun is None

test_install_value_type
  InstallResTypeVectors("STRING", use_descriptor, NULL, raw_descriptor)
  Verify: free_fun is None (value type indicator)

test_install_heap_type
  InstallResTypeVectors("GFXRES", some_load, some_free, NULL)
  Verify: free_fun is Some (heap type indicator)

test_count_resource_types
  Install 3 types
  CountResourceTypes → 3

test_count_after_init_has_14_builtin_types
  After InitResourceSystem, CountResourceTypes → 14

test_lookup_nonexistent_type_returns_none
  Lookup "sys.NOSUCHTYPE" → None

test_type_handler_stores_res_type_string
  InstallResTypeVectors("MYTYPE", ...)
  Lookup → handlers.resType == "MYTYPE"

test_type_handler_stores_function_pointers
  Define load/free/toString C-compatible functions
  InstallResTypeVectors with those pointers
  Lookup → verify all 3 pointers are stored correctly
```

#### Built-in value type handler tests
```
test_builtin_string_load
  UseDescriptorAsRes("hello world", &resdata)
  resdata.str == "hello world"

test_builtin_int32_load
  DescriptorToInt("42", &resdata)
  resdata.num == 42

test_builtin_int32_load_negative
  DescriptorToInt("-5", &resdata)
  resdata.num == (u32 representation of -5)

test_builtin_int32_load_non_numeric
  DescriptorToInt("abc", &resdata)
  resdata.num == 0

test_builtin_boolean_load_true
  DescriptorToBoolean("true", &resdata) → num = 1
  DescriptorToBoolean("True", &resdata) → num = 1
  DescriptorToBoolean("TRUE", &resdata) → num = 1

test_builtin_boolean_load_false
  DescriptorToBoolean("false", &resdata) → num = 0
  DescriptorToBoolean("anything", &resdata) → num = 0

test_builtin_color_load
  DescriptorToColor("rgb(0x1a, 0x00, 0x1a)", &resdata)
  resdata.num == 0x1a001aff (packed RGBA)

test_builtin_string_tostring
  resdata.str = "hello"
  RawDescriptor(&resdata, buf, 256) → buf == "hello"

test_builtin_int_tostring
  resdata.num = 42
  IntToString(&resdata, buf, 256) → buf == "42"

test_builtin_boolean_tostring_true
  resdata.num = 1
  BooleanToString(&resdata, buf, 256) → buf == "true"

test_builtin_boolean_tostring_false
  resdata.num = 0
  BooleanToString(&resdata, buf, 256) → buf == "false"

test_builtin_color_tostring_opaque
  resdata.num = 0x1a001aff
  ColorToString(&resdata, buf, 256) → buf == "rgb(0x1a, 0x00, 0x1a)"

test_builtin_color_tostring_transparent
  resdata.num = 0xff000080
  ColorToString(&resdata, buf, 256) → buf == "rgba(0xff, 0x00, 0x00, 0x80)"
```

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features 2>&1 | grep "FAILED"
```

## Structural Verification Checklist
- [ ] Type registration tests exist
- [ ] Built-in value type handler tests exist
- [ ] Tests compile

## Semantic Verification Checklist
- [ ] Tests fail with stubs (RED confirmed)
- [ ] Tests verify function pointer storage and retrieval
- [ ] Tests verify all 5 built-in value type loaders
- [ ] Tests verify all 4 built-in toString functions

## Success Criteria
- [ ] All tests compile and FAIL (RED)

## Failure Recovery
- rollback: `git checkout -- rust/src/resource/`

## Phase Completion Marker
Create: `project-plans/memandres/resource/.completed/P13.md`
