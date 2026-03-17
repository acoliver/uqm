# Plan: Audio Heart Stabilization and Completion

Plan ID: PLAN-20260314-AUDIO-HEART
Generated: 2026-03-14
Total Phase Entries: 25 (P00.5, verification phases, and P09.75 through P12a included where applicable)
Traceability Basis: specification.md sections + requirements.md requirement themes (no standalone REQ-ID registry exists in the supplied requirements document)

## Summary

The audio-heart subsystem is already **publicly switched to Rust** at the C ABI boundary — `USE_RUST_AUDIO_HEART` is active and all high-level sound functions are exported from `heart_ffi.rs`. However, it is **partially ported**: internal loaders are stubs, multi-track decoder loading is placeholder, there are behavioral deviations from C parity, diagnostic scaffolding pollutes output, and `#![allow(dead_code)]` suppresses warnings across all modules.

This plan stabilizes the existing port and closes every gap identified in `initialstate.md` against the specification and requirements.

## Critical Reminders

Before implementing any phase:
1. Preflight verification is complete (Phase 0.5)
2. Integration points are explicitly listed
3. TDD cycle is defined per slice
4. Lint/test/coverage gates are declared

## Scope

### In Scope
- Loader consolidation: single canonical implementation per resource type (music, SFX bank)
- Multi-track decoder loading with real decoders
- Music/speech control parity for ref-matching and wildcard-sensitive control/query behavior
- NORMAL_VOLUME conflict resolution (160 vs MAX_VOLUME)
- Pending-completion provider-side state machine for comm integration
- Comm-side adoption proof and focused integration verification for the pending-completion handshake
- Explicit build-system enforcement of `USE_RUST_AUDIO_HEART` ↔ Cargo `audio_heart` coupling
- WaitForSoundEnd full-spec compliance (paused=active, WAIT_ALL_SOURCES sentinel)
- Pre-initialization guard on all FFI-exposed APIs
- Parity diagnostic cleanup (remove/convert eprintln scaffolding)
- Warning suppression removal (#![allow(dead_code)] on all modules)
- Residual C code elimination (volume globals, resource helpers outside guard)
- End-state contract verification for handle identity, subtitle pointer stability, destroy semantics, and speech/track arbitration
- Requirement-coverage accounting for important in-scope contracts, including destroy semantics, subtitle iteration/null handling, borrowed-handle identity preservation, and speech-stop behavior

### Out of Scope
- Mixer internals changes
- Decoder format changes
- New audio features beyond C parity
- UIO/resource system porting
- Broad comm subsystem redesign beyond the minimal adoption/wiring needed to consume the provider-side pending-completion contract

## Architecture

```
C callers
    │
    ▼
audio_heart_rust.h (ABI contract)
    │
    ▼
heart_ffi.rs (FFI shim, feature-gated)
    │
    ├──► stream.rs (stream engine, decoder thread, mixer pump)
    ├──► trackplayer.rs (chunk assembly, seeking, callbacks)
    ├──► music.rs (music/speech playback, volume, fade)
    ├──► sfx.rs (SFX channels, positional audio, bank loading)
    ├──► control.rs (init/uninit, global queries, volume)
    ├──► fileinst.rs (file-load guard, canonical routing)
    ├──► loading.rs / equivalent shared loader module (final placement decided in P04)
    └──► types.rs (shared types, constants, error types)
```

## Traceability Policy

The supplied `requirements.md` is authoritative but prose-only; it does not define a stable standalone REQ-ID catalog. Therefore this plan uses:
- **Primary traceability:** exact `requirements.md` requirement themes/quoted requirement text
- **Secondary traceability:** `specification.md` section references

The overview and per-phase requirements below do **not** claim auditable REQ-* identifiers unless those identifiers are introduced in a future authoritative requirements source.

## Requirement Coverage Policy

In addition to the G1-G13 gap list, this plan must maintain a requirement-coverage matrix for the full in-scope contract surface. Each important contract must be marked as exactly one of:
- **Already satisfied / no code change required**
- **Implemented in phase X**
- **Verified in phase Y because analysis proved parity already exists**

At minimum, the matrix must account for:
- loader consolidation and file-backed resource loading
- multi-track decoder acquisition and timeline advancement
- pending-completion / comm subtitle synchronization
- `USE_RUST_AUDIO_HEART` ↔ Cargo `audio_heart` build coupling and mismatch-failure behavior
- wait-for-end selector semantics and shutdown behavior
- pre-init ABI failure behavior across the FFI surface
- wildcard-sensitive music/speech control and query semantics
- subtitle iteration and null-handling contracts
- destroy semantics
- borrowed-handle identity preservation in play/control paths
- standalone speech stop semantics and `snd_stop_speech` behavior

## Gap Summary

| # | Gap | Requirements / Spec Reference | Severity |
|---|-----|-------------------------------|----------|
| G1 | `music::get_music_data()` is stub — creates empty sample, no decoder | Requirements: Resource and UIO integration / Resource loading obligations / Loader consolidation; Spec §14.1, §14.4 | Critical |
| G2 | `sfx::get_sound_bank_data()` is stub — returns empty bank | Requirements: Resource and UIO integration / Resource loading obligations / Loader consolidation; Spec §14.2, §14.4 | Critical |
| G3 | `fileinst.rs` routes through stub internal helpers | Requirements: canonical file-backed resource loading; Spec §14.4 | Critical |
| G4 | `trackplayer::splice_multi_track()` creates chunks without decoders | Requirements: Multi-track assembly / Decoder integration; Spec §8.1 | Critical |
| G5 | `PLRPause` ignores ref-matching, always pauses | Requirements: Music behavior / Handle identity; Spec §10.4 | High |
| G6 | `NORMAL_VOLUME` conflict: 160 in types.rs, MAX_VOLUME in control.rs | Requirements: Volume behavior; Spec §6 | High |
| G7 | Pending-completion state machine missing (PollPendingTrackCompletion, CommitTrackAdvancement) | Requirements: Comm integration / Concurrency expectations / Subtitle synchronization; Spec §8.3.1 | High |
| G8 | `WaitForSoundEnd` doesn't handle paused-as-active or full sentinel semantics | Requirements: Lifecycle and Control APIs / Pre-initialization behavior; Spec §13.3 | Medium |
| G9 | No pre-init guard on FFI APIs | Requirements: Lifecycle and Control APIs / Error handling / ABI integration; Spec §13.1, §19.3 | Medium |
| G10 | 83 diagnostic eprintln calls (including [PARITY] prefixes) | Requirements: Maintainability and cleanup; Spec §23.2, §24 | Medium |
| G11 | `#![allow(dead_code)]` on 7 modules | Requirements: Maintainability and cleanup; Spec §23.2 | Medium |
| G12 | Residual C code (volume globals, resource helpers) still compiled | Requirements: Maintainability and cleanup; Spec §23.2 | Low |
| G13 | `InitSound` return code semantics (BOOLEAN vs int convention) | Requirements: ABI and C integration / Contract hierarchy; Spec §2.1, §19.3 | Low |

## Phase Structure

| Phase | Title | Focus |
|-------|-------|-------|
| P00.5 | Preflight Verification | Validate assumptions |
| P01 | Analysis | Gap mapping, integration points, old-code inventory, traceability matrix, requirement-coverage matrix |
| P02 | Pseudocode | Algorithmic pseudocode for all gap closures |
| P03 | Constants & Types Fix | G6: NORMAL_VOLUME, type consistency |
| P04 | Loader Consolidation — Stubs | G1/G2/G3: canonical loader ownership boundary + skeletons |
| P05 | Loader Consolidation — TDD | G1/G2/G3: seams, fixtures, and loader-focused tests |
| P06 | Loader Consolidation — Impl | G1/G2/G3: real canonical loaders + deferred/implemented integration verification |
| P07 | Multi-Track Decoder — TDD+Impl | G4: real decoders in splice_multi_track, using the decoder-acquisition seam established by P04-P06 |
| P08 | Music/Speech Control Parity | G5 + remaining wildcard/ref-matching/stop/query control semantics |
| P09 | Pending-Completion Integration Proof & Provider State Machine | G7: prove comm call path, then implement provider side |
| P09.5 | Comm Handshake Integration Verification | G7: verify consumer-side adoption and cross-subsystem behavior |
| P09.75 | Build/Feature Coupling Enforcement | Identify authoritative build path(s), enforce `USE_RUST_AUDIO_HEART` ↔ Cargo `audio_heart` coupling, fail fast on mismatch, and prove it with project build commands |
| P10 | Control API Hardening | G8/G9: WaitForSoundEnd, pre-init guards, ABI failure map |
| P11 | Diagnostic Cleanup | G10: Remove/convert eprintln scaffolding |
| P12 | Warning Suppression & C Residual | G11/G12: remove dead_code allows, C guard extension, final high-risk contract closure + checklist |

## Integration Contract

### Existing Callers
- All C game code calls through `audio_heart_rust.h` declarations
- Comm subsystem (`comm.c`, `commglue.c`) → track player APIs
- Oscilloscope (`oscill.c`) → `GraphForegroundStream`
- Resource system → `LoadSoundFile`, `LoadMusicFile`, `DestroySound`, `DestroyMusic`
- Game init/shutdown → `InitSound`, `UninitSound`, `InitStreamDecoder`, `UninitStreamDecoder`

### Existing Code Replaced/Removed
- C resource helpers in `music.c:158-236` and `sfx.c:162-298` (P12)
- C volume globals in `sound.c:26-69` (P12, coordinated with C-side guard extension)

### End-to-End Verification
- `cargo test --workspace --all-features` (all phases)
- Coupled and mismatched build-path verification for `USE_RUST_AUDIO_HEART` + Cargo `audio_heart` combinations (P09.75+)
- Full game build with `USE_RUST_AUDIO_HEART` + `--features audio_heart` (P09.75+)
- Game launch: music plays, comm dialogue with subtitles works, SFX fires (P09.5+)
- End-state contract checklist (P12/P12a): subtitle pointer stability, unique handle identity across repeated loads, destroy-on-active-resource behavior, speech/track arbitration, wildcard control semantics, subtitle iteration/null handling, and borrowed-handle identity preservation
