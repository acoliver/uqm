# Phase 06a: Subtitle Display Bridge Stub Verification

## Phase ID
`PLAN-20260325-COMMPT3.P06a`

## Prerequisites
- Required: Phase P06 completed
- Expected artifacts: Modified `comm.c`, `rust_comm.c`, `rust_comm.h` with subtitle stubs

## Verification Commands

```bash
cd rust && cargo check --workspace --all-features
cd rust && cargo test --workspace --all-features

# Circular routing broken
grep -n "rust_ClearSubtitles\|rust_CheckSubtitles\|rust_RedrawSubtitles" sc2/src/uqm/rust_comm.c | grep -v "extern\|declare\|//\|proto"

# Stubs exist
grep -n "comm_ClearSubtitles\|comm_CheckSubtitles\|comm_RedrawSubtitles" sc2/src/uqm/comm.c

# Forwarding correct
grep -B1 -A3 "void c_ClearSubtitles" sc2/src/uqm/rust_comm.c
grep -B1 -A3 "void c_CheckSubtitles" sc2/src/uqm/rust_comm.c
grep -B1 -A3 "void c_RedrawSubtitles" sc2/src/uqm/rust_comm.c
```

## Structural Verification Checklist
- [ ] Three `comm_*` functions defined in `comm.c` inside `#ifdef USE_RUST_COMM` (empty bodies)
- [ ] Three `comm_*` functions declared in `rust_comm.h`
- [ ] `c_ClearSubtitles` body calls `comm_ClearSubtitles()` (not `rust_ClearSubtitles`)
- [ ] `c_CheckSubtitles` body calls `comm_CheckSubtitles()` (not `rust_CheckSubtitles`)
- [ ] `c_RedrawSubtitles` body calls `comm_RedrawSubtitles()` (not `rust_RedrawSubtitles`)
- [ ] Both build modes compile and link

## Semantic Verification Checklist (Mandatory)
- [ ] Stubs have NO functional behavior (empty bodies only)
- [ ] Circular routing to Rust FFI is broken
- [ ] Rust `rust_ClearSubtitles`/`rust_CheckSubtitles`/`rust_RedrawSubtitles` exports unchanged (test use)
- [ ] All 268+ tests pass

## Semantic Negative-Proof Gate (Mandatory)
- [ ] `comm_ClearSubtitles` does NOT set `clear_subtitles`, `last_subtitle`, or `SubtitleText`
  (verified by reading empty body — confirms TDD phase P07 is needed)
- [ ] `comm_CheckSubtitles` does NOT call `GetTrackSubtitle()` (empty body — confirms P07 needed)
- [ ] `comm_RedrawSubtitles` does NOT call `add_text()` (empty body — confirms P07 needed)

## Phase Completion Marker
Create: `project-plans/20260311/commpt3/.completed/P06a.md`
