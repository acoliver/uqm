# Plan: Audio Heart — Streaming Pipeline Rust Port

Plan ID: `PLAN-20260225-AUDIO-HEART`
Generated: 2026-02-25
Total Phases: 22 (P00a through P21)

## Requirements

REQ-STREAM-INIT-01, REQ-STREAM-INIT-02, REQ-STREAM-INIT-03, REQ-STREAM-INIT-04, REQ-STREAM-INIT-05, REQ-STREAM-INIT-06, REQ-STREAM-INIT-07,
REQ-STREAM-PLAY-01, REQ-STREAM-PLAY-02, REQ-STREAM-PLAY-03, REQ-STREAM-PLAY-04, REQ-STREAM-PLAY-05, REQ-STREAM-PLAY-06, REQ-STREAM-PLAY-07, REQ-STREAM-PLAY-08, REQ-STREAM-PLAY-09, REQ-STREAM-PLAY-10, REQ-STREAM-PLAY-11, REQ-STREAM-PLAY-12, REQ-STREAM-PLAY-13, REQ-STREAM-PLAY-14, REQ-STREAM-PLAY-15, REQ-STREAM-PLAY-16, REQ-STREAM-PLAY-17, REQ-STREAM-PLAY-18, REQ-STREAM-PLAY-19, REQ-STREAM-PLAY-20,
REQ-STREAM-THREAD-01, REQ-STREAM-THREAD-02, REQ-STREAM-THREAD-03, REQ-STREAM-THREAD-04, REQ-STREAM-THREAD-05, REQ-STREAM-THREAD-06, REQ-STREAM-THREAD-07, REQ-STREAM-THREAD-08,
REQ-STREAM-PROCESS-01, REQ-STREAM-PROCESS-02, REQ-STREAM-PROCESS-03, REQ-STREAM-PROCESS-04, REQ-STREAM-PROCESS-05, REQ-STREAM-PROCESS-06, REQ-STREAM-PROCESS-07, REQ-STREAM-PROCESS-08, REQ-STREAM-PROCESS-09, REQ-STREAM-PROCESS-10, REQ-STREAM-PROCESS-11, REQ-STREAM-PROCESS-12, REQ-STREAM-PROCESS-13, REQ-STREAM-PROCESS-14, REQ-STREAM-PROCESS-15, REQ-STREAM-PROCESS-16,
REQ-STREAM-SAMPLE-01, REQ-STREAM-SAMPLE-02, REQ-STREAM-SAMPLE-03, REQ-STREAM-SAMPLE-04, REQ-STREAM-SAMPLE-05,
REQ-STREAM-TAG-01, REQ-STREAM-TAG-02, REQ-STREAM-TAG-03,
REQ-STREAM-SCOPE-01, REQ-STREAM-SCOPE-02, REQ-STREAM-SCOPE-03, REQ-STREAM-SCOPE-04, REQ-STREAM-SCOPE-05, REQ-STREAM-SCOPE-06, REQ-STREAM-SCOPE-07, REQ-STREAM-SCOPE-08, REQ-STREAM-SCOPE-09, REQ-STREAM-SCOPE-10, REQ-STREAM-SCOPE-11,
REQ-STREAM-FADE-01, REQ-STREAM-FADE-02, REQ-STREAM-FADE-03, REQ-STREAM-FADE-04, REQ-STREAM-FADE-05,
REQ-TRACK-ASSEMBLE-01, REQ-TRACK-ASSEMBLE-02, REQ-TRACK-ASSEMBLE-03, REQ-TRACK-ASSEMBLE-04, REQ-TRACK-ASSEMBLE-05, REQ-TRACK-ASSEMBLE-06, REQ-TRACK-ASSEMBLE-07, REQ-TRACK-ASSEMBLE-08, REQ-TRACK-ASSEMBLE-09, REQ-TRACK-ASSEMBLE-10, REQ-TRACK-ASSEMBLE-11, REQ-TRACK-ASSEMBLE-12, REQ-TRACK-ASSEMBLE-13, REQ-TRACK-ASSEMBLE-14, REQ-TRACK-ASSEMBLE-15, REQ-TRACK-ASSEMBLE-16, REQ-TRACK-ASSEMBLE-17, REQ-TRACK-ASSEMBLE-18, REQ-TRACK-ASSEMBLE-19,
REQ-TRACK-PLAY-01, REQ-TRACK-PLAY-02, REQ-TRACK-PLAY-03, REQ-TRACK-PLAY-04, REQ-TRACK-PLAY-05, REQ-TRACK-PLAY-06, REQ-TRACK-PLAY-07, REQ-TRACK-PLAY-08, REQ-TRACK-PLAY-09, REQ-TRACK-PLAY-10,
REQ-TRACK-SEEK-01, REQ-TRACK-SEEK-02, REQ-TRACK-SEEK-03, REQ-TRACK-SEEK-04, REQ-TRACK-SEEK-05, REQ-TRACK-SEEK-06, REQ-TRACK-SEEK-07, REQ-TRACK-SEEK-08, REQ-TRACK-SEEK-09, REQ-TRACK-SEEK-10, REQ-TRACK-SEEK-11, REQ-TRACK-SEEK-12, REQ-TRACK-SEEK-13,
REQ-TRACK-CALLBACK-01, REQ-TRACK-CALLBACK-02, REQ-TRACK-CALLBACK-03, REQ-TRACK-CALLBACK-04, REQ-TRACK-CALLBACK-05, REQ-TRACK-CALLBACK-06, REQ-TRACK-CALLBACK-07, REQ-TRACK-CALLBACK-08, REQ-TRACK-CALLBACK-09,
REQ-TRACK-SUBTITLE-01, REQ-TRACK-SUBTITLE-02, REQ-TRACK-SUBTITLE-03, REQ-TRACK-SUBTITLE-04,
REQ-TRACK-POSITION-01, REQ-TRACK-POSITION-02,
REQ-MUSIC-PLAY-01, REQ-MUSIC-PLAY-02, REQ-MUSIC-PLAY-03, REQ-MUSIC-PLAY-04, REQ-MUSIC-PLAY-05, REQ-MUSIC-PLAY-06, REQ-MUSIC-PLAY-07, REQ-MUSIC-PLAY-08,
REQ-MUSIC-SPEECH-01, REQ-MUSIC-SPEECH-02,
REQ-MUSIC-LOAD-01, REQ-MUSIC-LOAD-02, REQ-MUSIC-LOAD-03, REQ-MUSIC-LOAD-04, REQ-MUSIC-LOAD-05, REQ-MUSIC-LOAD-06,
REQ-MUSIC-RELEASE-01, REQ-MUSIC-RELEASE-02, REQ-MUSIC-RELEASE-03, REQ-MUSIC-RELEASE-04,
REQ-MUSIC-VOLUME-01,
REQ-SFX-PLAY-01, REQ-SFX-PLAY-02, REQ-SFX-PLAY-03, REQ-SFX-PLAY-04, REQ-SFX-PLAY-05, REQ-SFX-PLAY-06, REQ-SFX-PLAY-07, REQ-SFX-PLAY-08, REQ-SFX-PLAY-09,
REQ-SFX-POSITION-01, REQ-SFX-POSITION-02, REQ-SFX-POSITION-03, REQ-SFX-POSITION-04, REQ-SFX-POSITION-05,
REQ-SFX-VOLUME-01,
REQ-SFX-LOAD-01, REQ-SFX-LOAD-02, REQ-SFX-LOAD-03, REQ-SFX-LOAD-04, REQ-SFX-LOAD-05, REQ-SFX-LOAD-06, REQ-SFX-LOAD-07,
REQ-SFX-RELEASE-01, REQ-SFX-RELEASE-02, REQ-SFX-RELEASE-03, REQ-SFX-RELEASE-04,
REQ-VOLUME-INIT-01, REQ-VOLUME-INIT-02, REQ-VOLUME-INIT-03, REQ-VOLUME-INIT-04, REQ-VOLUME-INIT-05,
REQ-VOLUME-CONTROL-01, REQ-VOLUME-CONTROL-02, REQ-VOLUME-CONTROL-03, REQ-VOLUME-CONTROL-04, REQ-VOLUME-CONTROL-05,
REQ-VOLUME-SOURCE-01, REQ-VOLUME-SOURCE-02, REQ-VOLUME-SOURCE-03, REQ-VOLUME-SOURCE-04,
REQ-VOLUME-QUERY-01, REQ-VOLUME-QUERY-02, REQ-VOLUME-QUERY-03,
REQ-FILEINST-LOAD-01, REQ-FILEINST-LOAD-02, REQ-FILEINST-LOAD-03, REQ-FILEINST-LOAD-04, REQ-FILEINST-LOAD-05, REQ-FILEINST-LOAD-06, REQ-FILEINST-LOAD-07,
REQ-CROSS-THREAD-01, REQ-CROSS-THREAD-02, REQ-CROSS-THREAD-03, REQ-CROSS-THREAD-04,
REQ-CROSS-MEMORY-01, REQ-CROSS-MEMORY-02, REQ-CROSS-MEMORY-03, REQ-CROSS-MEMORY-04,
REQ-CROSS-CONST-01, REQ-CROSS-CONST-02, REQ-CROSS-CONST-03, REQ-CROSS-CONST-04, REQ-CROSS-CONST-05, REQ-CROSS-CONST-06, REQ-CROSS-CONST-07, REQ-CROSS-CONST-08,
REQ-CROSS-FFI-01, REQ-CROSS-FFI-02, REQ-CROSS-FFI-03, REQ-CROSS-FFI-04,
REQ-CROSS-ERROR-01, REQ-CROSS-ERROR-02, REQ-CROSS-ERROR-03,
REQ-CROSS-GENERAL-01, REQ-CROSS-GENERAL-02, REQ-CROSS-GENERAL-03, REQ-CROSS-GENERAL-04, REQ-CROSS-GENERAL-05, REQ-CROSS-GENERAL-06, REQ-CROSS-GENERAL-07, REQ-CROSS-GENERAL-08

