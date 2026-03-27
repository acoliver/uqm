# Phase 00a: Preflight Verification

## Phase ID
`PLAN-20260325-COMMPT3.P00a`

## Purpose
Verify all assumptions about types, interfaces, call paths, and toolchain
before implementation begins.

## Prerequisites
- Phase P00 completed (requirements locked)

## Toolchain Verification
- [ ] `cargo --version` — Rust toolchain available
- [ ] `rustc --version` — compiler version compatible
- [ ] `cargo clippy --version` — linter available
- [ ] C build system functional (`./build.sh` or equivalent)
- [ ] Both `USE_RUST_COMM=on` and `USE_RUST_COMM=off` builds compile

## Dependency Verification
- [ ] `parking_lot` crate present in `rust/Cargo.toml` (for RwLock)
- [ ] `serial_test` crate present for `#[serial]` test attribute

## Type/Interface Verification

### C-side bridges that must exist
- [ ] `c_SetColorMap(void *colormap)` — exists in `rust_comm.c` (line ~1186)
- [ ] `c_PlayMusic(void *song, int looping, int priority)` — exists in `rust_comm.c`
- [ ] `CommData.AlienColorMap` — accessed via `c_SetCommDataAlienColorMap` (confirm at `rust_comm.c:1607`)
- [ ] `CommData.AlienSong` — accessed via `c_SetCommDataAlienSong` (confirm at `rust_comm.c:1613`)
- [ ] `GetColorMapAddress()` — exists in C headers (check `libs/graphics/gfx_common.h` or `libs/graphics/gfxlib.h`)
- [ ] `PlayMusic()` — exists in C headers (check `libs/sound/sound.h`)

### Rust-side functions that must exist
- [ ] `set_colormap()` in `talk_segue.rs` — line ~997
- [ ] `play_alien_music()` in `talk_segue.rs` — line ~939
- [ ] `rust_DoCommunication()` in `ffi.rs` — line ~715
- [ ] `rust_ShowConversationSummary()` in `ffi.rs` — line ~860
- [ ] `do_communication()` in `talk_segue.rs` — line ~435
- [ ] `player_response_input()` in `talk_segue.rs` — line ~377
- [ ] `select_response()` in `talk_segue.rs` — line ~343
- [ ] `alien_talk_segue()` in `talk_segue.rs` — line ~303

### C-side subtitle access (critical for P04)
- [ ] `SubtitleText` — static variable in `comm.c:103`
- [ ] `clear_subtitles` — static variable in `comm.c:102`
- [ ] `last_subtitle` — static variable in `comm.c:104`
- [ ] `add_text()` — static function in `comm.c:152` (accessible within `comm.c` only)
- [ ] `optSubtitles` — global variable accessible from `comm.c`
- [ ] `GetTrackSubtitle()` — available via `libs/sound/trackplayer.h`
- [ ] `#ifdef USE_RUST_COMM` block at `comm.c:1715` exists for adding new functions

## Call-Path Feasibility

### Colormap path
- [ ] `talk_segue.rs:set_colormap` → `extern "C" c_SetColorMapFromCommData` → `rust_comm.c:c_SetColorMapFromCommData` → `SetColorMap(GetColorMapAddress(CommData.AlienColorMap))`

### Music path
- [ ] `talk_segue.rs:play_alien_music` → `extern "C" c_PlayAlienMusic` → `rust_comm.c:c_PlayAlienMusic` → `PlayMusic(CommData.AlienSong, TRUE, 1)`

### Subtitle path
- [ ] `c_ClearSubtitles` [rust_comm.c] → `comm_ClearSubtitles` [comm.c] (static vars accessible)
- [ ] `c_CheckSubtitles` [rust_comm.c] → `comm_CheckSubtitles` [comm.c] (GetTrackSubtitle accessible)
- [ ] `c_RedrawSubtitles` [rust_comm.c] → `comm_RedrawSubtitles` [comm.c] (add_text accessible)

### Lock discipline path
- [ ] `rust_DoCommunication` acquires `COMM_STATE.write()`, calls `do_communication`, gets result
- [ ] If `Selected(fn, ref)`: drops lock, calls `fn(ref)` (separate lock acquisitions in callback)
- [ ] Callback calls `rust_NPCPhrase_cb` which acquires `COMM_STATE.write()` (not nested)

## Test Infrastructure Verification
- [ ] `cd rust && cargo test --lib -- comm` passes (268 tests)
- [ ] No pre-existing test failures

## Blocking Issues

Check each item. If any fails, the plan must be revised before proceeding.

### Potential blockers
1. If `add_text()` is static in `comm.c`, subtitle implementations MUST live
   in `comm.c` (confirmed: this is the planned approach — functions in
   `#ifdef USE_RUST_COMM` block at line 1715+)
2. If `GetColorMapAddress` is a macro, `c_SetColorMapFromCommData` handles
   expansion in C (Rust never sees the macro)
3. `c_PlayMusic` and `c_SetColorMap` in `talk_segue.rs` extern block — verify
   no other callers before removing (if other callers exist, keep declarations)

## Gate Decision
- [ ] PASS: proceed to P01
- [ ] FAIL: revise plan (document blockers above)

## Phase Completion Marker
Create: `project-plans/20260311/commpt3/.completed/P00a.md`
