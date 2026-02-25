# Phase 14: Music + SFX Implementation

## Phase ID
`PLAN-20260225-AUDIO-HEART.P14`

## Prerequisites
- Required: Phase P13a (Music + SFX TDD Verification) passed
- Expected: 25+ tests across music.rs and sfx.rs

## Requirements Implemented (Expanded)

All MUSIC-* (21) and SFX-* (26) requirements fully implemented.

### Pseudocode traceability
- `plr_play_song`: pseudocode `music.md` lines 1-25
- `plr_stop`: pseudocode `music.md` lines 30-43
- `plr_playing`: pseudocode `music.md` lines 50-63
- `snd_play_speech`/`snd_stop_speech`: pseudocode `music.md` lines 90-114
- `get_music_data`: pseudocode `music.md` lines 120-143
- `release_music_data`: pseudocode `music.md` lines 150-170
- `set_music_volume`: pseudocode `music.md` lines 190-196
- `fade_music`: pseudocode `music.md` lines 200-212
- `play_channel`: pseudocode `sfx.md` lines 1-35
- `update_sound_position`: pseudocode `sfx.md` lines 80-105
- `get_sound_bank_data`: pseudocode `sfx.md` lines 120-172
- `release_sound_bank_data`: pseudocode `sfx.md` lines 180-200

## Implementation Tasks

### Files to modify
- `rust/src/sound/music.rs` — Replace all `todo!()` with implementations
  - marker: `@plan PLAN-20260225-AUDIO-HEART.P14`
  - marker: `@requirement REQ-MUSIC-*`
- `rust/src/sound/sfx.rs` — Replace all `todo!()` with implementations
  - marker: `@plan PLAN-20260225-AUDIO-HEART.P14`
  - marker: `@requirement REQ-SFX-*`

### music.rs implementation order
1. `set_music_volume` — simplest, state update + mixer call
2. `plr_play_song`/`plr_stop`/`plr_playing` — delegate to stream
3. `plr_seek`/`plr_pause`/`plr_resume` — delegate to stream
4. `snd_play_speech`/`snd_stop_speech` — delegate to stream on SPEECH_SOURCE
5. `get_music_data` — decoder loading + sample creation
6. `release_music_data` — cleanup with active-check
7. `check_music_res_name` — validation
8. `fade_music` — delegates to set_music_stream_fade

### sfx.rs implementation order
1. `update_sound_position` — 3D position math
2. `get_positional_object`/`set_positional_object` — accessor pair
3. `play_channel` — stop + setup + play
4. `stop_channel`/`channel_playing` — delegates
5. `set_channel_volume` — gain computation
6. `check_finished_channels` — polling loop
7. `get_sound_bank_data` — file parsing + decoder loading + buffer upload
8. `release_sound_bank_data` — cleanup with source-check

### Key implementation notes
- `MusicRef` is a raw pointer — all access requires `unsafe`
- Positional audio: `ATTENUATION = 160.0`, `MIN_DISTANCE = 0.5`
- `mixer_source_fv` needed for 3D position (verify it exists from P03)
- SFX bank loading: parse lines, each line is a WAV filename

## Verification Commands

```bash
cd /Users/acoliver/projects/uqm/rust && cargo test --lib --all-features -- sound::music::tests
cd /Users/acoliver/projects/uqm/rust && cargo test --lib --all-features -- sound::sfx::tests
cd /Users/acoliver/projects/uqm/rust && cargo test --lib --all-features
cd /Users/acoliver/projects/uqm/rust && cargo fmt --all --check
cd /Users/acoliver/projects/uqm/rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
```

## Structural Verification Checklist
- [ ] All `todo!()` removed from both files
- [ ] All tests pass
- [ ] fmt and clippy pass

## Semantic Verification Checklist (Mandatory)
- [ ] Music playback delegates to stream correctly
- [ ] Speech uses SPEECH_SOURCE, music uses MUSIC_SOURCE
- [ ] Positional audio math correct (distance, attenuation, min distance)
- [ ] SFX bank loading parses file list correctly
- [ ] Volume scaling applied with volume_scale factors
- [ ] Fade delegates to stream fade

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented\|todo!()" rust/src/sound/music.rs rust/src/sound/sfx.rs
# Must return 0 results
```

## Success Criteria
- [ ] All 25+ tests pass
- [ ] Zero deferred implementations
- [ ] Music and SFX fully operational (unit-level)

## Failure Recovery
- rollback: `git stash` or `git checkout -- rust/src/sound/music.rs rust/src/sound/sfx.rs`

## Phase Completion Marker
Create: `project-plans/audiorust/heart/.completed/P14.md`
