# Phase 17: Control + FileInst Implementation

## Phase ID
`PLAN-20260225-AUDIO-HEART.P17`

## Prerequisites
- Required: Phase P16a (Control + FileInst TDD Verification) passed
- Expected: 15+ tests across control.rs and fileinst.rs

## Requirements Implemented (Expanded)

All VOLUME-* (17) and FILEINST-* (7) requirements fully implemented.

### Pseudocode traceability
- `SoundSourceArray::new`: pseudocode `control.md` lines 1-27
- `VolumeState::new`: pseudocode `control.md` lines 30-36
- `init_sound/uninit_sound`: pseudocode `control.md` lines 40-47
- `stop_source`: pseudocode `control.md` lines 50-62
- `clean_source`: pseudocode `control.md` lines 70-90
- `stop_sound`: pseudocode `control.md` lines 100-104
- `set_sfx_volume`: pseudocode `control.md` lines 110-115
- `set_speech_volume`: pseudocode `control.md` lines 120-123
- `sound_playing`: pseudocode `control.md` lines 130-148
- `wait_for_sound_end`: pseudocode `control.md` lines 150-164
- `load_sound_file`: pseudocode `fileinst.md` lines 20-36
- `load_music_file`: pseudocode `fileinst.md` lines 40-56
- `destroy_sound/destroy_music`: pseudocode `fileinst.md` lines 60-66

## Implementation Tasks

### Files to modify
- `rust/src/sound/control.rs` — Replace all `todo!()`
  - marker: `@plan PLAN-20260225-AUDIO-HEART.P17`
  - marker: `@requirement REQ-VOLUME-*`
- `rust/src/sound/fileinst.rs` — Replace all `todo!()`
  - marker: `@plan PLAN-20260225-AUDIO-HEART.P17`
  - marker: `@requirement REQ-FILEINST-*`

### control.rs implementation order
1. `SoundSourceArray::new` — Allocate mixer sources
2. `VolumeState::new` — Set defaults
3. `init_sound`/`uninit_sound` — Lifecycle (mostly no-op for now)
4. `stop_source` — Stop mixer + clean
5. `clean_source` — Reset positional, unqueue buffers, rewind
6. `stop_sound` — Stop all SFX
7. `set_sfx_volume`/`set_speech_volume` — Gain application
8. `sound_playing` — Poll all sources
9. `wait_for_sound_end` — Blocking poll loop with quit check

### fileinst.rs implementation order
1. `FileLoadGuard` — RAII guard with Drop
2. `load_sound_file` — Guard + read + delegate
3. `load_music_file` — Guard + validate + delegate
4. `destroy_sound`/`destroy_music` — Pure delegation

### Key implementation notes
- `SoundSourceArray::new` is called in `lazy_static!` initialization — must not panic
- `wait_for_sound_end` must check `quit_posted()` via FFI
- `FileLoadGuard::drop` must be infallible

## Verification Commands

```bash
cd /Users/acoliver/projects/uqm/rust && cargo test --lib --all-features -- sound::control::tests
cd /Users/acoliver/projects/uqm/rust && cargo test --lib --all-features -- sound::fileinst::tests
cd /Users/acoliver/projects/uqm/rust && cargo test --lib --all-features
cd /Users/acoliver/projects/uqm/rust && cargo fmt --all --check
cd /Users/acoliver/projects/uqm/rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
```

## Structural Verification Checklist
- [ ] All `todo!()` removed from both files
- [ ] All tests pass
- [ ] fmt and clippy pass

## Semantic Verification Checklist (Mandatory)
- [ ] SOURCES initialized with valid mixer handles
- [ ] stop_source stops mixer and cleans
- [ ] Volume scaling applied correctly
- [ ] Concurrent load guard prevents re-entry
- [ ] Guard drop always clears resfile_name (even on error paths)

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented\|todo!()" rust/src/sound/control.rs rust/src/sound/fileinst.rs
# Must return 0 results
```

## Success Criteria
- [ ] All 15+ tests pass
- [ ] Zero deferred implementations

## Failure Recovery
- rollback: `git stash`

## Phase Completion Marker
Create: `project-plans/audiorust/heart/.completed/P17.md`
