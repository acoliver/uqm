# Phase 13: Music + SFX TDD

## Phase ID
`PLAN-20260225-AUDIO-HEART.P13`

## Prerequisites
- Required: Phase P12a (Music + SFX Stub Verification) passed
- Expected: Both modules compiling with stubs

## Requirements Implemented (Expanded)

All MUSIC-* requirements (21) and SFX-* requirements (26).

## Implementation Tasks

### Files to modify
- `rust/src/sound/music.rs` — Add test module
  - marker: `@plan PLAN-20260225-AUDIO-HEART.P13`
- `rust/src/sound/sfx.rs` — Add test module
  - marker: `@plan PLAN-20260225-AUDIO-HEART.P13`

### Music tests

**Playback (REQ-MUSIC-PLAY-01 through REQ-MUSIC-PLAY-08)**
1. `test_plr_play_song_null_ref_error` — Returns InvalidSample for null MusicRef
2. `test_plr_stop_no_match_noop` — Mismatched ref is no-op
3. `test_plr_playing_false_when_none` — Returns false when no current ref
4. `test_plr_pause_resume_delegates` — Delegates to stream pause/resume

**Speech (REQ-MUSIC-SPEECH-01, REQ-MUSIC-SPEECH-02)**
5. `test_snd_play_speech_uses_speech_source` — Plays on SPEECH_SOURCE
6. `test_snd_stop_speech_noop_when_none` — Returns Ok when no speech

**Loading (REQ-MUSIC-LOAD-01 through REQ-MUSIC-LOAD-06)**
7. `test_get_music_data_empty_filename_error` — Returns NullPointer
8. `test_check_music_res_name_returns_some` — Returns Some(filename)

**Release (REQ-MUSIC-RELEASE-01, REQ-MUSIC-RELEASE-02, REQ-MUSIC-RELEASE-03)**
9. `test_release_music_data_null_error` — Returns NullPointer for null ref

**Volume (REQ-MUSIC-VOLUME-01)**
10. `test_set_music_volume_updates_state` — Volume stored correctly

**Fade (REQ-VOLUME-CONTROL-03, REQ-VOLUME-CONTROL-04, REQ-VOLUME-CONTROL-05)**
11. `test_fade_music_zero_interval` — Immediate volume set
12. `test_fade_music_returns_completion_time` — Time = now + interval + 1

### SFX tests

**Playback (REQ-SFX-PLAY-01 through REQ-SFX-PLAY-09)**
13. `test_play_channel_invalid_channel_error` — Returns InvalidChannel
14. `test_play_channel_missing_sample_error` — Returns InvalidSample
15. `test_stop_channel_delegates` — Calls stop_source
16. `test_channel_playing_initial_false` — Returns false initially
17. `test_check_finished_channels_cleans` — Stopped sources cleaned

**Positional (REQ-SFX-POSITION-01 through REQ-SFX-POSITION-05)**
18. `test_update_sound_position_non_positional` — Sets (0, 0, -1)
19. `test_update_sound_position_positional` — Computes 3D coordinates
20. `test_sound_position_min_distance` — Enforces MIN_DISTANCE
21. `test_get_set_positional_object` — Round-trip

**Loading (REQ-SFX-LOAD-01 through REQ-SFX-LOAD-07)**
22. `test_get_sound_bank_data_empty_lines` — Skips empty entries
23. `test_get_sound_bank_data_all_none_error` — Returns ResourceNotFound

**Release (REQ-SFX-RELEASE-01, REQ-SFX-RELEASE-02, REQ-SFX-RELEASE-03)**
24. `test_release_sound_bank_data_empty_ok` — Empty bank is no-op

**Volume (REQ-SFX-VOLUME-01)**
25. `test_set_channel_volume_applies_gain` — Gain computed correctly

## Verification Commands

```bash
cd /Users/acoliver/projects/uqm/rust && cargo test --lib --all-features -- sound::music::tests
cd /Users/acoliver/projects/uqm/rust && cargo test --lib --all-features -- sound::sfx::tests
cd /Users/acoliver/projects/uqm/rust && cargo fmt --all --check
cd /Users/acoliver/projects/uqm/rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
```

## Structural Verification Checklist
- [ ] Test modules added to both files
- [ ] 25+ total tests
- [ ] Tests reference requirements

## Semantic Verification Checklist (Mandatory)
- [ ] Music tests cover: playback, speech, loading, release, volume, fade
- [ ] SFX tests cover: playback, positional, loading, release, volume
- [ ] Error path tests verify correct AudioError variants
- [ ] Positional audio math verified numerically

## Deferred Implementation Detection (Mandatory)
N/A — TDD phase

## Success Criteria
- [ ] 25+ tests written and compiling
- [ ] Tests are meaningful

## Failure Recovery
- rollback: `git checkout -- rust/src/sound/music.rs rust/src/sound/sfx.rs`

## Phase Completion Marker
Create: `project-plans/audiorust/heart/.completed/P13.md`
