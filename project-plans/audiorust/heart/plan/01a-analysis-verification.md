# Phase 01a: Analysis Verification

## Phase ID
`PLAN-20260225-AUDIO-HEART.P01a`

## Prerequisites
- Required: Phase P01 completed
- Expected files: `analysis/domain-model.md`

## Verification Commands

```bash
# Structural: verify analysis file exists with substantive content
test -f project-plans/audiorust/heart/analysis/domain-model.md
wc -l project-plans/audiorust/heart/analysis/domain-model.md  # expect > 100 lines
# No code produced — build verification N/A
```

## Structural Verification Checklist
- [ ] `analysis/domain-model.md` exists and has > 100 lines
- [ ] Contains: Entity Inventory section
- [ ] Contains: State Transition Diagrams section
- [ ] Contains: Edge/Error Handling Map section
- [ ] Contains: Integration Touchpoints section
- [ ] Contains: Old Code to Replace section
- [ ] Contains: Decoder Trait Gaps section (rust-heart.md Action Items #1-4)

## Semantic Verification Checklist

### Deterministic checks
- [ ] All 16+ entity types from spec listed: SoundSample, SoundTag, SoundSource, SoundChunk, FadeState, StreamEngine, TrackPlayerState, MusicState, SfxState, VolumeState, FileInstState, SoundSourceArray, MusicRef, SoundBank, SoundPosition, SubtitleRef
- [ ] All 14 AudioError variants listed with trigger conditions
- [ ] 6 C files listed for replacement: stream.c, trackplayer.c, music.c, sfx.c, sound.c, fileinst.c
- [ ] Lock ordering documented: TRACK_STATE → Source mutex → Sample mutex → FadeState mutex

### Subjective checks
- [ ] State machines are complete — do they cover all reachable states from the API? (SoundSource: Inactive→Playing→Paused→Stopped, FadeState: Inactive→Active→Completed, TrackPlayerState: Empty→Assembling→Playing→Stopped, FileInstState: Idle→Loading→Idle)
- [ ] Integration graph accurately matches spec §2.3 layered architecture — are all module→module dependencies represented?
- [ ] Threading model adequately covers both main thread and decoder thread interactions — is every shared data access point identified?
- [ ] Decoder trait gap resolution strategies are viable — will storing looping flag on SoundSample work for all stream processing scenarios?

## Deferred Implementation Detection
N/A — analysis phase, no code produced.

## Success Criteria
- [ ] Domain model complete and reviewed
- [ ] All spec entities accounted for
- [ ] Integration touchpoints explicit
- [ ] Decoder trait gaps documented with viable resolution strategies

## Failure Recovery
- rollback: N/A (documentation only)
- blocking issues: If spec ambiguities found, document in domain-model.md and flag for resolution before proceeding

## Phase Completion Marker
Create: `project-plans/audiorust/heart/.completed/P01a.md`
