# Phase 20a: FFI Implementation Verification

## Phase ID
`PLAN-20260225-AUDIO-HEART.P20a`

## Prerequisites
- Required: Phase P20 completed
- Expected: heart_ffi.rs fully implemented, all tests passing

## Verification Commands

```bash
cd /Users/acoliver/projects/uqm/rust && cargo test --lib --all-features -- sound::heart_ffi::tests
cd /Users/acoliver/projects/uqm/rust && cargo test --lib --all-features
cd /Users/acoliver/projects/uqm/rust && cargo fmt --all --check
cd /Users/acoliver/projects/uqm/rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
cd /Users/acoliver/projects/uqm/sc2 && ./build.sh uqm
# Deferred impl detection
grep -RIn "TODO\|FIXME\|HACK\|todo!()" rust/src/sound/heart_ffi.rs
# Verify symbols in static library
nm rust/target/release/libuqm_rust.a 2>/dev/null | grep -c "PLRPlaySong\|PlayChannel\|SpliceTrack"
```

## Structural Verification Checklist
- [ ] All `todo!()` removed from heart_ffi.rs (non-test code)
- [ ] All tests pass
- [ ] `cargo fmt` passes
- [ ] `cargo clippy` passes
- [ ] `build.sh uqm` succeeds

## Semantic Verification Checklist

### Deterministic checks
- [ ] All FFI tests pass: `cargo test --lib --all-features -- sound::heart_ffi::tests` shows 0 failures
- [ ] All workspace tests pass: `cargo test --lib --all-features` shows 0 failures
- [ ] Zero deferred markers: `grep -c "TODO\|FIXME\|HACK\|todo!()" rust/src/sound/heart_ffi.rs` returns 0 (excluding test module)
- [ ] All 60+ symbols present in static library (if built)
- [ ] Every unsafe block has a `// SAFETY:` comment: `grep -c "// SAFETY:" rust/src/sound/heart_ffi.rs` >= 10

### Subjective checks
- [ ] All FFI functions delegate to the correct Rust module function — does `PLRPlaySong` call `music::plr_play_song`? Does `PlayChannel` call `sfx::play_channel`?
- [ ] PlayChannel correctly resolves opaque SOUND handle — does it cast `*mut c_void` to `*mut SoundBank`, null-check, bounds-check the index, and pass the resolved sample to `play_channel`? (Technical Review Issue #6)
- [ ] Error code translation is consistent — does every function that returns `c_int` use the same error→code mapping? (REQ-CROSS-GENERAL-08: bool→1/0, count→0, pointer→null)
- [ ] SpliceTrack handles UTF-16→UTF-8 conversion safely — does it handle null terminator, invalid UTF-16, and empty strings?
- [ ] CCallbackWrapper correctly wraps C function pointers — does it store the raw pointer and invoke it via `unsafe`?
- [ ] Null pointer checks present on ALL pointer parameters — no path where a NULL pointer causes a Rust panic
- [ ] `Box::from_raw` for MusicRef/SoundBank is correctly paired with `Box::into_raw` — no double-free or use-after-free possible
- [ ] No `unwrap()` or `expect()` in production code paths — all error paths return C-compatible error codes

## Deferred Implementation Detection

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented\|todo!()" rust/src/sound/heart_ffi.rs
# Must return 0 results
```

## Success Criteria
- [ ] All 17+ tests pass
- [ ] Zero deferred implementations
- [ ] All 60+ FFI functions implemented
- [ ] PlayChannel handle resolution documented and implemented
- [ ] C build succeeds and links

## Failure Recovery
- rollback: `git stash` or `git checkout -- rust/src/sound/heart_ffi.rs`
- blocking issues: If C header signatures don't match Rust stubs, reconcile signatures first

## Phase Completion Marker
Create: `project-plans/audiorust/heart/.completed/P20a.md`
