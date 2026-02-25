# Phase 21a: Integration Verification

## Phase ID
`PLAN-20260225-AUDIO-HEART.P21a`

## Prerequisites
- Required: Phase P21 completed
- Expected: Complete C+Rust integration with conditional build flag

## Verification Commands

```bash
# Full Rust test suite
cd /Users/acoliver/projects/uqm/rust && cargo test --lib --all-features
cd /Users/acoliver/projects/uqm/rust && cargo fmt --all --check
cd /Users/acoliver/projects/uqm/rust && cargo clippy --workspace --all-targets --all-features -- -D warnings

# Build without flag (regression)
cd /Users/acoliver/projects/uqm/sc2 && ./build.sh uqm

# Build with flag
cd /Users/acoliver/projects/uqm/sc2 && USE_RUST_AUDIO_HEART=1 ./build.sh uqm

# Deferred impl check across all modules
grep -RIn "TODO\|FIXME\|HACK\|todo!()" \
  rust/src/sound/types.rs rust/src/sound/stream.rs rust/src/sound/trackplayer.rs \
  rust/src/sound/music.rs rust/src/sound/sfx.rs rust/src/sound/control.rs \
  rust/src/sound/fileinst.rs rust/src/sound/heart_ffi.rs

# Symbol count in static library
cd /Users/acoliver/projects/uqm/rust && cargo build --lib --all-features
nm target/debug/libuqm_rust.a | grep " T " | wc -l
```

## Checks

### Build Checks
- [ ] `cargo test --lib --all-features` — all pass (130+ tests)
- [ ] `cargo fmt` — passes
- [ ] `cargo clippy` — passes
- [ ] `./build.sh uqm` without flag — succeeds (C audio)
- [ ] `./build.sh uqm` with flag — succeeds (Rust audio)
- [ ] Zero deferred implementation markers

### Manual Verification Checks
- [ ] Title screen music plays (Rust path)
- [ ] Menu navigation SFX works
- [ ] Combat weapon sounds fire
- [ ] Communication screen speech plays with subtitles
- [ ] Oscilloscope waveform renders
- [ ] Volume controls work (music/SFX/speech)
- [ ] Music fade transitions work
- [ ] Speech seeking works
- [ ] No regression when flag is off

### Cross-Module Integration Checks
- [ ] stream.rs correctly reads from SOURCES (control.rs)
- [ ] trackplayer.rs correctly uses play_stream (stream.rs)
- [ ] music.rs correctly delegates to stream functions
- [ ] sfx.rs correctly uses stop_source/clean_source (control.rs)
- [ ] fileinst.rs correctly delegates to music/sfx loading
- [ ] heart_ffi.rs correctly wraps all module APIs

### Test Coverage Summary
- [ ] types.rs: 13+ tests (constants, errors, conversions)
- [ ] stream.rs: 29+ tests (sample, tags, fade, scope, playback, thread)
- [ ] trackplayer.rs: 25+ tests (splitting, assembly, playback, seeking, subtitles)
- [ ] music.rs: 12+ tests (playback, speech, loading, volume)
- [ ] sfx.rs: 13+ tests (playback, positional, loading)
- [ ] control.rs: 9+ tests (init, sources, volume, queries)
- [ ] fileinst.rs: 6+ tests (guard, loading, destroy)
- [ ] heart_ffi.rs: 17+ tests (null safety, errors, strings, callbacks)
- [ ] **Total: 124+ tests**

## Final Gate Decision
- [ ] PASS: Audio Heart Rust port complete, production-ready behind feature flag
- [ ] FAIL: Document issues, create follow-up plan

## Plan Completion
When all checks pass, the plan `PLAN-20260225-AUDIO-HEART` is complete.

Create final completion marker: `project-plans/audiorust/heart/.completed/PLAN-COMPLETE.md`

Contents:
- Plan ID: PLAN-20260225-AUDIO-HEART
- Completion timestamp
- Total phases executed: 22 (P00a through P21a)
- Total tests: 124+
- Total requirements satisfied: 234
- Files created: 8 Rust modules + 1 C header
- Files modified: mod.rs, config_unix.h, 6 sound headers, build system
- Status: COMPLETE
