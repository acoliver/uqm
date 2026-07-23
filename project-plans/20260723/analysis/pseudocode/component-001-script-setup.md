# Pseudocode 001: Script, Options, Baseline, Setup

```text
001: SNAPSHOT git status, binary diff, untracked list, HEAD, strict gate output
002: RECORD cargo-test -luqm_rust linker blocker and full actual Clippy diagnostic scale
003: REMEDIATE fmt/link/test/Clippy forward without weakening lints; preserve user edits
004: EXECUTE P00 probes: atomics, datagram, file/directory, process identity, dummy+hidden SDL,
005:   ABI SDL/MUSTLOCK, actual VControl binding query, production archive/link/rerun order
006: REQUIRE check/fmt/clippy/test + minimal linked harness + every probe exit zero
007: SEPARATE format-only evidence from semantic fixes; test each semantic fix
005: PARSE CLI automation pair; reject incomplete pair before game init
006: IF pair absent RETURN Inactive without output side effects
007: READ script bytes; UTF-8 decode; strict JSON deserialize
008: VALIDATE v1 root, positive budgets, closed actions, final finish
009: VALIDATE six MenuKey values, labels, counts, activity masks
010: ACCEPT semantic transition only as typed RestartMenuItem from/to
011: VERIFY linked capabilities include owns-main/threads/gfx/comm/restart
012: CREATE unique output files exclusively and initialize trace
013: DIGEST executable, script, content manifest, build config, initial config
014: INSTALL Running runtime and emit run_start
015: CALL run_uqm only after complete successful setup
016: ON setup error close owned files and return nonzero without game init
```
