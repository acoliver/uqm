# Phase 02: Pseudocode

## Phase ID
`PLAN-20260314-THREADING.P02`

## Prerequisites
- Required: Phase 01a (Analysis Verification) completed and passed
- Expected previous artifact: `project-plans/20260311/threading/.completed/P01a.md`

## Purpose

Produce numbered algorithmic pseudocode for all code changes required by gaps G1-G4. Gaps G5-G7 are comment/documentation changes that do not need pseudocode.

## G1: Return Value Propagation

### spawn_c_thread (Rust)

```text
01: FUNCTION spawn_c_thread(name: Option<&str>, func: CFuncPtr, data: *mut c_void) -> Result<Thread<c_int>>
02:   CAPTURE func_ptr = func as usize
03:   CAPTURE data_ptr = data as usize
04:   Thread::spawn(name, closure):
05:     ATTACH ThreadLocalGuard
06:     RECONSTRUCT func from func_ptr via transmute
07:     RECONSTRUCT data from data_ptr
08:     CALL result = func(data)     // captures c_int return value
09:     RETURN result                 // closure returns c_int, not ()
10: END FUNCTION
```

### rust_thread_spawn (Rust FFI)

```text
11: FUNCTION rust_thread_spawn(name, func, data) -> *mut RustThread
12:   PARSE name from C string
13:   CALL thread = spawn_c_thread(name, func, data)
14:   IF Ok(thread):
15:     RETURN Box::into_raw(Box::new(thread)) as *mut RustThread  // Thread<c_int>
16:   ELSE:
17:     RETURN null
18: END FUNCTION
```

### rust_thread_spawn_detached (Rust FFI)

```text
19: FUNCTION rust_thread_spawn_detached(name, func, data) -> void
20:   PARSE name from C string
21:   CALL result = spawn_c_thread(name, func, data)
22:   MATCH result:
23:     Ok(thread):
24:       DROP thread   // JoinHandle dropped = thread detached, runs to completion
25:     Err(e):
26:       LOG warning or document failure boundary
27: END FUNCTION
```

### rust_thread_join (Rust FFI)

```text
28: FUNCTION rust_thread_join(thread: *mut RustThread, out_status: *mut c_int) -> c_int
29:   IF thread is null:
30:     IF out_status is not null: WRITE 0 to *out_status
31:     RETURN 0
32:   RECONSTRUCT thread_box = Box::from_raw(thread as *mut Thread<c_int>)
33:   CALL join_result = thread_box.join()
34:   MATCH join_result:
35:     Ok(status):
36:       IF out_status is not null: WRITE status to *out_status
37:       RETURN 1
38:     Err(_):
39:       IF out_status is not null: WRITE 0 to *out_status
40:       RETURN 0
41: END FUNCTION
```

### WaitThread (C adapter)

```text
42: FUNCTION WaitThread(thread: Thread, status: *int)
43:   CAST t = (TrueThread) thread
44:   IF status is not null: WRITE 0 to *status
45:   IF t is null OR t->native is null: RETURN
46:   DECLARE out_status: int = 0
47:   CALL result = rust_thread_join(t->native, &out_status)
48:   IF status is not null:
49:     IF result == 1: WRITE out_status to *status
50:     ELSE: WRITE 0 to *status
51:   SET t->native = NULL
52: END FUNCTION
```

### rust_threads.h declaration update

```text
53: DECLARATION in rust_threads.h: int rust_thread_join(RustThread* thread, int* out_status);
```

### rust_thrcommon.c local extern declaration update

```text
54: DECLARATION in rust_thrcommon.c (line 45): extern int rust_thread_join(RustThread* thread, int* out_status);
```

## G2: SleepThreadUntil Async Pumping

### SleepThreadUntil (C adapter)

```text
55: FUNCTION SleepThreadUntil(wakeTime: TimeCount)
56:   LOOP forever:
57:     CALL Async_process()
58:     SET now = GetTimeCounter()
59:     IF wakeTime <= now: RETURN
60:     SET nextTimeMs = Async_timeBeforeNextMs()
61:     SET nextTime = CONVERT nextTimeMs to TimeCount:
62:       (nextTimeMs / 1000) * ONE_SECOND + ((nextTimeMs % 1000) * ONE_SECOND / 1000)
63:     IF wakeTime < nextTime: SET nextTime = wakeTime
64:     CALL SleepThread(nextTime - now)
65:   END LOOP
66: END FUNCTION
```

## G3: StartThread_Core lifecycle-handle decision

### StartThread_Core (C adapter)

```text
67: FUNCTION StartThread_Core(func, data, stackSize, name)
68:   IGNORE stackSize
69:   ALLOCATE thread = AllocThreadHandle(name)
70:   ALLOCATE startInfo = {func, data, thread}
71:   CALL native = rust_thread_spawn(name, RustThreadHelper, startInfo)
72:   IF native is NULL:
73:     HFree(startInfo)
74:     HFree(thread)
75:     RETURN
76:   SET thread->native = native
77:   // Keep joinable internal handle because ProcessThreadLifecycles
78:   // later calls WaitThread(t, NULL) -> rust_thread_join(t->native, ...)
79: END FUNCTION
```

**Decision:** Keep `rust_thread_spawn` for `StartThread_Core`. The spec's suggestion to use `rust_thread_spawn_detached` is `[Reference design]`, but the actual C lifecycle cleanup path requires a native handle today.

## G4: Detached helper cleanup scope and unresolved ABI mismatch

```text
80: FUNCTION rust_thread_spawn_detached(name, func, data) -> void
81:   PARSE name from C string
82:   CALL result = spawn_c_thread(name, func, data)
83:   MATCH result:
84:     Ok(thread):
85:       DROP thread
86:     Err(_):
87:       // Spawn failed before worker start.
88:       // Rust-side helper can contain the failure and avoid panic.
89:       // However, current ABI provides no synchronous failure signal back
90:       // to the C caller, so adapter-owned wrapper cleanup cannot be
91:       // guaranteed from this entry point alone.
92:       PASS
93: END FUNCTION
```

This pseudocode intentionally documents the limit of the current ABI instead of pretending the detached-failure contract is satisfied.

## Pseudocode Traceability Summary

| Pseudocode Lines | Gap | Phase Applied |
|-----------------|-----|---------------|
| 01-10 | G1: spawn_c_thread | P03 (stub), P05 (impl) |
| 11-18 | G1: rust_thread_spawn | P03 (stub) |
| 19-27 | G4: rust_thread_spawn_detached style cleanup | P07 (impl) |
| 28-41 | G1: rust_thread_join | P03 (stub), P05 (impl) |
| 42-52 | G1: WaitThread C adapter | P05 (impl) |
| 53 | G1: rust_threads.h declaration | P05 (impl) |
| 54 | G1: rust_thrcommon.c local extern declaration | P05 (impl) |
| 55-66 | G2: SleepThreadUntil | P06 (impl) |
| 67-79 | G3: StartThread_Core | P07 (docs only) |
| 80-93 | G4: detached error-handling scope note | P07 (impl/docs) |

## Phase Completion Marker
Create: `project-plans/20260311/threading/.completed/P02.md` with phase ID, timestamp, files changed, verification commands run, outputs summary, semantic findings, and follow-up notes.
