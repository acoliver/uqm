# Phase 11: Diagnostic Cleanup

## Phase ID
`PLAN-20260314-AUDIO-HEART.P11`

## Prerequisites
- Required: Phase P10a completed
- Expected: All functional gaps closed (loaders, multi-track, PLRPause, pending-completion, control hardening)

## Status Note on Quality Gates

Until P12 removes blanket module-level warning suppressions, the `clippy -D warnings` gates used in earlier phases are **provisional** rather than the first fully meaningful warning-cleanliness proof. P11 should avoid introducing new warning debt, but P12 is the first phase that makes the full warning gate authoritative.

## Requirements Implemented (Expanded)

### Diagnostic scaffolding removal
**Requirement text**: All `[PARITY]`-prefixed diagnostic output and development-only debug logging must be removed or converted to conditional logging behind a debug/trace configuration flag. The current diagnostic output in stream seek, track seek, subtitle logging, mixer pump diagnostics, and splice debug output is acceptable during development but not in the final subsystem.

Behavior contract:
- GIVEN: `eprintln!("[PARITY] ...")` calls exist in sound modules
- WHEN: Cleanup is applied
- THEN: All such calls are either removed or converted to `log::trace!()` / `log::debug!()`

- GIVEN: Operational logging (mixer pump startup, error paths)
- WHEN: Cleanup is applied
- THEN: Converted to appropriate log levels (warn/debug)

Why it matters:
- Production binaries should not emit debug output to stderr
- [PARITY] markers indicate development scaffolding, not production diagnostics
- log crate integration allows conditional output without compile-time overhead

## Implementation Tasks

### Files to modify

#### `rust/src/sound/stream.rs`
- Convert `eprintln!("[PARITY] ...")` in seek path to `log::trace!`
- Convert mixer pump startup/shutdown diagnostics to `log::debug!`
- Convert decoder thread error logging to `log::warn!`
- marker: `@plan PLAN-20260314-AUDIO-HEART.P11`

#### `rust/src/sound/trackplayer.rs`
- Convert `eprintln!("[PARITY] ...")` in seek path to `log::trace!`
- Convert splice debug output to `log::debug!`
- marker: `@plan PLAN-20260314-AUDIO-HEART.P11`

#### `rust/src/sound/heart_ffi.rs`
- Convert all `eprintln!` calls to appropriate log levels:
  - `[PARITY]` prefixed → `log::trace!`
  - Error conditions → `log::warn!`
  - Function entry/exit diagnostics → `log::debug!` or remove entirely
  - `SpliceTrack`/`SpliceMultiTrack` debug → `log::debug!`
  - `LoadSoundFile`/`LoadMusicFile` diagnostics → `log::debug!`
- marker: `@plan PLAN-20260314-AUDIO-HEART.P11`

#### `rust/src/sound/music.rs`
- Convert any `eprintln!` calls to `log::debug!` or `log::warn!`
- marker: `@plan PLAN-20260314-AUDIO-HEART.P11`

#### `rust/src/sound/sfx.rs`
- Convert any `eprintln!` calls to `log::debug!` or `log::warn!`
- marker: `@plan PLAN-20260314-AUDIO-HEART.P11`

#### `rust/src/sound/control.rs`
- Convert any `eprintln!` calls to `log::debug!` or `log::warn!`
- marker: `@plan PLAN-20260314-AUDIO-HEART.P11`

### Pseudocode traceability
- Implements PC-10 lines 01-08

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings 2>&1 | head -50
cargo test --workspace --all-features

# Verify no eprintln! remaining in audio-heart modules
grep -rn 'eprintln!' rust/src/sound/stream.rs rust/src/sound/trackplayer.rs rust/src/sound/heart_ffi.rs rust/src/sound/music.rs rust/src/sound/sfx.rs rust/src/sound/control.rs rust/src/sound/fileinst.rs rust/src/sound/types.rs

# Verify no [PARITY] remaining
grep -rn '\[PARITY\]' rust/src/sound/
```

## Structural Verification Checklist
- [ ] Zero `eprintln!` calls in audio-heart Rust modules
- [ ] Zero `[PARITY]` strings in audio-heart Rust modules
- [ ] All converted to `log::trace!`, `log::debug!`, or `log::warn!` as appropriate
- [ ] `log` crate is already a dependency (verify in Cargo.toml)

## Semantic Verification Checklist
- [ ] No visible stderr output during normal operation
- [ ] Error conditions still produce diagnostic output (via `log::warn!`)
- [ ] Tests still pass (no behavioral change)
- [ ] Log levels are appropriate (trace for parity markers, debug for operational, warn for errors)
- [ ] The phase does not overclaim full warning cleanliness before P12 removes blanket suppressions

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "eprintln\|PARITY" rust/src/sound/{stream,trackplayer,heart_ffi,music,sfx,control,fileinst,types}.rs
```

## Success Criteria
- [ ] Zero eprintln! in audio-heart modules
- [ ] Zero [PARITY] markers
- [ ] All tests pass
- [ ] log crate properly used

## Failure Recovery
- rollback: `git restore rust/src/sound/{stream,trackplayer,heart_ffi,music,sfx,control}.rs`

## Phase Completion Marker
Create: `project-plans/20260311/audio-heart/.completed/P11.md`
