# Phase 0.5: Preflight Verification

## Phase ID
`PLAN-20260314-COMM.P00.5`

## Purpose
Verify assumptions about toolchain, dependencies, types, call paths, and compatibility preconditions before implementation begins.

## Toolchain Verification

```bash
cargo --version
rustc --version
cargo clippy --version
```

- [ ] Rust toolchain is 1.75+ (required for `LazyLock` stabilization)
- [ ] `parking_lot` crate present in `Cargo.toml` (used by `COMM_STATE`)
- [ ] `serial_test` crate present in dev-dependencies (used by FFI tests)

## Dependency Verification

- [ ] `parking_lot` version in `/Users/acoliver/projects/uqm/rust/Cargo.toml`
- [ ] `serial_test` version in `/Users/acoliver/projects/uqm/rust/Cargo.toml`
- [ ] No `comm`-specific feature flags needed in `Cargo.toml`

## Type/Interface Verification

### Rust Side
- [ ] `COMM_STATE` is `LazyLock<RwLock<CommState>>` in `/Users/acoliver/projects/uqm/rust/src/comm/state.rs:16-17`
- [ ] `CommState` fields match expectations in `/Users/acoliver/projects/uqm/rust/src/comm/state.rs:21-59`
- [ ] `CommData` is `Default`-derivable in `/Users/acoliver/projects/uqm/rust/src/comm/types.rs`
- [ ] `ResponseEntry.response_func` is `Option<usize>` (needs change to `Option<extern "C" fn(u32)>`)
- [ ] `TrackManager` has no callback or phrase-completion model (confirms gap G5/G6)
- [ ] `AnimContext` has no `ANIMATION_DESC`, `BlockMask`, or talk/transit support (confirms gap G14)

### C Side
- [ ] `LOCDATA` struct definition accessible in `/Users/acoliver/projects/uqm/sc2/src/uqm/comm.h`
- [ ] `CONVERSATION` enum in `/Users/acoliver/projects/uqm/sc2/src/uqm/commglue.h:33-59`
- [ ] `RESPONSE_FUNC` typedef is `void (*)(RESPONSE_REF)` in `/Users/acoliver/projects/uqm/sc2/src/uqm/commglue.h:89`
- [ ] `RESPONSE_REF` is `COUNT` (unsigned) in `/Users/acoliver/projects/uqm/sc2/src/uqm/commglue.h:87`
- [ ] `CallbackFunction` type from `libs/callback.h` is `void (*)(void)`
- [ ] `ANIMATION_DESC` struct in `/Users/acoliver/projects/uqm/sc2/src/uqm/commanim.h`
- [ ] `MAX_ANIMATIONS` is 20 in `/Users/acoliver/projects/uqm/sc2/src/uqm/commanim.h`
- [ ] `USE_RUST_COMM` defined in `/Users/acoliver/projects/uqm/sc2/config_unix.h:86-87`

### Trackplayer Integration
- [ ] `SpliceTrack()` signature in `/Users/acoliver/projects/uqm/sc2/src/libs/sound/trackplayer.h`
- [ ] `SpliceMultiTrack()` signature in `/Users/acoliver/projects/uqm/sc2/src/libs/sound/trackplayer.h`
- [ ] `PlayTrack()`, `StopTrack()`, `JumpTrack()` in `/Users/acoliver/projects/uqm/sc2/src/libs/sound/trackplayer.h`
- [ ] `GetTrackSubtitle()` in `/Users/acoliver/projects/uqm/sc2/src/libs/sound/trackplayer.h`
- [ ] `GetFirstTrackSubtitle()`, `GetNextTrackSubtitle()`, `GetTrackSubtitleText()` in `/Users/acoliver/projects/uqm/sc2/src/libs/sound/trackplayer.h`
- [ ] `PollPendingTrackCompletion()` / `CommitTrackAdvancement()` availability and semantics verified against the audio-heart seam
- [ ] `PlayingTrack()` return type and semantics
- [ ] Determine: is trackplayer already ported to Rust or still C-owned?

## Call-Path Feasibility

### Init → Race dispatch → LOCDATA
- [ ] `init_race(CONVERSATION)` → `init_*_comm()` → returns `LOCDATA*`
- [ ] Verify `LOCDATA*` is static storage duration per race
- [ ] Verify all 27 `init_*_comm()` functions are declared in `commglue.h`
- [ ] Verify a thin C wrapper `c_init_race(comm_id) -> LOCDATA*` is feasible without moving the switch into Rust

