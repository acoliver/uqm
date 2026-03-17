# Phase 06a: Track Model & Trackplayer Integration Verification

## Phase ID
`PLAN-20260314-COMM.P06a`

## Prerequisites
- Required: Phase 06 completed

## Verification Commands

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
```

## Structural Verification Checklist
- [ ] TrackManager is an integration layer over the authoritative trackplayer, not a pure synthetic owner
- [ ] Summary module exists and is registered
- [ ] Pending-completion handling relies on trackplayer-owned single-slot semantics
- [ ] Replay-target handling relies on trackplayer-owned commit semantics
- [ ] Trackplayer FFI extern declarations present (or direct Rust calls)
- [ ] Summary enumeration API exists and is sourced from trackplayer history

## Semantic Verification Checklist

### Phrase Model
- [ ] `test_splice_creates_phrase` — each SpliceTrack call creates one phrase
- [ ] `test_multi_splice_single_phrase` — SpliceMultiTrack merges into one phrase
- [ ] `test_splice_text_phrase` — text-only phrase without audio
- [ ] `test_phrase_completion_fires_callback` — callback invoked at phrase end
- [ ] `test_phrase_completion_main_thread` — callback not on audio thread

### Subtitle History
- [ ] `test_history_recorded_at_queue_time` — present even if never played
- [ ] `test_history_skip_no_duplicate` — skipped phrase once in history
- [ ] `test_history_replay_no_duplicate` — replay doesn't add entries
- [ ] `test_history_queue_order` — order matches script emission order
- [ ] `test_history_enumeration_source_of_truth` — summary obtains entries via trackplayer enumeration APIs

### Replay Target
- [ ] `test_replay_target_last_committed` — reflects last committed phrase with content
- [ ] `test_replay_target_not_queue` — doesn't update on queue
- [ ] `test_replay_target_not_pending` — doesn't update on pending completion
- [ ] `test_replay_target_multi_commit` — last of chain is replay target

### Stop/Jump
- [ ] `test_stop_discards_pending` — pending callbacks not fired
- [ ] `test_jump_fires_current` — current phrase callback fires
- [ ] `test_jump_next_stays_queued` — phrases after current remain

## Deferred Implementation Detection

```bash
grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented" rust/src/comm/track.rs rust/src/comm/summary.rs
```

## Phase Completion Marker
Create: `project-plans/20260311/comm/.completed/P06a.md`
