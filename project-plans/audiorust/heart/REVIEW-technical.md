# Technical Review -- Audio Heart Plan

Date: 2026-02-25 (Round 4)
Reviewer: Manual review (deepthinker timed out)

---

## 1. Requirement Coverage

All 226+ EARS requirements assigned to phases across 35 categories. Phase table in 00-overview.md maps every category to specific phases. Individual REQ-* IDs used throughout.

**Result: PASS**

---

## 2. Technical Feasibility

### 2.1 Streaming Thread
- Condvar/buffer processing loop well-specified with numbered pseudocode
- 100ms idle sleep matches C behavior
- Buffer processing handles: decode, queue, EOF, callbacks, fade
- parking_lot::Condvar (no spurious wakeups) documented

**Verdict: PASS**

### 2.2 Lock Ordering
Hierarchy documented in specification.md section 2.4:
TRACK_STATE -> MUSIC_STATE -> Source -> Sample -> FadeState

Verified across all pseudocode files:
- stream.md: Source->Sample ordering followed
- trackplayer.md: TRACK_STATE->Source ordering followed
- music.md: MUSIC_STATE accessed independently
- sfx.md: Source accessed independently

Known exception documented: play_stream buffer-fill loop calls on_end_chunk (which acquires TRACK_STATE) while holding Source+Sample. Matches C behavior. Safe because play_stream is main-thread-only.

**Verdict: PASS (with documented exception)**

### 2.3 SoundDecoder Trait Extensions
set_looping(), decode_all(), get_time() added in P03-P05 types phases, before needed by P08 stream impl. decode_all() uses pre-allocation based on length() with permanent error break.

**Verdict: PASS**

### 2.4 All-Arc Pointer Strategy
Unified strategy documented in specification.md, rust-heart.md, 20-ffi-impl.md, heart_ffi.md:
- TFB_CreateSoundSample: Arc::new(Mutex::new(sample)) -> Arc::into_raw
- PlayStream/accessors: Arc::increment_strong_count + Arc::from_raw (borrow)
- TFB_DestroySoundSample: Arc::from_raw (consuming)
- SoundBank: Box (single-owner, not shared)

**Verdict: PASS**

### 2.5 Mixer Extension (P02b)
New phase adds PositionX/Y/Z to SourceProp and MixerSource. Positions stored but not used for panning (matching C no-op). Unblocks P14 sfx impl.

**Verdict: PASS**

### 2.6 TrackPlayer
- Chunk splicing with iterative Drop (REQ-TRACK-ASSEMBLE-19)
- Navigation functions restructured to avoid deadlocks
- Subtitle timing handled via text field on SoundChunk

**Verdict: PASS**

### 2.7 FadeMusic
Linear volume interpolation with 10ms update granularity. Fade state machine clear. FadeState lock dropped before calling set_music_volume (avoids deadlock).

**Verdict: PASS**

---

## 3. Integration Completeness

- P21 wires with USE_RUST_HEART: YES
- 60+ FFI functions listed in P18-P20: YES
- All 6 C files replaceable: YES

**Result: PASS**

---

## 4. Verification Quality

Checked P08a (stream), P11a (trackplayer), P14a (music-sfx), P20a (FFI), P21a (integration):
- All have deterministic checks (test names, compilation)
- All have subjective behavioral checks specific to each phase
- All have deferred implementation detection grep commands
- All have failure recovery with git restore
- All have Requirements Implemented and Implementation Tasks sections (N/A for verification)

**Result: PASS**

---

## 5. Issues Found

All 5 minor issues from the initial review have been resolved:

1. ~~SpliceMultiTrack chunk boundary detail~~ -- Already detailed: chunk boundary diagram, duration calculation via `decoder.length()` with `decode_all()` fallback, seek-or-recreate pattern, and per-chunk start_time accumulation.
2. ~~Fade-in-progress replacement~~ -- FIXED: set_music_stream_fade now documents that a new fade REPLACES an in-progress fade, starting from the current volume (not the old fade's target).
3. ~~WaitForSoundEnd polling interval~~ -- Already specified: 10ms sleep matching C's TaskSwitch() granularity.
4. ~~GraphForegroundStream scope buffer path~~ -- FIXED: Added FFI rendering path comment explaining that C calls via FFI, passes a C-allocated i32 array, and Rust writes pre-computed Y coordinates. No raw buffer pointer is shared.
5. ~~Memory budget constraints~~ -- Already documented: specification.md section 7a has detailed memory budgets for scope buffers (~295KB max), sound sample buffers, and decoded audio.

No remaining issues.

---

## 6. Verdict

**PASS** -- All issues resolved. The plan is technically sound with comprehensive requirement coverage, consistent lock ordering, unified Arc pointer strategy, subagent mapping for COORDINATING.md, and thorough verification phases. Ready for execution.
