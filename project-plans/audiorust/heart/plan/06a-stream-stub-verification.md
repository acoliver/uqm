# Phase 06a: Stream Stub Verification

## Phase ID
`PLAN-20260225-AUDIO-HEART.P06a`

## Prerequisites
- Required: Phase P06 completed
- Expected files: `rust/src/sound/stream.rs`, updated `mod.rs`

## Verification Commands

```bash
cd /Users/acoliver/projects/uqm/rust && cargo check --lib --all-features
cd /Users/acoliver/projects/uqm/rust && cargo fmt --all --check
cd /Users/acoliver/projects/uqm/rust && cargo clippy --workspace --all-targets --all-features -- -D warnings
cd /Users/acoliver/projects/uqm/sc2 && ./build.sh uqm
```

## Structural Verification Checklist
- [ ] `rust/src/sound/stream.rs` exists
- [ ] `mod.rs` updated with `pub mod stream;`
- [ ] `@plan PLAN-20260225-AUDIO-HEART.P06` marker present
- [ ] `cargo check` passes
- [ ] `cargo fmt` passes
- [ ] `cargo clippy` passes

## Semantic Verification Checklist

### Deterministic checks
- [ ] All 19+ public functions have signatures: `grep -c "pub fn\|pub(crate) fn" rust/src/sound/stream.rs` >= 19
- [ ] StreamEngine struct has all 5 fields: sources, fade, decoder_thread, shutdown, wake
- [ ] Module importable: `use crate::sound::stream::play_stream;` compiles
- [ ] Correct use of `parking_lot::Mutex`, `parking_lot::Condvar`: `grep -c "parking_lot" rust/src/sound/stream.rs` >= 2
- [ ] `AtomicBool` used for shutdown flag: `grep -c "AtomicBool" rust/src/sound/stream.rs` >= 1
- [ ] No fake success behavior — all stubs use `todo!()`: `grep -c "todo!()" rust/src/sound/stream.rs` >= 15

### Subjective checks
- [ ] All parameter types match spec — does `play_stream` accept the right `Arc<parking_lot::Mutex<...>>`/usize types? Will the signatures be compatible with what trackplayer and music modules need to call?
- [ ] Return types match spec — do functions that can fail return `AudioResult<()>`? Do query functions return the correct type (bool, usize)?
- [ ] StreamCallbacks trait referenced correctly from types.rs — is the import path correct?
- [ ] Init ordering constraint documented — does `init_stream_decoder` doc comment mention it must be called after `mixer_init()`?
- [ ] Import paths to mixer and decoder modules are correct and will resolve at link time

### GIVEN/WHEN/THEN contracts
- GIVEN the stream module is compiled, WHEN trackplayer.rs imports `play_stream`, THEN it compiles successfully
- GIVEN the stream module is compiled, WHEN music.rs imports `stop_stream`, `pause_stream`, `resume_stream`, `seek_stream`, THEN they compile successfully
- GIVEN StreamEngine is defined, WHEN it references `parking_lot::Mutex`, `parking_lot::Condvar`, and `AtomicBool`, THEN the type definition compiles

## Deferred Implementation Detection

```bash
# Only todo!() allowed in stub phase
grep -n "todo!()" rust/src/sound/stream.rs | wc -l
# Should be > 0 (stubs exist) but controlled
grep -n "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/sound/stream.rs
# Should return 0 results (only todo!() macro is allowed)
```

## Success Criteria
- [ ] All signatures compile
- [ ] Module registered in mod.rs
- [ ] Other modules can import from stream.rs
- [ ] Init ordering constraint documented
- [ ] C build not broken

## Failure Recovery
- rollback: `git checkout -- rust/src/sound/mod.rs` and `rm rust/src/sound/stream.rs`
- blocking issues: If type signatures in types.rs need adjustment, fix in types.rs first

## Phase Completion Marker
Create: `project-plans/audiorust/heart/.completed/P06a.md`
