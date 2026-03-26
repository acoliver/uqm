# Phase 0.5: Preflight Verification

## Phase ID
`PLAN-20260326-COMMPT2.P00.5`

## Purpose
Verify all assumptions about the codebase, toolchain, and existing bridges before
any implementation begins. If any check fails, stop and revise the plan.

## Toolchain Verification

- [ ] `cargo --version` — Rust compiler available
- [ ] `rustc --version` — compatible Rust edition (2021+)
- [ ] `cargo clippy --version` — linter available
- [ ] `cargo fmt --version` — formatter available
- [ ] C compiler available and `USE_RUST_COMM` build mode works

```bash
cargo --version
rustc --version
cargo clippy --version
cargo fmt --version
```

## Dependency Verification

- [ ] `serial_test` crate in `rust/Cargo.toml` (used by comm tests)
- [ ] `libc` or equivalent FFI crate available
- [ ] `rust_comm.c` compiles under `USE_RUST_COMM` flag

## Existing C Bridge Wrapper Verification

Verify that these bridge functions already exist in `sc2/src/uqm/rust_comm.c`
and have correct signatures in `sc2/src/uqm/rust_comm.h`:

### Input Bridges (already exist)
- [ ] `c_GetPulsedMenuKey(int key_index)` → returns `int` (rust_comm.c:749)
- [ ] `c_GetCurrentMenuKey(int key_index)` → returns `int` (rust_comm.c:755)
- [ ] `c_SetMenuSounds(unsigned int, unsigned int)` (rust_comm.c:761)

### Track Bridges (already exist)
- [ ] `c_SpliceTrack(UNICODE*, UNICODE*, UNICODE*, CallbackFunction)` (rust_comm.c:320)
- [ ] `c_PlayTrack()` (rust_comm.c:333)
- [ ] `c_StopTrack()` (rust_comm.c:339)
- [ ] `c_PlayingTrack()` → returns count (rust_comm.c)

### Phrase Bridges (already exist)
- [ ] `c_get_conversation_phrase(const void*, int)` → returns `const unsigned char*` (rust_comm.c:283)

### Resource Destroy Bridges (already exist)
- [ ] `c_DestroyDrawable(unsigned int)` (rust_comm.c:809)
- [ ] `c_DestroyFont(unsigned int)` (rust_comm.c:815)
- [ ] `c_DestroyColorMap(unsigned int)` (rust_comm.c:821)
- [ ] `c_DestroyMusic(unsigned int)` (rust_comm.c:827)
- [ ] `c_DestroyStringTable(unsigned int)` (rust_comm.c:833)

### Rendering Bridges (exist but are stubs)
- [ ] `c_FeedbackPlayerPhrase(const char*)` (rust_comm.c:665) — currently stub
- [ ] `c_RefreshResponses(unsigned char, unsigned char, unsigned char)` (rust_comm.c:673) — currently stub
- [ ] `c_SelectConversationSummary(void)` (rust_comm.c:684) — currently stub
- [ ] `c_DrawSISComWindow(void)` (rust_comm.c:690) — already implemented

### Game State Bridges (already exist)
- [ ] `c_CheckAbort()` (rust_comm.c:769)
- [ ] `c_WonLastBattle()` (rust_comm.c:775)
- [ ] `c_GetLastActivityAbortFlag()` (rust_comm.c:781)
- [ ] `c_ClearLastActivityLoadFlag()` (rust_comm.c:787)
- [ ] `c_GetOptSmoothScroll()` (rust_comm.c:793)

### Music Bridges (already exist)
- [ ] `c_PlayMusic(void*, int, int)` (rust_comm.c:715)
- [ ] `c_FadeMusic(int, int)` (rust_comm.c:721)
- [ ] `c_StopMusic()` (rust_comm.c:727)

