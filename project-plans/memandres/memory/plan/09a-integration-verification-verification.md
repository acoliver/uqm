# Phase 09a: Integration Verification — Final Verification

## Phase ID
`PLAN-20260224-MEM-SWAP.P09a`

## Prerequisites
- Required: Phase 09 completed
- All integration checks executed

## Final Verification Checks

### Runtime
- [ ] Game launched with Rust memory — no crash
- [ ] Log output confirmed: "Rust memory management initialized."
- [ ] Menu navigation exercised — no crash
- [ ] Content loaded (graphics, sound) — no corruption
- [ ] Game exited cleanly — no crash
- [ ] Log output confirmed: "Rust memory management deinitialized."

### Build
- [ ] Rust-path build succeeds
- [ ] C-path regression build succeeds
- [ ] `USE_RUST_MEM` is active (uncommented) in final state

### Automated
- [ ] `cargo test --workspace --all-features` passes
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings` passes
- [ ] `cargo fmt --all --check` passes
- [ ] Full C build succeeds: `cd sc2 && ./build.sh uqm`

### Completeness
- [ ] All requirements satisfied:
  - REQ-MEM-001: Header macro redirect active
  - REQ-MEM-002: C source guard in place
  - REQ-MEM-003: Build system conditional working
  - REQ-MEM-004: Config flag enabled
  - REQ-MEM-005: OOM log level uses `LogLevel::Fatal`
  - REQ-MEM-006: Behavioral equivalence verified at runtime
  - REQ-MEM-007: Both build paths compile

### Files Changed (Complete List)

| File | Change |
|---|---|
| `rust/src/logging.rs` | Added `LogLevel::Fatal` constant + `test_fatal_alias` test |
| `rust/src/memory.rs` | Changed `LogLevel::User` → `LogLevel::Fatal` in 3 OOM paths |
| `sc2/src/libs/memlib.h` | Added `#ifdef USE_RUST_MEM` block with extern decls + macros |
| `sc2/src/libs/memory/w_memlib.c` | Added `#ifdef USE_RUST_MEM` / `#error` guard |
| `sc2/src/libs/memory/Makeinfo` | Added conditional to exclude `w_memlib.c` |
| `sc2/config_unix.h` | Added `#define USE_RUST_MEM` |

## Plan Evaluation Checklist (Gate Before Marking Complete)

- [x] Uses plan ID + sequential phases
- [x] Preflight verification defined (P00a)
- [x] Requirements are expanded and testable
- [x] Integration points are explicit (memlib.h macro redirect)
- [x] Legacy code replacement/removal is explicit (w_memlib.c excluded, not deleted)
- [x] Pseudocode line references present
- [x] Verification phases include semantic checks
- [x] Lint/test/coverage gates are defined
- [x] No reliance on placeholder completion

## Gate Decision
- [ ] PASS: Plan PLAN-20260224-MEM-SWAP is complete
- [ ] FAIL: identify remaining issues
