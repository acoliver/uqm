# Phase 06a: PCM Decode Stub Verification

## Phase ID
`PLAN-20260225-AIFF-DECODER.P06a`

## Prerequisites
- Required: Phase 06 completed
- Expected: `decode_pcm()` and `decode_sdx2()` stubs exist, `decode()` dispatches

## Verification Commands

```bash
cd /Users/acoliver/projects/uqm/rust

# Compile check
cargo check --all-features

# All existing tests still pass
cargo test --lib --all-features -- aiff

# Verify dispatch and stubs exist
grep "fn decode_pcm" src/sound/aiff.rs
grep "fn decode_sdx2" src/sound/aiff.rs
grep "comp_type" src/sound/aiff.rs | grep "match\|Match"

# Format and lint
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings

# Deferred implementation check
grep -n "todo!()" src/sound/aiff.rs
```

## Structural Verification Checklist
- [ ] `decode()` dispatches via match on `self.comp_type`
- [ ] `fn decode_pcm(&mut self, buf: &mut [u8]) -> DecodeResult<usize>` exists
- [ ] `fn decode_sdx2(&mut self, buf: &mut [u8]) -> DecodeResult<usize>` exists
- [ ] Both decode methods contain `todo!()`
- [ ] `cargo check --all-features` succeeds
- [ ] `cargo fmt --all --check` passes

## Semantic Verification Checklist (Mandatory)

### Deterministic Checks
- [ ] Parser tests still pass (no regression from decode refactor)
- [ ] `decode()` no longer contains `todo!()` — it dispatches to decode_pcm/decode_sdx2
- [ ] `todo!()` shows in decode_pcm, decode_sdx2, and seek only

### Subjective Checks
- [ ] Is the decode dispatch a real `match self.comp_type` (not a hardcoded path)?
- [ ] Does the dispatch cover ALL CompressionType variants exhaustively?
- [ ] Is there any risk that the dispatch refactor could break existing parser behavior?

## Deferred Implementation Detection

```bash
# Stub phase: todo!() allowed in decode_pcm and decode_sdx2
cd /Users/acoliver/projects/uqm/rust && grep -n "todo!()" src/sound/aiff.rs
# Should show: decode_pcm, decode_sdx2, seek
```

## Success Criteria
- [ ] `cargo check --all-features` succeeds
- [ ] All parser tests pass (no regression)
- [ ] `decode()` dispatches to `decode_pcm`/`decode_sdx2`
- [ ] `decode_pcm()` and `decode_sdx2()` exist as `todo!()` stubs

## Failure Recovery
- Return to Phase 06 and fix the dispatch implementation
- rollback: `git checkout -- rust/src/sound/aiff.rs`

## Phase Completion Marker
Create: `project-plans/audiorust/decoder/.completed/P06a.md`

Contents:
- phase ID: PLAN-20260225-AIFF-DECODER.P06a
- timestamp
- verification result: PASS/FAIL
- gate decision: proceed to P07 or return to P06
