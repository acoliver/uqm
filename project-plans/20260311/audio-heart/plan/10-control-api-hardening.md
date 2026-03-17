# Phase 10: Control API Hardening

## Phase ID
`PLAN-20260314-AUDIO-HEART.P10`

## Prerequisites
- Required: Phase P09.5 completed
- Expected: Pending-completion integration verified, canonical loaders, behavioral fixes done

## Requirements Implemented (Expanded)

### WaitForSoundEnd spec compliance
**Requirement text**: The wait-for-end operation must support the WAIT_ALL_SOURCES sentinel, treat paused sources as active, handle invalid indices by defaulting to all-sources, return immediately before initialization, and terminate promptly when shutdown signaling is already active or becomes active during the wait, including concurrent teardown scenarios.

Behavior contract:
- GIVEN: Source 5 (music) is paused
- WHEN: `wait_for_sound_end(5)` is called
- THEN: Wait does NOT return (paused = active)

- GIVEN: Channel 999 passed (out of range, not WAIT_ALL_SOURCES)
- WHEN: `wait_for_sound_end(999)` is called
- THEN: Treated as wait-for-all (default branch)

- GIVEN: WAIT_ALL_SOURCES (`0xFFFFFFFF`) passed
- WHEN: `wait_for_sound_end(WAIT_ALL_SOURCES)` is called
- THEN: Waits for all sources 0-6

- GIVEN: QuitPosted is already true before entering wait
- WHEN: `WaitForSoundEnd` is called
- THEN: Returns promptly without blocking

### Pre-initialization guards
**Requirement text**: When any FFI-exposed playback, query, loading, or control API is called before `init_stream_decoder` has completed, the audio-heart subsystem shall produce the ABI-defined failure outcome for that specific function.

Behavior contract:
- GIVEN: `init_stream_decoder` has NOT been called
- WHEN: `LoadMusicFile("test.ogg")` is called via FFI
- THEN: Returns null pointer

- GIVEN: `init_stream_decoder` has NOT been called
- WHEN: `PLRPlaySong` is called via FFI
- THEN: Returns without modifying playback state (silent no-op)

- GIVEN: `init_stream_decoder` has NOT been called
- WHEN: `WaitForSoundEnd` is called via FFI
- THEN: Returns immediately without blocking

Why it matters:
- WaitForSoundEnd is concurrency-sensitive, not just selector-sensitive
- Pre-init guard prevents crashes from calling APIs before mixer is ready
- These are correctness requirements, not polish

## Pre-Init ABI Failure Map (authoritative work item for this phase)

P10 must produce and use a function-by-function inventory of the FFI surface. `grep -c 'is_initialized'` is not sufficient proof of coverage. The implementation and verification for this phase must maintain a table with, at minimum, these columns:
- FFI export name
- category (`lifecycle`, `loading`, `playback`, `query`, `control`, `destroy`, `subtitle/speech`, `comm hook`)
- requires init? (`yes` / `no`)
- pre-init ABI outcome (`NULL`, `0`, `-1`, `FALSE`, immediate return, or other exact contract value)
- notes / requirement/spec source

The inventory must cover every exported function in `rust/src/sound/heart_ffi.rs`, including but not limited to:
- lifecycle: `InitSound`, `UninitSound`, `InitStreamDecoder`, `UninitStreamDecoder`
- loading / destroy: `LoadMusicFile`, `DestroyMusic`, `LoadSoundFile`, `DestroySound`
- playback and control: `PLRPlaySong`, `PLRStop`, `PLRPause`, `PLRResume`, `PLRPlaying`, `PLRSeek`, `PlayChannel`, `StopChannel`, `ChannelPlaying`, `WaitForSoundEnd`
- query / timing: `GetTrackPosition`, `PlayingTrack`, `GetNextTrackTag`, and other exported status APIs
- speech/subtitle APIs and any comm-facing exports introduced by P09/P09.5

Any function omitted from this table blocks completion of P10.

## Implementation Tasks

### TDD — Files to modify

#### `rust/src/sound/control.rs` — add tests
- `test_wait_for_sound_end_all_sources_returns_when_idle` — existing, verify still works
- `test_wait_for_sound_end_specific_source` — specific source index
- `test_wait_for_sound_end_invalid_defaults_to_all` — out-of-range index treated as all
- `test_wait_for_sound_end_paused_source_is_active` — paused source keeps wait alive
- `test_wait_for_sound_end_quit_posted_pre_entry_returns_promptly` — quit already true before wait
- `test_wait_for_sound_end_quit_posted_during_wait_breaks_loop` — quit breaks active wait
- `test_wait_for_sound_end_handles_concurrent_teardown` — no access to freed/released source state during teardown path
- `test_wait_for_sound_end_paused_allocated_handle_without_active_mixer_is_still_active` — paused-stream exception wins over mixer state
- `test_wait_for_sound_end_mixed_all_sources_states` — all-sources wait across streaming/non-streaming/paused/inactive mix
- marker: `@plan PLAN-20260314-AUDIO-HEART.P10`

#### `rust/src/sound/stream.rs` — add init state tracking tests
- `test_is_initialized_false_initially`
- `test_is_initialized_true_after_init`
- marker: `@plan PLAN-20260314-AUDIO-HEART.P10`

#### `rust/src/sound/heart_ffi.rs` — add FFI-level tests where practical
- `test_wait_for_sound_end_ffi_returns_immediately_before_init`
- `test_get_track_position_ffi_pre_init_returns_abi_failure_value`
- representative tests from each ABI-failure class in the pre-init coverage table
- marker: `@plan PLAN-20260314-AUDIO-HEART.P10`

