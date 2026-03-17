# Phase 09: Talk Segue & Main Loop

## Phase ID
`PLAN-20260314-COMM.P09`

## Prerequisites
- Required: Phase 08a completed
- Expected: encounter lifecycle, animation engine, track model, phrase state, glue layer, input bridge wrappers, and minimal display primitives required by the main loop all in place

## Requirements Implemented (Expanded)

### SS-REQ-006–011: Talk segue playback loop
**Requirement text**: On speech playback, enter talking loop. Poll subtitles, handle seek/skip/cancel. Pause animations during seek.

### CB-REQ-001–010: Callback ordering and reentrancy
**Requirement text**: All callbacks on main thread. Phrase callbacks fire after audio finishes. No nested callbacks. Lock discipline for reentrancy.

### RS-REQ-013, RS-REQ-014, RS-REQ-016: Response callback preconditions
**Requirement text**: On response selection: clear responses, display feedback, stop track, clear subtitles, fade music. If no responses, timeout with replay then exit.

### CB-REQ-005: State transition ordering
**Requirement text**: Phrases complete → subtitles clear → TalkingFinished → animation to silent → responses available.

### IN-REQ-011, CB-REQ-008–009: Lock discipline
**Requirement text**: No exclusive lock held during C callbacks. Audio thread never acquires comm state lock.

## Implementation Tasks

### Files to create

- `rust/src/comm/talk_segue.rs` — Talk segue and DoCommunication main loop
  - marker: `@plan PLAN-20260314-COMM.P09`
  - marker: `@requirement SS-REQ-006 through SS-REQ-011, CB-REQ-001 through CB-REQ-010`

  **Functions:**

  - `alien_talk_segue(state: &mut CommState, first_call: bool) -> CommResult<()>`
    - First call: init speech graphics, set colormap, draw initial alien frame, do intro transition, start music, init animations
    - Start track playback: `play_track()`
    - Set talking state
    - Transition to talking animation
    - Call inner `do_talk_segue()` for the playback loop
    - Post-playback: clear subtitles, set slider stop, transition to silent, fade music foreground

  - `do_talk_segue(state: &mut CommState) -> CommResult<TalkResult>`
    - **The main playback loop** (called via DoInput pattern):
    ```
    while playing_track():
        if check_abort(): return Abort
        if cancel_pressed():
            jump_track()  // skip to end
            return Ended
        if left_or_right_pressed():
            enter_seek_mode()
        if not seeking:
            poll_subtitles()
        if seeking:
            pause_animations()
        else:
            process_comm_animations(delta)
        update_speech_graphics(oscilloscope, slider)
        poll_pending_completion():
            if completion pending:
                callback = claim_completion()
                // LOCK DISCIPLINE: release lock before callback
                drop(state_guard)
                callback()
                state_guard = reacquire()
                commit_track_advancement()
        sleep_frame(1/60s)
    return Finished
    ```

  - `do_communication() -> CommResult<()>`
    - **The main dialogue state machine** (spec §7.3):
    ```
    loop:
        if not talking_finished:
            talk_segue(wait_track=true)
            continue
        if response_count == 0:
            timeout_with_replay()
            break
        // Player response phase
        selected = do_response_input()
        if selected is valid:
            select_response(selected)  // RS-REQ-016 preconditions
            break or continue based on callback results
    ```

  - `select_response(state: &mut CommState, index: usize) -> CommResult<()>`
    - Clear response display
    - Copy feedback text (RS-REQ-010)
    - Stop track (RS-REQ-016)
    - Clear subtitles (RS-REQ-016)
    - Fade music to background (RS-REQ-016)
    - Get response callback and ref from state
    - Clear responses (RS-REQ-016 — prior list cleared before callback)
    - **Release COMM_STATE lock** (CB-REQ-008)
    - Call response callback with response_ref: `callback(response_ref)`
    - **Reacquire COMM_STATE lock**
    - If new phrases queued: set talking_finished = false
    - If no responses and no phrases: conversation over

  - `timeout_with_replay(state: &mut CommState) -> CommResult<()>`
    - Wait with replay capability (Left to replay last phrase)
    - Timeout exits conversation

  - `do_response_input(state: &mut CommState) -> CommResult<Option<usize>>`
    - Input loop via DoInput pattern
    - Up: select_prev, redraw
    - Down: select_next, redraw
    - Select: return selected index
    - Cancel: show_conversation_summary (unless final battle, CV-REQ-006)
    - Left: replay last phrase (SS-REQ-012)

  - `replay_last_phrase(state: &mut CommState) -> CommResult<()>`
    - Get replay target from track model
    - If exists: replay via trackplayer (no callback re-fire, no history duplication)