**Total: 234 requirements**

## Critical Reminders

Before implementing any phase:
1. Preflight verification is complete (Phase 0a)
2. Integration points are explicitly listed
3. TDD cycle is defined per slice (stub → TDD → impl)
4. Lint/test/coverage gates are declared
5. No `unwrap()`/`expect()` in production code
6. Use `parking_lot::Mutex` (not `std::sync::Mutex`)
7. All `unsafe` confined to FFI boundary (`heart_ffi.rs`)
8. Lock ordering: TRACK_STATE → Source mutex → Sample mutex → FadeState mutex

## Phase Execution Order

```
P00a (preflight) → P01 (analysis) → P01a (verify) →
P02 (pseudocode) → P02a (verify) →
P03 (types stub) → P03a (verify) → P04 (types TDD) → P04a (verify) → P05 (types impl) → P05a (verify) →
P06 (stream stub) → P06a (verify) → P07 (stream TDD) → P07a (verify) → P08 (stream impl) → P08a (verify) →
P09 (trackplayer stub) → P09a (verify) → P10 (trackplayer TDD) → P10a (verify) → P11 (trackplayer impl) → P11a (verify) →
P12 (music+sfx stub) → P12a (verify) → P13 (music+sfx TDD) → P13a (verify) → P14 (music+sfx impl) → P14a (verify) →
P15 (control+fileinst stub) → P15a (verify) → P16 (control+fileinst TDD) → P16a (verify) → P17 (control+fileinst impl) → P17a (verify) →
P18 (FFI stub) → P18a (verify) → P19 (FFI TDD) → P19a (verify) → P20 (FFI impl) → P20a (verify) →
P21 (integration) → P21a (verify)
```