### Public Entry Points
- [ ] Trace current `RaceCommunication()` caller chain and identify all game-state reads it performs
- [ ] Trace current `RaceCommunication()` → `InitCommunication()` handoff and any last-battle / hyperspace / interplanetary special cases
- [ ] Verify Rust can own `RaceCommunication()` in `USE_RUST_COMM` mode while preserving C fallback routing
- [ ] Identify exact SIS update call(s) used when a save was just loaded

### Script → Comm API → Rust FFI
- [ ] Trace `NPCPhrase(index)` → `NPCPhrase_cb(index, NULL)` → needs Rust implementation
- [ ] Trace `Response(i, a)` → `DoResponsePhrase(i, a, 0)` → needs Rust implementation
- [ ] Trace `PHRASE_ENABLED(p)` → accesses `CommData.ConversationPhrases` → needs Rust support
- [ ] Trace `DISABLE_PHRASE(p)` → mutates string table → needs Rust alternative

### Rust → C Callback Invocation
- [ ] Verify Rust can call `extern "C" fn(u32)` safely (response callbacks)
- [ ] Verify Rust can call `extern "C" fn()` safely (phrase callbacks)
- [ ] Verify lock release-and-reacquire pattern is feasible with `parking_lot::RwLock`

### Build System
- [ ] Verify Rust staticlib links into UQM binary
- [ ] Verify C compilation of comm module with `USE_RUST_COMM` defined
- [ ] Check build system for comm.c/commglue.c/commanim.c compilation rules

## Existing-Behavior Verification Obligations

The following requirements are currently expected to remain implemented by existing code paths, but they still require proof before signoff:

- [ ] RS-REQ-001: response registration stores ref/text/callback correctly in current Rust response system
- [ ] RS-REQ-003: explicit-text responses bypass phrase lookup in current Rust response system
- [ ] RS-REQ-004: max-eight-response cap exists and is enforced
- [ ] RS-REQ-005: overflow response registrations are rejected/ignored without corruption
- [ ] SS-REQ-009: subtitle-disable configuration path exists and suppresses subtitle rendering only
- [ ] AO-REQ-013: oscilloscope logic preserves required visualization continuity behavior in existing code
- [ ] OL-REQ-006: pre-init operations return errors instead of panicking
- [ ] OL-REQ-007: double-init returns error instead of corrupting state
- [ ] OL-REQ-008: uninit clears owned state sufficiently for reuse

For each verified existing requirement, record the file/function path and the specific test or inspection evidence in P01a.

## Required Phrase-State Compatibility Audit (PS-REQ-007)

Before implementation, audit all 27 race scripts and explicitly record whether each script satisfies the narrowed phrase-disable invariants:

- [ ] No script calls `NPCPhrase` on a phrase it has previously disabled
- [ ] No script resolves response text from a disabled phrase index
- [ ] No script or helper depends on observing NUL-prefixed disabled phrase text

Audit artifact requirements:
- [ ] Produce a 27-script checklist or matrix with one row per script
- [ ] For each script, record verification method (static inspection and/or targeted grep evidence)
- [ ] If any violating path is found, document the exact script/function and add a compatibility branch requirement for preserving legacy NUL-mutation on that path
- [ ] Mark P00.5 FAIL if the audit is not complete

## Test Infrastructure Verification

- [ ] `cargo test --workspace --all-features` passes currently
- [ ] Existing comm tests in `/Users/acoliver/projects/uqm/rust/src/comm/` all pass
- [ ] `#[serial]` attribute available for FFI tests requiring global state

## Blocking Issues

- If trackplayer is not accessible via FFI from Rust, a trackplayer FFI bridge must be created first (this would be a dependency/prerequisite, not part of this plan).
- If `LOCDATA` binary layout is required by other subsystems (not just comm scripts), the FFI accessor approach may need adjustment.
- If any script fails the PS-REQ-007 audit, either the compatibility path must be added to this plan before implementation or the script must be corrected.
- If `RaceCommunication()` depends on wider encounter-flow logic that cannot be cleanly wrapped, that boundary must be documented before Phase 8 starts.

## Gate Decision

- [ ] PASS: proceed to Phase 1
- [ ] FAIL: revise plan (document blocking issues)
