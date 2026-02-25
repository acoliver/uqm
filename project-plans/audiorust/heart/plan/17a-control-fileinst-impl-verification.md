# Phase 17a: Control + FileInst Implementation Verification

## Phase ID
`PLAN-20260225-AUDIO-HEART.P17a`

## Prerequisites
- Required: Phase P17 completed
- Expected: control.rs and fileinst.rs fully implemented, all tests passing

## Verification Commands

```bash
cd /Users/acoliver/projects/uqm/rust && cargo test --lib --all-features -- sound::control::tests
cd /Users/acoliver/projects/uqm/rust && cargo test --lib --all-features -- sound::fileinst::tests
cd /Users/acoliver/projects/uqm/rust && cargo test --lib --all-features
cd /Users/acoliver/projects/uqm/rust && cargo fmt --all --check
cd /Users/acoliver/projects/uqm/rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
cd /Users/acoliver/projects/uqm/sc2 && ./build.sh uqm
# Deferred impl detection
grep -RIn "TODO\|FIXME\|HACK\|todo!()" rust/src/sound/control.rs rust/src/sound/fileinst.rs
```

## Structural Verification Checklist
- [ ] All `todo!()` removed from control.rs and fileinst.rs (non-test code)
- [ ] All tests pass
- [ ] `cargo fmt` passes
- [ ] `cargo clippy` passes
- [ ] `build.sh uqm` succeeds

## Semantic Verification Checklist

### Deterministic checks
- [ ] All control tests pass: `cargo test --lib --all-features -- sound::control::tests` shows 0 failures
- [ ] All fileinst tests pass: `cargo test --lib --all-features -- sound::fileinst::tests` shows 0 failures
- [ ] All workspace tests pass: `cargo test --lib --all-features` shows 0 failures
- [ ] Zero deferred markers: `grep -c "TODO\|FIXME\|HACK\|todo!()" rust/src/sound/control.rs rust/src/sound/fileinst.rs` returns 0 for both

### Subjective checks
- [ ] `init_sound` creates SoundSourceArray with NUM_SOUNDSOURCES mixer handles — does it call `mixer_gen_sources(NUM_SOUNDSOURCES)` and populate each entry?
- [ ] `stop_source` correctly stops mixer and resets source — does it call `mixer_source_stop`, unqueue all buffers, and clear the sample reference?
- [ ] `clean_source` resets all source fields to initial state — rewind, clear sample, clear scope, reset flags
- [ ] `stop_sound` stops ALL SFX sources (0..NUM_SFX_CHANNELS) — not speech or music
- [ ] Volume calculations produce correct gain — verify the formula matches the C implementation
- [ ] `wait_for_sound_end` polls with 10ms sleep (matching C TaskSwitch granularity) and breaks on `quit_posted()` — does it handle the quit case correctly?
- [ ] `FileLoadGuard` RAII pattern ensures cleanup even on error paths — is `Drop` implemented to clear `cur_resfile_name`?
- [ ] `load_sound_file` sets name, calls `get_sound_bank_data`, and clears name — is the guard used?
- [ ] `load_music_file` validates filename, sets name, calls `get_music_data`, and clears name — does it check for empty filename (REQ-MUSIC-LOAD-01)?
- [ ] No `unwrap()` or `expect()` in production code paths

## Deferred Implementation Detection

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented\|todo!()" rust/src/sound/control.rs rust/src/sound/fileinst.rs
# Must return 0 results
```

## Success Criteria
- [ ] All 20+ tests pass across both modules
- [ ] Zero deferred implementations
- [ ] Control and fileinst fully operational (unit-level)
- [ ] RAII guard pattern working correctly

## Failure Recovery
- rollback: `git stash` or `git checkout -- rust/src/sound/control.rs rust/src/sound/fileinst.rs`
- blocking issues: If mixer or file I/O APIs differ, adapt and document

## Phase Completion Marker
Create: `project-plans/audiorust/heart/.completed/P17a.md`
