# Pseudocode 004: Summary Guard and Stale Marker Elimination

Plan ID: `PLAN-20260325-COMMPT3`
Requirements: REQ-CS-002, REQ-CS-003, REQ-SM-001, REQ-SM-002
Implementation Phase: P06

## rust_ShowConversationSummary Rewrite (ffi.rs:860-889)

```
01: FUNCTION rust_ShowConversationSummary() -> c_int
02:   #[cfg(not(test))]
03:   {
04:     // Production: delegate to C-side summary which uses DoInput + C rendering
05:     extern "C" { fn c_SelectConversationSummary(); }
06:     CALL c_SelectConversationSummary()
07:     RETURN 1
08:   }
09:
10:   #[cfg(test)]
11:   {
12:     // Test mode: use Rust SummaryView for unit testing
13:     COMM_STATE.write().rebuild_summary()
14:     view = SummaryView::new(lines_per_page=10)
15:     total = view.init(COMM_STATE.read().summary())
16:     IF total == 0 THEN RETURN 1
17:     LOOP
18:       MATCH view.advance_page()
19:         NextPage => CONTINUE
20:         Exit => RETURN 1
21:         Aborted => RETURN 0
22:     END LOOP
23:   }
24: END FUNCTION
```

## Stale Markers to Eliminate

### talk_segue.rs line 1002

```
25: CURRENT: "// The colormap ptr comes from the C CommData; pass null for now."
26: ACTION:  Remove comment entirely. The new c_SetColorMapFromCommData() call
27:          makes it self-documenting. (Done in P03.)
```

### ffi.rs lines 879-881

```
28: CURRENT: "// Advance through pages until Exit or Abort (abort not yet wired — use
29:          // a simple bounded loop so we can't spin forever in production if
30:          // input handling is not yet implemented)."
31: ACTION:  Replace with production delegation to c_SelectConversationSummary.
32:          The Rust model loop is retained only under #[cfg(test)].
```

### Additional markers to verify

```
33: ffi.rs:402 — "the USE_RUST_COMM stubs in commanim.c"
34:   STATUS: Reference to existing C code, not deferred work → KEEP (REQ-SM-002)
35:
36: phrase_state.rs:30 — "not yet disabled this encounter"
37:   STATUS: Doc comment describing design semantics, not deferred work → KEEP (REQ-SM-002)
38:
39: state.rs:88 — "not yet initialized"
40:   STATUS: Doc comment describing ~0 sentinel value, not deferred work → KEEP (REQ-SM-002)
```

## Verification Sweep

```
41: COMMAND grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|will be implemented\|not yet\|stub" rust/src/comm/*.rs
42:   FILTER OUT: #[cfg(test)] blocks, doc comments describing design (not deferred work),
43:               references to existing C code patterns (e.g. "stubs in commanim.c")
44:   EXPECTED: Zero matches in production code paths
45:
46: COMMAND grep -RIn "TODO\|FIXME\|HACK\|placeholder\|for now\|stub\|P11: Stub" sc2/src/uqm/rust_comm.c
47:   EXPECTED: Zero matches
```

## Requirement-to-Line Contracts

| Requirement | Pseudocode Lines | Contract |
|---|---|---|
| REQ-CS-002 | 01-08 | Production delegates to `c_SelectConversationSummary()` |
| REQ-CS-003 | 10-23 | `SummaryView` under `#[cfg(test)]` only |
| REQ-SM-001 | 25-32, 41-47 | Zero deferred markers in production paths |
| REQ-SM-002 | 33-40 | Exemptions: C reference, design semantics, sentinel description |

## Ordering Constraints

- Lines 25-26: The "for now" comment in `set_colormap` is removed as part of
  P03 (colormap bridge fix), not P06. P06 is the cleanup verification pass.
- Lines 28-31: The "not yet wired" comments are removed as part of P06
  (this phase) when `rust_ShowConversationSummary` is rewritten.
- P06 should verify that ALL markers are gone, including those fixed in
  earlier phases.

## Side Effects

- `rust_ShowConversationSummary` production path changes from Rust model to
  C delegation — no user-visible difference (it was dead code)
- The `c_SelectConversationSummary` extern declaration is needed in the
  `#[cfg(not(test))]` block — it may already exist in talk_segue.rs's
  c_bridge module. Verify and add if needed.
