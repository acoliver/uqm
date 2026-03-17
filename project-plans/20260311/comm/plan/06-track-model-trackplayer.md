# Phase 06: Track Model & Trackplayer Integration

## Phase ID
`PLAN-20260314-COMM.P06`

## Prerequisites
- Required: Phase 05ba completed
- Expected: FFI signatures corrected, SpliceTrack has timestamps/callback params, and the C-side trackplayer wrapper seam exists in `sc2/src/uqm/rust_comm.c` / `sc2/src/uqm/rust_comm.h`
- **Dependency**: Must determine during preflight (P00.5) whether trackplayer is C-owned or Rust-owned. This phase's integration approach depends on that answer.

## Requirements Implemented (Expanded)

### TP-REQ-001: Phrase as logical unit
**Requirement text**: Each SpliceTrack call produces exactly one phrase in the trackplayer's queue. Phrases are the unit for completion callbacks, subtitle transitions, history entries, and skip advancement.

### TP-REQ-003, TP-REQ-010, TP-REQ-012: Phrase completion and callback delivery
**Requirement text**: Phrase completion fires callback before next phrase begins. Callbacks execute on main thread. The next phrase does not become current until after the callback returns.

Behavior contract:
- GIVEN: Two phrases queued with callbacks
- WHEN: First phrase finishes playing
- THEN: First callback fires on main thread, first phrase committed, then second phrase becomes current

### TP-REQ-008: Subtitle history at queue time
**Requirement text**: History recorded at queue time, not playback time. Skipped phrases appear once. Replay doesn't duplicate.

### TP-REQ-009: Replay target
**Requirement text**: Replay target is most recently committed phrase with content. Updates only on commit, not on queue or pending.

### TP-REQ-007: StopTrack discards pending
**Requirement text**: StopTrack halts without firing callbacks for unplayed phrases.

## Implementation Tasks

### Owner-boundary rule

This phase must preserve the specification's ownership boundary: the authoritative trackplayer owns pending-completion state, subtitle-history enumeration, replay-target semantics, and phrase advancement. Comm integrates with those APIs; it does not create a competing shadow owner of those behaviors.

### Approach A: Trackplayer is C-owned (likely scenario)

Rust comm calls C trackplayer through FFI. The existing Rust `TrackManager` becomes a thin wrapper that delegates to C functions and adapts script-facing comm calls to trackplayer operations.

### Approach B: Trackplayer is Rust-owned

Rust comm calls Rust trackplayer directly, but still consumes the trackplayer-owned enumeration/pending-completion/replay APIs rather than duplicating them inside comm state.

### Files to create / modify (Approach A — C trackplayer via FFI)

- `rust/src/comm/track.rs` — Major redesign
  - Replace synthetic timeline with a thin integration layer over authoritative trackplayer operations
  - Add lightweight Rust-side phrase request structs only as needed to marshal parameters into the trackplayer
  - Delegate actual playback, pending-completion ownership, replay-target semantics, and subtitle-history enumeration to the trackplayer via FFI extern declarations
  - marker: `@plan PLAN-20260314-COMM.P06`
  - marker: `@requirement TP-REQ-001, TP-REQ-003, TP-REQ-008, TP-REQ-009, TP-REQ-012`

- `rust/src/comm/track.rs` — FFI extern declarations for C trackplayer
  ```rust
  extern "C" {
      fn c_SpliceTrack(clip: u32, text: *const c_char, timestamps: *const f32, ts_count: u32, callback: Option<extern "C" fn()>);
      fn c_SpliceMultiTrack(clips: *const u32, clip_count: u32, text: *const c_char);
      fn c_PlayTrack();
      fn c_StopTrack();
      fn c_JumpTrack();
      fn c_PlayingTrack() -> u32;
      fn c_GetTrackSubtitle() -> *const c_char;
      fn c_GetFirstTrackSubtitle() -> *const c_void;
      fn c_GetNextTrackSubtitle(iter: *const c_void) -> *const c_void;
      fn c_GetTrackSubtitleText(iter: *const c_void) -> *const c_char;
      fn c_FastForward_Page();
      fn c_FastForward_Smooth();
      fn c_FastReverse_Page();
      fn c_FastReverse_Smooth();
      fn c_PollPendingTrackCompletion() -> i32;  // returns 1 if completion pending
      fn c_CommitTrackAdvancement();
      fn c_ReplayLastPhrase();
  }
  ```

