# Phase 16a: Control + FileInst TDD Verification

## Phase ID
`PLAN-20260225-AUDIO-HEART.P16a`

## Prerequisites
- Required: Phase P16 completed
- Expected: control.rs and fileinst.rs test modules

## Verification Commands

```bash
cd /Users/acoliver/projects/uqm/rust && cargo test --lib --all-features -- sound::control::tests 2>&1
cd /Users/acoliver/projects/uqm/rust && cargo test --lib --all-features -- sound::fileinst::tests 2>&1
cd /Users/acoliver/projects/uqm/rust && cargo fmt --all --check
cd /Users/acoliver/projects/uqm/rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
cd /Users/acoliver/projects/uqm/sc2 && ./build.sh uqm
```

## Structural Verification Checklist
- [ ] Test modules exist in both `control.rs` and `fileinst.rs`
- [ ] control.rs has 12+ test functions
- [ ] fileinst.rs has 8+ test functions
- [ ] All tests compile
- [ ] `cargo fmt` passes
- [ ] `cargo clippy` passes

## Semantic Verification Checklist

### Deterministic checks
- [ ] Control test count >= 12: `cargo test --lib --all-features -- sound::control::tests 2>&1 | grep "test result"`
- [ ] FileInst test count >= 8: `cargo test --lib --all-features -- sound::fileinst::tests 2>&1 | grep "test result"`
- [ ] Control tests include: `test_volume_state_defaults`, `test_stop_source_stops_mixer`, `test_clean_source_resets_state`, `test_stop_sound_stops_all_sfx`, `test_sound_playing_any_active`, `test_set_sfx_volume_applies_gain`
- [ ] FileInst tests include: `test_concurrent_load_guard_rejects`, `test_file_load_guard_raii_cleanup`, `test_load_sound_file_success`, `test_load_music_file_success`

### Subjective checks
- [ ] Volume state tests verify default values — is `music_volume` defaulting to `NORMAL_VOLUME` (160)?
- [ ] Stop/clean source tests verify correct mixer calls — do they test that `mixer_source_stop` is called and source fields are cleared?
- [ ] Volume tests verify gain calculation — does `set_sfx_volume` compute `volume / MAX_VOLUME * sfx_volume_scale` correctly?
- [ ] FileInst guard tests verify RAII cleanup — does the guard clear `cur_resfile_name` even if the load function panics/errors?
- [ ] Concurrent load tests verify `ConcurrentLoad` error — does attempting to load while another load is in progress return `Err(AudioError::ConcurrentLoad)`?
- [ ] wait_for_sound_end tests verify poll loop behavior — does it break on quit_posted?

## Deferred Implementation Detection
N/A — TDD phase, stubs still have `todo!()`.

## Success Criteria
- [ ] 20+ tests written and compiling across both modules
- [ ] Tests are meaningful behavioral assertions
- [ ] All requirement areas covered: volume, sources, sound control, file loading, concurrency guard

## Failure Recovery
- rollback: `git checkout -- rust/src/sound/control.rs rust/src/sound/fileinst.rs`
- blocking issues: If mock needed for mixer or file I/O, create in test modules

## Phase Completion Marker
Create: `project-plans/audiorust/heart/.completed/P16a.md`
