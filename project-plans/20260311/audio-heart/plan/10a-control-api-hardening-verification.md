# Phase 10a: Control API Hardening — Verification

## Phase ID
`PLAN-20260314-AUDIO-HEART.P10a`

## Prerequisites
- Required: Phase P10 completed

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings 2>&1 | head -50
cargo test --workspace --all-features

# Verify the P10 pre-init coverage table exists
grep -n 'Pre-Init ABI Failure Map' project-plans/20260311/audio-heart/plan/10-control-api-hardening.md

# Enumerate FFI exports for table cross-check
grep -n '^pub extern "C" fn\|^pub unsafe extern "C" fn' rust/src/sound/heart_ffi.rs

# Verify WaitForSoundEnd rewrite
grep -B2 -A18 'fn wait_for_sound_end' rust/src/sound/control.rs
```

## Structural Verification Checklist
- [ ] `is_initialized()` in stream.rs
- [ ] P10 contains a populated pre-init ABI failure map
- [ ] Every FFI export in `heart_ffi.rs` is accounted for in that map
- [ ] Pre-init guards in heart_ffi.rs match the set of functions marked `requires init = yes`
- [ ] `wait_for_sound_end` accepts u32 and handles all selector values

## Semantic Verification Checklist
- [ ] Pre-init returns correct failure values per the P10 table and §19.3/C ABI contract
- [ ] Representative tests exist for each ABI failure-value class used in the table
- [ ] Paused sources treated as active in wait
- [ ] Default branch (invalid index) waits for all
- [ ] WAIT_ALL_SOURCES is verified at the FFI boundary as `0xFFFFFFFF`
- [ ] QuitPosted breaks wait

## Success Criteria
- [ ] All verification commands pass
- [ ] API hardening complete
- [ ] Coverage is proven by inventory, not grep counts

## Phase Completion Marker
Create: `project-plans/20260311/audio-heart/.completed/P10a.md`
