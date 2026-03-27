# Phase 12a: Summary Guard Stub Verification

## Phase ID
`PLAN-20260325-COMMPT3.P12a`

## Prerequisites
- Required: Phase P12 completed
- Expected artifacts: Modified `ffi.rs` with cfg-bifurcated summary function

## Verification Commands

```bash
cd rust && cargo check --workspace --all-features
cd rust && cargo test --workspace --all-features

grep -B2 -A15 "rust_ShowConversationSummary" rust/src/comm/ffi.rs
grep -A5 "cfg(not(test))" rust/src/comm/ffi.rs | grep "c_SelectConversationSummary"
```

## Structural Verification Checklist
- [ ] `rust_ShowConversationSummary` has cfg-bifurcated structure
- [ ] Production path calls `c_SelectConversationSummary()`
- [ ] Test path retains SummaryView model
- [ ] Project compiles, tests pass

## Semantic Verification Checklist (Mandatory)
- [ ] Production path contains ONLY `c_SelectConversationSummary()` call + return
- [ ] No `SummaryView`, `rebuild_summary`, or `advance_page` in production path
- [ ] Test path is functionally unchanged from before

## Semantic Negative-Proof Gate (Mandatory)
- [ ] Stale markers ("abort not yet wired", "input handling is not yet implemented")
  still present in the code — confirms P13/P14 are needed for marker cleanup
- [ ] Production path delegates to C but no test verifies the delegation behavior
  yet — confirms TDD phase P13 is needed

## Phase Completion Marker
Create: `project-plans/20260311/commpt3/.completed/P12a.md`
