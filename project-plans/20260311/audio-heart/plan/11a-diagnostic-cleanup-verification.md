# Phase 11a: Diagnostic Cleanup — Verification

## Phase ID
`PLAN-20260314-AUDIO-HEART.P11a`

## Prerequisites
- Required: Phase P11 completed

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings 2>&1 | head -50
cargo test --workspace --all-features

# Zero eprintln! in sound modules
grep -rn 'eprintln!' rust/src/sound/{stream,trackplayer,heart_ffi,music,sfx,control,fileinst,types}.rs | wc -l
# Expected: 0

# Zero [PARITY] in sound modules
grep -rn 'PARITY' rust/src/sound/ | wc -l
# Expected: 0
```

## Structural Verification Checklist
- [ ] No `eprintln!` in audio-heart modules
- [ ] No `[PARITY]` markers
- [ ] log crate calls present where needed

## Semantic Verification Checklist
- [ ] No behavioral changes (only output mechanism changed)
- [ ] Error conditions still logged at appropriate levels
- [ ] Tests pass unchanged

## Success Criteria
- [ ] All verification commands pass
- [ ] Diagnostic scaffolding fully cleaned up

## Phase Completion Marker
Create: `project-plans/20260311/audio-heart/.completed/P11a.md`
