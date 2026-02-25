# Phase 01a: Analysis Verification

## Phase ID
`PLAN-20260225-AUDIO-HEART.P01a`

## Prerequisites
- Required: Phase P01 completed
- Expected files: `analysis/domain-model.md`

## Verification Checklist

### Structural
- [ ] `analysis/domain-model.md` exists and has > 100 lines
- [ ] Contains: Entity Inventory section
- [ ] Contains: State Transition Diagrams section
- [ ] Contains: Edge/Error Handling Map section
- [ ] Contains: Integration Touchpoints section
- [ ] Contains: Old Code to Replace section
- [ ] Contains: Decoder Trait Gaps section

### Semantic
- [ ] All 16+ entity types from spec are listed (SoundSample, SoundTag, SoundSource, SoundChunk, FadeState, StreamEngine, TrackPlayerState, MusicState, SfxState, VolumeState, FileInstState, SoundSourceArray, MusicRef, SoundBank, SoundPosition, SubtitleRef)
- [ ] State machines for: SoundSource (Inactive→Playing→Paused→Stopped), FadeState (Inactive→Active→Completed), TrackPlayerState (Empty→Assembling→Playing→Stopped), FileInstState (Idle→Loading→Idle)
- [ ] All 14 AudioError variants mapped to trigger conditions
- [ ] Lock ordering documented (TRACK_STATE → Source → Sample → FadeState)
- [ ] Threading model covers main thread + decoder thread
- [ ] 6 C files listed for replacement

## Gate Decision
- [ ] PASS: proceed to P02
- [ ] FAIL: revise analysis
