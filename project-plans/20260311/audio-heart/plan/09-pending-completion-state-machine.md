# Phase 09: Pending-Completion Integration Proof & Provider State Machine

## Phase ID
`PLAN-20260314-AUDIO-HEART.P09`

## Prerequisites
- Required: Phase P08a completed
- Expected: PLRPause ref-matching working, loaders canonical

## Requirements Implemented (Expanded)

### Pending-completion provider-side contract
**Requirement text**: The trackplayer maintains a single-slot pending completion state. The subsystem shall expose claim-and-clear polling and commit operations for the comm subsystem's main-thread poll loop rather than invoking caller-visible progression callbacks directly on the decoder thread.

Behavior contract:
- GIVEN: on_tagged_buffer fires for the last chunk of a phrase
- WHEN: A completion is stored in pending state
- THEN: the consumer-side main-thread poll loop can claim it exactly once and later commit advancement

- GIVEN: Completion has been claimed and executed on main thread
- WHEN: commit is called
- THEN: `PlayingTrack()` updates, next phrase subtitle state becomes visible, and the active phrase advances coherently

- GIVEN: StopTrack is called while a completion is pending
- WHEN: StopTrack executes
- THEN: pending completion is discarded without invoking callback

### Phrase-to-chunk mapping
**Requirement text**: Completion is emitted only after the last chunk of the logical phrase finishes playback. Subtitle page transitions within a single phrase are internal presentation events, not phrase-completion boundaries.

Why it matters:
- The comm subsystem's main-thread poll loop depends on these operations
- Without this state machine, phrase callbacks fire on the decoder thread (deadlock risk)
- This is the bridge between audio-heart (provider) and comm (consumer)

## Implementation Tasks

### Step 1 — Integration proof (mandatory before ABI changes)

Before adding new exports/header declarations, prove the consumer adoption path:
- Locate the exact comm-side polling loop(s) that currently observe subtitle / phrase progression
- Identify where `PollPendingTrackCompletion` and `CommitTrackAdvancement` would be called from in that loop
- Determine whether an existing ABI-visible callback representation already exists or whether a new opaque/bridged representation is required
- Define the exact C ABI signature(s) only after the bridging representation is understood
- Record whether the current replacement architecture allows adding these exports without breaking existing consumers

**Required output:**
- file:line references for the consumer poll/adoption path in comm
- chosen ABI bridging representation and rationale
- explicit note on whether header changes are required or whether an existing surface can be reused

### Step 2 — TDD for provider state machine

#### `rust/src/sound/trackplayer.rs` — add tests
- `test_pending_completion_initially_none` — no pending completion at start
- `test_poll_pending_returns_none_when_empty` — poll returns None
- `test_record_pending_completion` — after recording, poll returns Some-equivalent state
- `test_poll_clears_pending` — after poll, second poll returns None
- `test_stop_track_clears_pending` — stop discards without invoking
- `test_commit_advancement_updates_track_num` — PlayingTrack updates after commit
- `test_pending_not_overwritten_before_claim` — second record deferred if slot occupied
- `test_seek_defers_advancement_when_pending_slot_occupied` — seek behavior respects occupied pending slot
- marker: `@plan PLAN-20260314-AUDIO-HEART.P09`

### Step 3 — Provider implementation

#### `rust/src/sound/trackplayer.rs`
- Add pending-completion state to `TrackPlayerState`
- The stored representation must match the integration-proof decision rather than assuming `Option<Box<dyn Fn(i32) + Send>>` is ABI-transferable
- Add function(s) to claim-and-clear pending completion state
- Add `commit_track_advancement()`
- Modify `on_tagged_buffer`:
  - After validating tag and updating cur_sub_chunk
  - If chunk has completion semantics and this is the last chunk of the logical phrase:
    - If pending slot already occupied, defer (do not overwrite)
    - Else record pending completion metadata
- Modify `stop_track()`:
  - Clear pending completion state before dropping chunk sequence
- marker: `@plan PLAN-20260314-AUDIO-HEART.P09`

#### `rust/src/sound/heart_ffi.rs`
- Only if Step 1 proves new exports are required:
  - add FFI exports for pending-completion poll/commit using the exact ABI representation selected in Step 1
- If a different integration path is selected, document and implement that path instead
- marker: `@plan PLAN-20260314-AUDIO-HEART.P09`

#### `sc2/src/libs/sound/audio_heart_rust.h`
- Update only if Step 1 proves header additions are required
- New declarations must match the final chosen ABI representation exactly
- marker: `@plan PLAN-20260314-AUDIO-HEART.P09`

### Signature / ABI normalization requirements
- The phase must specify final public signatures consistently
- If a poll API returns an opaque token/pointer/enum rather than a callback object, state that explicitly
- If `CommitTrackAdvancement` depends on previously claimed metadata, define that dependency clearly
- Do not leave a `void*` placeholder in the plan without explaining what it points to and who owns it

### Pseudocode traceability
- Implements PC-07 lines 01-27, revised by the integration proof's chosen ABI representation

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings 2>&1 | head -50
cargo test --workspace --all-features

# Verify integration-proof references recorded
grep -n 'PollPendingTrackCompletion\|CommitTrackAdvancement\|CheckSubtitles' project-plans/20260311/audio-heart/plan/09-pending-completion-state-machine.md
```

## Structural Verification Checklist
- [ ] Integration proof identifies exact comm call path before any ABI/header change is finalized
- [ ] Pending state added to TrackPlayerState in a representation justified by the ABI design
- [ ] Claim-and-clear function exists
- [ ] `commit_track_advancement` function exists
- [ ] `stop_track` clears pending state
- [ ] `on_tagged_buffer` records completion only for logical phrase completion
- [ ] FFI/header changes are made only if the integration proof shows they are necessary

## Semantic Verification Checklist
- [ ] Poll/claim returns empty when nothing is pending
- [ ] Claim/clear is atomic from the provider perspective
- [ ] StopTrack clears pending without invoking
- [ ] Commit advancement updates PlayingTrack
- [ ] Second pending recording deferred if slot occupied
- [ ] Seek behavior is defined when a completion is already pending
- [ ] No callback invocation on decoder thread while holding track state locks

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder" rust/src/sound/trackplayer.rs rust/src/sound/heart_ffi.rs sc2/src/libs/sound/audio_heart_rust.h | head -20
```

## Success Criteria
- [ ] Comm adoption path is proven, not assumed
- [ ] Pending-completion state machine works
- [ ] All tests pass
- [ ] Any ABI/header additions are justified and internally consistent

## Failure Recovery
- rollback: restore only files touched after the integration-proof step
- blocking issues: exact ABI bridging representation may need adjustment; comm consumer path may require coordinated changes

## Phase Completion Marker
Create: `project-plans/20260311/audio-heart/.completed/P09.md`
