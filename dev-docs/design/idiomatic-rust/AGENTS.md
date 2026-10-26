# Instructions for agents implementing a work unit

You implement one work unit of the idiomatic Rust migration as one PR. The design is
in this directory: [architecture.md](architecture.md) (target),
[quality-gates.md](quality-gates.md) (checks), [plan.md](plan.md) (phases) and
[work-items.md](work-items.md) (the units). Your unit's GitHub issue is the source of
truth for its scope and acceptance.

## Pick up a unit

1. Choose an open issue labelled `ir-unit`, without `in-progress` or `blocked`, whose
   "Depends on" issues are all closed. Prefer `critical-path` units.
2. Apply the shared-file rule against every issue labelled `in-progress`:
   - your `touches` paths are reserved for you; a directory entry covers everything
     below it;
   - you may not start if any of your `touches` overlaps another unit's `touches` or
     `shared`, or any of your `shared` paths overlaps another unit's `touches`;
   - `shared` paths (registries, dispatchers, tables where each unit edits only its
     own entries) may be held by several units. When one of them merges, the others
     rebase on `main` and rerun their acceptance before merging;
   - `rust/xtask/quality-baseline.toml` is shared by every unit. After a rebase, take
     `main`'s version and regenerate it with `cargo xtask quality move` and
     `--tighten`; never merge it by hand.
   If the rule blocks you, pick another unit.
3. Add the `in-progress` label and comment that you are starting.
4. Work in a worktree under `tmp/wt/<unit-id>` on branch `ir/<unit-id>-<short-name>`
   from current `main`. At most five worktrees exist; remove yours, and its target
   directory, when the PR merges.
5. Read the issue, then the documents it lists under "Read first".

## Rules

- Change no game behaviour. RNG draws, element order, sound queueing, integer widths,
  wrapping arithmetic, save bytes and netplay bytes stay as they are.
- Do not weaken a test, assertion, threshold, lint or gate. Do not delete a test that
  still covers live behaviour; rewrite it against the new API with the same data.
- Add no mutable global state: no new `static mut`, `thread_local!`, `OnceLock`,
  `LazyLock`, `lazy_static`, or `static` holding atomics, locks or cells. Immutable
  tables are fine. Moving an existing global, or adding a field to one, is allowed
  when the unit says so. The plan adds exactly two: U0-05's set-once replay time
  source `TIME_SOURCE` in `time::source` (U1-04 moves it into `uqm-engine`, U7-05
  removes it) and the `app::bridge` context slot of U1-02 (U6-04 removes it).
- Change, migrate and delete in one PR; the PR is not done until the replaced code
  is gone. Your unit introduces the new representation or API, moves every user of
  the thing it replaces, and deletes the replaced code. Every item under the issue's
  Deletes must be absent and its zero-count check must pass. No PR leaves code with
  zero callers or two live implementations of the same thing; no compatibility shim
  or "remove later" comment survives a merge.
- Delete a path only after every consumer of it has moved, and in the same PR as the
  last of them. Adding a new API in one PR and migrating its users in another is not
  allowed.
- Adapters: a transitional adapter may exist only while live, unconverted users of the
  old path remain in code that a later unit owns. Use only the adapters listed in
  work-items.md (Transitional adapters); the unit that converts an adapter's last user
  deletes it. Do not add another; if your unit seems to need one, stop and file.
- Gate counters only fall. Never edit `rust/xtask/quality-baseline.toml` by hand; use
  `cargo xtask quality --tighten` and `cargo xtask quality move`. Never apply
  `quality-growth-approved`.
- Stay inside the unit's scope and out-of-scope lists. Mechanical moves and semantic
  changes go in separate commits of the same PR; a mechanical move changes nothing
  inside moved files except visibility, and a crate move is the unit's last commit.
- Fail fast: a broken invariant panics with context; no defensive fallbacks for
  states our own code cannot produce.
