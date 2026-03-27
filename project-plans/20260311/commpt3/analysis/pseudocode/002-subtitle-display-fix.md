# Pseudocode 002: Subtitle Display Bridge Fix

Plan ID: `PLAN-20260325-COMMPT3`
Requirements: REQ-SD-001, REQ-SD-002, REQ-SD-003, REQ-SD-004, REQ-SD-005
Implementation Phase: P04

## Problem

Current subtitle bridges in `rust_comm.c:562-576` create a circular route:
```
c_ClearSubtitles()  → rust_ClearSubtitles()  → SubtitleDisplay.clear() [Rust, no rendering]
c_CheckSubtitles()  → rust_CheckSubtitles()  → SubtitleDisplay.check() [Rust, no rendering]
c_RedrawSubtitles() → rust_RedrawSubtitles() → SubtitleDisplay.redraw() [Rust, no rendering]
```

The Rust `SubtitleDisplay` model has no access to C's drawing surface.
Subtitles never appear on screen.

## Fix Strategy

Since `SubtitleText`, `clear_subtitles`, `last_subtitle`, and `add_text` are
all static in `comm.c`, the subtitle implementations MUST live in `comm.c`
inside the `#ifdef USE_RUST_COMM` block (starting at line 1715). The
`rust_comm.c` bridge functions then forward to these `comm.c` functions.

## C Implementation: comm_ClearSubtitles (comm.c, #ifdef USE_RUST_COMM)

```
01: FUNCTION comm_ClearSubtitles()
02:   clear_subtitles = TRUE
03:   last_subtitle = NULL
04:   SubtitleText.pStr = NULL
05:   SubtitleText.CharCount = 0
06: END FUNCTION
```

## C Implementation: comm_CheckSubtitles (comm.c, #ifdef USE_RUST_COMM)

```
07: FUNCTION comm_CheckSubtitles()
08:   pStr = GetTrackSubtitle()
09:   baseline = CommData.AlienTextBaseline
10:   align = CommData.AlienTextAlign
11:
12:   IF pStr != SubtitleText.pStr
13:      OR SubtitleText.baseline.x != baseline.x
14:      OR SubtitleText.baseline.y != baseline.y
15:      OR SubtitleText.align != align THEN
16:     clear_subtitles = TRUE
17:     SubtitleText.baseline = baseline
18:     SubtitleText.align = align
19:     IF SubtitleText.pStr == pStr THEN
20:       log_add(log_Warning, "Dialog text and location changed out of sync")
21:     END IF
22:     SubtitleText.pStr = pStr
23:     IF pStr != NULL THEN
24:       SubtitleText.CharCount = ~0    // unlimited
25:     ELSE
26:       SubtitleText.CharCount = 0
27:     END IF
28:   END IF
29: END FUNCTION
```

## C Implementation: comm_RedrawSubtitles (comm.c, #ifdef USE_RUST_COMM)

```
30: FUNCTION comm_RedrawSubtitles()
31:   IF NOT optSubtitles THEN
32:     RETURN
33:   END IF
34:   IF SubtitleText.pStr != NULL THEN
35:     t = SubtitleText                  // copy struct
36:     add_text(1, &t)                   // C font rendering to screen
37:   END IF
38: END FUNCTION
```

## C Bridge: updated rust_comm.c forwarding

```
39: FUNCTION c_ClearSubtitles()
40:   CALL comm_ClearSubtitles()         // forward to comm.c implementation
41: END FUNCTION
42:
43: FUNCTION c_CheckSubtitles()
44:   CALL comm_CheckSubtitles()         // forward to comm.c implementation
45: END FUNCTION
46:
47: FUNCTION c_RedrawSubtitles()
48:   CALL comm_RedrawSubtitles()        // forward to comm.c implementation
49: END FUNCTION
```

## Declaration Requirements

```
50: IN rust_comm.h:
51:   DECLARE void comm_ClearSubtitles(void);
52:   DECLARE void comm_CheckSubtitles(void);
53:   DECLARE void comm_RedrawSubtitles(void);
54:   // These are implemented in comm.c, called from rust_comm.c
```

## Requirement-to-Line Contracts

| Requirement | Pseudocode Lines | Contract |
|---|---|---|
| REQ-SD-001 | 39-48 | Bridge forwards to `comm_*` (C), NOT `rust_*` (Rust FFI) |
| REQ-SD-002 | 01-06 | `comm_ClearSubtitles` sets clear_subtitles=TRUE, last_subtitle=NULL, pStr=NULL, CharCount=0 |
| REQ-SD-003 | 07-29 | `comm_CheckSubtitles` reads `GetTrackSubtitle()`, compares, updates SubtitleText |
| REQ-SD-004 | 30-37 | `comm_RedrawSubtitles` checks optSubtitles, calls `add_text(1, &t)` |
| REQ-SD-005 | N/A | Rust FFI exports unchanged — test-only (no pseudocode change needed) |

## Validation Points

- Lines 01-06: Exact match to `comm.c:1661-1667` (`ClearSubtitles`)
- Lines 07-29: Exact match to `comm.c:1670-1701` (`CheckSubtitles`)
- Lines 30-37: Exact match to `comm.c:1646-1657` (`RedrawSubtitles`)
- Line 19: Log warning for out-of-sync text/location matches C behavior
- Lines 39-48: Break circular routing — no more `rust_*` calls

## Rust-Side Changes

The Rust `rust_ClearSubtitles`, `rust_CheckSubtitles`, `rust_RedrawSubtitles`
FFI exports at `ffi.rs:825-847` remain unchanged for test use. In production,
they are no longer called by the subtitle bridge — the bridge goes directly
to C rendering via `comm.c`.

## Ordering Constraints

- `comm_CheckSubtitles`, `comm_ClearSubtitles`, `comm_RedrawSubtitles` must
  be placed in `comm.c` inside `#ifdef USE_RUST_COMM` (after line 1715)
  because they access static variables (`SubtitleText`, `clear_subtitles`,
  `last_subtitle`, `add_text`, `optSubtitles`)
- Declarations go in `rust_comm.h` so `rust_comm.c` can call them

## Error Handling

- `GetTrackSubtitle() == NULL`: Sets `CharCount = 0`, `RedrawSubtitles` skips
- `optSubtitles == false`: `RedrawSubtitles` returns immediately (player
  disabled subtitles in options)
