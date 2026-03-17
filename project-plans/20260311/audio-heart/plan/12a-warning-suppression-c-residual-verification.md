# Phase 12a: Warning Suppression & C Residual — Verification

## Phase ID
`PLAN-20260314-AUDIO-HEART.P12a`

## Prerequisites
- Required: Phase P12 completed

## Verification Commands

```bash
# Full quality gate
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features

# No blanket suppression
grep -rn '#!\[allow(dead_code' rust/src/sound/{stream,trackplayer,music,sfx,control,fileinst,heart_ffi,types,loading}.rs | wc -l
# Expected: 0

# No PARITY markers
grep -rn 'PARITY' rust/src/sound/ | wc -l
# Expected: 0

# No eprintln!
grep -rn 'eprintln!' rust/src/sound/{stream,trackplayer,heart_ffi,music,sfx,control,fileinst,types,loading}.rs | wc -l
# Expected: 0

# No TODO/FIXME/HACK in implementation
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/sound/{stream,trackplayer,music,sfx,control,fileinst,heart_ffi,types,loading}.rs | wc -l
# Expected: 0

# C residual guarded
grep -c 'USE_RUST_AUDIO_HEART' sc2/src/libs/sound/sound.c sc2/src/libs/sound/music.c sc2/src/libs/sound/sfx.c
```

## Structural Verification Checklist
- [ ] No module-level `#![allow(dead_code)]` in audio-heart modules
- [ ] C residual code guarded by `USE_RUST_AUDIO_HEART`
- [ ] No link errors

## Semantic Verification Checklist
- [ ] All tests pass
- [ ] clippy clean with `-D warnings`
- [ ] No behavioral regressions
- [ ] Subsystem meets spec §23.1 and §23.2 end-state requirements

## End-State Checklist (Spec §23)

### §23.1 Functional Correctness
- [ ] Internal music loader loads real files → MusicRef
- [ ] Internal SFX bank loader parses/decodes/uploads → SoundBank
- [ ] fileinst routes through canonical loaders
- [ ] Multi-track creates chunks with real decoders
- [ ] PLRPause matches C ref-matching semantics
- [ ] NORMAL_VOLUME = 160 (single canonical value)
- [ ] init_sound/uninit_sound are correct lifecycle hooks

### §23.2 Maintainability
- [ ] No residual C code compiled outside guard
- [ ] No [PARITY] diagnostic output
- [ ] No blanket warning suppression

## Success Criteria
- [ ] All end-state requirements verified
- [ ] Subsystem is COMPLETE

## Phase Completion Marker
Create: `project-plans/20260311/audio-heart/.completed/P12a.md`

This is the final phase. After P12a completion, the audio-heart subsystem stabilization is complete.