### Display primitives pulled into this phase because the main loop depends on them

P09 is responsible for the **minimum visible behavior required for the loop to be meaningfully complete**. This phase therefore includes wiring and use of:

- response feedback display used by `select_response()`
- subtitle clear/update behavior used during talk playback and post-selection cleanup
- summary-entry plumbing invoked by Cancel during response selection
- speech-graphics state transitions needed by the talk loop (`play`, `seek`, `stop` slider states)

P10 is limited to completing overflow rendering, full response-list drawing/polish, oscilloscope rendering fidelity, and full summary-page rendering/navigation. P09a cannot pass unless the user-visible seams needed by the main loop already exist, even if their later polish remains in P10.

### Lock discipline pattern (critical)

Every C callback invocation in this phase MUST follow this pattern:

```rust
fn invoke_callback_safely(
    callback: extern "C" fn(u32),
    arg: u32,
) {
    // Precondition: COMM_STATE lock is held by caller
    // We cannot hold it during the callback because the callback may re-enter

    // 1. Drop the write guard
    // (caller must pass ownership of guard, or we use a pattern like:)
    drop(state_guard);

    // 2. Call C callback (may call rust_NPCPhrase, rust_DoResponsePhrase, etc.)
    callback(arg);

    // 3. Reacquire
    let state_guard = COMM_STATE.write();
}
```

For phrase callbacks (called from poll loop):
```rust
fn poll_and_dispatch_completion(state: &mut CommState) {
    // Check for pending completion (lock-free check via atomic)
    if c_PollPendingTrackCompletion() != 0 {
        let callback = state.get_pending_callback();
        if let Some(cb) = callback {
            // Release lock, call callback, reacquire
            drop(state); // actually need to restructure for guard management
            cb();
            state = COMM_STATE.write();
        }
        c_CommitTrackAdvancement();
    }
}
```

### Files to modify

- `rust/src/comm/state.rs`
  - Add `first_talk_call: bool` field for alien_talk_segue first-call tracking
  - Add `seek_state: SeekState` for seek mode tracking
  - Add the minimum display-state fields required by the P09 loop: response-feedback text, subtitle dirty/current state, summary-entry state, and slider/talk UI mode
  - marker: `@plan PLAN-20260314-COMM.P09`

- `rust/src/comm/ffi.rs`
  - Add/update `rust_DoCommunication() -> c_int`
  - Add `rust_TalkSegue(wait: c_int) -> c_int`
  - Add any minimal display-bridge exports needed for feedback text, subtitle clear/update, and summary entry before P10 polish
  - marker: `@plan PLAN-20260314-COMM.P09`

- `rust/src/comm/mod.rs`
  - Add `pub mod talk_segue;`
  - Ensure minimal display helpers consumed by P09 are available here even if their richer rendering behavior is completed in P10

### Concrete seam ownership and source-path mapping

- input wrappers consumed by this phase are defined earlier in `sc2/src/uqm/rust_comm.c`, but their authoritative behavior comes from `sc2/src/uqm/comm.c`:
  - `DoTalkSegue()` for pulsed/current input semantics and seek behavior
  - `PlayerResponseInput()` for response navigation, replay, summary-entry, and menu-sound behavior
  - `DoLastReplay()` for timeout/replay loop behavior
  - `TalkSegue()` / `DoCommunication()` for `DoInput(...)` loop shape
