# Pseudocode 003: DoCommunication Response Dispatch Rewrite

Plan ID: `PLAN-20260325-COMMPT3`
Requirements: REQ-RL-001, REQ-RL-002, REQ-RL-003, REQ-RL-004, REQ-DC-001,
              REQ-DC-002, REQ-DC-003, REQ-DC-004, REQ-DC-005
Implementation Phase: P05

## Current Problem (ffi.rs:715-752)

`rust_DoCommunication` has a convoluted structure:

1. Line 723: calls `do_communication(&mut state)` which internally calls
   `player_response_input` and returns `Continue` or `Done`
2. Lines 732-747: the FFI wrapper then calls `player_response_input` AGAIN
   to detect if `Selected` occurred, consuming input twice per frame
3. Lines 742-746: lock-drop-before-callback relies on detecting the Selected
   state via the redundant second call
4. If the first `player_response_input` consumed the Select input, the second
   call will not see it — the pattern is unreliable

## Solution: Single-Pass State Machine

`do_communication` already returns `CommunicationResult::Continue` when
`player_response_input` returns `Selected`. The fix is to make
`do_communication` return a richer result that distinguishes between
"Continue because talking" and "Continue because Selected with callback".

## New CommunicationResult Enum

```
01: ENUM CommunicationResult
02:   Talking           // alien is talking, continue loop
03:   ResponseContinue  // player is browsing responses, continue loop
04:   Selected(fn, ref) // player selected a response, callback ready
05:   Done              // encounter finished
06: END ENUM
```

## Rewritten do_communication (talk_segue.rs)

```
07: FUNCTION do_communication(state: &mut CommState) -> CommunicationResult
08:   IF NOT state.is_talking_finished() THEN
09:     alien_talk_segue(state, WAIT_TRACK_ALL)
10:     RETURN CommunicationResult::Talking
11:   END IF
12:
13:   IF check_abort(state) THEN
14:     RETURN CommunicationResult::Done
15:   END IF
16:
17:   IF state.responses().count() == 0 THEN
18:     run_last_replay(state)
19:     RETURN CommunicationResult::Done
20:   END IF
21:
22:   // Show responses and handle input
23:   input_result = player_response_input(state)
24:
25:   MATCH input_result
26:     PlayerInputResult::Selected =>
27:       callback_info = select_response(state)
28:       IF callback_info IS Some((fn, ref)) THEN
29:         RETURN CommunicationResult::Selected(fn, ref)
30:       ELSE
31:         RETURN CommunicationResult::ResponseContinue
32:       END IF
33:     PlayerInputResult::Continue =>
34:       RETURN CommunicationResult::ResponseContinue
35:     PlayerInputResult::Summary =>
36:       RETURN CommunicationResult::ResponseContinue
37:     PlayerInputResult::Replay =>
38:       RETURN CommunicationResult::ResponseContinue
39:   END MATCH
40: END FUNCTION
```

## Rewritten rust_DoCommunication (ffi.rs)

```
41: FUNCTION rust_DoCommunication() -> c_int
42:   state = COMM_STATE.write()
43:
44:   result = do_communication(&mut state)
45:
46:   MATCH result
47:     CommunicationResult::Talking =>
48:       DROP state
49:       RETURN 1
50:
51:     CommunicationResult::ResponseContinue =>
52:       DROP state
53:       RETURN 1
54:
55:     CommunicationResult::Selected(callback_fn, response_ref) =>
56:       DROP state                    // CRITICAL: release lock before callback
57:       callback_fn(response_ref)     // C race script re-enters Rust FFI
58:       RETURN 1
59:
60:     CommunicationResult::Done =>
61:       DROP state
62:       RETURN 0                      // end DoInput loop
63:   END MATCH
64: END FUNCTION
```

## Lock Discipline Invariant

```
65: INVARIANT: COMM_STATE write lock lifecycle per rust_DoCommunication call
66:   ACQUIRE state = COMM_STATE.write()       // line 42
67:   CALL do_communication(&mut state)         // line 44
68:     (all state machine work happens here)
69:   MATCH result                              // line 46
70:     IF Selected: extract callback info from result
71:   DROP state                                // lines 48/52/56/61
72:   IF Selected: callback_fn(response_ref)    // line 57 — AFTER lock drop
73:   RETURN                                    // lines 49/53/58/62
74:
75: The write lock is ALWAYS dropped before any C callback invocation.
76: The C callback re-enters Rust through separate lock acquisitions:
77:   rust_NPCPhrase_cb    → COMM_STATE.write() (separate acquisition)
78:   rust_DoResponsePhrase → COMM_STATE.write() (separate acquisition)
79:   rust_SetSegue         → COMM_STATE.write() (separate acquisition)
80:   rust_DisablePhrase    → COMM_STATE.write() (separate acquisition)
81: No nested write locking occurs.
```

## Requirement-to-Line Contracts

| Requirement | Pseudocode Lines | Contract |
|---|---|---|
| REQ-DC-001 | 07-40, 41-64 | Single frame iteration — exactly one state machine step per call |
| REQ-DC-002 | 08-11 | Talking phase: `alien_talk_segue` only, no `player_response_input` |
| REQ-DC-003 | 22-39 | Response phase: `player_response_input` called exactly once |
| REQ-DC-004 | 17-20 | No responses → `run_last_replay` → `Done` |
| REQ-DC-005 | 13-15 | Abort/load check → immediate `Done` |
| REQ-RL-001 | 55-58 | Lock dropped before callback invocation |
| REQ-RL-002 | 26-29, 55-58 | Select → extract → drop → invoke sequence |
| REQ-RL-003 | 56-57, 65-81 | No lock held during C callback |
| REQ-RL-004 | 26-29 | Pre-callback work under lock via `select_response` |

## Validation Points

- Line 08-10: Talking phase runs exactly one `alien_talk_segue` iteration
- Line 13-14: Abort check happens after talking check (matches C order)
- Line 17-19: No-responses case runs last-replay then exits
- Line 23: `player_response_input` called exactly ONCE (not twice)
- Line 27: `select_response` called only when Selected — pre-callback work
  (stop track, clear subtitles, fade music, feedback, clear responses)
  happens while lock is held
- Line 56: Lock explicitly dropped before callback
- Line 57: Callback invoked with no lock held

## Error Handling

- `select_response` returns `None` if callback is NULL → treated as Continue
- CHECK_ABORT during callback → next frame's `do_communication` returns Done
- Double-input consumption eliminated by single `player_response_input` call

## Side Effects

- `do_communication` return type changes from `CommunicationResult` to a
  richer enum — existing callers (`rust_DoCommunication` in `ffi.rs`) must
  be updated
- Test code calling `do_communication` must match on new variants
- The old `CommunicationResult::Continue` variant is replaced by `Talking`,
  `ResponseContinue`, and `Selected`
