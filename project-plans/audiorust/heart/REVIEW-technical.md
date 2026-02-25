# Technical Review -- Audio Heart Plan

Date: 2026-02-25
Reviewer: deepthinker subagent (deepthinker-ho3a3r)
Verdict: NEEDS REVISIONS (12 issues)

---

## 1. Requirement Coverage

All 226+ EARS requirements from rust-heart.md are assigned to phases. The plan covers every requirement category:
STREAM-INIT (7), STREAM-PLAY (20), STREAM-THREAD (8), STREAM-PROCESS (16), STREAM-SAMPLE (5), STREAM-TAG (3), STREAM-SCOPE (11), STREAM-FADE (5), TRACK-ASSEMBLE (19), TRACK-PLAY (10), TRACK-SEEK (13), TRACK-CALLBACK (9), TRACK-SUBTITLE (4), TRACK-POSITION (2), MUSIC-PLAY (8), MUSIC-SPEECH (2), MUSIC-LOAD (6), MUSIC-RELEASE (4), MUSIC-VOLUME (1), SFX-PLAY (9), SFX-POSITION (5), SFX-VOLUME (1), SFX-LOAD (7), SFX-RELEASE (4), VOLUME-INIT (5), VOLUME-CONTROL (5), VOLUME-SOURCE (4), VOLUME-QUERY (3), FILEINST-LOAD (7), CROSS-THREAD (4), CROSS-MEMORY (4), CROSS-CONST (8), CROSS-FFI (4), CROSS-ERROR (3), CROSS-GENERAL (8).

Result: PASS -- all requirement categories assigned to specific phases.

---

## 2. Technical Feasibility

### 2.1 Streaming Thread
The condvar/buffer processing loop is well-specified with numbered pseudocode. The 100ms idle sleep matches C behavior. Buffer processing correctly handles: decode into buffer, queue to mixer, check EOF, trigger callbacks, handle fade. Lock ordering (TRACK_STATE -> Source -> Sample -> FadeState) is now explicitly documented.

Concern: The plan specifies `parking_lot::Condvar` but doesn't address the subtle difference between `parking_lot::Condvar::wait` (which doesn't have spurious wakeups on some platforms) vs `std::sync::Condvar::wait` (which does). This matters for correctness of the wakeup logic.

### 2.2 SoundDecoder Trait Extensions
The plan now includes adding `set_looping()`, `decode_all()`, and `get_time()` in the P03-P05 types phases, before they're needed by P08 stream impl. This ordering is correct.

Concern: `decode_all()` as a default trait method that loops `decode()` until EOF -- the plan should specify the buffer growth strategy (doubling? fixed chunks?) to avoid O(n^2) reallocation for large files.

### 2.3 Mixer Integration
Direct Rust calls (not FFI round-trip) are correctly specified. All referenced mixer functions exist in the codebase.

### 2.4 TrackPlayer Chunk Splicing
The pseudocode handles linked-list chunk management with proper subtitle timing. Iterative Drop for SoundChunk is now included.

Concern: The `SpliceMultiTrack` algorithm that splits one decoder into multiple chunks based on silence detection or timestamps is complex and the pseudocode is somewhat high-level. More detail on the chunk boundary calculation would reduce implementation risk.

### 2.5 FadeMusic
Linear volume interpolation over time with 10ms update granularity. The fade state machine (FadeNone/FadeIn/FadeOut) is clear.

Concern: The plan doesn't specify what happens if a new fade is requested while one is in progress. The C code replaces the current fade -- this should be explicit.

### 2.6 Scope Buffer
Ring buffer for oscilloscope data with AGC/VAD is specified. The buffer size and sample format match C behavior.

---

## 3. Integration Completeness