- track-completion seam:
  - `c_PollPendingTrackCompletion()` / `c_CommitTrackAdvancement()` wrappers must be anchored to the audio-heart seam established in preflight and P06 verification artifacts
- display primitives used now by the loop are sourced from existing C-visible behavior in `sc2/src/uqm/comm.c`:
  - `FeedbackPlayerPhrase()` for selected-response feedback text
  - `ClearSubtitles()` / `CheckSubtitles()` / `RedrawSubtitles()` for subtitle update/clear semantics
  - `SelectConversationSummary()` for summary-entry contract
  - `InitSpeechGraphics()` / `UpdateSpeechGraphics()` for play/seek/stop slider-state behavior

### Intermediate build invariants (must hold after P09/P09a)

- Rust is authoritative for talk-loop control flow, callback dispatch ordering, and response-selection state transitions in Rust mode.
- C remains authoritative only for wrappers and legacy fallback mode.
- Mixed mode after this phase is valid only if the following user-visible behaviors already work in Rust mode:
  - selecting a response displays feedback text before callback dispatch,
  - subtitles clear/update correctly during playback and selection,
  - Cancel enters the summary path when allowed, and
  - slider/talk state reflects play/seek/stop transitions.
- P10 may improve rendering completeness and polish, but must not be the first phase where those visible seams exist.

### Pseudocode traceability
- Uses pseudocode lines: 220-265 (talk segue, do_communication), 280-286 (response input), 305-315 (lock discipline)

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] `talk_segue.rs` created with all functions
- [ ] Lock release before every C callback (response, phrase, lifecycle)
- [ ] Lock reacquire after every C callback return
- [ ] Pending completion poll in playback loop
- [ ] CB-REQ-005 ordering: phrases→subtitles→finished→animation→responses
- [ ] Response preconditions (RS-REQ-016) applied before callback
- [ ] Replay uses replay target, no callback re-fire
- [ ] Summary access from Cancel during response selection
- [ ] Minimal display primitives required by the loop exist in or before this phase rather than first landing in P10

## Semantic Verification Checklist (Mandatory)
- [ ] Test: talk segue enters on NPC speech, exits on completion
- [ ] Test: cancel during talk jumps to end
- [ ] Test: seek pauses animations, resume restores
- [ ] Test: subtitle poll updates display during playback
- [ ] Test: phrase completion callback fires between phrases (not during)
- [ ] Test: response selection clears prior state (RS-REQ-016)
- [ ] Test: response callback receives correct response_ref
- [ ] Test: new phrases after callback re-enter talk segue
- [ ] Test: no responses + no phrases = conversation exit
- [ ] Test: timeout with replay works (Left replays)
- [ ] Test: lock not held during C callback (no deadlock with re-entrant calls)
- [ ] Test: audio thread signal doesn't require comm lock (CB-REQ-009)
- [ ] Test: CB-REQ-005 ordering verified in sequence
- [ ] Test: selected-response feedback text is visible before callback dispatch
- [ ] Test: Cancel enters summary path in response selection without waiting for P10-only code
- [ ] Test: subtitle clear/update behavior visible during the P09 main loop

## Deferred Implementation Detection (Mandatory)

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/comm/talk_segue.rs
```

## Success Criteria
- [ ] Full talk segue and main loop implemented
- [ ] Lock discipline verified
- [ ] Callback ordering correct
- [ ] Main-loop-visible UI dependencies are functionally present by the end of this phase
- [ ] All tests pass

## Failure Recovery
- rollback: `git restore rust/src/comm/talk_segue.rs rust/src/comm/state.rs`
- blocking: P08 input bridge and minimal display seams must already exist; do not defer them to a later discovery in P10/P11

## Phase Completion Marker
Create: `project-plans/20260311/comm/.completed/P09.md`
