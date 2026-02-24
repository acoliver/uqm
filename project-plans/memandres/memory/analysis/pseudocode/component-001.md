# Pseudocode: Memory Allocator Swap Components

## Component 1: Header Macro Redirect (`memlib.h`)

```text
01: FILE memlib.h
02: INCLUDE stddef.h
03: INCLUDE types.h
04:
05: IF USE_RUST_MEM defined THEN
06:   DECLARE extern rust_hmalloc(size: size_t) -> void*
07:   DECLARE extern rust_hfree(ptr: void*) -> void
08:   DECLARE extern rust_hcalloc(size: size_t) -> void*
09:   DECLARE extern rust_hrealloc(ptr: void*, size: size_t) -> void*
10:   DECLARE extern rust_mem_init() -> bool
11:   DECLARE extern rust_mem_uninit() -> bool
12:
13:   DEFINE macro HMalloc(s) -> rust_hmalloc(s)
14:   DEFINE macro HFree(p) -> rust_hfree(p)
15:   DEFINE macro HCalloc(s) -> rust_hcalloc(s)
16:   DEFINE macro HRealloc(p, s) -> rust_hrealloc(p, s)
17:   DEFINE macro mem_init() -> rust_mem_init()
18:   DEFINE macro mem_uninit() -> rust_mem_uninit()
19: ELSE
20:   DECLARE extern mem_init() -> bool
21:   DECLARE extern mem_uninit() -> bool
22:   DECLARE extern HMalloc(size: size_t) -> void*
23:   DECLARE extern HFree(ptr: void*) -> void
24:   DECLARE extern HCalloc(size: size_t) -> void*
25:   DECLARE extern HRealloc(ptr: void*, size: size_t) -> void*
26: ENDIF
```

## Component 2: C Source Guard (`w_memlib.c`)

```text
30: FILE w_memlib.c
31: IF USE_RUST_MEM defined THEN
32:   EMIT compiler error "w_memlib.c should not be compiled when USE_RUST_MEM is enabled"
33: ENDIF
34: // rest of file unchanged
```

## Component 3: Build System Conditional (`Makeinfo`)

```text
40: FILE sc2/src/libs/memory/Makeinfo
41: IF USE_RUST_MEM == "1" OR uqm_USE_RUST_MEM == "1" THEN
42:   SET uqm_CFILES = ""
43: ELSE
44:   SET uqm_CFILES = "w_memlib.c"
45: ENDIF
```

## Component 4: Config Flag (`config_unix.h`)

```text
50: FILE sc2/config_unix.h
51: APPEND after existing USE_RUST_* block:
52:   DEFINE USE_RUST_MEM
53:   COMMENT "Defined if using Rust memory allocator"
```

## Component 5: Rust LogLevel Fatal Alias (`logging.rs`)

```text
60: FILE rust/src/logging.rs
61: IN LogLevel enum impl block:
62:   ADD constant Fatal = LogLevel::User
63:   COMMENT "Alias for User — matches C log_Fatal == log_User"
```

## Component 6: Rust memory.rs Log Level Update (`memory.rs`)

```text
70: FILE rust/src/memory.rs
71: REPLACE all LogLevel::User in OOM paths WITH LogLevel::Fatal
72:   LINE rust_hmalloc OOM: LogLevel::User -> LogLevel::Fatal
73:   LINE rust_hcalloc OOM: LogLevel::User -> LogLevel::Fatal
74:   LINE rust_hrealloc OOM: LogLevel::User -> LogLevel::Fatal
```

## Traceability

| Pseudocode Lines | Requirement |
|---|---|
| 01-26 | REQ-MEM-001 (Header Macro Redirect) |
| 30-34 | REQ-MEM-002 (C Source Guard) |
| 40-45 | REQ-MEM-003 (Build System Conditional) |
| 50-53 | REQ-MEM-004 (Config Flag) |
| 60-63, 70-74 | REQ-MEM-005 (OOM Log Level Correctness) |
| All | REQ-MEM-006 (Behavioral Equivalence) |
| 01-26, 40-45 | REQ-MEM-007 (Build Both Paths) |
