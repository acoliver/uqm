# Phase 12: Warning Suppression Removal & C Residual Cleanup

## Phase ID
`PLAN-20260314-AUDIO-HEART.P12`

## Prerequisites
- Required: Phase P11a completed
- Expected: All functional gaps closed, diagnostics cleaned up

## Requirements Implemented (Expanded)

### Warning suppression removal
**Requirement text**: `#![allow(dead_code, unused_imports)]` on all modules removed; all code is used or explicitly cfg-gated.

Behavior contract:
- GIVEN: 7 modules have `#![allow(dead_code, unused_imports, unused_variables)]`
- WHEN: Attributes removed
- THEN: Code compiles cleanly with no dead_code warnings (items are either used or cfg-gated)

### C residual elimination
**Requirement text**: Residual C implementations whose functionality has been fully absorbed shall be eliminated, so that the replacement path is complete rather than partial.

Behavior contract:
- GIVEN: C code in `sound.c:26-69` (volume globals), `music.c:158-236` (resource helpers), `sfx.c:162-298` (bank helpers)
- WHEN: Rust canonical loaders are working
- THEN: C residual code is guarded out or replaced by Rust FFI exports

### Final high-risk contract closure
**Requirement text**: High-risk ABI-visible contracts must be both accounted for and verified explicitly, not only by smoke testing.

The final verification in this phase must explicitly cover:
- stable subtitle pointer identity semantics for comm polling
- subtitle iteration and null-handling contracts
- distinct handle identity across repeated loads of the same resource
- borrowed-handle identity preservation in play/control paths
- destroy-on-active-resource behavior and explicit destroy API semantics
- speech/track arbitration asymmetry on the shared speech source
- standalone speech stop behavior, including `snd_stop_speech`
- wildcard control semantics beyond PLRPause where applicable

Why it matters:
- Warning suppression hides real bugs (unused imports, dead code that should be connected)
- C residual code creates maintenance confusion and potential double-ownership
- High-risk ABI-visible contracts can be missed by a generic game-launch smoke test
- Clean compilation is a project standard

## Requirement Coverage Matrix Maintenance

Before P12 can be considered complete, the plan execution must update the requirement-coverage matrix introduced in P01 so every important in-scope contract is marked as one of:
- already satisfied / no code change required,
- implemented in a named phase, or
- verified in a named phase because analysis proved parity already exists.

P12 is the enforcement point that closes any remaining “verification-only with no implementation owner” holes. No important contract listed below may remain in an implicit or unowned state by the end of this phase.

## Implementation Tasks

### Warning Suppression Removal — Files to modify

#### `rust/src/sound/stream.rs`
- Remove module-level blanket warning suppression
- Fix any resulting warnings:
  - items called only from `heart_ffi.rs`: add `#[cfg(feature = "audio_heart")]` or narrowly scoped per-item allowances with justification
  - genuinely unused items: remove
  - unused imports: remove
  - unused variables: prefix with `_` or use
- marker: `@plan PLAN-20260314-AUDIO-HEART.P12`

#### `rust/src/sound/trackplayer.rs`
- Remove module-level blanket warning suppression
- Fix warnings as above
- marker: `@plan PLAN-20260314-AUDIO-HEART.P12`

#### `rust/src/sound/music.rs`
- Remove module-level blanket warning suppression
- Fix warnings as above
- marker: `@plan PLAN-20260314-AUDIO-HEART.P12`

#### `rust/src/sound/sfx.rs`
- Remove module-level blanket warning suppression
- Fix warnings as above
- marker: `@plan PLAN-20260314-AUDIO-HEART.P12`

#### `rust/src/sound/control.rs`
- Remove module-level blanket warning suppression
- Fix warnings as above
- marker: `@plan PLAN-20260314-AUDIO-HEART.P12`

#### `rust/src/sound/fileinst.rs`
- Remove module-level blanket warning suppression
- Fix warnings as above
- marker: `@plan PLAN-20260314-AUDIO-HEART.P12`

#### `rust/src/sound/heart_ffi.rs`
- Remove module-level blanket warning suppression
- Fix warnings as above
- marker: `@plan PLAN-20260314-AUDIO-HEART.P12`

#### `rust/src/sound/types.rs`
- Remove module-level blanket warning suppression
- Fix warnings as above
- marker: `@plan PLAN-20260314-AUDIO-HEART.P12`

### C Residual Cleanup — Files to modify

#### `sc2/src/libs/sound/sound.c`
- Extend `#ifndef USE_RUST_AUDIO_HEART` guard to cover volume globals and shared-state code (lines 26-69)
- OR: verify that Rust-side volume state fully replaces these globals
- If C callers outside audio-heart still reference these globals directly, add Rust FFI exports or retain the C code with documented justification
- marker: `@plan PLAN-20260314-AUDIO-HEART.P12`

#### `sc2/src/libs/sound/music.c`
- Extend `#ifndef USE_RUST_AUDIO_HEART` guard to cover resource helpers (lines 158-236)
- marker: `@plan PLAN-20260314-AUDIO-HEART.P12`