## Execution Tracker

| Phase | Status | Verified | Semantic Verified | Notes |
|------:|--------|----------|-------------------|-------|
| P00a  | ⬜     | ⬜       | N/A               |       |
| P01   | ⬜     | ⬜       | ⬜                |       |
| P01a  | ⬜     | ⬜       | ⬜                |       |
| P02   | ⬜     | ⬜       | ⬜                |       |
| P02a  | ⬜     | ⬜       | ⬜                |       |
| P03   | ⬜     | ⬜       | ⬜                |       |
| P03a  | ⬜     | ⬜       | ⬜                |       |
| P04   | ⬜     | ⬜       | ⬜                |       |
| P04a  | ⬜     | ⬜       | ⬜                |       |
| P05   | ⬜     | ⬜       | ⬜                |       |
| P05a  | ⬜     | ⬜       | ⬜                |       |
| P06   | ⬜     | ⬜       | ⬜                |       |
| P06a  | ⬜     | ⬜       | ⬜                |       |
| P07   | ⬜     | ⬜       | ⬜                |       |
| P07a  | ⬜     | ⬜       | ⬜                |       |
| P08   | ⬜     | ⬜       | ⬜                |       |
| P08a  | ⬜     | ⬜       | ⬜                |       |
| P09   | ⬜     | ⬜       | ⬜                |       |
| P09a  | ⬜     | ⬜       | ⬜                |       |
| P10   | ⬜     | ⬜       | ⬜                |       |
| P10a  | ⬜     | ⬜       | ⬜                |       |
| P11   | ⬜     | ⬜       | ⬜                |       |
| P11a  | ⬜     | ⬜       | ⬜                |       |
| P12   | ⬜     | ⬜       | ⬜                |       |
| P12a  | ⬜     | ⬜       | ⬜                |       |
| P13   | ⬜     | ⬜       | ⬜                |       |
| P13a  | ⬜     | ⬜       | ⬜                |       |
| P14   | ⬜     | ⬜       | ⬜                |       |
| P14a  | ⬜     | ⬜       | ⬜                |       |
| P15   | ⬜     | ⬜       | ⬜                |       |
| P15a  | ⬜     | ⬜       | ⬜                |       |
| P16   | ⬜     | ⬜       | ⬜                |       |
| P16a  | ⬜     | ⬜       | ⬜                |       |
| P17   | ⬜     | ⬜       | ⬜                |       |
| P17a  | ⬜     | ⬜       | ⬜                |       |
| P18   | ⬜     | ⬜       | ⬜                |       |
| P18a  | ⬜     | ⬜       | ⬜                |       |
| P19   | ⬜     | ⬜       | ⬜                |       |
| P19a  | ⬜     | ⬜       | ⬜                |       |
| P20   | ⬜     | ⬜       | ⬜                |       |
| P20a  | ⬜     | ⬜       | ⬜                |       |
| P21   | ⬜     | ⬜       | ⬜                |       |
| P21a  | ⬜     | ⬜       | ⬜                |       |
