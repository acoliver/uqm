# Technical Review — Audio Heart Plan

*Reviewed by deepthinker subagent, 2026-02-25*

## Verdict: CONDITIONAL PASS (9 issues)

The plan covers the full scope of the Audio Heart Rust port with 22 phases. All 6 C files (stream.c, trackplayer.c, music.c, sfx.c, sound.c, fileinst.c) will be replaceable after execution. However, 9 issues need attention before or during implementation.

## 1. Requirement Coverage

The plan maps all EARS requirements from rust-heart.md (STREAM-*, TRACK-*, MUSIC-*, SFX-*, VOLUME-*, FILEINST-*, CROSS-*) to specific phases. The specification.md reformats them with REQ-* prefixes. Every requirement is assigned to at least one phase.

## 2. Technical Feasibility

- Streaming thread design (condvar + AtomicBool shutdown) is sound
- Buffer processing loop correctly follows C's pattern
- Direct mixer calls (not FFI round-trip) are correctly specified
- SoundDecoder trait extensions (set_looping, decode_all, get_time) are noted as needed but must be added to decoder.rs before stream impl phase

## 3. Integration Completeness

- P21 wires in with USE_RUST_HEART flag in config_unix.h
- All 60+ FFI functions are covered in the FFI phases (P18-P20)
- Integration verification includes end-to-end manual testing

## 4. Issues Found

1. **[Must-fix]** SoundDecoder trait needs `set_looping()`, `decode_all()`, `get_time()` added before P08 (stream impl). Plan should add a prerequisite step or note this in P03 types phase.
2. **[Must-fix]** Mixer needs `mixer_source_fv()` for 3D positioning (SFX-POSITION-01). Not addressed in plan phases.
3. **[Should-fix]** Lock ordering (source → sample) not explicitly documented in stream impl phase.
4. **[Should-fix]** `StreamEngine` lazy_static depends on mixer being initialized first — init ordering constraint.
5. **[Should-fix]** Consistent `parking_lot::Mutex` usage not enforced in all pseudocode examples.
6. **[Minor]** PlayChannel FFI handle resolution mechanism (opaque SOUND → SoundBank + index) not detailed.
7. **[Minor]** Recursive Drop on SoundChunk linked list could stack overflow for very long chains.
8. **[Minor]** TrackPlayerState raw pointers require careful lifetime management documentation.
9. **[Minor]** Plan doesn't explicitly address the 4 "must-fix" API gaps identified in the rust-heart.md review notes.