#### `sc2/src/libs/sound/sfx.c`
- Extend `#ifndef USE_RUST_AUDIO_HEART` guard to cover bank helpers (lines 162-298)
- marker: `@plan PLAN-20260314-AUDIO-HEART.P12`

**CAUTION**: C residual cleanup requires careful verification that no other C code (outside the guard) depends on these functions/globals. Before guarding:
1. `grep -rn 'musicVolume\|musicVolumeScale\|sfxVolumeScale\|speechVolumeScale\|soundSource\[' sc2/src/`
2. `grep -rn '_GetMusicData\|_ReleaseMusicData\|CheckMusicResName' sc2/src/`
3. `grep -rn '_GetSoundBankData\|_ReleaseSoundBankData' sc2/src/`
4. If callers exist outside the guard, redirect them to Rust FFI or retain the C code with explicit justification

### High-risk contract verification / closure tasks

Add targeted tests and/or scripted verification for:
1. **Subtitle pointer stability and iteration/null handling**
   - current subtitle pointer remains stable while subtitle text is unchanged
   - pointer identity changes when subtitle advances
   - pointer returns null after track stop/end
   - iteration over subtitle progression follows the required null/non-null transitions without stale-pointer reuse
2. **Handle identity and borrowed-handle preservation**
   - two loads of the same resource path return distinct handles
   - borrowed play/control paths preserve raw-handle identity semantics for comparisons
3. **Destroy semantics**
   - destroying an active music/sound handle stops or detaches active playback safely
   - explicit destroy API contracts match the required null/no-op and active-resource behavior
4. **Speech/track arbitration and speech stop behavior**
   - standalone speech is rejected while track owns the speech source
   - track playback can preempt standalone speech as specified
   - `snd_stop_speech` clears standalone speech state without disturbing active track playback
5. **Wildcard semantics**
   - wildcard handle behavior verified for applicable control/query APIs, not only `PLRPause`

If any of these areas were not fully implemented by earlier phases, P12 must close the remaining code gaps rather than merely record a failing checklist item.

### Pseudocode traceability
- Implements PC-11 lines 01-12

## Verification Commands

```bash
# Full clean build
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features

# Verify no blanket warning suppression in audio-heart modules
grep -rn '#!\[allow(dead_code' rust/src/sound/{stream,trackplayer,music,sfx,control,fileinst,heart_ffi,types}.rs

# Verify requirement coverage matrix has been updated with final ownership/verification status
grep -n 'requirement-coverage matrix\|Requirement Coverage Matrix' project-plans/20260311/audio-heart/plan/00-overview.md project-plans/20260311/audio-heart/plan/01-analysis.md project-plans/20260311/audio-heart/plan/12-warning-suppression-c-residual.md

# Verify C guards extended
grep -n 'USE_RUST_AUDIO_HEART' sc2/src/libs/sound/sound.c sc2/src/libs/sound/music.c sc2/src/libs/sound/sfx.c

# Build C side to verify no link errors
# (project-specific build command)
```

## Structural Verification Checklist
- [ ] No module-level `#![allow(dead_code, ...)]` in any audio-heart file
- [ ] All dead_code warnings resolved (items used, cfg-gated, or removed)
- [ ] All unused_imports removed
- [ ] C residual code guarded or replaced
- [ ] No link errors from C side
- [ ] Requirement-coverage matrix updated so all important in-scope contracts have an explicit owner/status
- [ ] End-state contract tests/checks added for subtitle pointers, subtitle iteration/null handling, handle identity, borrowed-handle identity, destroy semantics, arbitration, speech-stop behavior, and wildcard behavior

## Semantic Verification Checklist
- [ ] All tests pass
- [ ] clippy passes with `-D warnings`
- [ ] C build succeeds with `USE_RUST_AUDIO_HEART` enabled
- [ ] No behavioral changes from cleanup except intended end-state contract closure
- [ ] No C callers broken by guarding residual code
- [ ] Subtitle pointer identity semantics proven explicitly
- [ ] Subtitle iteration/null handling proven explicitly
- [ ] Repeated loads of the same resource return distinct handles
- [ ] Borrowed-handle identity semantics proven in play/control paths
- [ ] Destroy-on-active-resource behavior proven safe
- [ ] Explicit destroy API semantics match the contract
- [ ] Speech/track arbitration asymmetry proven
- [ ] `snd_stop_speech` behavior proven explicitly
- [ ] Wildcard behavior beyond PLRPause verified where applicable

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/sound/{stream,trackplayer,music,sfx,control,fileinst,heart_ffi,types,loading}.rs
```

## Success Criteria
- [ ] Clean compilation with no blanket warning suppression
- [ ] C residual code guarded
- [ ] High-risk ABI contracts are both owned and verified explicitly
- [ ] All tests and clippy pass
- [ ] Subsystem is complete per the final-state requirements

## Failure Recovery
- rollback: `git restore rust/src/sound/ sc2/src/libs/sound/{sound,music,sfx}.c`
- blocking issues:
  - C callers of guarded functions may need Rust FFI exports
  - Some items may need narrowly scoped per-item `#[allow(dead_code)]` if only used under cfg

## Phase Completion Marker
Create: `project-plans/20260311/audio-heart/.completed/P12.md`
