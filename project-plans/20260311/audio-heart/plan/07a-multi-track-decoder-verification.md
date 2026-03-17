# Phase 07a: Multi-Track Decoder — Verification

## Phase ID
`PLAN-20260314-AUDIO-HEART.P07a`

## Prerequisites
- Required: Phase P07 completed

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings 2>&1 | head -50
cargo test --workspace --all-features

# Verify no placeholder chunks in multi-track path
grep -n 'decoder: None' rust/src/sound/trackplayer.rs | grep -i multi
# Expected: 0 matches in multi-track code (may still appear in subtitle-only chunks)

# Verify dec_offset advancement
grep -n 'dec_offset' rust/src/sound/trackplayer.rs | grep -i multi
```

## Structural Verification Checklist
- [ ] `splice_multi_track` creates chunks with decoders from loaded audio
- [ ] FFI shim loads decoders via loading module
- [ ] dec_offset advances correctly
- [ ] Tests exist and pass

## Semantic Verification Checklist
- [ ] Multi-track audio would actually produce sound during playback (decoders present)
- [ ] Timeline is correct (total program length reflects all tracks)
- [ ] Error handling works (failed decoder load produces graceful degradation)

## Success Criteria
- [ ] All verification commands pass
- [ ] No placeholder decoder loading in multi-track path

## Phase Completion Marker
Create: `project-plans/20260311/audio-heart/.completed/P07a.md`
