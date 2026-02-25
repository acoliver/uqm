# Phase 13a: Music + SFX TDD Verification

## Phase ID
`PLAN-20260225-AUDIO-HEART.P13a`

## Prerequisites
- Required: Phase P13 completed
- Expected: music.rs and sfx.rs test modules with 20+ tests each

## Verification Commands

```bash
cd /Users/acoliver/projects/uqm/rust && cargo test --lib --all-features -- sound::music::tests 2>&1
cd /Users/acoliver/projects/uqm/rust && cargo test --lib --all-features -- sound::sfx::tests 2>&1
cd /Users/acoliver/projects/uqm/rust && cargo fmt --all --check
cd /Users/acoliver/projects/uqm/rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
cd /Users/acoliver/projects/uqm/sc2 && ./build.sh uqm
```

## Structural Verification Checklist
- [ ] Test modules exist in both `music.rs` and `sfx.rs`
- [ ] music.rs has 12+ test functions
- [ ] sfx.rs has 15+ test functions
- [ ] All tests compile
- [ ] `cargo fmt` passes
- [ ] `cargo clippy` passes

## Semantic Verification Checklist

### Deterministic checks
- [ ] Music test count >= 12: `cargo test --lib --all-features -- sound::music::tests 2>&1 | grep "test result"`
- [ ] SFX test count >= 15: `cargo test --lib --all-features -- sound::sfx::tests 2>&1 | grep "test result"`
- [ ] Music tests include: `test_plr_play_song_invalid_ref`, `test_plr_stop_clears_ref`, `test_fade_music_zero_volume`, `test_get_music_data_creates_sample`, `test_release_music_stops_active`
- [ ] SFX tests include: `test_play_channel_missing_sample`, `test_stop_channel_clears`, `test_update_sound_position_positional`, `test_update_sound_position_non_positional`, `test_get_sound_bank_data_parses`, `test_release_sound_bank_data_cleans`

### Subjective checks
- [ ] Music fade tests verify smooth volume transition — does `test_fade_music_zero_volume` test that FadeMusic(0, duration) schedules a fade to silence?
- [ ] Music load/release tests verify full lifecycle — do they test get_music_data → use → release_music_data without leaks?
- [ ] SFX positional tests verify coordinate math — do they check that x/160.0, y/160.0 produces correct position? Is non-positional (0,0,-1) tested?
- [ ] SFX bank loading tests verify parsing — do they test multi-line sound lists with up to 256 entries?
- [ ] Error path tests use correct AudioError variants — InvalidSample for bad ref, NullPointer for null, ResourceNotFound for missing files
- [ ] No trivially-passing tests

## Deferred Implementation Detection
N/A — TDD phase, stubs still have `todo!()`.

## Success Criteria
- [ ] 27+ tests written and compiling across both modules
- [ ] Tests are meaningful behavioral assertions
- [ ] All requirement areas covered: music play/stop/fade/speech/load/release, sfx play/stop/position/bank

## Failure Recovery
- rollback: `git checkout -- rust/src/sound/music.rs rust/src/sound/sfx.rs`
- blocking issues: If stream.rs mock needed for music tests, create in test module

## Phase Completion Marker
Create: `project-plans/audiorust/heart/.completed/P13a.md`