- `rust/src/comm/summary.rs` — Conversation summary model/pagination over trackplayer enumeration
  - marker: `@plan PLAN-20260314-COMM.P06`
  - marker: `@requirement SS-REQ-013, SS-REQ-014, SS-REQ-015, SS-REQ-017`
  - `enumerate_trackplayer_history() -> Vec<String>` — consumes trackplayer enumeration APIs in queue order
  - `paginate_subtitles(entries: &[String], ...) -> Vec<Page>` — summary pagination/model only
  - `clear_cached_pages()` — comm-local cached pagination state only, not subtitle ownership

### Files to modify

- `rust/src/comm/state.rs`
  - Add only comm-local fields needed for summary view state and transient UI bookkeeping
  - Do **not** introduce comm-owned `SubtitleHistory`, `PendingCompletion`, or `replay_target` as authoritative state in the C-owned trackplayer path
  - Add `poll_pending_completion()` helper that claims pending completion from trackplayer, dispatches callback, then calls `CommitTrackAdvancement()`
  - marker: `@plan PLAN-20260314-COMM.P06`

- `rust/src/comm/ffi.rs`
  - Update `rust_SpliceTrack` / `rust_SpliceMultiTrack` to delegate to the authoritative trackplayer path
  - Add `rust_GetFirstTrackSubtitle`, `rust_GetNextTrackSubtitle`, `rust_GetTrackSubtitleText` FFI exports if C-facing glue needs them
  - Add `rust_ReplayLastPhrase` FFI export
  - marker: `@plan PLAN-20260314-COMM.P06`

### Invariant if limited local mirroring is unavoidable

If a small amount of comm-local mirroring is temporarily required for tests or adapters, it must be explicitly documented as derived cache state only, with these invariants:
- the trackplayer remains source of truth,
- mirrored state is rebuilt from trackplayer-owned APIs,
- no comm-local mirror may determine callback order, replay target, or subtitle history contents independently.

### Pseudocode traceability
- Uses pseudocode lines: references to 59, 63, 67, 74 (phrase queuing), 83-86 (splice), 234 (pending completion polling), 291-300 (summary enumeration)

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] TrackManager redesigned as integration layer rather than synthetic owner
- [ ] No comm-owned shadow `SubtitleHistory` source of truth for the C-owned trackplayer path
- [ ] No comm-owned shadow `PendingCompletion` source of truth for the C-owned trackplayer path
- [ ] No comm-owned shadow `replay_target` source of truth for the C-owned trackplayer path
- [ ] Trackplayer FFI extern declarations present
- [ ] Summary model/pagination module present and fed from trackplayer enumeration

## Semantic Verification Checklist (Mandatory)
- [ ] Test: SpliceTrack creates one phrase per call
- [ ] Test: SpliceMultiTrack merges clips into single phrase
- [ ] Test: subtitle history comes from trackplayer queue-time recording, not playback-time reconstruction
- [ ] Test: skipped phrase appears exactly once in history
- [ ] Test: replay does not duplicate history entries
- [ ] Test: StopTrack discards pending callbacks
- [ ] Test: JumpTrack fires current phrase callback
- [ ] Test: pending completion remains single-slot at the trackplayer seam (no double completion)
- [ ] Test: replay target updates only on commit, not on queue
- [ ] Test: replay target is last committed phrase with content
- [ ] Test: subtitle enumeration returns all entries in queue order
- [ ] Test: comm-local summary pagination cache reset does not affect underlying trackplayer history

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/comm/track.rs rust/src/comm/summary.rs
```

## Success Criteria
- [ ] Phrase-level semantics implemented through authoritative trackplayer integration
- [ ] Subtitle history and replay target are sourced from the trackplayer owner boundary
- [ ] Trackplayer integration approach selected and wired
- [ ] All TP-REQ tests pass

## Failure Recovery
- rollback: `git restore rust/src/comm/track.rs rust/src/comm/state.rs`
- blocking: If trackplayer FFI functions don't exist yet, they must be created as C wrappers first

## Phase Completion Marker
Create: `project-plans/20260311/comm/.completed/P06.md`
