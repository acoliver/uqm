# Phase 02: Pseudocode

## Phase ID
`PLAN-20260314-MEMORY.P02`

## Prerequisites
- Required: Phase P01 (Analysis) completed and verified
- Gap analysis accepted

## Pseudocode Component 1: Zero-size OOM fix

Applies to `rust_hmalloc`, `rust_hcalloc`, `rust_hrealloc` zero-size paths.

```text
01: FUNCTION rust_hmalloc(size)
02:   IF size == 0 THEN
03:     ptr = libc::malloc(1)
04:     IF ptr IS NULL THEN
05:       log_add(Fatal, "HMalloc() FATAL: out of memory.")
06:       abort()
07:     RETURN ptr
08:   ptr = libc::malloc(size)
09:   IF ptr IS NULL THEN
10:     log_add(Fatal, "HMalloc() FATAL: out of memory.")
11:     abort()
12:   RETURN ptr

13: FUNCTION rust_hcalloc(size)
14:   IF size == 0 THEN
15:     ptr = libc::malloc(1)
16:     IF ptr IS NULL THEN
17:       log_add(Fatal, "HCalloc() FATAL: out of memory.")
18:       abort()
19:     memset(ptr, 0, 1)
20:     RETURN ptr
21:   ptr = libc::malloc(size)
22:   IF ptr IS NULL THEN
23:     log_add(Fatal, "HCalloc() FATAL: out of memory.")
24:     abort()
25:   memset(ptr, 0, size)
26:   RETURN ptr

27: FUNCTION rust_hrealloc(ptr, size)
28:   IF size == 0 THEN
29:     IF ptr IS NOT NULL THEN
30:       libc::free(ptr)
31:     new_ptr = libc::malloc(1)
32:     IF new_ptr IS NULL THEN
33:       log_add(Fatal, "HRealloc() FATAL: out of memory.")
34:       abort()
35:     RETURN new_ptr
36:   new_ptr = libc::realloc(ptr, size)
37:   IF new_ptr IS NULL THEN
38:     log_add(Fatal, "HRealloc() FATAL: out of memory.")
39:     abort()
40:   RETURN new_ptr
```

## Pseudocode Component 2: `copy_argv_to_c` deallocator fix

```text
41: FUNCTION copy_argv_to_c(argv)
42:   c_strings = empty Vec
43:   FOR EACH arg IN argv
44:     c_string = CString::new(arg)
45:     c_strings.push(c_string.into_raw())
46:
47:   // rust_hmalloc aborts on OOM — no null check needed
48:   array_ptr = rust_hmalloc(sizeof(*mut i8) * (argv.len() + 1))
49:
50:   FOR EACH (i, ptr) IN c_strings
51:     write(array_ptr[i], ptr)
52:   write(array_ptr[argv.len()], null)
53:
54:   RETURN (array_ptr, c_strings)
55:
56: // Cleanup for c_strings must use CString::from_raw(), not libc::free()
57: // The array_ptr must be freed via rust_hfree()
```

## Pseudocode Component 3: Unit-test gap closure

```text
58: TEST "null_free_is_safe"
59:   rust_hfree(std::ptr::null_mut())  // must not crash
60:
61: TEST "realloc_null_ptr_acts_as_malloc"
62:   ptr = rust_hrealloc(std::ptr::null_mut(), 64)
63:   ASSERT ptr IS NOT NULL
64:   write bytes to ptr
65:   verify bytes
66:   rust_hfree(ptr)
```

## Pseudocode Component 4: Rust-side ABI integration tests

```text
67: TEST "allocate_and_free_via_exported_abi"
68:   ptr = rust_hmalloc(64)
69:   ASSERT ptr IS NOT NULL
70:   write bytes to ptr
71:   verify bytes
72:   rust_hfree(ptr)
73:
74: TEST "calloc_zero_fill_via_exported_abi"
75:   ptr = rust_hcalloc(128)
76:   ASSERT ptr IS NOT NULL
77:   ASSERT all 128 bytes are zero
78:   rust_hfree(ptr)
79:
80: TEST "realloc_preserves_data_via_exported_abi"
81:   ptr = rust_hmalloc(32)
82:   write pattern to ptr
83:   ptr2 = rust_hrealloc(ptr, 256)
84:   ASSERT ptr2 IS NOT NULL
85:   ASSERT first 32 bytes match pattern
86:   rust_hfree(ptr2)
87:
88: TEST "zero_size_normalization_via_exported_abi"
89:   p1 = rust_hmalloc(0)
90:   ASSERT p1 IS NOT NULL
91:   p2 = rust_hcalloc(0)
92:   ASSERT p2 IS NOT NULL
93:   p3 = rust_hrealloc(std::ptr::null_mut(), 0)
94:   ASSERT p3 IS NOT NULL
95:   rust_hfree(p1)
96:   rust_hfree(p2)
97:   rust_hfree(p3)
98:
99: TEST "lifecycle_smoke_via_exported_abi"
100:  ASSERT rust_mem_init() == true
101:  ASSERT rust_mem_uninit() == true
102:
103: TEST "realloc_zero_from_live_pointer_via_exported_abi"
104:  ptr = rust_hmalloc(16)
105:  ASSERT ptr IS NOT NULL
106:  ptr2 = rust_hrealloc(ptr, 0)
107:  ASSERT ptr2 IS NOT NULL
108:  rust_hfree(ptr2)
```

## Pseudocode Component 5: Traceability markers

```text
109: FOR EACH exported function IN rust/src/memory.rs
110:   add @plan PLAN-20260314-MEMORY.P05 marker
111:   add systematic @requirement REQ-MEM-* markers matching actual contract
112:
113: FOR copy_argv_to_c
114:   add @plan PLAN-20260314-MEMORY.P04 marker
115:   add @requirement markers for local ownership/documentation obligations only
```