- P21 wires in with USE_RUST_HEART flag in config_unix.h: YES
- All 60+ FFI functions listed in P18-P20: YES (PlayStream, StopStream, PauseStream, ResumeStream, SeekStream, PlayingStream, TFB_CreateSoundSample, TFB_DestroySoundSample, TFB_SetSoundSampleData, TFB_GetSoundSampleData, TFB_SetSoundSampleCallbacks, TFB_GetSoundSampleDecoder, TFB_FindTaggedBuffer, TFB_TagBuffer, TFB_ClearBufferTag, SetMusicStreamFade, GraphForegroundStream, InitStreamDecoder, UninitStreamDecoder, SpliceTrack, SpliceMultiTrack, PlayTrack, StopTrack, JumpTrack, PauseTrack, ResumeTrack, PlayingTrack, FastReverse_Smooth, FastForward_Smooth, FastReverse_Page, FastForward_Page, GetTrackPosition, GetTrackSubtitle, GetFirstTrackSubtitle, GetNextTrackSubtitle, GetTrackSubtitleText, PLRPlaySong, PLRStop, PLRPlaying, PLRSeek, PLRPause, PLRResume, snd_PlaySpeech, snd_StopSpeech, SetMusicVolume, FadeMusic, CheckMusicResName, DestroyMusic, PlayChannel, StopChannel, ChannelPlaying, SetChannelVolume, CheckFinishedChannels, UpdateSoundPosition, GetPositionalObject, SetPositionalObject, DestroySound, StopSource, CleanSource, StopSound, SoundPlaying, WaitForSoundEnd, SetSFXVolume, SetSpeechVolume, InitSound, UninitSound, LoadSoundFile, LoadMusicFile)
- All 6 C files replaceable: YES

Result: PASS

---

## 4. Verification Quality

Checked P08a (stream verification), P11a (trackplayer verification), P14a (music-sfx verification):

All three contain:
- Deterministic checks (specific test function names, cargo test commands, compilation checks)
- Subjective behavioral checks (e.g., "Does the streaming thread correctly wake on condvar signal?", "Does buffer processing handle EOF and trigger OnEndChunk callback?", "Does FadeMusic produce smooth volume transitions over the specified duration?")
- Deferred Implementation Detection grep commands
- Failure Recovery with git restore commands

The subjective checks are specific to each phase's requirements, not generic boilerplate.

Result: PASS

---

## 5. Issues Found

1. **[Must-fix]** `decode_all()` buffer growth strategy unspecified -- could cause O(n^2) reallocation. Specify doubling or pre-allocation based on `length()`.
2. **[Must-fix]** `SpliceMultiTrack` chunk boundary calculation is high-level -- needs more pseudocode detail to be safely implementable.
3. **[Should-fix]** Fade-in-progress replacement behavior not explicitly documented (what happens when FadeMusic is called during an active fade).
4. **[Should-fix]** `parking_lot::Condvar` vs `std::sync::Condvar` spurious wakeup semantics should be noted.
5. **[Should-fix]** The plan references `_GetMusicData` and `_ReleaseMusicData` as resource handlers but doesn't detail how Rust hooks into the C resource system's vtable for music resources.
6. **[Should-fix]** `WaitForSoundEnd` blocking behavior -- the plan says "block until source stops" but doesn't specify the polling interval or whether it uses a condvar.
7. **[Minor]** The plan could benefit from a dependency graph showing which phases depend on which.
8. **[Minor]** Some pseudocode line numbers referenced in impl phases don't match current pseudocode files (may have shifted during fixes).
9. **[Minor]** P21 integration should mention that `music.h`, `sfx.h`, `sound.h` C headers need `#ifdef USE_RUST_HEART` guards for the function declarations.
10. **[Minor]** The plan doesn't address backwards compatibility testing -- running with USE_RUST_HEART disabled should still work with C code.
11. **[Minor]** `GraphForegroundStream` scope buffer rendering path is underspecified -- how does the Rust scope buffer get read by the C graphics code?
12. **[Minor]** No explicit memory budget or size constraints for scope buffer, sound sample buffers, or decoded audio buffers.

---

## 6. Verdict

NEEDS REVISIONS -- The plan is comprehensive and well-structured with good requirement coverage and verification quality. The 2 must-fix issues (decode_all buffer strategy, SpliceMultiTrack detail) should be addressed before implementation. The should-fix items can be resolved during implementation but should be documented. The plan is close to being executable.
