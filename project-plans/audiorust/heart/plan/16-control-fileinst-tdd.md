# Phase 16: Control + FileInst TDD

## Phase ID
`PLAN-20260225-AUDIO-HEART.P16`

## Prerequisites
- Required: Phase P15a (Control + FileInst Stub Verification) passed
- Expected: Both modules compiling with stubs

## Requirements Implemented (Expanded)

All VOLUME-* requirements (17) and FILEINST-* requirements (7).

## Implementation Tasks

### Files to modify
- `rust/src/sound/control.rs` — Add test module
  - marker: `@plan PLAN-20260225-AUDIO-HEART.P16`
- `rust/src/sound/fileinst.rs` — Add test module
  - marker: `@plan PLAN-20260225-AUDIO-HEART.P16`

### Control tests

**Initialization (REQ-VOLUME-INIT-01..05)**
1. `test_volume_state_defaults` — NORMAL_VOLUME, 1.0 scales
2. `test_init_sound_ok` — Returns Ok

**Source Management (REQ-VOLUME-SOURCE-01..04)**
3. `test_stop_source_invalid_index_error` — Returns InvalidSource
4. `test_clean_source_invalid_index_error` — Returns InvalidSource
5. `test_stop_sound_all_sfx_channels` — Stops FIRST_SFX_SOURCE..LAST_SFX_SOURCE

**Volume (REQ-VOLUME-CONTROL-01..02)**
6. `test_set_sfx_volume_all_channels` — Applied to all SFX sources
7. `test_set_speech_volume_speech_source` — Applied to SPEECH_SOURCE only

**Queries (REQ-VOLUME-QUERY-01..03)**
8. `test_sound_playing_false_when_idle` — Returns false initially
9. `test_wait_for_sound_end_returns_when_not_playing` — Returns immediately

### FileInst tests

**Concurrent Load Guard (REQ-FILEINST-LOAD-01,07)**
10. `test_concurrent_load_rejected` — Returns ConcurrentLoad
11. `test_load_guard_cleanup_on_drop` — Guard drop clears resfile_name

**Sound File Loading (REQ-FILEINST-LOAD-02..03)**
12. `test_load_sound_file_delegates` — Calls get_sound_bank_data

**Music File Loading (REQ-FILEINST-LOAD-04..06)**
13. `test_load_music_file_delegates` — Calls get_music_data

**Destroy Delegates (REQ-SFX-RELEASE-04, REQ-MUSIC-RELEASE-04)**
14. `test_destroy_sound_delegates` — Calls release_sound_bank_data
15. `test_destroy_music_delegates` — Calls release_music_data

## Verification Commands

```bash
cd /Users/acoliver/projects/uqm/rust && cargo test --lib --all-features -- sound::control::tests
cd /Users/acoliver/projects/uqm/rust && cargo test --lib --all-features -- sound::fileinst::tests
cd /Users/acoliver/projects/uqm/rust && cargo fmt --all --check
cd /Users/acoliver/projects/uqm/rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
```

## Structural Verification Checklist
- [ ] Test modules added to both files
- [ ] 15+ total tests
- [ ] Tests reference requirements

## Semantic Verification Checklist (Mandatory)
- [ ] Initialization tests verify default values
- [ ] Source management tests verify correct error variants
- [ ] Volume tests verify gain computation
- [ ] Concurrent load tests verify RAII guard behavior
- [ ] Delegate tests verify correct function dispatch

## Deferred Implementation Detection (Mandatory)
N/A — TDD phase

## Success Criteria
- [ ] 15+ tests written and compiling

## Failure Recovery
- rollback: `git checkout -- rust/src/sound/control.rs rust/src/sound/fileinst.rs`

## Phase Completion Marker
Create: `project-plans/audiorust/heart/.completed/P16.md`
