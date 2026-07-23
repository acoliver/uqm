# Phase 04: Pure Sticky-Terminal Runtime Model

Phase ID: `PLAN-20260723-RUNTIME-AUTOMATION.P04`

Require `.completed/P03.md`. Own only `REQ-STATE-001..004` and the pure classification model for `REQ-WATCH-004`. It does not own `REQ-EXIT-*` or `REQ-FFI-*`, expose extern C, call current-activity FFI, write C keys, integrate actual clear sites/outer lifecycle, observe graphics, or claim teardown/capture.

## Files

Create `rust/src/automation/{outcome,runtime,sync_model}.rs`; modify module/error/trace as required. Side effects are traits/fakes only.

## State contracts/TDD

1. RED first-wins terminal and absorbing transition property tests; later errors are secondary and never success.
2. RED terminal command output always includes release-all, OR-abort intent, stop=true.
3. RED complete shell model: saturating ABI entry -> acquire inactive neutral fast path (no TLS/allocation/lock/external work) -> active-gate count -> depth -> catch -> pure reserve transition -> unlock -> effect -> ordered publish/cancel -> validated commit -> conservative fallback.
4. RED fixed lock-free mirrors: terminal/status, abort, phase, capture request, and six owned-key mask/values; nested entry and unusable/poisoned lock release/abort without locking and never resume scheduling.
5. RED two-phase stale/duplicate/version/generation commit and RAII cancellation model.
6. RED finalization: clear active/capture, drain active shells/reservations, atomic take once, ordered run_end/close once, late callback cannot use writer.
7. RED lock-order instrumentation rejects runtime-mutex overlap with C/SDL/graphics/log/wait/file and rejects runtime+ordered-I/O nesting.
8. RED typed distinction cooperative timeout versus parent hard hang.
9. Property tests arbitrary terminal/error/late-callback sequences.

Use pseudocode 003 and execution-contract §§3-4. Run focused and strict gates. No production unsafe or FFI is expected. Worker handoff only.