- Never enable Rust incremental compilation. `rust/.cargo/config.toml` keeps
  `[build] incremental = false`; do not set `CARGO_INCREMENTAL` or
  `incremental = true` in a profile. It costs disk and does not help this workload.
  The intended build-speed gain is crate-level reuse after the splits.
- Keep build outputs and logs inside the repository under `tmp/`. Kill earlier game
  processes (`pkill -x uqm`) before a native run.

### Unsafe during the migration

The end state allows `unsafe` only in the `UNSAFE_ALLOWED` modules of `uqm-platform`
(quality-gates.md 3.3). Until then:

- Code converted to the target design (`Native` hooks, sessions, content, modes) adds
  no `unsafe`. `uqm-save`, `uqm-content` and `uqm-modes` forbid it from creation.
- `unsafe` may move: from a caller into a safe wrapper in `uqm-engine` (for example a
  C-style graphics or sound call needed by a mode), into the `src/engine/` staging
  accessors, or into `engine::platform::os` (U7-06). The PR's workspace total of each
  `unsafe` metric must not rise; `cargo xtask quality --base origin/main` checks it.
- New `unsafe` that is not such a move is allowed only where the unit states an
  unsafe allowance (U1-02's staging accessors). Those units carry `owner-review`, and
  the owner applies `quality-growth-approved`.
- The soundness fixes that turn safe pointer-taking functions into `unsafe fn` land
  in U0-01, before the first baseline (U0-02), so they are not growth.
- `uqm-engine` becomes `forbid(unsafe_code)` in U7-06 and the root in U8-01.

### Goldens

- `rust/tests/golden/` is created by U0-06: the parity corpus, the eight fixtures,
  the per-scenario counts and the V3b save captures, each after three byte-identical
  recordings, with the owner's approval on that PR.
- Later, a new golden file may be added only with the new corpus or fixture entry
  that a unit's spec lists, and with the same three-recording evidence.
- No PR modifies or deletes an existing golden, except a behaviour-fix PR that the
  owner labels `parity-golden-update`, with the reason in the commit. A refactor that
  needs a golden change stops (below).

## Verify

Run every command in the issue's Acceptance section from the repository root; issue
bodies spell out each check bundle and the unit's V4 commands. Then:

1. `cd rust && cargo xtask quality --base origin/main` shows no growth, and the exit
   numbers in the issue moved as stated.
2. Every item under Deletes is gone: its zero-count check (`quality report ...
   --expect 0`, `quality ident ... --expect 0`, `test ! -e`, or an `rg` that must not
   match) passes, and no adapter outside work-items.md's list remains.
3. From U0-06 on, `cd rust && cargo xtask parity check --base origin/main` is
   identical.
4. The four native scenarios pass (`V2`). If you cannot run macOS natively, say so in
   the PR and rely on the `failure-scenarios` job.
5. For `V4`, inspect both galleries with the image-comparer, not by file size.

Open the PR titled `<unit-id>: <title>` with `Closes #<issue>`, the Deletes list
checked off item by item, the exit numbers (base and PR values), the acceptance output
summaries and, for crate splits, the `--timings` comparison. CI must be green on every job before review.

## Stop and file an issue instead

Stop, add `blocked` to the unit, and open an issue linked to it when:

- a parity or save check differs and the cause is not a mistake in your own change;
- the unit needs more than about 6,000 non-mechanical changed lines (git mv for crate
  splits, renames, accessor swaps and generated golden data do not count), or files
  outside its `touches` and `shared` lists;
- an item under Deletes cannot go in this PR, or the change needs an adapter that
  work-items.md does not list;
- a budget or baseline would have to rise, or a golden would have to change;
- a consumer of something the unit deletes has not moved yet;
- a dependency's result is missing or wrong (fix it in its own unit, not here);
- a check fails intermittently. Compare a passing and a failing run input by input,
  find the input that varies, and report it. Do not retry, raise timeouts or loosen
  limits until the cause is known;
- the design documents disagree with the code. Cite file:line in the issue.

The issue states what you found, the evidence (commands and output), and the
smallest change that would unblock the unit.
