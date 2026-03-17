# Phase 09a: Talk Segue & Main Loop Verification

## Phase ID
`PLAN-20260314-COMM.P09a`

## Prerequisites
- Required: Phase 09 completed

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] `talk_segue.rs` exists and is registered
- [ ] `do_communication()` implements the main dialogue state machine
- [ ] `talk_segue()` implements the playback loop
- [ ] `select_response()` implements RS-REQ-016 preconditions
- [ ] `do_response_input()` implements response navigation
- [ ] Lock release/reacquire around all callbacks

## Semantic Verification Checklist

### Talk Segue
- [ ] `test_talk_segue_enters_on_speech` — segue activated when phrases queued
- [ ] `test_talk_segue_cancel_skips` — cancel jumps to track end
- [ ] `test_talk_segue_seek_pauses_anim` — animations pause during seek
- [ ] `test_talk_segue_seek_resumes` — animations resume after seek
- [ ] `test_talk_segue_subtitle_poll` — subtitles update each frame

### Main Loop (DoCommunication)
- [ ] `test_do_comm_talks_then_responds` — alternates NPC talk → player response
- [ ] `test_do_comm_no_responses_exits` — no responses = conversation over
- [ ] `test_do_comm_callback_queues_more` — callback adds phrases, re-enters talk
- [ ] `test_do_comm_exit_via_segue` — setSegue triggers conversation end

### Response Selection
- [ ] `test_select_response_preconditions` — RS-REQ-016 all preconditions met
- [ ] `test_select_response_clears_prior` — old responses cleared before callback
- [ ] `test_select_response_callback_ref` — callback receives correct ref
- [ ] `test_select_response_feedback_text` — chosen text displayed

### Callback Ordering (CB-REQ-005)
- [ ] `test_ordering_phrases_complete_first` — all phrases done before finished
- [ ] `test_ordering_subtitles_cleared` — subtitles cleared after phrases
- [ ] `test_ordering_talking_finished` — TalkingFinished set after clear
- [ ] `test_ordering_anim_to_silent` — animation transitions after finished
- [ ] `test_ordering_responses_available` — responses shown last

### Lock Discipline
- [ ] `test_no_lock_during_response_callback` — lock released during callback
- [ ] `test_no_lock_during_phrase_callback` — lock released during callback
- [ ] `test_reentrant_safe` — callback calls NPCPhrase without deadlock
- [ ] `test_audio_thread_no_lock` — audio signals via atomic, not lock

## Deferred Implementation Detection

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/comm/talk_segue.rs
```

## Phase Completion Marker
Create: `project-plans/20260311/comm/.completed/P09a.md`