### Resource Load/Context Bridges (DO NOT EXIST — need P06)
- [ ] `c_LoadGraphic` — MISSING, needed for P06
- [ ] `c_LoadFont` — MISSING, needed for P06
- [ ] `c_LoadColorMap` — MISSING, needed for P06
- [ ] `c_LoadMusic` — MISSING (distinct from PlayMusic), needed for P06
- [ ] `c_LoadStringTable` — MISSING, needed for P06
- [ ] `c_CaptureDrawable` — MISSING, needed for P06
- [ ] `c_CaptureColorMap` — MISSING, needed for P06
- [ ] `c_CaptureStringTable` — MISSING, needed for P06
- [ ] `c_CreateContext` — MISSING, needed for P06
- [ ] `c_DestroyContext` — MISSING, needed for P06
- [ ] `c_SetContext` — MISSING, needed for P06
- [ ] `c_SetContextFGFrame` — MISSING, needed for P06
- [ ] `c_SetContextClipRect` — MISSING, needed for P06
- [ ] `c_SetContextBackGroundColor` — MISSING, needed for P06
- [ ] `c_CreateDrawable` — MISSING, needed for P06
- [ ] `c_SetFrameTransparentColor` — MISSING, needed for P06
- [ ] `c_ClearDrawable` — MISSING, needed for P06
- [ ] `c_BatchGraphics` — MISSING, needed for P06
- [ ] `c_UnbatchGraphics` — MISSING, needed for P06
- [ ] `c_SetTransitionSource` — MISSING, needed for P06
- [ ] `c_ScreenTransition` — MISSING, needed for P06
- [ ] `c_DrawSISFrame` — MISSING, needed for P06
- [ ] `c_DrawSISMessage` — MISSING, needed for P06
- [ ] `c_DrawSISTitle` — MISSING, needed for P06
- [ ] `c_DoInput` — MISSING, needed for P06

## Key Constant Verification

Verify these constants match between C and Rust:

```bash
# Check C-side constants
grep -n "KEY_MENU_SELECT\|KEY_MENU_CANCEL\|KEY_MENU_UP\|KEY_MENU_DOWN\|KEY_MENU_LEFT\|KEY_MENU_RIGHT" sc2/src/uqm/controls.h
```

Expected values:
- [ ] `KEY_MENU_SELECT` = 0
- [ ] `KEY_MENU_UP` = 1
- [ ] `KEY_MENU_DOWN` = 2
- [ ] `KEY_MENU_CANCEL` = 3
- [ ] `KEY_MENU_LEFT` = 4
- [ ] `KEY_MENU_RIGHT` = 5

## Existing Comm Test Verification

```bash
# All existing comm tests must pass before any changes
cargo test --workspace --all-features -- comm 2>&1 | tail -5
```

- [ ] All 267+ comm tests pass
- [ ] No test failures or compilation errors

## Build Mode Verification

```bash
# Verify USE_RUST_COMM=on build compiles
# (project-specific build command)
```

- [ ] `USE_RUST_COMM=on` compiles and links
- [ ] `USE_RUST_COMM=off` compiles and links (no regressions)

## Call-Path Feasibility

Verify the planned call paths are reachable:

- [ ] `comm.c:1458` calls `rust_HailAlien()` under `#ifdef USE_RUST_COMM`
- [ ] `commglue.c` routes `NPCPhrase_cb` to `rust_NPCPhrase_cb` under guard
- [ ] `talk_segue.rs` `c_bridge` module is the production FFI path
- [ ] `c_bridge` already declares `c_GetPulsedMenuKey` — NO, it does NOT. Verify.

```bash
grep -n "c_GetPulsedMenuKey" rust/src/comm/talk_segue.rs
```

- [ ] If `c_GetPulsedMenuKey` is NOT in talk_segue.rs c_bridge extern block, P03 must add it

## Test Infrastructure

- [ ] `rust/src/comm/ffi.rs` has `#[cfg(test)] mod tests` with `serial_test` usage
- [ ] `rust/src/comm/talk_segue.rs` has test infrastructure
- [ ] Tests use `CommState` directly (not through FFI) for unit testing

## Blocking Issues

List any blockers found during preflight. If non-empty, stop and revise plan.

## Gate Decision
- [ ] PASS: proceed to P01
- [ ] FAIL: revise plan — describe blockers
