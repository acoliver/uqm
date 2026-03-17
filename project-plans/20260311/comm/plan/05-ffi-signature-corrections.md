# Phase 05: FFI Signature Corrections

## Phase ID
`PLAN-20260314-COMM.P05`

## Prerequisites
- Required: Phase 04a completed
- Expected files: `phrase_state.rs`, `glue.rs`, `segue.rs` from P04

## Requirements Implemented (Expanded)

### RS-REQ-011: Response callback ABI
**Requirement text**: When the selected response callback is dispatched, the communication subsystem shall pass the selected response reference as an argument to the callback, matching the established C convention where `RESPONSE_FUNC` receives `RESPONSE_REF`.

Behavior contract:
- GIVEN: A response registered with `DoResponsePhrase(42, my_callback, NULL)`
- WHEN: The player selects that response and the callback is dispatched
- THEN: `my_callback(42)` is called — the callback receives `42` as its argument

### OL-REQ-009, OL-REQ-010: Subtitle pointer safety
**Requirement text**: Strings returned across integration boundaries shall have safe lifetime behavior.

Behavior contract:
- GIVEN: C code calls `rust_GetSubtitle()`
- WHEN: The returned string pointer is used
- THEN: The string data is stable because it was copied to a C-owned buffer (not a dangling Rust reference)

### TP-REQ-002: Multi-track splice
**Requirement text**: Multiple audio clips merged via multi-track merge shall produce a single phrase.

### TP-REQ-005: JumpTrack semantics
**Requirement text**: Skip shall advance to end of current phrase only. No offset parameter.

## Implementation Tasks

### Files to modify

- `rust/src/comm/response.rs`
  - **Change `ResponseEntry.response_func` type** from `Option<usize>` to `Option<extern "C" fn(u32)>`
  - Update all methods that store/retrieve the callback
  - Update `execute_selected()` to pass `response_ref` as argument
  - marker: `@plan PLAN-20260314-COMM.P05`
  - marker: `@requirement RS-REQ-011, RS-REQ-012`

- `rust/src/comm/ffi.rs`
  - **Fix `rust_DoResponsePhrase`**: change `func` param from `Option<extern "C" fn()>` to `Option<extern "C" fn(u32)>`
  - **Fix `rust_ExecuteResponse`**: call `func(response_ref)` instead of `func()`
  - **Fix `rust_SpliceTrack`**: add `timestamps: *const f32`, `timestamp_count: c_uint`, `callback: Option<extern "C" fn()>` parameters
  - **Add `rust_SpliceMultiTrack`**: proposed concrete implementation `(tracks: *const c_uint, track_count: c_uint, text: *const c_char)` — queues multi-clip phrase
  - **Fix `rust_JumpTrack`**: remove `offset` parameter — JumpTrack skips to end of current phrase (no seek offset)
  - **Fix `rust_GetSubtitle`**: return a stable C string by copying into a static buffer or thread-local, not returning a pointer to lock-guarded Rust data
  - **Add `rust_FastForward_Page`**, `rust_FastForward_Smooth`, `rust_FastReverse_Page`, `rust_FastReverse_Smooth` FFI exports
  - **Add `rust_PlayingTrack() -> c_uint`** FFI export
  - marker: `@plan PLAN-20260314-COMM.P05`
  - marker: `@requirement TP-REQ-002, TP-REQ-005, OL-REQ-009, OL-REQ-010`

- `rust/src/comm/state.rs`
  - Update `add_response()` signature to accept `Option<extern "C" fn(u32)>` instead of `Option<usize>`
  - marker: `@plan PLAN-20260314-COMM.P05`

- `sc2/src/uqm/rust_comm.h`
  - Fix `rust_DoResponsePhrase` declaration: `void (*func)(unsigned int)` instead of `void (*func)(void)`
  - Fix `rust_SpliceTrack` declaration: add timestamps and callback parameters
  - Add `rust_SpliceMultiTrack` declaration
  - Fix `rust_JumpTrack` declaration: no parameters
  - Add `rust_FastForward_Page`, `rust_FastForward_Smooth`, `rust_FastReverse_Page`, `rust_FastReverse_Smooth`
  - Add `rust_PlayingTrack`

### Signature wording note

The specification's example FFI prototypes are non-normative. This phase uses the signatures above as the **proposed concrete implementation shape** for this codebase, with the true hard requirement being preservation of externally visible behavior and source-compatibility contracts.

### Subtitle safety approach

Replace the current `rust_GetSubtitle` implementation:

**Current** (unsafe):
```rust
// Returns pointer to lock-guarded Rust data — invalid after lock release
let state = COMM_STATE.read();
match state.current_subtitle() {
    Some(s) => s.as_ptr() as *const c_char,
    None => std::ptr::null(),
}
```

**Fixed** (safe):
```rust
// Thread-local static buffer for subtitle string
thread_local! {
    static SUBTITLE_BUF: RefCell<Vec<u8>> = RefCell::new(Vec::with_capacity(512));
}

#[no_mangle]
pub extern "C" fn rust_GetSubtitle() -> *const c_char {
    let state = COMM_STATE.read();
    match state.current_subtitle() {
        Some(s) => {
            SUBTITLE_BUF.with(|buf| {
                let mut buf = buf.borrow_mut();
                buf.clear();
                buf.extend_from_slice(s.as_bytes());
                buf.push(0); // null terminator
                buf.as_ptr() as *const c_char
            })
        }
        None => std::ptr::null(),
    }
}
```

### Pseudocode traceability
- FFI shape changes — no pseudocode lines (API corrections per spec §14)

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] `ResponseEntry.response_func` is `Option<extern "C" fn(u32)>`
- [ ] `rust_DoResponsePhrase` accepts `Option<extern "C" fn(u32)>`
- [ ] `rust_ExecuteResponse` passes response_ref to callback
- [ ] `rust_SpliceTrack` has timestamps and callback parameters
- [ ] `rust_SpliceMultiTrack` exists
- [ ] `rust_JumpTrack` has no parameters
- [ ] `rust_GetSubtitle` uses thread-local buffer for safety
- [ ] `rust_comm.h` declarations match Rust exports

## Semantic Verification Checklist (Mandatory)
- [ ] Test: register response with `extern "C" fn(u32)` callback, execute, verify arg received
- [ ] Test: `rust_GetSubtitle` returns stable pointer after lock release
- [ ] Test: `rust_JumpTrack` advances to end of current phrase (not arbitrary offset)
- [ ] Test: `rust_SpliceMultiTrack` creates single phrase from multiple clips
- [ ] Existing FFI tests updated and passing (response, track, subtitle tests)
- [ ] No compilation errors in C headers (type agreement verified)

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/comm/ffi.rs rust/src/comm/response.rs
```

## Success Criteria
- [ ] Callback ABI matches C convention
- [ ] Subtitle pointer is safe for C consumption
- [ ] Concrete FFI signatures are documented as implementation choices, not overclaimed as normative spec text
- [ ] All tests pass

## Failure Recovery
- rollback: `git restore rust/src/comm/ffi.rs rust/src/comm/response.rs rust/src/comm/state.rs`
- blocking: Existing C callers of old FFI signatures must be updated simultaneously

## Phase Completion Marker
Create: `project-plans/20260311/comm/.completed/P05.md`
