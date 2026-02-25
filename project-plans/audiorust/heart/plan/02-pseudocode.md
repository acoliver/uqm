# Phase 02: Pseudocode

## Phase ID
`PLAN-20260225-AUDIO-HEART.P02`

## Prerequisites
- Required: Phase P01a (Analysis Verification) passed
- Expected files: `analysis/domain-model.md`

## Requirements Implemented (Expanded)

All requirements are represented in pseudocode form. This phase produces algorithmic pseudocode for every public and significant internal function across all 7 modules.

### Coverage
- `stream.md`: 17 algorithms covering init/uninit, create/destroy sample, play/stop/pause/resume/seek stream, decoder task, source processing, fade, scope, tagging
- `trackplayer.md`: 15 algorithms covering splice, multi-track, split_sub_pages, timestamps, play/stop/jump/pause/resume, seeking, callbacks, subtitles
- `music.md`: 10 algorithms covering play/stop/playing/seek/pause/resume, speech, load/release, volume, fade
- `sfx.md`: 9 algorithms covering play/stop channel, positional audio, sound bank load/release
- `control.md`: 10 algorithms covering source array init, volume, stop/clean, sound_playing, wait
- `fileinst.md`: 4 algorithms covering RAII guard, load_sound_file, load_music_file, destroy delegates
- `heart_ffi.md`: 7 sections covering all 60+ FFI shim functions

Behavior contract:
- GIVEN: The domain model from P01 identifies all entities and state transitions
- WHEN: Pseudocode is written for every public function in every module
- THEN: Each algorithm is numbered, includes validation points, error handling, ordering constraints, integration boundaries, and side effects

## Implementation Tasks

### Files to create
- `analysis/pseudocode/stream.md` — 17 numbered algorithms
  - marker: `@plan PLAN-20260225-AUDIO-HEART.P02`
- `analysis/pseudocode/trackplayer.md` — 15 numbered algorithms
  - marker: `@plan PLAN-20260225-AUDIO-HEART.P02`
- `analysis/pseudocode/music.md` — 10 numbered algorithms
  - marker: `@plan PLAN-20260225-AUDIO-HEART.P02`
- `analysis/pseudocode/sfx.md` — 9 numbered algorithms
  - marker: `@plan PLAN-20260225-AUDIO-HEART.P02`
- `analysis/pseudocode/control.md` — 10 numbered algorithms
  - marker: `@plan PLAN-20260225-AUDIO-HEART.P02`
- `analysis/pseudocode/fileinst.md` — 4 numbered algorithms
  - marker: `@plan PLAN-20260225-AUDIO-HEART.P02`
- `analysis/pseudocode/heart_ffi.md` — 7 sections
  - marker: `@plan PLAN-20260225-AUDIO-HEART.P02`

## Verification Commands

```bash
# Verify all 7 pseudocode files exist and have content
for f in stream trackplayer music sfx control fileinst heart_ffi; do
  test -f "project-plans/audiorust/heart/analysis/pseudocode/${f}.md" && \
  echo "OK: ${f}.md ($(wc -l < "project-plans/audiorust/heart/analysis/pseudocode/${f}.md") lines)"
done
```

## Structural Verification Checklist
- [ ] All 7 pseudocode files created
- [ ] Each file has numbered algorithm lines
- [ ] Each algorithm references REQ-* requirements
- [ ] Validation points present in each algorithm
- [ ] Error handling documented

## Semantic Verification Checklist (Mandatory)
- [ ] Every public API function from spec §3 has pseudocode
- [ ] Every callback method has pseudocode
- [ ] Every internal helper (decode_all, get_decoder_time, etc.) has pseudocode
- [ ] Pseudocode covers thread interaction points
- [ ] Lock acquisition/release documented in multi-threaded algorithms
- [ ] Integration calls to mixer API documented

## Deferred Implementation Detection (Mandatory)
N/A — pseudocode phase, no code

## Success Criteria
- [ ] 7 pseudocode files with substantive content
- [ ] All 60+ public functions covered
- [ ] All requirement IDs referenced

## Failure Recovery
- rollback: N/A (documentation only)
- blocking issues: If algorithm cannot be specified due to spec ambiguity, document gap

## Phase Completion Marker
Create: `project-plans/audiorust/heart/.completed/P02.md`