### Implementation — Files to modify

#### `rust/src/sound/stream.rs`
- Add `static STREAM_INITIALIZED: AtomicBool = AtomicBool::new(false)`
- In `init_stream_decoder()`: set `STREAM_INITIALIZED.store(true, Ordering::Release)` on success
- In `uninit_stream_decoder()`: set `STREAM_INITIALIZED.store(false, Ordering::Release)` before cleanup
- Add `pub fn is_initialized() -> bool`: returns `STREAM_INITIALIZED.load(Ordering::Acquire)`
- marker: `@plan PLAN-20260314-AUDIO-HEART.P10`

#### `rust/src/sound/control.rs`
- Rewrite `wait_for_sound_end(channel: u32)` to accept raw u32 (matching C ABI):
  - Match on channel value:
    - valid source index range: wait for that source only
    - WAIT_ALL_SOURCES (`0xFFFFFFFF`): wait for all sources
    - anything else: treat as all sources (default branch)
  - Return immediately if shutdown/quit is already active before the loop
  - For each source in the wait set:
    - check if source has a sample (no sample = inactive)
    - check if source handle is allocated (no handle = inactive)
    - if `pause_time != 0`, treat as active regardless of current mixer playback state
    - check `stream_should_be_playing` for streaming sources
    - query mixer state for non-streaming sources when safe
  - Exit promptly if shutdown/quit becomes active during the loop
  - Avoid observing freed/released resources during concurrent teardown
  - Sleep 10ms between polls
- Explicitly confirm the selector width and sentinel against the C declaration at the FFI boundary:
  - authoritative sentinel target for this plan is `WAIT_ALL_SOURCES = 0xFFFFFFFF` on the raw `u32` ABI path
  - record the normalization from the earlier `0xFFFF` observation in analysis as an ABI-cleanup issue, not a second valid sentinel
  - add at least one FFI-boundary test proving the chosen sentinel reaches `wait_for_sound_end` unaltered
- marker: `@plan PLAN-20260314-AUDIO-HEART.P10`

#### `rust/src/sound/heart_ffi.rs`
- Add pre-init guard to every FFI function that requires initialization
- Drive the guards from the P10 pre-init coverage table rather than ad hoc insertion
- Preserve per-function ABI failure values according to the failure-mode map
- Explicitly document and test pre-init behavior for `GetTrackPosition` so the chosen ABI failure value cannot be mistaken for a successful in-range position
- marker: `@plan PLAN-20260314-AUDIO-HEART.P10`

#### `project-plans/20260311/audio-heart/plan/10-control-api-hardening.md`
- Update this phase document in-place with the completed pre-init ABI failure map once the inventory is finalized during execution
- Keep the matrix adjacent to this phase so P10a can verify against it directly

### Pseudocode traceability
- Implements PC-08 lines 01-22 (WaitForSoundEnd), strengthened for shutdown/teardown timing cases
- Implements PC-09 lines 01-15 (Pre-init guard)

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings 2>&1 | head -50
cargo test --workspace --all-features

# Verify the pre-init coverage table exists and is populated
grep -n 'Pre-Init ABI Failure Map' project-plans/20260311/audio-heart/plan/10-control-api-hardening.md

# Verify every FFI export is accounted for against the table during execution review
grep -n '^pub extern "C" fn\|^pub unsafe extern "C" fn' rust/src/sound/heart_ffi.rs

# Verify WaitForSoundEnd handles raw u32 selector
grep -A 14 'fn wait_for_sound_end' rust/src/sound/control.rs
```

## Structural Verification Checklist
- [ ] `STREAM_INITIALIZED` AtomicBool added to stream.rs
- [ ] `is_initialized()` function exists
- [ ] `wait_for_sound_end` accepts u32, handles WAIT_ALL_SOURCES, paused-as-active, invalid-selector default, and shutdown timing
- [ ] P10 contains a function-by-function pre-init ABI failure map for the full FFI surface
- [ ] Pre-init guards added to all required FFI functions
- [ ] Each guard returns the correct ABI failure value from the coverage table

## Semantic Verification Checklist
- [ ] Paused source is treated as active for wait purposes
- [ ] Invalid channel index defaults to all-sources wait
- [ ] WAIT_ALL_SOURCES sentinel works at the FFI boundary and internal control path
- [ ] The raw selector width/sentinel normalization is recorded and tested against the C declaration
- [ ] Pre-init: LoadMusicFile returns null before init
- [ ] Pre-init: WaitForSoundEnd returns immediately before init at the FFI boundary
- [ ] Pre-init: void functions are silent no-ops before init
- [ ] Representative functions from each ABI failure class are tested
- [ ] QuitPosted breaks the wait loop, including when already true before entry
- [ ] Concurrent teardown path is safe and exits without touching released resources

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder" rust/src/sound/control.rs rust/src/sound/heart_ffi.rs rust/src/sound/stream.rs | head -20
```

## Success Criteria
- [ ] WaitForSoundEnd fully spec-compliant, including race-sensitive cases
- [ ] Pre-init guards on all required FFI functions with explicit coverage proof
- [ ] All tests pass

## Failure Recovery
- rollback: `git restore rust/src/sound/{control,stream,heart_ffi}.rs`
- blocking issues: Accessing pause_time requires careful locking in the wait loop

## Phase Completion Marker
Create: `project-plans/20260311/audio-heart/.completed/P10.md`
