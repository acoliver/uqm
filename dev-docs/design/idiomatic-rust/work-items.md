# Work items: the migration as GitHub issues

This file breaks [plan.md](plan.md) into 40 work units. Each unit is one GitHub issue
and one PR that a single agent can finish and land with every CI job green.
Implementing agents read [AGENTS.md](AGENTS.md) first, then their unit's issue.

Once the issues exist they are the source of truth, and this file is the readable
record of the plan they were created from. Every issue cites
`dev-docs/design/idiomatic-rust/` under "Read first", so the design documents are
committed there before the issues are created.

This breakdown replaces the earlier 63 units (issues #598 to #660). Each new unit lists
the old units it replaces under **Replaces**; the mapping is complete (see "Old units").

## Structure

- **One epic** (`IR`, #588): the summary and a checklist of the phase issues.
- **Nine phase issues** (`P0`..`P8`, #589 to #597): exit criteria and a checklist of
  their units. A phase closes when all its units have merged and its exit criteria hold
  on `main`.
- **40 unit issues** (`U<phase>-<nn>`): scope, files, change, deletes, out of scope,
  acceptance commands, exit numbers and risks.

Every unit is a vertical slice. It introduces the new representation or API, migrates
every user of the thing it replaces, and deletes the replaced code in the same PR. Its
**Deletes** list names what goes (files, functions, types, statics, extern and
`no_mangle` symbols, allows), and its acceptance proves each item gone with a positive
zero-count check (`cargo xtask quality report ... --expect 0`,
`cargo xtask quality ident ... --expect 0`, `test ! -e`, or an `rg` that must not
match). No unit leaves code with zero callers or two live implementations of the same
thing. Units that only add tooling say so under Deletes and replace nothing.

A semantic unit is at most about 6,000 changed lines; mechanical commits (git mv for
crate splits, renames, accessor swaps, generated golden data) do not count. Inside a
unit, mechanical and semantic changes are separate commits, and V1 is the oracle for
each. A crate move is the last commit of its unit and changes nothing inside moved files
except visibility. Each unit leaves `main` building, passing every CI job and playable.

Rules that hold for every unit are in [AGENTS.md](AGENTS.md) and are not repeated per
unit: no behaviour change, no weakened assertion, no new global, goldens created only by
U0-06 and changed only by owner-labelled fixes, change + migrate + delete in one PR,
adapters only as listed below, gate counters only fall, transitional `unsafe` only as
AGENTS.md allows.

## Phases

| Phase | Title | Units | IDs |
|---|---|---|---|
| 0 | soundness, gates, replay harness, uqm-save | 6 | U0-01..U0-06 |
| 1 | battle core pilot (Stage A, Stage B, first Native hooks) and uqm-engine | 4 | U1-01..U1-04 |
| 2 | ships, combat AI, every record reader, Stage C | 9 | U2-01..U2-09 |
| 3 | Super Melee and netplay | 2 | U3-01..U3-02 |
| 4 | world state, queues and main loop; engine core into uqm-engine | 4 | U4-01..U4-04 |
| 5 | navigation and planets | 4 | U5-01..U5-04 |
| 6 | comm, starbase and menus; uqm-content and uqm-modes | 4 | U6-01..U6-04 |
| 7 | platform and content loading; uqm-platform | 6 | U7-01..U7-06 |
| 8 | closeout | 1 | U8-01 |
| | **Total** | **40** | |

Handoffs that keep every PR green:

- Soundness fixes and the dormant-code removal (U0-01) land before the first baseline
  (U0-02); debt lints arrive with their classifier (U0-03); `quality ident`, used by
  every later Deletes check, exists from U0-02.
- Record and queue statics get one definition (U1-01) before the ids, `Cx`, the
  dispatcher and the process loop change together (U1-02); the Native hook variant
  arrives with its first users (U1-03).
- Every race (U2-01 to U2-05, U2-08) and every record reader (U2-06, U2-07) converts
  before Stage C deletes the records (U2-09); U2-08 depends on all other race and reader
  units, so it converts the last legacy hook and deletes `Hook::Legacy` and the battle
  entry points of `app::bridge` that only legacy hooks called; `raw_records` = 0 is
  U2-09's precondition.
- The netplay globals move into `NetplaySession` together with the melee menu, the
  connection dialog and the battle code that read them (U3-01).
- `GlobData` moves to the owned `World` with all its readers in one PR (U4-01); the
  GAME_STATE queues and `event_q` move to `OrderedPool` with all their consumers, and
  `collections/queue.rs` goes with its `abi/` wrappers and the `mainloop/state_abi.rs`
  queue accessors, in one PR (U4-02).
- The comm statics move into the `Comm` session with every reader and writer, the
  dialogue adapters, hail, credits and the locdata copier included (U6-01), before the
  adapters take their content and modes form (U6-02).
- The last semantic commits of U6-04 bring `planned.uqm-content` and
  `planned.uqm-modes` to 0 before its move commit.
- Video moves with graphics (U7-01 presentation, U7-02 screen format); the Files port
  arrives with every one of its callers and the `uio_*` deletion (U7-03).
- Automation's POSIX process and descriptor code and `platform.rs` move behind
  `platform::os` in the same unit that carves `uqm-platform` (U7-06), before the root
  `forbid(unsafe_code)` (U8-01).

## Transitional adapters

An adapter may exist only while live, unconverted users of the old path remain in code
that a later unit owns. These are all of them; each is deleted by the unit that converts
its last user. Any other adapter is a stop-and-file case (AGENTS.md).

| Id | Adapter | Created | Deleted by | Live users until then |
|---|---|---|---|---|
| A1 | Hook::Legacy (ElementHook::Legacy, CollisionHook::Legacy), the legacy arm of invoke_* and the legacy_hooks budget | U1-02 | U2-08 | unconverted races (U2-01 to U2-05, U2-08) and the hyperspace and IP element callbacks (U2-07) |
| A2 | race-level dispatch through RACE_DESC function pointers for races without a race(species) arm, and their static templates in INITIALIZERS | U1-03 | U2-08 | races not yet converted (U2-01 to U2-05, U2-08) |
| A3 | the Stage B view over raw records: ElementTable's raw decoding (HELEMENT <-> ElementId, parent() by address range, generation side array) over the Stage A authority statics, and LinkQueue's QUEUE backing for race_q | U1-02 (ElementTable), U2-06 (LinkQueue) | U2-09 | unconverted races and record readers dereference ELEMENT, STARSHIP and DisplayArray directly until U2-08 |
| A4 | app::bridge: the published NonNull<Cx> slot, bridge::enter and the counted bridge accessors (bridge_calls budget) | U1-02 | U2-08 (battle entry points), U6-04 (the rest) | Rust entry points called from legacy hooks (AllocElement, do_damage, ProcessSound) until U2-08, which converts the last legacy hook; mode entry points without Game or Cx (comm, planets, ui, front_end, starbase) until U6-04 |
| A5 | present_hook and readback_hook fn-pointer fields in the graphics backend state, set by automation | U1-04 | U7-01 | automation capture and the presented-frame callback |
| A6 | the time::source module and its TIME_SOURCE set-once static (Real or Replay clock): src/time/source.rs, moved with the counter to crates/engine/src/time/source.rs by U1-04 | U0-05 | U7-05 | the install in src/main.rs, GetTimeCounter, the replay PacingEnv and the callback alarm clock |
| A7 | root re-exports of modules moved into crates (savegame codec, the libs layer, src/engine, content, modes, platform) | U0-04, U1-04, U4-04, U6-04, U7-06 | U8-01 | root code that still names the old module paths |

The three-stage storage transition (architecture.md 4.1) is kept only for the battle
records (A1 to A4): 28 races, the AI and four groups of readers dereference
`ELEMENT`, `STARSHIP` and `DisplayArray` directly, about 40k changed lines in all, so
no single PR can convert them with the storage. Everywhere else the design now changes
consumers and representation together. Adapters of the earlier breakdown that no longer
exist:

| Former adapter | Earlier units | Now |
|---|---|---|
| World accessors over the static GlobData | P4-01 to P4-05 | U4-01 moves readers straight to the owned World |
| LinkQueue views over QUEUE storage for the GAME_STATE queues and event_q | P4-03 to P4-04 | U4-02 changes consumers and storage together |
| dual generator dispatch (trait impl for converted stars, vtable for the rest) | P5-01 to P5-02 | U5-01 converts every generator |
| pSolarSysState kept for orbit, scan, starmap and other readers | P5-03 to P5-05 | U5-02 converts every reader and deletes it |
| comm statics kept for the dialogue adapters | P6-01 to P6-02 | U6-01 moves every reader, the dialogue adapters included, to the Comm session; U6-02 changes only the adapters' form |
| uio_* functions as wrappers over Vfs | P7-04 to P7-07 | U7-03 moves every caller to Files |
| OrderedPool, LinkQueue and the Native hook variant with no users | P2-01, P1-02 | each arrives with its first users (U2-09, U2-06, U1-03) |

## Dependency graph: waves and critical path

**Shared-file rule** (the full text is in AGENTS.md). A unit reserves its `touches`
paths while it is in progress; a directory entry covers everything below it. Its
`shared` paths are registries, dispatchers and tables where each unit edits only its own
entries (for example `src/ships/code_resources.rs`, `src/ships/hook_registry.rs`,
`xtask/src/lib.rs`, `xtask/src/quality/macros.rs`). Two units may be in progress
together unless one's `touches` overlaps the other's `touches` or `shared`. Units that
hold the same shared path merge one at a time: after the first merges, the second
rebases on `main` and reruns its acceptance. `rust/xtask/quality-baseline.toml` is
shared by every unit and is regenerated by xtask after a rebase, never merged by hand.

**Waves.** A wave lists units that can be in progress at once under that rule: every
dependency has merged in an earlier wave, no reservation overlaps, and at most five
worktrees exist. Units are placed in order of their longest remaining dependency chain.
"Shared" lists pairs that hold a common shared path and so merge in sequence.

| Wave | Units | Shared |
|---|---|---|
| 1 | U0-01 |  |
| 2 | U0-02, U0-05 |  |
| 3 | U0-03 |  |
| 4 | U0-04 |  |
| 5 | U0-06 |  |
| 6 | U1-01 |  |
| 7 | U1-02, U1-04 | U1-02/U1-04 |
| 8 | U1-03 |  |
| 9 | U2-01, U2-03, U2-04, U2-06, U2-02 | U2-01/U2-03, U2-01/U2-04, U2-01/U2-02, U2-03/U2-04, U2-03/U2-02, U2-04/U2-02 |
| 10 | U2-05, U2-07, U3-01 |  |
| 11 | U2-08 |  |
| 12 | U2-09 |  |
| 13 | U4-01 |  |
| 14 | U4-02, U3-02 |  |
| 15 | U4-03 |  |
| 16 | U4-04 |  |
| 17 | U5-01, U6-03, U5-04 |  |
| 18 | U5-02 |  |
| 19 | U6-01 |  |
| 20 | U6-02 |  |
| 21 | U5-03 |  |
| 22 | U6-04 |  |
| 23 | U7-01 |  |
| 24 | U7-03, U7-05 | U7-03/U7-05 |
| 25 | U7-02 |  |
| 26 | U7-04 |  |
| 27 | U7-06 |  |
| 28 | U8-01 |  |

The race units have no order among themselves except U2-08, which waits for the others.
Lanes inside a phase (the `lane` field) group units by subsystem.

**Critical path.** The longest dependency chain weighted by changed lines, mechanical
included: 24 units, about 119,500 changed lines.

U0-01 → U0-02 → U0-03 → U0-04 → U0-06 → U1-01 → U1-02 → U1-03 → U2-01 → U2-08 → U2-09 → U4-01 → U4-02 → U4-04 → U5-01 → U5-02 → U5-03 → U6-04 → U7-01 → U7-03 → U7-02 → U7-04 → U7-06 → U8-01.

The race unit on the path stands for any of U2-01, U2-03 and U2-04, which have the same
weight. Between U5-01 and U6-04 two chains have the same weight, about 8,300 changed
lines each: U5-02 → U5-03 (shown) and U6-01 → U6-02. All four carry `critical-path`;
they share files, so they run one after another (waves 18 to 21). The remaining units,
U0-05, U1-04, U2-02, U2-03, U2-04, U2-05, U2-06, U2-07, U3-01, U3-02, U4-03, U5-04,
U6-03 and U7-05, run in parallel lanes.

## Old units

Every unit of the earlier breakdown maps to exactly one new unit. When the new issues
are created, each old issue is closed as superseded with a link to its new unit.

| Old unit | Issue | New unit |
|---|---|---|
| P0-01 | #598 | U0-01 |
| P0-02 | #599 | U0-01 |
| P0-03 | #600 | U0-02 |
| P0-04 | #601 | U0-03 |
| P0-05 | #602 | U0-04 |
| P0-06 | #603 | U0-05 |
| P0-07 | #604 | U0-06 |
| P0-08 | #605 | U0-06 |
| P1-01 | #606 | U1-01 |
| P1-02 | #607 | U1-02 |
| P1-03 | #608 | U1-02 |
| P1-04 | #609 | U1-03 |
| P1-05 | #610 | U1-04 |
| P1-06 | #611 | U1-04 |
| P2-01 | #612 | U2-09 |
| P2-02 | #613 | U2-01 |
| P2-03 | #614 | U2-01 |
| P2-04 | #615 | U2-02 |
| P2-05 | #616 | U2-03 |
| P2-06 | #617 | U2-04 |
| P2-07 | #618 | U2-05 |
| P2-08 | #619 | U2-02 |
| P2-09 | #620 | U2-03 |
| P2-10 | #621 | U2-04 |
| P2-11 | #622 | U2-08 |
| P2-12 | #623 | U2-06 |
| P2-13 | #624 | U2-07 |
| P2-14 | #625 | U2-06 |
| P2-15 | #626 | U2-08 |
| P2-16 | #627 | U2-09 |
| P3-01 | #628 | U3-01 |
| P3-02 | #629 | U3-01 |
| P3-03 | #630 | U3-02 |
| P4-01 | #631 | U4-01 |
| P4-02 | #632 | U4-03 |
| P4-03 | #633 | U4-02 |
| P4-04 | #634 | U4-02 |
| P4-05 | #635 | U4-01 |
| P4-06 | #636 | U4-04 |
| P4-07 | #637 | U4-04 |
| P5-01 | #638 | U5-01 |
| P5-02 | #639 | U5-01 |
| P5-03 | #640 | U5-02 |
| P5-04 | #641 | U5-02 |
| P5-05 | #642 | U5-03 |
| P5-06 | #643 | U5-04 |
| P6-01 | #644 | U6-01 |
| P6-02 | #645 | U6-02 |
| P6-03 | #646 | U6-03 |
| P6-04 | #647 | U6-04 |
| P6-05 | #648 | U6-04 |
| P7-01 | #649 | U7-01 |
| P7-02 | #650 | U7-01 |
| P7-03 | #651 | U7-02 |
| P7-04 | #652 | U7-03 |
| P7-05 | #653 | U7-02 |
| P7-06 | #654 | U7-04 |
| P7-07 | #655 | U7-03 |
| P7-08 | #656 | U7-05 |
| P7-09 | #657 | U7-06 |
| P7-10 | #658 | U7-06 |
| P8-01 | #659 | U8-01 |
| P8-02 | #660 | U8-01 |

## Labels

| Label | Meaning |
|---|---|
| `idiomatic-rust` | Idiomatic Rust migration (epic, phases and units) |
| `epic` | Parent roadmap and roll-up issue |
| `ir-phase` | One phase of the migration |
| `ir-unit` | One work unit, landed as one PR |
| `phase-0` .. `phase-8` | The unit's phase |
| `kind-tooling` | xtask, CI or harness code |
| `kind-semantic` | Changes how code works, possibly with mechanical commits; V1 traces must stay identical |
| `kind-mechanical` | Moves, renames, accessor swaps or dead-code deletion only |
| `critical-path` | On the critical path above |
| `owner-review` | Needs the owner's approval before merge (U0-06, U1-01, U1-02, U1-03) |
| `in-progress` | An agent has claimed this unit |
| `blocked` | Stopped; see the linked issue |
| `quality-growth-approved` | Owner only: approves baseline growth |
| `parity-golden-update` | Owner only: a behaviour fix that changes existing goldens |

Every unit issue carries `idiomatic-rust`, `ir-unit`, `phase-<n>` and `kind-<kind>`,
plus `critical-path` and `owner-review` where they apply. Agents set `in-progress` and
`blocked`; they never apply `quality-growth-approved` or `parity-golden-update`.

## Issue bodies

Each unit issue holds `<!-- ir:{id} -->` as its stable key, links to the epic and its
phase, its dependencies as issue links, the old issues it replaces, "Read first:
AGENTS.md, then {refs}", and the sections of its entry below. Every check bundle named
in Acceptance is expanded into its commands with `{unit}` replaced by the unit id, and
the V4 block is copied as written, so an issue states its complete acceptance set. The
issue ends with:

```markdown
### Done when
- [ ] one PR titled `{id}: {title}` merged with every CI job green
- [ ] every item under Deletes is gone, shown by the acceptance checks; no adapter other than those listed in work-items.md
- [ ] exit numbers reported (base and PR values); no existing golden changed
```

Phase issues hold `<!-- ir:P{n} -->`, the exit criteria and a checklist of unit
issues; the epic holds `<!-- ir:IR -->`, the summary and a checklist of phase issues.
Issues are created in dependency order so that every "Depends on" link resolves.

## Check bundles

Commands run from the repository root; `{unit}` is the unit id. `S0` is CI before
U0-03 merges, `S` from U0-03 until U8-01, `S8` within U8-01 (after `abi-check` is
deleted). `P` exists from U0-06 and `V3` in full from U0-06. A unit with no dependency
on U0-03 lists "S0 or S" and runs the one `main` uses when its PR opens. Native runs
(`V2`, `V4`) need macOS; a unit that cannot run them locally relies on the
`failure-scenarios` job for `V2` and says so in the PR. `V4` galleries are inspected
with the image-comparer, not by file size.

**S0**: CI as it is before U0-03 merges.

```sh
cd rust
cargo fmt --all --check
cargo run --locked --manifest-path xtask/Cargo.toml -- abi-check
cargo clippy --workspace --all-targets --locked --no-default-features --features audio_heart,debug-process -- -D warnings
cargo test --workspace --locked --no-default-features --features audio_heart,debug-process
```

**S**: standard gates (rust, supply, coverage jobs), from U0-03 until U8-01.

```sh
cd rust
cargo fmt --all --check
cargo xtask quality --base origin/main
cargo xtask clippy --base origin/main
cargo test --workspace --locked --no-default-features --features audio_heart,debug-process,native
cargo xtask abi-check
cargo xtask deny
cargo xtask coverage --base origin/main
```

**S8**: standard gates after `abi-check` is deleted, in U8-01.

```sh
cd rust
cargo fmt --all --check
cargo xtask quality --base origin/main
cargo xtask clippy --base origin/main
cargo test --workspace --locked --no-default-features --features audio_heart,debug-process,native
cargo xtask deny
cargo xtask coverage --base origin/main
```

**P**: replay parity (parity job), from U0-06. "P (entries)" names the corpus entries
the unit is expected to exercise; the command always checks all of them.

```sh
cd rust
cargo xtask parity check --base origin/main
cargo xtask parity counts check --all
```

**V2**: native failure scenarios on a real macOS window (failure-scenarios job).

```sh
pkill -x uqm || true
cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native run \
  --artifacts "$PWD/tmp/{unit}/v2" --content "$PWD/sc2/content/packages" \
  --scenarios "lander-destroyed-v1,out-of-fuel-v1,lost-battle-v1,super-melee-v1"
```

**V3**: save compatibility; V3a from U0-04, V3b from U0-06.

```sh
cd rust
cargo test -p uqm-save --locked --test codec_roundtrip
cargo test --locked --test save_runtime_capture
```

**R5**: release build (the game-build job runs it on five targets).

```sh
cd rust
cargo build --release --locked --bin uqm --features bundled-sdl
cd .. && python3 -m unittest discover -s scripts/release -p 'test_*.py'
```

**V4**: `native smoke` (main-menu-v1, real-sol-starbase-to-mars, load-save-v1,
comm-encounter-v1, battle-v1 from `rust/xtask/smoke.json`) plus the unit's own
scenarios, each unit's block below. None of the 45 `load-*-v1` scenarios starts in
hyperspace (41 assert `IN_INTERPLANETARY`), so hyperspace work uses
`fly-to-alpha-centauri-v1`, `load-save-v1` (which flies into hyperspace) and
`load-alpha-centauri-v1`. Units that touch world state or saves run all 45.

## Units

## Phase 0: soundness, gates, replay harness, uqm-save

Exit: soundness fixes and dormant code removal landed before the first baseline; quality, clippy, deny, coverage and parity jobs green on a PR and on the following push to main; native-files = 0; not_unsafe_ptr_arg_deref = 0; clashing_extern_declarations = 0; abi-check covers statics; every item-generating first-party macro has a walker handler; quality ident available; three identical recordings for every corpus entry and fixture; counts golden for every rust/scripts/*.json; V3a and V3b pass for 74 files; uqm-save is a member with forbid(unsafe_code); cargo xtask runs without compiling the game.

### U0-01 Soundness: unsafe fns, foreign static types, one story authority; delete the dormant parallel models

Phase 0 · semantic · lane `soundness` · about 2,200 semantic and 5,500 mechanical changed lines · depends on none · **critical path**

**Replaces** P0-01 (#598), P0-02 (#599).

**Scope.** The 20 not_unsafe_ptr_arg_deref allows (queue.rs, mainloop options/init_sequence/do_restart, battle gravity/velocity/collision/battle_controls), every mismatched foreign static, the story-state shadow, and the dormant parallel models (ships/traits.rs, the 28 non-_native race files and their registry path, four unused battle files). Lands before U0-02 so the first baseline records the corrected unsafe fn counts and none of the dead code.

**Touches** (reserved): `src/collections/queue.rs`, `src/mainloop/`, `src/main.rs`, `src/automation/input.rs`, `src/automation/setup.rs`, `src/automation/lifecycle.rs`, `src/battle/`, `src/ships/`, `src/navigation/`, `src/planets/`, `src/planet_side/orbit_scan.rs`, `src/comm/`, `src/state/`, `src/savegame/`, `src/starbase/`, `src/ui/shipyard.rs`, `src/front_end/gameopt.rs`, `src/front_end/uqmdebug.rs`, `src/game_init/kernel_abi.rs`, `src/time/clock_abi.rs`, `src/time/clock_bridge.rs`, `src/supermelee/netplay/mod.rs`, `src/graphics/native_abi.rs`, `src/abi/`, `xtask/src/abi_check.rs`.
**Shared:** none.

**Change**
- First commit (mechanical): make the dormant candidates pub(crate), confirm dead_code reports them unused outside tests, then delete them. Tests that cover live behaviour move to the _native code; a constant table used by both halves moves to the _native file.
- Turn the 20 not_unsafe_ptr_arg_deref allows into unsafe fn with # Safety docs, starting with InitQueue (collections/queue.rs:84-117); callers the compiler flags get unsafe blocks with SAFETY comments.
- Give each foreign static its defining type: DisplayArray (orbit_scan.rs:79, 150 vs 330), CurStarDescPtr (six declarations), GlobData (CGlobData in savegame/runtime/live.rs:30, comm/commglue_abi.rs:73, mainloop/ffi.rs:45).
- Drop the ignored _shadow parameter of getGameState/setGameState (navigation/globals.rs:19-27) and the dead GameState[155] field (abi/ship_abi.rs:3500).
- Extend abi-check (xtask/src/abi_check.rs) to foreign static types and to the cross-crate direction rule (QG 3 table); it is still run through its manifest path.

**Deletes** (gone when the PR merges; the acceptance checks prove it)
- files src/ships/traits.rs (with ShipState's unsafe impl Send, :61), the 28 non-_native race files in src/ships/races/ (androsynth.rs .. zoqfotpik.rs), src/battle/{process_loop,display_list,tactical,integration}.rs
- the dormant registry path ships/registry.rs:1012-1090
- 20 #[allow(clippy::not_unsafe_ptr_arg_deref)] in 8 files; 7 #[allow(clashing_extern_declarations)] (comm/commglue_abi.rs:71, comm/comm_abi.rs:23, navigation/mod.rs:10, comm/sis_graphics.rs:192, comm/oscill_abi.rs:41, supermelee/netplay/mod.rs:9, graphics/native_abi.rs:8)
- the mismatched foreign static declarations: DisplayArray with 150 entries (orbit_scan.rs:79), the non-defining CurStarDescPtr declarations, the CGlobData copies (savegame/runtime/live.rs:30, comm/commglue_abi.rs:73, mainloop/ffi.rs:45)
- the _shadow parameter of getGameState/setGameState and the GameState[155] field

**Out of scope.** any behaviour change; renaming the _native files.

**Acceptance**
- S0 (check bundle)
- `cd rust && rg -q 'clashing_extern_declarations|not_unsafe_ptr_arg_deref' src; test $? -eq 1   # 1 = no match; 2 (error) fails`
- `cd rust && for f in src/ships/traits.rs src/battle/process_loop.rs src/battle/display_list.rs src/battle/tactical.rs src/battle/integration.rs; do test ! -e "$f" || exit 1; done`
- `cd rust && { ls src/ships/races | rg -qv '^(.+_native|samatra_state|mod)\.rs$'; test $? -eq 1; }`
- `cd rust && rg -q 'unsafe impl Send for ShipState' src; test $? -eq 1`
- `cd rust && rg -q '_shadow' src/navigation/globals.rs; test $? -eq 1`
- `mkdir -p tmp/U0-01 && cd rust && cargo build --workspace --locked > ../tmp/U0-01/build.log 2>&1 && { grep -q 'never used' ../tmp/U0-01/build.log; test $? -eq 1; }`
- V2 (check bundle)

**Exit numbers.** allows: not_unsafe_ptr_arg_deref 20 -> 0, clashing_extern_declarations 7 -> 0; Rust types for GlobData 2 -> 1; about 5.5k production lines removed; unsafe impl count -1; abi-check checks statics.

**Risks**
- DisplayArray is declared with 150 entries in orbit_scan.rs and defined with 330; keep the loop bounds the code uses today (an explicit 150), never .len(), or the scan changes.
- Removing GameState[155] changes the GAME_STATE layout; update every layout assertion and confirm no reader with rg before deleting.
- A constant table used by both halves must move to the _native file, not be deleted.

### U0-02 xtask foundation: deny-set lints, aliases, game behind a feature, quality walker, counters, quality ident and baseline

Phase 0 · tooling · lane `gates` · about 4,000 changed lines · depends on U0-01 · **critical path**

**Replaces** P0-03 (#600).

**Scope.** The xtask entry point, the source walk with handlers for every item-generating first-party macro, the shrink-only baseline and its counters, and quality ident, the zero-count check that every later unit's Deletes list uses. The debt lints and the clippy classifier come in U0-03; the raw clippy step stays until then.

**Touches** (reserved): `Cargo.toml`, `clippy.toml`, `.cargo/config.toml`, `xtask/Cargo.toml`, `xtask/src/main.rs`, `xtask/src/quality/`, `.github/workflows/ci.yaml`.
**Shared:** `xtask/src/lib.rs`.

**Change**
- [workspace.lints] with the deny set only (QG 2.1 without the four debt lints) and rust/clippy.toml; [lints] workspace = true in uqm and uqm-xtask; deny-lint hits are fixed here or the lint is listed for U0-03's debt set in the PR.
- xtask's uqm-rust dependency optional under default feature native; native subcommands under cfg(feature = "native"); the xtask (no default features) and xtask-native aliases; CI FEATURES gains native; .cargo/config.toml keeps incremental = false.
- Source walk and identity (QG 3.2) with a handler and a test for each of the 16 item-generating macro_rules! at 35d33879b, one entry per macro in xtask/src/quality/macros.rs: global! (game_init/kernel_abi.rs:17, mainloop/options_abi.rs:9), screen_global! (graphics/gfx_common_abi.rs:15), accessor! (mainloop/state_abi.rs:14), operation! (mainloop/main2_abi.rs:2, mainloop/restart_menu/menu_abi.rs:6, ships/native_adapters.rs:53), define_hash_table! (collections/hash_table.rs:396), energy_hook! (planets/generate/home_exports.rs:268), moons_hook!, story_energy!, story_pickup! (planets/generate/late_exports.rs:216,332,411), menu_handler! (front_end/setupmenu.rs:146), callbacks! (comm/races/comandr.rs:587), game_state_fields! (state/schema.rs:6), assert_c_layout! (c_bindings.rs:112). Any other macro whose transcriber parses as items fails the gate; a handler whose macro no longer exists also fails, so the unit that deletes a macro deletes its handler.
- Counters loc, complexity, ffi (with raw_records: ELEMENT, HELEMENT, STARSHIP, HSTARSHIP, RACE_DESC, RaceDescPtr, DisplayArray, disp_q, race_q outside the record module and pure re-exports; queue_calls: calls into collections::queue outside its typed views; hook_calls: calls of an element hook field outside invoke_*), unsafe, allows, native-files; cargo xtask quality report <counter>[:<scope>] prints a value; --expect N exits nonzero when it differs, and --expect-only <scope>,... exits nonzero when any other scope is nonzero. Scopes are a subsystem, a file path under it, or a crate.
- cargo xtask quality ident <Name>... [--defs] counts identifier and path tokens (with --defs, items and fields defining the name: a fn with a body, struct, enum, union, type alias, trait, const, static with an initializer, mod, macro_rules! or field; extern-block declarations and use re-exports are not definitions) in every target of every workspace member, test targets and #[cfg(test)] items included, outside comments, doc comments and string literals (QG 3 table); it takes --expect N and --expect-only <scope>,... as report does. C traceability doc lines (/// C: <name>) do not count.
- Baseline with --base, --tighten, move and move --prefix (QG 4 rules 1-3, 5); CI runs cargo xtask quality $BASE_ARG in the rust job.

**Deletes** (gone when the PR merges; the acceptance checks prove it)
- the shell c-ratchet job in .github/workflows/ci.yaml (replaced by quality native-files)

**Out of scope.** debt lints, xtask clippy, layers, deps, coverage, deny (U0-03); any change under rust/src except deny-lint fixes.

**Acceptance**
- S0 (check bundle)
- `cd rust && cargo test -p uqm-xtask --no-default-features --locked quality::`
- `cd rust && cargo xtask quality --base origin/main`
- `cd rust && cargo xtask quality --tighten && git diff --exit-code xtask/quality-baseline.toml`
- `cd rust && cargo xtask quality report native-files --expect 0`
- `cd rust && cargo xtask quality ident InitQueue --defs --expect 2   # collections/queue.rs:84 and the wrapper at abi/ship_abi.rs:1212; the extern declarations in rust/tests/*_native.rs and the re-export at abi/navigation_abi.rs:1026 do not count`
- `mkdir -p tmp/U0-02 && cd rust && cargo tree -p uqm-xtask --no-default-features -e normal --locked > ../tmp/U0-02/tree.txt && { grep -q 'uqm-rust ' ../tmp/U0-02/tree.txt; test $? -eq 1; }`
- `{ grep -q '^  c-ratchet:' .github/workflows/ci.yaml; test $? -eq 1; }`

**Exit numbers.** native-files = 0 (absolute); cargo xtask compiles 0 game crates; 16 of 16 item-generating macros handled; quality ident available; baseline committed; c-ratchet job removed.

**Risks**
- --no-default-features on a workspace command also disables xtask's native feature; CI FEATURES must include native or the native-harness tests silently drop.
- A cfg predicate the walker cannot evaluate must fail the gate, never be guessed.
- quality ident must skip comments and strings by syn tokens, not by regex, or C doc lines make deletion checks fail.

### U0-03 Debt lints with their classifier, quality layers and deps, xtask clippy, coverage and deny, CI jobs

Phase 0 · tooling · lane `gates` · about 3,000 changed lines · depends on U0-02 · **critical path**

**Replaces** P0-04 (#601).

**Scope.** The debt lints arrive together with the classifier that baselines them and with the CI step that replaces raw clippy, so no intermediate PR runs them under -D warnings.

**Touches** (reserved): `Cargo.toml`, `xtask/src/quality/`, `xtask/src/clippy.rs`, `xtask/src/coverage.rs`, `xtask/src/deny.rs`, `deny.toml`, `.github/workflows/ci.yaml`.
**Shared:** `xtask/src/lib.rs`.

**Change**
- Add the debt lints (unsafe_op_in_unsafe_fn, undocumented_unsafe_blocks, unwrap_used, expect_used) to [workspace.lints]; cargo xtask clippy with --force-warn for the debt set and per-file counts (QG 2.3) replaces the raw clippy step in the same PR; existing hits such as savegame/encode.rs:44 are baseline entries.
- quality layers (QG 3.5): CRATE_DEPS, OS crates per crate, banned std APIs, the root module layer table, and PLANNED for every planned crate from the layer table (uqm-engine, uqm-content, uqm-modes, uqm-platform); reports planned.<crate>.
- quality deps (QG 3.4) passing on the current lockfile with bundled-sdl.
- cargo xtask coverage, per file, following [moved] prefixes; rust/deny.toml and cargo xtask deny.
- CI: base fetch, label-change triggers, GROWTH plumbing; jobs rust, supply, coverage (QG 5).

**Deletes** (gone when the PR merges; the acceptance checks prove it)
- the raw `cargo clippy --workspace ... -- -D warnings` step of the rust job (replaced by cargo xtask clippy)

**Out of scope.** the parity job (U0-06); changes under rust/src.

**Acceptance**
- S (check bundle)
- `cd rust && cargo test -p uqm-xtask --no-default-features --locked layers::`
- `cd rust && cargo test -p uqm-xtask --no-default-features --locked clippy::`
- `cd rust && cargo test -p uqm-xtask --no-default-features --locked coverage::`
- `cd rust && cargo xtask quality report planned.uqm-engine && cargo xtask quality report planned.uqm-content && cargo xtask quality report planned.uqm-modes && cargo xtask quality report planned.uqm-platform`
- `{ grep -q 'run: cargo clippy' .github/workflows/ci.yaml; test $? -eq 1; }`
- the PR's rust, supply and coverage jobs are green, and so is the push to main after merge

**Exit numbers.** planned.<crate> recorded for four planned crates (the regex scan gives 20 for uqm-engine; the syn value is the baseline); clippy deny set: 0 findings; debt set baselined per file; cargo xtask deny: 0 findings; raw clippy step removed.

**Risks**
- cargo metadata --locked fails on a stale lockfile; fix the lockfile in a separate PR if so.

### U0-04 uqm-save crate with the save codec; V3a codec round trip

Phase 0 · mechanical · lane `crates` · about 500 semantic and 1,500 mechanical changed lines · depends on U0-03 · **critical path**

**Replaces** P0-05 (#602).

**Scope.** The first workspace crate: the save byte format only.

**Touches** (reserved): `Cargo.toml`, `crates/save/`, `src/savegame/mod.rs`, `src/savegame/format.rs`, `src/savegame/types.rs`, `src/savegame/encode.rs`, `src/savegame/decode.rs`, `src/savegame/error.rs`, `src/state/planet_info.rs`, `src/state/game_state.rs`, `tests/save_writer_fill.rs`.
**Shared:** `xtask/src/quality/crate_deps.rs`.

**Change**
- members = ["xtask", "crates/*"]; add crates/save (uqm-save, version.workspace, publish = false, [lints] workspace = true, #![forbid(unsafe_code)]).
- git mv savegame/{format,types,encode,decode,error}.rs; define CannedNode and NUM_GAME_STATE_BYTES in uqm-save; state re-exports them and asserts at compile time that its bit table needs exactly NUM_GAME_STATE_BYTES.
- Root savegame re-exports the moved modules (transitional until U8-01); cargo xtask quality move --prefix; CRATE_DEPS edge uqm -> uqm-save.
- V3a: crates/save/tests/codec_roundtrip.rs over the 73 saves in rust/scripts/saves/ and tests/fixtures/savegame/legacy-save.00; tests/save_writer_fill.rs covers the element-amount fill (savegame/live/save.rs:65-69).

**Deletes** (gone when the PR merges; the acceptance checks prove it)
- src/savegame/{format,types,encode,decode,error}.rs in the root (moved by git mv)
- the root definitions of CannedNode (state/planet_info.rs) and NUM_GAME_STATE_BYTES (state/game_state.rs)

**Out of scope.** LZH (U7-03); savegame/live and runtime (U4-04); .mle (U3-02).

**Acceptance**
- S (check bundle)
- `cd rust && cargo test -p uqm-save --locked --test codec_roundtrip`
- `cd rust && cargo test --locked --test save_writer_fill`
- `cd rust && for f in format types encode decode error; do test ! -e src/savegame/$f.rs || exit 1; done`
- `cd rust && cargo xtask quality ident CannedNode NUM_GAME_STATE_BYTES --defs --expect-only uqm-save`
- V2 (check bundle)
- R5 (check bundle)

**Exit numbers.** workspace members 2 -> 3; V3a: 74 of 74 files byte-identical.

**Risks**
- xtask native_save.rs reads uqm_rust::savegame; the root re-export must keep that path.

### U0-05 Replay mode: replay clock for the game and its alarms, ReplayAudio, scripted input at admission

Phase 0 · semantic · lane `replay` · about 2,500 changed lines · depends on none

**Replaces** P0-06 (#603).

**Scope.** uqm --replay <scenario>: the inputs that seeds leave variable, including the SDL clock read by the callback alarm queue.

**Touches** (reserved): `src/time/mod.rs`, `src/time/source.rs`, `src/time/counter_abi.rs`, `src/threading/pacing.rs`, `src/callback/ffi.rs`, `src/sound/rodio_audio.rs`, `src/sound/replay_audio.rs`, `src/comm/commanim_abi.rs`, `src/comm/talk_segue.rs`, `src/automation/input_ffi.rs`, `src/automation/coordinator.rs`, `src/automation/scheduler.rs`, `src/cli.rs`, `src/main.rs`, `tests/replay_smoke.rs`.
**Shared:** `src/sound/mod.rs`.

**Change**
- New module src/time/source.rs, declared in src/time/mod.rs: enum TimeSource { Real, Replay } and the set-once static TIME_SOURCE, one of the plan's two new globals (U1-04 moves the module into uqm-engine, U7-05 removes it); main.rs installs it once at start from --replay; it is the only SDL_GetTicks reader; GetTimeCounter (time/counter_abi.rs:27-29) and the replay PacingEnv read it; sleep_until runs one process_async and pump_and_flush pass, then sets now = max(now, wake); a loop that reads the clock 10,000 times without sleeping panics with its caller.
- callback/ffi.rs now_ms (:12-14) reads time::source in milliseconds instead of SDL_GetTicks, so Alarm_processOne (:181-200) compares alarm deadlines, such as the SIS flash scheduled from game time (ui/sis.rs:1059-1073), with replay time; a test advances replay time while real time also passes and checks alarm dispatch order.
- ReplayAudio runs the rodio command enum (sound/rodio_audio.rs:20) on the game thread and answers track, channel and position queries from replay time and decoded lengths; the rodio thread is not started.
- The scenario is read before start; injected input is consumed only at input admission, by count; no automation I/O thread in replay. --replay-summary <file> writes {completed, inputs_admitted, steps, presents, final_activity}.
- tests/replay_smoke.rs runs super-melee-v1 twice and asserts completed, nonzero inputs_admitted, steps and presents, and equal summaries.

**Deletes** (gone when the PR merges; the acceptance checks prove it)
- direct SDL_GetTicks reads outside time::source: callback/ffi.rs now_ms (:12-14), time/counter_abi.rs:27-29, comm/commanim_abi.rs:258-297, comm/talk_segue.rs:411-451

**Out of scope.** trace format and xtask parity (U0-06); any change to the real-time path.

**Acceptance**
- S0 or S (whichever main runs when the PR opens)
- `cd rust && cargo test --locked replay::`
- `cd rust && cargo test --locked --test replay_smoke`
- `cd rust && rg -q 'SDL_GetTicks' src --glob '!src/time/source.rs'; test $? -eq 1`
- V2 (check bundle)

**Exit numbers.** two replay runs of super-melee-v1 give equal, nonempty summaries; clock sources in replay: 1 (SDL_GetTicks callers outside time::source: 0).

**Risks**
- Hidden clock reads in comm animation (comm/commanim_abi.rs:258-297) and speech (comm/talk_segue.rs:411-451) must go through time::source; the 10,000-read guard finds loops, not independent clocks, so the rg check is part of acceptance.

### U0-06 Migration trace and parity tooling, corpus, fixtures, scenario counts, V3b captures and the parity job; Phase 0 exit

Phase 0 · tooling · lane `replay` · about 4,500 semantic and 1,800 mechanical changed lines · depends on U0-03, U0-04, U0-05 · **critical path** · `owner-review`

**Replaces** P0-07 (#604), P0-08 (#605).

**Scope.** V1 as one gate: the I/S/P trace, the state digest, the hook-name registry, cargo xtask parity, the nine corpus entries, the eight fixtures, counts for every scenario, the V3b captures, and the parity job that guards them. The goldens are created under the initial-creation rule. Mechanical lines are generated golden data.

**Touches** (reserved): `src/automation/trace.rs`, `src/automation/parity.rs`, `src/automation/melee_fixture.rs`, `src/ships/hook_registry.rs`, `src/battle/battle_native.rs`, `src/mainloop/game_loop.rs`, `xtask/src/parity.rs`, `xtask/parity-corpus.toml`, `tests/golden/`, `tests/save_runtime_capture.rs`, `scripts/`, `.github/workflows/ci.yaml`.
**Shared:** `xtask/src/lib.rs`, `src/ships/mod.rs`.

**Change**
- A checkpoint call where rust_battle_frame_advance runs and at each mode's loop end; I, S and P lines with the netplay CRC, the state digest and the canvas digest; state digest per element, ship and mode checkpoint as plan.md V1 lists; no addresses or padding hashed; field values to a side file.
- src/ships/hook_registry.rs lists every hook in ships/races/*_native.rs, battle and navigation under a stable name; events spawn, hook, death.
- cargo xtask parity record|check builds uqm --release as a subprocess and prints the first differing step and fields; parity counts record|check runs every rust/scripts/*.json under replay and compares input_seen, present_seen and the action-phase sequence with tests/golden/parity/counts/<scenario>.toml.
- Corpus manifest with melee-cyborg-a..d, melee-input, campaign-battle, samatra, hyperspace, solar-flight, lander, comm, new-game; check asserts from events that each of the 25 melee ships spawned and fought two different opponent species. New scenario files only where an entry needs one; existing scenario JSON does not change.
- Fixtures androsynth-blazer, pkunk-rebirth, chmmr-satellites, death-spawn, mixed-collision, pool-full, frame-sets, star-contact on melee_fixture setups, a few hundred steps each.
- Goldens under rust/tests/golden/parity/ and rust/tests/golden/saves/ (V3b: per save, load under replay with the clock frozen, enter the saved mode, capture at the first input admission; normalisations listed as data). Each is committed only after three byte-identical recordings: macOS, ubuntu CI, UQM_REPLAY_JITTER under CPU load. The owner approves this PR as the initial creation.
- parity job: fails when a file under rust/tests/golden/ that exists at base is modified or deleted without parity-golden-update, or when a new golden file arrives without its manifest entry. cargo xtask quality --tighten; record the coverage floor.

**Deletes** (gone when the PR merges; the acceptance checks prove it)
- Nothing: the unit adds the V1 gate and replaces no code. Every item it adds has a caller in the same PR (the parity job, the manifest, the tests).

**Out of scope.** converting any race (mixed-collision runs on today's code).

**Acceptance**
- S (check bundle)
- P (check bundle)
- `cd rust && cargo test -p uqm-xtask --no-default-features --locked parity::`
- `cd rust && cargo xtask parity record --all --out ../tmp/U0-06/mac && UQM_REPLAY_JITTER=7 cargo xtask parity record --all --out ../tmp/U0-06/jitter && diff -r ../tmp/U0-06/mac ../tmp/U0-06/jitter`
- the ubuntu parity job artifact equals ../tmp/U0-06/mac
- `cd rust && for f in androsynth-blazer pkunk-rebirth chmmr-satellites death-spawn mixed-collision pool-full frame-sets star-contact; do cargo xtask parity check --entry $f || exit 1; done`
- V3 (check bundle)
- V2 (check bundle)

**Exit numbers.** every hook has a registry name (a test enumerates them); corpus 9 of 9 and fixtures 8 of 8 with three identical recordings; counts golden for every rust/scripts/*.json; V3b 74 of 74; parity job green on the PR and on the push to main; baseline tightened.

**Risks**
- The checkpoint call must not change frame order; it reads only.
- Any nondeterminism found here is a bug: find the varying input and fix it at its source; never retry or loosen.
- mixed-collision needs one converted race; record it now on today's code so U1-03 must reproduce it.

## Phase 1: battle core pilot (Stage A, Stage B, first Native hooks) and uqm-engine

Exit: one definition of each battle record and queue static; every hook call goes through invoke_*; Race, ElementHooks and Cx APIs approved; VUX, Androsynth and the battle core's own hooks are Native with 0 unsafe; stale-id lookups = 0; shared battle test harness in tests/support/; uqm-engine is a member; planned.uqm-engine = 0; about 50k production lines left the root.

### U1-01 Stage A: one authority for battle records and queue statics (hook fields unchanged)

Phase 1 · mechanical · lane `battle` · about 1,800 mechanical changed lines · depends on U0-06 · **critical path** · `owner-review`

**Replaces** P1-01 (#606).

**Scope.** Record types and their queues get one definition in the staging module src/engine/. Record layouts and hook fields stay exactly as they are.

**Touches** (reserved): `src/engine/`, `src/abi/ship_abi.rs`, `src/abi/navigation_abi.rs`, `src/abi/netplay_abi.rs`, `src/abi/global_state_abi.rs`, `src/game_init/kernel_abi.rs`, `src/battle/process_native.rs`.
**Shared:** `src/lib.rs`.

**Change**
- engine::battle::record holds the only ELEMENT, STARSHIP, RACE_DESC, QUEUE, PRIMITIVE; the copies at navigation_abi.rs:3479, netplay_abi.rs:248 and global_state_abi.rs:139,188 become re-exports.
- engine::battle::legacy holds disp_q and race_q (game_init/kernel_abi.rs:36,43), DisplayArray, DisplayLinks, DisplayFreeList (battle/process_native.rs:259-266); symbol names unchanged.
- [budget] planned_engine_core with a finite limit (QG 3.5, 4); the owner applies quality-growth-approved.

**Deletes** (gone when the PR merges; the acceptance checks prove it)
- the record copies at navigation_abi.rs:3479, netplay_abi.rs:248, global_state_abi.rs:139 and the [u128; 24] RACE_DESC at global_state_abi.rs:188, with their layout assertions and link_name aliases
- the definitions of disp_q and race_q in game_init/kernel_abi.rs:36,43 and of DisplayArray, DisplayLinks, DisplayFreeList in battle/process_native.rs:259-266 (moved to engine::battle::legacy)

**Out of scope.** hook representation, ids, Cx and the dispatcher (U1-02); any race logic.

**Acceptance**
- S (check bundle)
- P (check bundle)
- `cd rust && for t in ELEMENT STARSHIP RACE_DESC; do cargo xtask quality ident $t --defs --expect 1 || exit 1; done`
- `cd rust && cargo xtask quality ident disp_q race_q DisplayArray DisplayLinks DisplayFreeList --defs --expect-only engine/battle/legacy.rs`
- `cd rust && rg -q 'link_name\s*=\s*"(disp_q|race_q|DisplayArray|DisplayLinks|DisplayFreeList)"' src; test $? -eq 1`
- `cd rust && cargo xtask quality report planned_engine_core`
- V2 (check bundle)

**Exit numbers.** Rust types named ELEMENT 4 -> 1; RACE_DESC 4 -> 1; one definition of each queue static; planned_engine_core budget present and within its limit.

**Risks**
- Keep #[no_mangle] names so unconverted extern declarations still link.

### U1-02 Stage B: ElementTable, ids, Cx, app::bridge, hook enums and invoke_*; process loop and collisions on ids; battle statics into Battle

Phase 1 · semantic · lane `battle` · about 4,200 semantic and 1,500 mechanical changed lines · depends on U1-01 · **critical path** · `owner-review`

**Replaces** P1-02 (#607), P1-03 (#608).

**Scope.** The Stage B view and its first consumer in one transition: the hook representation changes once ElementId and Cx exist, the dispatcher is functional from its first commit, and the process loop uses the new types in the same PR. The Native hook variant arrives with its first users in U1-03; LinkQueue arrives with its first users in U2-06.

**Touches** (reserved): `src/engine/battle/`, `src/app/`, `src/battle/`, `src/ships/races/`, `src/ships/native_adapters.rs`, `src/ships/battle_bridge.rs`, `src/navigation/interplanetary.rs`, `src/navigation/hyper_transition.rs`, `src/navigation/hyper_encounters.rs`, `src/navigation/hyper_radar.rs`, `src/abi/ship_abi.rs`, `src/pickship/ffi.rs`, `src/supermelee/native/battle_picker.rs`, `src/automation/battle_fight.rs`, `src/automation/battle_outcome.rs`, `src/automation/campaign_state.rs`, `src/automation/melee_fixture.rs`, `src/automation/script.rs`, `tests/`.
**Shared:** `src/ships/hook_registry.rs`, `src/lib.rs`.

**Change**
- ElementTable over the authority with a generation side array; ElementId/ShipId; HELEMENT <-> ElementId by slot arithmetic; accessors decode C fields (parent() by address range); ReinitQueue bumps every generation.
- Cx { b, world, rng }; app::bridge holds one published NonNull<Cx> (the plan's second new global, removed in U6-04), LIFO, panics when entered empty; AllocElement, do_damage and ProcessSound enter through it.
- Hook fields become ElementHook/CollisionHook (Legacy variant); every legacy write becomes Some(..::Legacy(f)) in one mechanical commit over ships/races, battle, navigation and the race tests; every hook call goes through invoke_*, which publishes Cx.
- Process loop on head()/succ(id), reading the successor where C does (process_native.rs:746-752, :494); process_collisions keeps recursion, next_test and head() restarts and player-first order (:642-668) with copy-call-reread.
- zoom_out, nth_frame, OPT_MAX_ZOOM_OUT (:259-266) and battle_counter (battle_native.rs:79) move into Battle; their readers (pickship/ffi.rs, supermelee/native/battle_picker.rs, battle/{encount_native,ffi,galaxy,lifecycle}.rs, automation/{battle_fight,battle_outcome,campaign_state,melee_fixture,script}.rs) use counted bridge accessors.
- [budget] bridge_calls and legacy_hooks with finite limits; the staging accessors' unsafe is this unit's unsafe allowance; the owner applies quality-growth-approved.

**Deletes** (gone when the PR merges; the acceptance checks prove it)
- every direct call of an element hook field (each now goes through invoke_*)
- the raw pred/succ walks of the element loop and process_collisions in battle/process_native.rs (:494, :642-668, :746-752)
- static mut zoom_out, nth_frame, OPT_MAX_ZOOM_OUT (process_native.rs:259-266) and battle_counter (battle_native.rs:79)
- the Option<unsafe extern "C" fn> hook field types of ELEMENT (abi/ship_abi.rs:1302-1319)

**Out of scope.** converting any race or hook body (U1-03); LinkQueue (U2-06).

**Acceptance**
- S (check bundle)
- P (check bundle)
- `cd rust && cargo test --locked engine::battle::`
- `cd rust && cargo xtask quality report hook_calls --expect 0`
- `cd rust && rg -q 'static mut (zoom_out|nth_frame|OPT_MAX_ZOOM_OUT|battle_counter)\b' src; test $? -eq 1`
- `cd rust && cargo xtask quality report legacy_hooks && cargo xtask quality report bridge_calls`
- V2 (check bundle)
- V4 (smoke plus 7 scenarios); inspect both galleries with the image-comparer:

  ```sh
  pkill -x uqm || true
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native smoke --artifacts "$PWD/tmp/U1-02/v4-smoke" --content "$PWD/sc2/content/packages"
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native gallery "$PWD/tmp/U1-02/v4-smoke" --output "$PWD/tmp/U1-02/v4-smoke-gallery"
  pkill -x uqm || true
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native run --artifacts "$PWD/tmp/U1-02/v4-run" --content "$PWD/sc2/content/packages" \
    --scenarios "battle-v1,cyborg-encounter-v1,super-melee-mutual-loss-v1,thraddash-guard-v1,probe-encounter-v1,fly-to-alpha-centauri-v1,load-alpha-centauri-v1"
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native gallery "$PWD/tmp/U1-02/v4-run" --output "$PWD/tmp/U1-02/v4-run-gallery"
  ```

**Exit numbers.** hook calls outside invoke_*: 0; battle static_mut -4; legacy_hooks at its limit; bridge_calls within its budget; stale-id lookups across the corpus = 0.

**Risks**
- Pointers handed to legacy code must derive from the static's address, never from a Rust reference (4.1 rule 2).
- Androsynth's save and restore (androsynth_native.rs:389-397) copies the whole hook value; androsynth-blazer is the oracle.
- Reading succ at a different point changes which appended elements a pass visits; death-spawn is the oracle.

### U1-03 Proof: Race, ElementHooks and Native hooks; VUX, Androsynth, the battle core's own hooks and the star callbacks; shared battle test harness

Phase 1 · semantic · lane `battle` · about 4,600 changed lines · depends on U1-02 · **critical path** · `owner-review`

**Replaces** P1-04 (#609).

**Scope.** The API proof. The Native variant arrives with real users: two races, the battle core's own hooks that every ship uses, and the hyperspace star callbacks. No further conversion merges before this PR.

**Touches** (reserved): `src/ships/races/vux_native.rs`, `src/ships/races/androsynth_native.rs`, `src/battle/ship_native.rs`, `src/battle/tactrans_native.rs`, `src/battle/world_elements.rs`, `src/battle/weapon_native.rs`, `src/battle/battle_native.rs`, `src/battle/gravity.rs`, `src/battle/status_native.rs`, `src/ships/native_adapters.rs`, `src/navigation/hyper_transition.rs`, `src/engine/battle/`, `tests/vux_native.rs`, `tests/androsynth_native.rs`, `tests/support/`.
**Shared:** `src/ships/code_resources.rs`, `src/ships/hook_registry.rs`, `src/ships/races/mod.rs`.

**Change**
- trait Race, ElementHooks and the Cx API as architecture.md 5.1-5.3; ElementHook and CollisionHook gain the Native variant; race(species) in ships/races/mod.rs with an arm per converted race, and the race-level dispatch reads RACE_DESC function pointers only for races without an arm (adapter A2).
- The battle core's hook functions become Native: ship_preprocess, ship_postprocess, collision (ship_native.rs:391-394), ship_death, new_ship, preprocess_dead_ship, explosion_preprocess, cleanup_dead_ship, ship_transition, cycle_ion_trail, animation_preprocess (tactrans_native.rs), the asteroid, rubble, planet and crew hooks (world_elements.rs), gravity_postprocess, PostProcessStatus, the weapon hooks (weapon_native.rs) and flee_preprocess (battle_native.rs:210); ships/native_adapters.rs:136-139 installs them as Native, so converted races never write a Legacy core hook.
- VUX and Androsynth as Race impls with Native hooks, including Androsynth's hook save and restore (androsynth_native.rs:389-397).
- star_collision and star_death (navigation/hyper_transition.rs:191-229) on Cx.
- tests/support/battle.rs: one battle test harness (element and ship builders over ElementTable, hook calls by registry name through invoke_*) for legacy and converted races; the VUX and Androsynth tests use it and keep their golden data. Each race unit moves its tests onto it.

**Deletes** (gone when the PR merges; the acceptance checks prove it)
- static mut DESC in vux_native.rs and ANDROSYNTH_DESC, and the unsafe extern "C" hook functions of both races
- the unsafe extern "C" fn(*mut ELEMENT) forms of the battle core hook functions listed under Change, and their Legacy writes in battle/ and ships/native_adapters.rs
- the extern "C" forms of star_collision and star_death
- the extern "C" declarations and raw builders in tests/vux_native.rs and tests/androsynth_native.rs

**Out of scope.** other races; the remaining navigation callbacks (U2-07).

**Acceptance**
- S (check bundle)
- P (check bundle)
- `cd rust && for f in mixed-collision death-spawn androsynth-blazer star-contact chmmr-satellites; do cargo xtask parity check --entry $f || exit 1; done`
- `cd rust && cargo test --locked --test vux_native --test androsynth_native`
- `cd rust && for f in vux androsynth; do for m in unsafe static_mut legacy_hooks; do cargo xtask quality report $m:ships/races/${f}_native.rs --expect 0 || exit 1; done; done`
- `cd rust && for s in battle ships/native_adapters.rs; do cargo xtask quality report legacy_hooks:$s --expect 0 || exit 1; done`
- `cd rust && cargo xtask quality ident ANDROSYNTH_DESC --expect 0`
- V2 (check bundle)
- owner approval comment on the PR
- V4 (smoke plus 4 scenarios); inspect both galleries with the image-comparer:

  ```sh
  pkill -x uqm || true
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native smoke --artifacts "$PWD/tmp/U1-03/v4-smoke" --content "$PWD/sc2/content/packages"
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native gallery "$PWD/tmp/U1-03/v4-smoke" --output "$PWD/tmp/U1-03/v4-smoke-gallery"
  pkill -x uqm || true
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native run --artifacts "$PWD/tmp/U1-03/v4-run" --content "$PWD/sc2/content/packages" \
    --scenarios "battle-v1,cyborg-encounter-v1,super-melee-mutual-loss-v1,fly-to-alpha-centauri-v1"
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native gallery "$PWD/tmp/U1-03/v4-run" --output "$PWD/tmp/U1-03/v4-run-gallery"
  ```

**Exit numbers.** vux_native.rs and androsynth_native.rs: unsafe 0, raw-pointer parameters 0; legacy_hooks in battle and native_adapters 0; legacy_hooks budget reduced by the converted hook count.

**Risks**
- Every later race PR builds on this API and harness; changes after approval cost all of them.
- A Native core hook calls an unconverted race's RACE_DESC function through the dispatcher, which publishes Cx; no record reference may be held across that call (4.1 rule 3).

### U1-04 uqm-engine: invert the libs layer's references to the game, then move the layer into the crate

Phase 1 · semantic · lane `crates` · about 1,200 semantic and 1,500 mechanical changed lines · depends on U1-01

**Replaces** P1-05 (#610), P1-06 (#611).

**Scope.** planned.uqm-engine to 0 (13 crate:: paths and 7 foreign statics) in a semantic commit, then the mechanical move of about 50k production lines in a second commit that changes nothing inside moved files except visibility.

**Touches** (reserved): `Cargo.toml`, `build.rs`, `crates/engine/`, `src/graphics/`, `src/sound/`, `src/resource/`, `src/video/`, `src/strings/`, `src/threading/`, `src/input/`, `src/io/`, `src/network/`, `src/network.rs`, `src/callback/`, `src/collections/`, `src/math/`, `src/memory.rs`, `src/logging.rs`, `src/log_abi.rs`, `src/bridge_log.rs`, `src/platform.rs`, `src/time/mod.rs`, `src/time/counter_abi.rs`, `src/time/source.rs`, `src/abi/graphics_abi.rs`, `src/mainloop/options_abi.rs`, `src/mainloop/dir_prep.rs`, `src/automation/input_ffi.rs`, `src/automation/capture.rs`, `src/automation/native_window.rs`, `src/automation/child_session.rs`, `src/ships/native.rs`.
**Shared:** `src/ships/code_resources.rs`, `src/game_init/kernel_abi.rs`, `src/lib.rs`, `xtask/src/quality/crate_deps.rs`.

**Change**
- Semantic commit: PresentedFrame moves into graphics; the present callback and readback become fn-pointer fields present_hook and readback_hook of the existing graphics backend state (GRAPHICS_BACKEND, graphics/sdl/lifecycle_abi.rs:39) set by automation at init (adapter A5, removed in U7-01); no new static.
- The native window placement (graphics/ffi.rs:254, ffi_video.rs:25) is passed into graphics init by its caller.
- contentDir, configDir, saveDir, meleeDir, contentMountHandle (mainloop/options_abi.rs:18-20), repository and rootDir (game_init/kernel_abi.rs:44-45) are defined in io and input_templates (:21) in input, with unchanged symbol names.
- resource exposes type registration and kernel init registers the ship code type in today's order (resource/ffi_bridge.rs:60); guard_descriptor_inheritance moves to threading (critical_timer.rs:53); INTERSECT_CONTROL is defined in graphics and re-exported by the ships ABI (geometry_ffi.rs:438).
- Move commit: git mv the listed modules to crates/engine/src/ with module paths kept; crates/engine/lib.rs declares them; the root re-exports them (transitional until U8-01); time splits: time/counter_abi.rs and U0-05's time/source.rs move to crates/engine/src/time/, declared by a new crates/engine/src/time/mod.rs, and the root keeps the game clock (time/mod.rs, clock_abi.rs, clock_bridge.rs) and re-exports both from its time/mod.rs; features bundled-sdl, debug-process, audio_heart forwarded; rust/build.rs moves to crates/engine/build.rs; external crates used only by moved code move to crates/engine/Cargo.toml; widen the roughly 18 pub(crate) items used across the boundary to pub.
- quality move --prefix for each module; CRATE_DEPS uqm -> uqm-engine -> uqm-save; [crate_os."uqm-engine"] baseline; delete the planned.uqm-engine entry.

**Deletes** (gone when the PR merges; the acceptance checks prove it)
- the 13 upward crate:: paths (graphics/ffi.rs:137,254,565,566,574,596; graphics/ffi_video.rs:25; graphics/sdl/lifecycle_abi.rs:45,47; graphics/geometry_ffi.rs:438; input/sdl_input.rs:14; resource/ffi_bridge.rs:60; threading/critical_timer.rs:53)
- the 7 foreign static declarations in the libs layer (repository at graphics/gfxload.rs:12; input_templates at input/sdl_input.rs:57; contentDir at resource/ffi_bridge.rs:67, sound/heart_ffi.rs:77, strings/getstr.rs:27, video/control.rs:21, abi/graphics_abi.rs:998) and their definitions in the game
- the test duplicates of the dir handles in mainloop/dir_prep.rs:168-176
- rust/build.rs and the root copies of the moved modules (git mv), including src/time/counter_abi.rs and src/time/source.rs; the planned.uqm-engine baseline entry

**Out of scope.** the game clock (time/clock_*.rs stays in the root); renames; removing re-exports (U8-01).

**Acceptance**
- S (check bundle)
- P (identical without re-recording)
- `cd rust && cargo test -p uqm-engine --locked`
- `mkdir -p tmp/U1-04 && cd rust && cargo tree -p uqm-engine -e normal --locked > ../tmp/U1-04/tree.txt && { grep -q 'uqm-rust ' ../tmp/U1-04/tree.txt; test $? -eq 1; }`
- `cd rust && for d in graphics sound resource video strings threading input io network callback collections math; do test ! -e src/$d || exit 1; done && test ! -e build.rs`
- `cd rust && cargo xtask quality ident contentDir repository input_templates --defs --expect-only uqm-engine`
- `cd rust && test ! -e src/time/counter_abi.rs && test ! -e src/time/source.rs && cargo xtask quality ident TIME_SOURCE GetTimeCounter --defs --expect-only uqm-engine`
- `cd rust && rg -q 'fn (test_)?dir_handles|static mut (contentDir|configDir|saveDir|meleeDir)' src/mainloop/dir_prep.rs; test $? -eq 1`
- `cd rust && cargo build --release --locked --bin uqm && target/release/uqm --check-content --contentdir=../sc2/content`
- `mkdir -p tmp/U1-04 && cd rust && cargo build --locked && touch src/comm/mod.rs && cargo build --locked > ../tmp/U1-04/rebuild.log 2>&1 && { grep -q 'Compiling uqm-engine ' ../tmp/U1-04/rebuild.log; test $? -eq 1; }`
- `cd rust && cargo build --release --locked --timings   # attach the clean before and after reports`
- V2 (check bundle)
- R5 (check bundle)
- V4 (smoke plus 2 scenarios); inspect both galleries with the image-comparer:

  ```sh
  pkill -x uqm || true
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native smoke --artifacts "$PWD/tmp/U1-04/v4-smoke" --content "$PWD/sc2/content/packages"
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native gallery "$PWD/tmp/U1-04/v4-smoke" --output "$PWD/tmp/U1-04/v4-smoke-gallery"
  pkill -x uqm || true
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native run --artifacts "$PWD/tmp/U1-04/v4-run" --content "$PWD/sc2/content/packages" \
    --scenarios "introduction-v1,setup-menu-v1"
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native gallery "$PWD/tmp/U1-04/v4-run" --output "$PWD/tmp/U1-04/v4-run-gallery"
  ```

**Exit numbers.** planned.uqm-engine 20 -> 0 and its entry deleted; workspace members 3 -> 4; root production lines about -50k.

**Risks**
- Resource type registration order assigns type indices; keep it.
- The capture generation must still be read inside present, as graphics/ffi.rs:565-574 does.
- A #[no_mangle] symbol defined in the engine and declared only in root extern blocks must still link with GNU ld (game-build Linux jobs).
- Do the move commit last, right after a rebase; units holding moved paths rebase after the merge.

## Phase 2: ships, combat AI, every record reader, Stage C

Exit: raw_records outside engine::battle = 0 before Stage C; legacy_hooks = 0 and its budget deleted; no RACE_DESC function-pointer dispatch; ships and battle have no ffi or unsafe baseline entries; OrderedPool is the element and ship storage; corpus assertions hold for every race.

### U2-01 Races: Kohr-Ah, Druuge, Utwig, Zoq-Fot-Pik, Mycon, Yehat, Supox, Syreen, Mmrnmhrm, Ur-Quan

Phase 2 · semantic · lane `races` · about 5,400 changed lines · depends on U1-03 · **critical path**

**Replaces** P2-02 (#613), P2-03 (#614).

**Scope.** Kohr-Ah, Druuge, Utwig, Zoq-Fot-Pik, Mycon, Yehat, Supox, Syreen, Mmrnmhrm, Ur-Quan to impl Race with Native hooks (src 3,197 + tests 2,455 physical lines, minus unchanged lines). At most one hard race (Melnorme, Chmmr, Pkunk, Orz) per unit, each with its fixture.

**Touches** (reserved): `src/ships/races/blackurq_native.rs`, `src/ships/races/druuge_native.rs`, `src/ships/races/utwig_native.rs`, `src/ships/races/zoqfot_native.rs`, `src/ships/races/mycon_native.rs`, `src/ships/races/yehat_native.rs`, `src/ships/races/supox_native.rs`, `src/ships/races/syreen_native.rs`, `src/ships/races/mmrnmhrm_native.rs`, `src/ships/races/urquan_native.rs`, `tests/blackurq_native.rs`, `tests/druuge_native.rs`, `tests/utwig_native.rs`, `tests/zoqfot_native.rs`, `tests/mycon_native.rs`, `tests/yehat_native.rs`, `tests/supox_native.rs`, `tests/syreen_native.rs`, `tests/mmrnmhrm_native.rs`, `tests/urquan_native.rs`.
**Shared:** `src/ships/code_resources.rs`, `src/ships/hook_registry.rs`, `src/ships/races/mod.rs`, `tests/support/battle.rs`.

**Change**
- Each race becomes impl Race; descriptor() returns plain data; its static mut template goes and race(species) gains its arm, so its RACE_DESC function-pointer entries are no longer read.
- Element hooks become fn(&mut Cx, ..) in ElementHooks; RNG draws and sound calls keep their order.
- Kohr-Ah keeps fighter and FIRE allocation order; Ur-Quan fighters keep allocation order; pool-full is their oracle.
- Mmrnmhrm swaps descriptors on transform; the per-ship RaceDesc copy is the one edited.
- Each race's tests move onto tests/support/battle.rs, drop their extern "C" declarations and keep their golden data.

**Deletes** (gone when the PR merges; the acceptance checks prove it)
- the static mut race templates: DRUUGE_DESC, MYCON_DESC, SUPOX_DESC, SYREEN_DESC, MMRNMHRM_DESC; static mut DESC in blackurq_native.rs, utwig_native.rs, zoqfot_native.rs, yehat_native.rs, urquan_native.rs
- the unsafe extern "C" hook and race functions in the 10 race files and their legacy hook writes
- the extern "C" symbol declarations and raw element builders in the race test files

**Out of scope.** ships loader and race templates (U2-08); AI (U2-06); Stage C representation (U2-09).

**Acceptance**
- S (check bundle)
- P (melee-cyborg-a..d assert weapon and special hooks ran for these races)
- `cd rust && cargo xtask parity check --entry pool-full`
- `cd rust && cargo test --locked --test blackurq_native --test druuge_native --test utwig_native --test zoqfot_native --test mycon_native --test yehat_native --test supox_native --test syreen_native --test mmrnmhrm_native --test urquan_native`
- `cd rust && for f in blackurq druuge utwig zoqfot mycon yehat supox syreen mmrnmhrm urquan; do for m in unsafe static_mut legacy_hooks; do cargo xtask quality report $m:ships/races/${f}_native.rs --expect 0 || exit 1; done; done`
- `cd rust && cargo xtask quality ident DRUUGE_DESC MYCON_DESC SUPOX_DESC SYREEN_DESC MMRNMHRM_DESC --expect 0`
- V2 (check bundle)
- V4 (smoke plus 2 scenarios); inspect both galleries with the image-comparer:

  ```sh
  pkill -x uqm || true
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native smoke --artifacts "$PWD/tmp/U2-01/v4-smoke" --content "$PWD/sc2/content/packages"
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native gallery "$PWD/tmp/U2-01/v4-smoke" --output "$PWD/tmp/U2-01/v4-smoke-gallery"
  pkill -x uqm || true
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native run --artifacts "$PWD/tmp/U2-01/v4-run" --content "$PWD/sc2/content/packages" \
    --scenarios "battle-v1,super-melee-mutual-loss-v1"
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native gallery "$PWD/tmp/U2-01/v4-run" --output "$PWD/tmp/U2-01/v4-run-gallery"
  ```

**Exit numbers.** these 10 files: unsafe 0, static_mut 0, legacy_hooks 0, raw-pointer parameters 0; legacy_hooks budget reduced by their hook count.

**Risks**
- Shared registry files: rebase and rerun acceptance after any other race unit merges.

### U2-02 Races: Thraddash, Ilwrath, Melnorme, SIS flagship

Phase 2 · semantic · lane `races` · about 5,100 changed lines · depends on U1-03

**Replaces** P2-04 (#615), P2-08 (#619).

**Scope.** Thraddash, Ilwrath, Melnorme, SIS flagship to impl Race with Native hooks (src 1,834 + tests 3,234; Melnorme is the hard race). At most one hard race (Melnorme, Chmmr, Pkunk, Orz) per unit, each with its fixture.

**Touches** (reserved): `src/ships/races/thraddash_native.rs`, `src/ships/races/ilwrath_native.rs`, `src/ships/races/melnorme_native.rs`, `src/ships/races/sis_native.rs`, `tests/thraddash_native.rs`, `tests/ilwrath_native.rs`, `tests/melnorme_native.rs`, `tests/sis_native.rs`.
**Shared:** `src/ships/code_resources.rs`, `src/ships/hook_registry.rs`, `src/ships/races/mod.rs`, `tests/support/battle.rs`.

**Change**
- Each race becomes impl Race; descriptor() returns plain data; its static mut template goes and race(species) gains its arm, so its RACE_DESC function-pointer entries are no longer read.
- Element hooks become fn(&mut Cx, ..) in ElementHooks; RNG draws and sound calls keep their order.
- Thraddash afterburner trail elements and Ilwrath cloaking keep their flag and frame order; Melnorme confusion pulses keep their element order.
- SIS reads crew and commander name through the GlobData definition until U4-01.
- Each race's tests move onto tests/support/battle.rs, drop their extern "C" declarations and keep their golden data.

**Deletes** (gone when the PR merges; the acceptance checks prove it)
- the static mut race templates: THRADDASH_DESC, ILWRATH_DESC, MELNORME_DESC; static mut DESC in sis_native.rs
- the unsafe extern "C" hook and race functions in the 4 race files and their legacy hook writes
- the extern "C" symbol declarations and raw element builders in the race test files

**Out of scope.** ships loader and race templates (U2-08); AI (U2-06); Stage C representation (U2-09).

**Acceptance**
- S (check bundle)
- P (melee-cyborg-a..d assert weapon and special hooks ran for these races)
- `cd rust && cargo xtask parity check --entry campaign-battle`
- `cd rust && cargo test --locked --test thraddash_native --test ilwrath_native --test melnorme_native --test sis_native`
- `cd rust && for f in thraddash ilwrath melnorme sis; do for m in unsafe static_mut legacy_hooks; do cargo xtask quality report $m:ships/races/${f}_native.rs --expect 0 || exit 1; done; done`
- `cd rust && cargo xtask quality ident THRADDASH_DESC ILWRATH_DESC MELNORME_DESC --expect 0`
- V2 (check bundle)
- V4 (smoke plus 5 scenarios); inspect both galleries with the image-comparer:

  ```sh
  pkill -x uqm || true
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native smoke --artifacts "$PWD/tmp/U2-02/v4-smoke" --content "$PWD/sc2/content/packages"
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native gallery "$PWD/tmp/U2-02/v4-smoke" --output "$PWD/tmp/U2-02/v4-smoke-gallery"
  pkill -x uqm || true
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native run --artifacts "$PWD/tmp/U2-02/v4-run" --content "$PWD/sc2/content/packages" \
    --scenarios "battle-v1,super-melee-mutual-loss-v1,thraddash-guard-v1,campaign-s06c-utwig-bomb,load-connected-final-fleet-v1"
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native gallery "$PWD/tmp/U2-02/v4-run" --output "$PWD/tmp/U2-02/v4-run-gallery"
  ```

**Exit numbers.** these 4 files: unsafe 0, static_mut 0, legacy_hooks 0, raw-pointer parameters 0; legacy_hooks budget reduced by their hook count.

**Risks**
- Shared registry files: rebase and rerun acceptance after any other race unit merges.

### U2-03 Races: Slylandro, Umgah, Chmmr, Spathi

Phase 2 · semantic · lane `races` · about 5,400 changed lines · depends on U1-03

**Replaces** P2-05 (#616), P2-09 (#620).

**Scope.** Slylandro, Umgah, Chmmr, Spathi to impl Race with Native hooks (src 1,555 + tests 3,786; Chmmr is the hard race). At most one hard race (Melnorme, Chmmr, Pkunk, Orz) per unit, each with its fixture.

**Touches** (reserved): `src/ships/races/slylandro_native.rs`, `src/ships/races/umgah_native.rs`, `src/ships/races/chmmr_native.rs`, `src/ships/races/spathi_native.rs`, `tests/slylandro_native.rs`, `tests/umgah_native.rs`, `tests/chmmr_native.rs`, `tests/spathi_native.rs`.
**Shared:** `src/ships/code_resources.rs`, `src/ships/hook_registry.rs`, `src/ships/races/mod.rs`, `tests/support/battle.rs`.

**Change**
- Each race becomes impl Race; descriptor() returns plain data; its static mut template goes and race(species) gains its arm, so its RACE_DESC function-pointer entries are no longer read.
- Element hooks become fn(&mut Cx, ..) in ElementHooks; RNG draws and sound calls keep their order.
- Slylandro lightning and Umgah antimatter cone keep their element order.
- Chmmr satellites walk pred links (chmmr_native.rs:406-460, 494-507) through ElementTable.
- Each race's tests move onto tests/support/battle.rs, drop their extern "C" declarations and keep their golden data.

**Deletes** (gone when the PR merges; the acceptance checks prove it)
- the static mut race templates: SLYLANDRO_DESC, UMGAH_DESC, CHMMR_DESC, SPATHI_DESC
- the unsafe extern "C" hook and race functions in the 4 race files and their legacy hook writes
- the extern "C" symbol declarations and raw element builders in the race test files

**Out of scope.** ships loader and race templates (U2-08); AI (U2-06); Stage C representation (U2-09).

**Acceptance**
- S (check bundle)
- P (melee-cyborg-a..d assert weapon and special hooks ran for these races)
- `cd rust && cargo xtask parity check --entry chmmr-satellites`
- `cd rust && cargo test --locked --test slylandro_native --test umgah_native --test chmmr_native --test spathi_native`
- `cd rust && for f in slylandro umgah chmmr spathi; do for m in unsafe static_mut legacy_hooks; do cargo xtask quality report $m:ships/races/${f}_native.rs --expect 0 || exit 1; done; done`
- `cd rust && cargo xtask quality ident SLYLANDRO_DESC UMGAH_DESC CHMMR_DESC SPATHI_DESC --expect 0`
- V2 (check bundle)
- V4 (smoke plus 3 scenarios); inspect both galleries with the image-comparer:

  ```sh
  pkill -x uqm || true
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native smoke --artifacts "$PWD/tmp/U2-03/v4-smoke" --content "$PWD/sc2/content/packages"
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native gallery "$PWD/tmp/U2-03/v4-smoke" --output "$PWD/tmp/U2-03/v4-smoke-gallery"
  pkill -x uqm || true
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native run --artifacts "$PWD/tmp/U2-03/v4-run" --content "$PWD/sc2/content/packages" \
    --scenarios "battle-v1,super-melee-mutual-loss-v1,load-connected-chmmr-bomb-v1"
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native gallery "$PWD/tmp/U2-03/v4-run" --output "$PWD/tmp/U2-03/v4-run-gallery"
  ```

**Exit numbers.** these 4 files: unsafe 0, static_mut 0, legacy_hooks 0, raw-pointer parameters 0; legacy_hooks budget reduced by their hook count.

**Risks**
- Satellite order follows pred links of the shared storage; the fixture is the oracle.
- Shared registry files: rebase and rerun acceptance after any other race unit merges.

### U2-04 Races: Shofixti, Arilou, Pkunk, Earthling, probe

Phase 2 · semantic · lane `races` · about 5,400 changed lines · depends on U1-03

**Replaces** P2-06 (#617), P2-10 (#621).

**Scope.** Shofixti, Arilou, Pkunk, Earthling, probe to impl Race with Native hooks (src 1,726 + tests 3,658; Pkunk is the hard race). At most one hard race (Melnorme, Chmmr, Pkunk, Orz) per unit, each with its fixture.

**Touches** (reserved): `src/ships/races/shofixti_native.rs`, `src/ships/races/arilou_native.rs`, `src/ships/races/pkunk_native.rs`, `src/ships/races/human_native.rs`, `src/ships/races/probe_native.rs`, `tests/shofixti_native.rs`, `tests/arilou_native.rs`, `tests/pkunk_native.rs`, `tests/human_native.rs`, `tests/probe_native.rs`.
**Shared:** `src/ships/code_resources.rs`, `src/ships/hook_registry.rs`, `src/ships/races/mod.rs`, `tests/support/battle.rs`.

**Change**
- Each race becomes impl Race; descriptor() returns plain data; its static mut template goes and race(species) gains its arm, so its RACE_DESC function-pointer entries are no longer read.
- Element hooks become fn(&mut Cx, ..) in ElementHooks; RNG draws and sound calls keep their order.
- Shofixti's second template (NEW_SHOFIXTI_DESC) goes too; Arilou's farray comparison (arilou_native.rs:216-224) uses the accessor that Stage C turns into FrameSetRef owner equality.
- Pkunk rebirth restores hooks and keeps the hTarget hack (pkunk_native.rs:401-442); NEW_PKUNK_DESC goes.
- Each race's tests move onto tests/support/battle.rs, drop their extern "C" declarations and keep their golden data.

**Deletes** (gone when the PR merges; the acceptance checks prove it)
- the static mut race templates: SHOFIXTI_DESC, NEW_SHOFIXTI_DESC, ARILOU_DESC, PKUNK_DESC, NEW_PKUNK_DESC, HUMAN_DESC, PROBE_DESC
- the unsafe extern "C" hook and race functions in the 5 race files and their legacy hook writes
- the extern "C" symbol declarations and raw element builders in the race test files

**Out of scope.** ships loader and race templates (U2-08); AI (U2-06); Stage C representation (U2-09).

**Acceptance**
- S (check bundle)
- P (melee-cyborg-a..d assert weapon and special hooks ran for these races)
- `cd rust && cargo xtask parity check --entry frame-sets`
- `cd rust && cargo xtask parity check --entry pkunk-rebirth`
- `cd rust && cargo test --locked --test shofixti_native --test arilou_native --test pkunk_native --test human_native --test probe_native`
- `cd rust && for f in shofixti arilou pkunk human probe; do for m in unsafe static_mut legacy_hooks; do cargo xtask quality report $m:ships/races/${f}_native.rs --expect 0 || exit 1; done; done`
- `cd rust && cargo xtask quality ident SHOFIXTI_DESC NEW_SHOFIXTI_DESC ARILOU_DESC PKUNK_DESC NEW_PKUNK_DESC HUMAN_DESC PROBE_DESC --expect 0`
- V2 (check bundle)
- V4 (smoke plus 3 scenarios); inspect both galleries with the image-comparer:

  ```sh
  pkill -x uqm || true
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native smoke --artifacts "$PWD/tmp/U2-04/v4-smoke" --content "$PWD/sc2/content/packages"
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native gallery "$PWD/tmp/U2-04/v4-smoke" --output "$PWD/tmp/U2-04/v4-smoke-gallery"
  pkill -x uqm || true
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native run --artifacts "$PWD/tmp/U2-04/v4-run" --content "$PWD/sc2/content/packages" \
    --scenarios "battle-v1,super-melee-mutual-loss-v1,probe-encounter-v1"
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native gallery "$PWD/tmp/U2-04/v4-run" --output "$PWD/tmp/U2-04/v4-run-gallery"
  ```

**Exit numbers.** these 5 files: unsafe 0, static_mut 0, legacy_hooks 0, raw-pointer parameters 0; legacy_hooks budget reduced by their hook count.

**Risks**
- Arilou teleport draws TFB_Random in a specific order (arilou_native.rs:260-290); keep it.
- The hTarget hack relies on a raw handle value; model it explicitly and keep the fixture byte-identical.
- Shared registry files: rebase and rerun acceptance after any other race unit merges.

### U2-05 Races: Chenjesu, Sa-Matra

Phase 2 · semantic · lane `races` · about 2,800 changed lines · depends on U1-03

**Replaces** P2-07 (#618).

**Scope.** Chenjesu, Sa-Matra to impl Race with Native hooks (src 1,321 + tests 1,429). At most one hard race (Melnorme, Chmmr, Pkunk, Orz) per unit, each with its fixture.

**Touches** (reserved): `src/ships/races/chenjesu_native.rs`, `src/ships/races/samatra_native.rs`, `src/ships/races/samatra_state.rs`, `tests/chenjesu_native.rs`, `tests/samatra_native.rs`.
**Shared:** `src/ships/code_resources.rs`, `src/ships/hook_registry.rs`, `src/ships/races/mod.rs`, `tests/support/battle.rs`.

**Change**
- Each race becomes impl Race; descriptor() returns plain data; its static mut template goes and race(species) gains its arm, so its RACE_DESC function-pointer entries are no longer read.
- Element hooks become fn(&mut Cx, ..) in ElementHooks; RNG draws and sound calls keep their order.
- Chenjesu DOGIs and crystal shards keep their element order.
- Sa-Matra state (samatra_state.rs, statics EVENTS and GENERATORS) becomes RaceState.
- Each race's tests move onto tests/support/battle.rs, drop their extern "C" declarations and keep their golden data.

**Deletes** (gone when the PR merges; the acceptance checks prove it)
- the static mut race templates: CHENJESU_DESC; static mut DESC in samatra_native.rs
- the unsafe extern "C" hook and race functions in the 2 race files and their legacy hook writes
- the extern "C" symbol declarations and raw element builders in the race test files
- the samatra_state.rs statics EVENTS (:52) and GENERATORS (:84)

**Out of scope.** ships loader and race templates (U2-08); AI (U2-06); Stage C representation (U2-09).

**Acceptance**
- S (check bundle)
- P (melee-cyborg-a..d assert weapon and special hooks ran for these races)
- `cd rust && cargo xtask parity check --entry samatra`
- `cd rust && cargo test --locked --test chenjesu_native --test samatra_native`
- `cd rust && for f in chenjesu samatra; do for m in unsafe static_mut legacy_hooks; do cargo xtask quality report $m:ships/races/${f}_native.rs --expect 0 || exit 1; done; done`
- `cd rust && cargo xtask quality ident CHENJESU_DESC --expect 0`
- `cd rust && rg -q '^static (EVENTS|GENERATORS)' src/ships/races/samatra_state.rs; test $? -eq 1`
- V2 (check bundle)
- V4 (smoke plus 3 scenarios); inspect both galleries with the image-comparer:

  ```sh
  pkill -x uqm || true
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native smoke --artifacts "$PWD/tmp/U2-05/v4-smoke" --content "$PWD/sc2/content/packages"
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native gallery "$PWD/tmp/U2-05/v4-smoke" --output "$PWD/tmp/U2-05/v4-smoke-gallery"
  pkill -x uqm || true
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native run --artifacts "$PWD/tmp/U2-05/v4-run" --content "$PWD/sc2/content/packages" \
    --scenarios "battle-v1,load-connected-samatra-remnants-v1,load-connected-samatra-distraction-v1"
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native gallery "$PWD/tmp/U2-05/v4-run" --output "$PWD/tmp/U2-05/v4-run-gallery"
  ```

**Exit numbers.** these 2 files: unsafe 0, static_mut 0, legacy_hooks 0, raw-pointer parameters 0; legacy_hooks budget reduced by their hook count.

**Risks**
- Shared registry files: rebase and rerun acceptance after any other race unit merges.

### U2-06 Battle-side record readers on ids: combat AI, pickers, encounter build, battle sounds, remaining battle files; LinkQueue

Phase 2 · semantic · lane `readers` · about 5,300 changed lines · depends on U1-03

**Replaces** P2-12 (#623), P2-14 (#625).

**Scope.** The Cyborg AI and every STARSHIP and RACE_DESC reader outside the races, so Stage C can delete the records. LinkQueue<T> lands here with its first users, the race_q walkers.

**Touches** (reserved): `src/battle/cyborg_native.rs`, `src/battle/ai.rs`, `src/battle/ai_entry.rs`, `src/battle/sounds.rs`, `src/battle/build_native.rs`, `src/battle/encount_native.rs`, `src/battle/battle_controls.rs`, `src/battle/c_bridge.rs`, `src/battle/collision.rs`, `src/battle/element.rs`, `src/battle/galaxy.rs`, `src/battle/ship_runtime.rs`, `src/battle/weapon.rs`, `src/engine/battle/link_queue.rs`, `src/pickship/ffi.rs`, `src/pickship/mod.rs`, `src/supermelee/native/battle_picker.rs`, `src/supermelee/netplay/checksum.rs`, `src/supermelee/netplay/netmelee.rs`, `src/ui/shipstat.rs`.
**Shared:** `src/engine/battle/mod.rs`.

**Change**
- LinkQueue<T>, a typed view over a QUEUE with head, succ, pred, alloc, put, remove and count over LinkId<T>; its QUEUE backing for race_q is adapter A3 until U2-09.
- cyborg_native.rs walks the queue (:170) through ElementTable; concerns and targets use ElementId; intelligence() is reached through Race; ai.rs and ai_entry.rs lose their raw records.
- pickship/ffi.rs (:164-180, :408-424) and supermelee/native/battle_picker.rs (:73-74, :391, :416) walk race_q through LinkQueue<Starship> and build picks by ShipId; RaceDescPtr = null writes go through the Starship accessor.
- battle/sounds.rs holds ElementId instead of *mut ELEMENT (:3-9) and reads positions through ElementTable instead of indexing DisplayArray (:128-138, :161-166); a freed element stops being positional.
- build_native and encount_native build race_q through LinkQueue; the netplay checksum (checksum.rs:55) and netmelee read ships by id, byte-for-byte unchanged.
- battle_controls, c_bridge, collision, element, galaxy, ship_runtime, weapon and ui/shipstat move to ids, or are deleted when dead_code shows no live caller (listed in the PR).

**Deletes** (gone when the PR merges; the acceptance checks prove it)
- raw ELEMENT/STARSHIP/RACE_DESC access in battle/cyborg_native.rs, ai.rs and ai_entry.rs
- the raw race_q walks and RaceDescPtr = null writes in pickship/ffi.rs, supermelee/native/battle_picker.rs, battle/build_native.rs, battle/encount_native.rs, supermelee/netplay/checksum.rs and netmelee.rs
- battle/sounds.rs's *mut ELEMENT (:3-9) and its DisplayArray indexing (:128-138, :161-166)
- each of battle_controls, c_bridge, collision, element, galaxy, ship_runtime, weapon and ui/shipstat that dead_code shows without a live caller

**Out of scope.** splitting tactical_intelligence (it mirrors one C function); melee session and UI ownership (U3-02).

**Acceptance**
- S (check bundle)
- P (all melee-cyborg-* entries, melee-input, campaign-battle)
- `cd rust && cargo test --locked --test netplay_packets`
- `cd rust && cargo test --locked engine::battle::link_queue`
- `cd rust && for s in battle/cyborg_native.rs battle/ai.rs battle/ai_entry.rs battle/sounds.rs battle/build_native.rs battle/encount_native.rs pickship supermelee ui; do cargo xtask quality report raw_records:$s --expect 0 || exit 1; done`
- `cd rust && cargo xtask quality report unsafe:battle/cyborg_native.rs --expect 0`
- V2 (check bundle)
- V4 (smoke plus 7 scenarios); inspect both galleries with the image-comparer:

  ```sh
  pkill -x uqm || true
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native smoke --artifacts "$PWD/tmp/U2-06/v4-smoke" --content "$PWD/sc2/content/packages"
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native gallery "$PWD/tmp/U2-06/v4-smoke" --output "$PWD/tmp/U2-06/v4-smoke-gallery"
  pkill -x uqm || true
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native run --artifacts "$PWD/tmp/U2-06/v4-run" --content "$PWD/sc2/content/packages" \
    --scenarios "cyborg-encounter-v1,battle-v1,thraddash-guard-v1,probe-encounter-v1,super-melee-mutual-loss-v1,campaign-s06c-utwig-bomb,load-connected-final-fleet-v1"
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native gallery "$PWD/tmp/U2-06/v4-run" --output "$PWD/tmp/U2-06/v4-run-gallery"
  ```

**Exit numbers.** raw_records in the AI, pickship, supermelee, ui and these battle files: 0; cyborg_native, ai, ai_entry unsafe 0.

**Risks**
- AI reads many fields per element; one misread field shows as a different input in the trace at a later step.
- The netplay checksum order is link order; it must not change.
- Sound positions that outlive their element must behave as today: stop updating at free, as the null check does.

### U2-07 Non-battle element readers on ids: navigation callbacks, planet side, save runtime, debug and automation

Phase 2 · semantic · lane `readers` · about 2,800 changed lines · depends on U1-03

**Replaces** P2-13 (#624).

**Scope.** Every non-battle, non-picker file that reads element records or DisplayArray (about 110 references at 35d33879b), including the hyperspace and IP element callbacks, which are Legacy hook users.

**Touches** (reserved): `src/navigation/hyperspace.rs`, `src/navigation/hyper_radar.rs`, `src/navigation/hyper_transition.rs`, `src/navigation/hyper_encounters.rs`, `src/navigation/interplanetary.rs`, `src/navigation/globals.rs`, `src/planet_side/collision_adapter.rs`, `src/planet_side/entities.rs`, `src/planet_side/orbit_scan.rs`, `src/planets/render/c_api.rs`, `src/savegame/runtime/queues.rs`, `src/savegame/runtime/hyper_save.rs`, `src/savegame/runtime/global.rs`, `src/savegame/runtime/live.rs`, `src/front_end/uqmdebug.rs`, `src/automation/battle_fight.rs`, `src/automation/flagship_turn.rs`, `src/automation/input_ffi.rs`, `src/automation/melee_fixture.rs`, `src/game_init/cleanup.rs`, `src/game_init/macro_abi.rs`, `src/mainloop/main2_abi.rs`, `src/mainloop/restart_menu/menu_abi.rs`, `src/comm/locdata.rs`.
**Shared:** none.

**Change**
- Hyperspace and IP element callbacks (hyper_encounters, hyper_radar, hyper_transition, interplanetary) become Native hooks on Cx; pParent writes (navigation/interplanetary.rs:428) go through the accessor.
- collision_adapter, entities, orbit_scan and planets/render/c_api.rs read primitives through ElementTable's DisplayArray accessor.
- savegame/runtime/{queues,hyper_save,global,live}.rs, uqmdebug, the automation readers, game_init cleanup and macro_abi, and the main2 and restart-menu ABI on ids.

**Deletes** (gone when the PR merges; the acceptance checks prove it)
- raw record and DisplayArray access in the touched files
- the Legacy hook functions and writes of the hyperspace and IP element callbacks
- the raw pParent write at navigation/interplanetary.rs:428

**Out of scope.** hyperspace UI (U5-04); pickers and battle files (U2-06).

**Acceptance**
- S (check bundle)
- P (hyperspace, solar-flight, lander, star-contact)
- `cd rust && for s in navigation planet_side planets savegame front_end automation game_init mainloop comm; do cargo xtask quality report raw_records:$s --expect 0 || exit 1; done`
- `cd rust && cargo xtask quality report legacy_hooks:navigation --expect 0`
- V3 (check bundle)
- V2 (check bundle)
- V4 (smoke plus 5 scenarios); inspect both galleries with the image-comparer:

  ```sh
  pkill -x uqm || true
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native smoke --artifacts "$PWD/tmp/U2-07/v4-smoke" --content "$PWD/sc2/content/packages"
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native gallery "$PWD/tmp/U2-07/v4-smoke" --output "$PWD/tmp/U2-07/v4-smoke-gallery"
  pkill -x uqm || true
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native run --artifacts "$PWD/tmp/U2-07/v4-run" --content "$PWD/sc2/content/packages" \
    --scenarios "fly-to-alpha-centauri-v1,load-alpha-centauri-v1,real-sol-planetside-collision-proof,explore-planet-v1,load-north-ready-v1"
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native gallery "$PWD/tmp/U2-07/v4-run" --output "$PWD/tmp/U2-07/v4-run-gallery"
  ```

**Exit numbers.** raw_records in these subsystems: 0; legacy_hooks in navigation: 0.

**Risks**
- hyper_save.rs writes the hyperspace save step in link order; V3b is the oracle.

### U2-08 Orz and the race dispatch: race(species), owned race data; delete Hook::Legacy and the race templates

Phase 2 · semantic · lane `races` · about 5,500 changed lines · depends on U2-01, U2-02, U2-03, U2-04, U2-05, U2-06, U2-07 · **critical path**

**Replaces** P2-11 (#622), P2-15 (#626).

**Scope.** Orz is the last legacy race. The unit depends on every other race unit and both reader units, so it converts the last user of Hook::Legacy and of the RACE_DESC function-pointer dispatch, then converts the loader and catalogue and deletes every legacy race path, including the battle entry points of app::bridge that only legacy hooks called.

**Touches** (reserved): `src/ships/races/orz_native.rs`, `src/ships/races/mod.rs`, `tests/orz_native.rs`, `tests/support/battle.rs`, `src/ships/code_resources.rs`, `src/ships/loader_native.rs`, `src/ships/master_native.rs`, `src/ships/native.rs`, `src/ships/native_adapters.rs`, `src/ships/battle_bridge.rs`, `src/ships/ffi_contract.rs`, `src/ships/ffi.rs`, `src/ships/catalog.rs`, `src/ships/lifecycle.rs`, `src/ships/writeback.rs`, `src/ships/queue.rs`, `src/ships/runtime.rs`, `src/ships/types.rs`, `src/ships/hook_registry.rs`, `src/engine/battle/hooks.rs`, `src/abi/ship_abi.rs`, `src/abi/navigation_abi.rs`, `src/app/bridge.rs`, `src/battle/process_native.rs`, `src/battle/world_elements.rs`, `src/battle/sounds.rs`, `src/battle/c_bridge.rs`.
**Shared:** none.

**Change**
- First commit asserts the precondition: legacy_hooks outside orz_native.rs is 0, and the only callers of the extern "C" AllocElement, do_damage and ProcessSound are Orz's legacy hooks, files this unit reserves (for example ships/battle_bridge.rs:463) and the re-exports (a caller elsewhere belongs to an earlier unit: stop and file).
- Orz as impl Race; marines boarding other ships keep their parent and target handling through ElementTable accessors.
- INITIALIZERS (code_resources.rs:75) replaced by race(species); Starship owns its RaceDesc copy; RaceDescPtr goes; data: *mut c_void becomes enum RaceState.
- Loader, master list (on LinkQueue<Starship>) and writeback on owned values.
- Delete Hook::Legacy, ElementProcessFunc/ElementCollisionFunc, the legacy arm of invoke_*, the RACE_DESC function-pointer fields and the legacy-race support in tests/support/battle.rs; delete the legacy_hooks budget.
- With the last legacy hook gone, the battle entry points of app::bridge have no caller and go in the same PR: the #[no_mangle] extern "C" AllocElement (battle/process_native.rs:832), do_damage (battle/world_elements.rs:193) and ProcessSound (battle/sounds.rs:120) that enter through bridge::enter, their bridge paths in src/app/bridge.rs, and their re-exports (abi/navigation_abi.rs:3503,4807; battle/c_bridge.rs:86,127 if U2-06 kept that file). Native code reaches the same operations through Cx.

**Deletes** (gone when the PR merges; the acceptance checks prove it)
- ORZ_DESC and Orz's unsafe extern "C" functions
- INITIALIZERS (code_resources.rs:75) and its 28-entry template table
- RaceDescPtr and STARSHIP's data: *mut c_void
- the RACE_DESC function-pointer fields and the race-level dispatch through them (adapter A2)
- ElementHook::Legacy, CollisionHook::Legacy, the legacy arm of invoke_*, ElementProcessFunc and ElementCollisionFunc (abi/ship_abi.rs) and the legacy_hooks budget (adapter A1)
- the legacy-race builders in tests/support/battle.rs
- the battle entry points of app::bridge (adapter A4, battle part): the extern "C" AllocElement, do_damage and ProcessSound, their bridge::enter paths in src/app/bridge.rs and their re-exports

**Out of scope.** element storage (U2-09).

**Acceptance**
- S (check bundle)
- P (check bundle)
- `cd rust && cargo test --locked --test orz_native --test ship_loader_native --test ship_build_queue`
- `cd rust && cargo xtask quality report legacy_hooks --expect 0`
- `cd rust && cargo xtask quality ident ORZ_DESC INITIALIZERS RaceDescPtr ElementProcessFunc ElementCollisionFunc --expect 0`
- `cd rust && cargo xtask quality report static_mut:ships --expect 0 && cargo xtask quality report unsafe:ships/races --expect 0`
- `cd rust && rg -q 'extern "C" fn (AllocElement|do_damage|ProcessSound)\b' src; test $? -eq 1`
- `cd rust && rg -q '\b(AllocElement|do_damage|ProcessSound)\b' src/app/bridge.rs src/abi/navigation_abi.rs; test $? -eq 1`
- V2 (check bundle)
- V4 (smoke plus 4 scenarios); inspect both galleries with the image-comparer:

  ```sh
  pkill -x uqm || true
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native smoke --artifacts "$PWD/tmp/U2-08/v4-smoke" --content "$PWD/sc2/content/packages"
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native gallery "$PWD/tmp/U2-08/v4-smoke" --output "$PWD/tmp/U2-08/v4-smoke-gallery"
  pkill -x uqm || true
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native run --artifacts "$PWD/tmp/U2-08/v4-run" --content "$PWD/sc2/content/packages" \
    --scenarios "battle-v1,super-melee-mutual-loss-v1,starbase-visit-v1,load-connected-final-fleet-v1"
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native gallery "$PWD/tmp/U2-08/v4-run" --output "$PWD/tmp/U2-08/v4-run-gallery"
  ```

**Exit numbers.** ships static_mut 32 -> 0; ships/races unsafe 0; RaceDescPtr references 0; legacy_hooks = 0 and budget deleted; battle entry points of app::bridge 3 -> 0.

**Risks**
- Marines alter the boarded ship's crew; the melee-cyborg corpus entries containing Orz are the oracle.
- VUX limpets edit the hit ship's own characteristics; the copy must be per ship, as C copies the template.

### U2-09 Stage C: OrderedPool and Id<T>; element and ship storage on pools, typed links, FrameSetRef

Phase 2 · semantic · lane `battle` · about 3,800 changed lines · depends on U2-08, U1-04 · **critical path**

**Replaces** P2-01 (#612), P2-16 (#627).

**Scope.** The storage type and its first users in one PR: the record representation is replaced once no raw reader remains.

**Touches** (reserved): `crates/engine/src/collections/pool.rs`, `crates/engine/tests/pool_order.rs`, `src/engine/battle/`, `src/battle/process_native.rs`, `src/abi/ship_abi.rs`, `tests/support/battle.rs`.
**Shared:** `crates/engine/src/collections/mod.rs`.

**Change**
- OrderedPool<T, N> and Id<T> { ix, gen } with alloc, free, get, head/succ/pred and exhaustion returning None, as architecture.md 4; order tests against collections/queue.rs: allocation 1, 2, 3; LIFO reuse of freed slots; exhaustion at 150; a randomized operation sequence applied to a QUEUE and to an OrderedPool gives the same visit order.
- A commit asserts the precondition: raw_records outside engine::battle is 0.
- Element storage, DisplayArray and race_q on OrderedPool; links are slot indices; pParent becomes Parent; hTarget becomes Option<ElementId>; unions become fields; farray becomes FrameSetRef; LinkQueue<Starship> is backed by the race pool.
- Pools live in Battle if the trace shows no element survives a mode switch, otherwise in Kernel.

**Deletes** (gone when the PR merges; the acceptance checks prove it)
- the Stage A authority statics disp_q, race_q, DisplayArray, DisplayLinks, DisplayFreeList
- the record types ELEMENT, HELEMENT, STARSHIP, HSTARSHIP, RACE_DESC with their raw links, pParent, hTarget, unions and farray (abi/ship_abi.rs, engine::battle::record)
- ElementTable's raw decoding (HELEMENT <-> ElementId slot arithmetic, parent() by address range, the generation side array) and LinkQueue's QUEUE backing for race_q (adapter A3)

**Out of scope.** GAME_STATE queues (U4-02); GlobData (U4-01).

**Acceptance**
- S (check bundle)
- P (every corpus entry and fixture)
- `cd rust && cargo test -p uqm-engine --locked --test pool_order`
- `cd rust && cargo xtask quality report raw_records --expect 0`
- `cd rust && cargo xtask quality ident disp_q race_q DisplayArray DisplayLinks DisplayFreeList ELEMENT HELEMENT STARSHIP HSTARSHIP RACE_DESC --expect 0`
- V3 (check bundle)
- V2 (check bundle)
- V4 (smoke plus 7 scenarios); inspect both galleries with the image-comparer:

  ```sh
  pkill -x uqm || true
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native smoke --artifacts "$PWD/tmp/U2-09/v4-smoke" --content "$PWD/sc2/content/packages"
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native gallery "$PWD/tmp/U2-09/v4-smoke" --output "$PWD/tmp/U2-09/v4-smoke-gallery"
  pkill -x uqm || true
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native run --artifacts "$PWD/tmp/U2-09/v4-run" --content "$PWD/sc2/content/packages" \
    --scenarios "battle-v1,cyborg-encounter-v1,super-melee-mutual-loss-v1,campaign-s06c-utwig-bomb,load-connected-final-fleet-v1,load-connected-samatra-remnants-v1,fly-to-alpha-centauri-v1"
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native gallery "$PWD/tmp/U2-09/v4-run" --output "$PWD/tmp/U2-09/v4-run-gallery"
  ```

**Exit numbers.** ships and battle: no ffi or unsafe baseline entries; Stage A authority statics 5 -> 0; record types 0; pool order tests 4 of 4.

**Risks**
- A pool that reuses slots in another order than FreeLink changes element order; the randomized comparison is the oracle.
- Accessors written in Stage B confine call-site changes; if the PR passes 6,000 non-mechanical lines, stop and file (AGENTS.md).

## Phase 3: Super Melee and netplay

Exit: supermelee and pickship unsafe = 0; packet byte fixtures unchanged; two-peer loopback battle agrees on every checksum and matches melee-input; .mle round-trips in uqm-save.

### U3-01 Netplay: enum Packet with a byte codec, NetplaySession and the Net port

Phase 3 · semantic · lane `netplay` · about 5,700 semantic and 600 mechanical changed lines · depends on U1-02, U1-04

**Replaces** P3-01 (#628), P3-02 (#629).

**Scope.** The wire codec, connection state owned by a session with every reader and writer of the netplay globals moved to it, and sockets behind a port. May run during Phase 2 whenever no unit in progress holds its battle, picker, menu or abi files.

**Touches** (reserved): `src/supermelee/netplay/`, `src/abi/netplay_abi.rs`, `src/abi/ship_abi.rs`, `src/abi/navigation_abi.rs`, `src/battle/netplay.rs`, `src/battle/battle_native.rs`, `src/battle/ffi.rs`, `src/battle/tactrans_native.rs`, `src/supermelee/native/battle_picker.rs`, `src/supermelee/native/menu.rs`, `src/supermelee/native/menu/live/`, `src/front_end/cnctdlg.rs`, `src/engine/battle/net.rs`, `crates/engine/src/network/`, `crates/engine/src/network.rs`, `tests/netplay_packets.rs`, `tests/fixtures/netplay/`, `tests/netplay_loopback.rs`.
**Shared:** `crates/engine/src/host.rs`, `src/engine/battle/mod.rs`.

**Change**
- packetTypeData (packet.rs:29) and the #[repr(C)] packet structs become enum Packet with explicit encode/decode; handlers match on the enum; NETPLAY_PROTOCOL_VERSION (packet.rs:238) unchanged; byte fixtures per packet type next to tests/netplay_packets.rs.
- netConnections, netplayOptions and the other netplay statics become a NetplaySession; the netplay functions that read them (netmelee, netinput, notifyall, checksum, netoptions) become its methods.
- The melee menu owns the session: MELEE_STATE (abi/ship_abi.rs:1476-1494) gains a netplay: NetplaySession field (its Copy derive goes; U3-02 carries the field into MeleeSession). The menu's handlers use it: supermelee/native/menu.rs (:103, :157, :269-270), menu/live/boundary.rs (connection and set_connection, :10-21), menu/live/battle.rs (input delay and notifications, :47, :65) and menu/live/{changes,editing}.rs. The connection dialog takes &mut NetplaySession from its caller (menu.rs:65) instead of reading netplayOptions (front_end/cnctdlg.rs:251-266, :342, :363).
- For a battle, start() (menu/live/battle.rs:72) lends the session to the battle and gets it back, unchanged in identity, when the battle returns. Battle holds it for the battle's duration behind BattleNet, a narrow trait declared in the new src/engine/battle/net.rs (input exchange and flush, checksum and frame-count notices, ship-selection notice, battle-end handshake state) and implemented by NetplaySession, as content-to-mode callbacks are (architecture.md 2), so the staging module names no supermelee type and planned_engine_core does not rise. battle_native.rs (connection at :159-165, input and checksum at :242-323), battle/ffi.rs, tactrans_native.rs (:231-306) and battle_picker.rs (connections at :63-67, :110-239, :473-539) call it through Battle. No bridge accessor is added; bridge_calls does not rise.
- engine::host::Net with the six socket operations; std::net implementation in uqm-engine::network, or the libc/WinSock module kept and audited if non-blocking connect needs it.
- In-memory loopback Net; tests/netplay_loopback.rs runs a two-peer battle and compares checksums every interval with the melee-input golden.

**Deletes** (gone when the PR merges; the acceptance checks prove it)
- packetTypeData and the #[repr(C)] packet structs
- netConnections (netmelee.rs:20 and the declaration at checksum.rs:13), NUM_NET_CONNECTIONS, netplayOptions (netoptions.rs:4), netStateData, LOCAL_CHECKSUM_BUFFER, BUFFERS and INPUT_DELAY (netinput.rs), and the netplay statics in battle/netplay.rs
- the extern declarations of netConnections and netplayOptions (abi/ship_abi.rs:1093, 4168; abi/navigation_abi.rs:4301; abi/netplay_abi.rs:928) and the raw connection accessors over netConnections (menu/live/boundary.rs:10-21, battle/battle_native.rs:159-165, supermelee/native/battle_picker.rs:63-67)
- the extern "C" socket exports in network that netplay called directly (only the module behind Net remains)

**Out of scope.** melee setup and teams (U3-02).

**Acceptance**
- S (check bundle)
- P (check bundle)
- `cd rust && cargo test --locked --test netplay_packets`
- `cd rust && cargo test --locked --test netplay_loopback`
- `cd rust && cargo xtask quality report static_mut:supermelee/netplay --expect 0`
- `cd rust && cargo xtask quality ident packetTypeData netConnections netplayOptions NUM_NET_CONNECTIONS netStateData --expect 0`
- `cd rust && cargo xtask quality report no_mangle:network --expect 0`
- V2 (check bundle)
- R5 (check bundle)
- V4 (smoke plus 2 scenarios); inspect both galleries with the image-comparer:

  ```sh
  pkill -x uqm || true
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native smoke --artifacts "$PWD/tmp/U3-01/v4-smoke" --content "$PWD/sc2/content/packages"
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native gallery "$PWD/tmp/U3-01/v4-smoke" --output "$PWD/tmp/U3-01/v4-smoke-gallery"
  pkill -x uqm || true
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native run --artifacts "$PWD/tmp/U3-01/v4-run" --content "$PWD/sc2/content/packages" \
    --scenarios "netplay-dialog-v1,super-melee-mutual-loss-v1"
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native gallery "$PWD/tmp/U3-01/v4-run" --output "$PWD/tmp/U3-01/v4-run-gallery"
  ```

**Exit numbers.** packetTypeData gone; netplay static_mut 0; files naming netConnections or netplayOptions 13 -> 0; network unsafe confined to one module; bridge_calls unchanged.

**Risks**
- Never regenerate byte fixtures from the new codec; they are the oracle.
- Non-blocking connect differs between std::net and WinSock; the Windows game-build jobs are the check.
- Connections must survive between battles of one melee session exactly as the globals did; the session is lent to Battle and returned, never rebuilt. netplay-dialog-v1 and the loopback test cover it.
- The unit is near the size limit; if it passes 6,000 non-mechanical lines, stop and file.

### U3-02 Super Melee setup, teams and ship picking as a session; .mle into uqm-save

Phase 3 · semantic · lane `netplay` · about 3,000 changed lines · depends on U3-01, U2-09

**Replaces** P3-03 (#630).

**Scope.** Session and UI ownership for supermelee and pickship; their record access was converted in U2-06. May run during Phase 4 outside U4-01.

**Touches** (reserved): `src/supermelee/setup/`, `src/supermelee/native/`, `src/supermelee/c_bridge.rs`, `src/supermelee/mod.rs`, `src/supermelee/types.rs`, `src/pickship/`, `crates/save/src/mle.rs`, `crates/save/tests/mle_roundtrip.rs`.
**Shared:** `crates/save/src/lib.rs`.

**Change**
- Setup, team and picker state become a MeleeSession, which replaces MELEE_STATE and owns the NetplaySession that U3-01 placed in it; pickship takes Battle and ShipId.
- The .mle byte layout (supermelee/setup/persistence.rs) moves to uqm-save with round-trip fixtures; file access stays as it is until U7-03.

**Deletes** (gone when the PR merges; the acceptance checks prove it)
- the supermelee extern "C" fns (267), no_mangle (221) and static_mut (16), and the pickship extern "C" entry points
- the .mle byte layout code in supermelee/setup/persistence.rs (moved to uqm-save)

**Out of scope.** melee menus outside supermelee/; team-file I/O through Files (U7-03).

**Acceptance**
- S (check bundle)
- P (melee-input, melee-cyborg-*)
- `cd rust && cargo test -p uqm-save --locked --test mle_roundtrip`
- `cd rust && for s in supermelee pickship; do for m in extern_c_fn no_mangle static_mut unsafe; do cargo xtask quality report $m:$s --expect 0 || exit 1; done; done`
- V2 (check bundle)
- V4 (smoke plus 2 scenarios); inspect both galleries with the image-comparer:

  ```sh
  pkill -x uqm || true
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native smoke --artifacts "$PWD/tmp/U3-02/v4-smoke" --content "$PWD/sc2/content/packages"
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native gallery "$PWD/tmp/U3-02/v4-smoke" --output "$PWD/tmp/U3-02/v4-smoke-gallery"
  pkill -x uqm || true
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native run --artifacts "$PWD/tmp/U3-02/v4-run" --content "$PWD/sc2/content/packages" \
    --scenarios "super-melee-mutual-loss-v1,netplay-dialog-v1"
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native gallery "$PWD/tmp/U3-02/v4-run" --output "$PWD/tmp/U3-02/v4-run-gallery"
  ```

**Exit numbers.** supermelee extern "C" fn 267 -> 0, no_mangle 221 -> 0, static_mut 16 -> 0; supermelee and pickship unsafe = 0.

**Risks**
- Team files written by older builds must still load; keep the legacy fixtures.

## Phase 4: world state, queues and main loop; engine core into uqm-engine

Exit: GlobData references = 0; story lookups by string = 0; Observer replaces the Coordinator hooks; queue_calls = 0; collections/queue.rs deleted; battle core, World, Game, Kernel and GameClock live in uqm-engine; src/engine/ staging removed; full V4 passes.

### U4-01 Typed story keys; GlobData, the story globals and the kernel statics become the owned World and Kernel in Game

Phase 4 · semantic · lane `world` · about 3,000 semantic and 6,000 mechanical changed lines · depends on U2-09 · **critical path**

**Replaces** P4-01 (#631), P4-05 (#635).

**Scope.** Stage B and Stage C for GlobData in one PR: every reader moves straight to the owned World, so no accessor layer over the static survives. It reserves src/, so it runs alone and lands quickly.

**Touches** (reserved): `src/`.
**Shared:** none.

**Change**
- StoryKey constants generated from the existing key table; get_game_state("..") calls (state/game_state_keys.rs:477) and raw bit indices (planets/pstarmap.rs:202-213) use them; call sites with names unknown to the table keep returning 0 explicitly, listed in the PR.
- Mechanical commit: every GlobData field read and write (689 references in 111 files) becomes a World accessor call.
- Semantic commit: GlobData becomes Game::world, created by app at start; code holding Cx or &mut Game reads it directly; mode entry points not yet converted reach it through counted bridge accessors (adapter A4).
- CurStarDescPtr (planets/starmap.rs:12) becomes World::current_star; GLOBAL_GAME_STATE and GLOBAL_STATE_FILES (state/ffi.rs:15-16, state/state_file.rs:321) move into World; the global! kernel statics (game_init/kernel_abi.rs:23-46) become Kernel.

**Deletes** (gone when the PR merges; the acceptance checks prove it)
- the static GlobData and the CGlobData type, with its CQueue field mirror and their layout assertions (comm/locdata.rs:81-92, :200-210)
- CurStarDescPtr, GLOBAL_GAME_STATE and both GLOBAL_STATE_FILES
- the string-keyed get_game_state("..") lookup and the raw bit indices in planets/pstarmap.rs:202-213
- the global! kernel statics in game_init/kernel_abi.rs:23-46

**Out of scope.** the GAME_STATE queue representation (U4-02); the main loop (U4-04).

**Acceptance**
- S (check bundle)
- P (star-contact)
- `cd rust && cargo xtask quality ident GlobData CGlobData CQueue CurStarDescPtr GLOBAL_GAME_STATE GLOBAL_STATE_FILES --expect 0`
- `cd rust && rg -q 'get_game_state\("' src; test $? -eq 1`
- `cd rust && for s in state game_init navigation c_bindings.rs; do cargo xtask quality report static_mut:$s --expect 0 || exit 1; done`
- V3 (check bundle)
- V2 (check bundle)
- V4 (smoke plus 46 scenarios); inspect both galleries with the image-comparer:

  ```sh
  pkill -x uqm || true
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native smoke --artifacts "$PWD/tmp/U4-01/v4-smoke" --content "$PWD/sc2/content/packages"
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native gallery "$PWD/tmp/U4-01/v4-smoke" --output "$PWD/tmp/U4-01/v4-smoke-gallery"
  pkill -x uqm || true
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native run --artifacts "$PWD/tmp/U4-01/v4-run" --content "$PWD/sc2/content/packages" \
    --scenarios "load-alpha-centauri-v1,load-bomb-preguard-v1,load-bomb-refueled-v1,load-broken-ultron-v1,load-burvixese-caster-v1,load-caster-diversion-v1,load-connected-ambush-v1,load-connected-arilou-v1,load-connected-chmmr-bomb-v1,load-connected-combat-prep-v1,load-connected-final-fleet-v1,load-connected-full-drives-v1,load-connected-funded-fleet-v1,load-connected-jugger-prep-v1,load-connected-lure-prep-v1,load-connected-lure-v1,load-connected-rainbow-funded-v1,load-connected-samatra-distraction-v1,load-connected-samatra-remnants-v1,load-connected-shattered-v1,load-connected-sun-device-v1,load-connected-syreen-v1,load-connected-taalo-v1,load-connected-talking-pet-v1,load-connected-umgah-fleet-v1,load-connected-vault-resupply-v1,load-connected-vault-v1,load-funded-sol-v1,load-ilwrath-approach-v1,load-north-artifacts-v1,load-north-ready-v1,load-north-refueled-v1,load-portal-sol-v1,load-portal-staging-v1,load-repaired-ultron-v1,load-save-v1,load-south-refueled-v1,load-southeast-staging-v1,load-spindle-pkunk-v1,load-starbase-armed-v1,load-thraddash-diverted-v1,load-utwig-alliance-v1,load-utwig-bomb-v1,load-war-cleared-v1,load-west-bomb-refueled-v1,starbase-visit-v1"
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native gallery "$PWD/tmp/U4-01/v4-run" --output "$PWD/tmp/U4-01/v4-run-gallery"
  ```

**Exit numbers.** GlobData references 0; story lookups by string 0; static_mut 0 in state, game_init, navigation, c_bindings; bridge_calls within budget.

**Risks**
- An unknown key returns 0 today; turning it into a compile error must not change that value.
- The bridge budget cannot rise without the owner; if it does not fit, stop and file.
- GAME_STATE QUEUE headers move with World; no code may hold a pointer to a queue header across Game construction.

### U4-02 GAME_STATE queues and event_q on OrderedPool behind LinkQueue; every consumer; delete collections/queue.rs

Phase 4 · semantic · lane `world` · about 2,100 semantic and 3,000 mechanical changed lines · depends on U4-01 · **critical path**

**Replaces** P4-03 (#633), P4-04 (#634).

**Scope.** Consumers and storage together: World and GameClock expose LinkQueue views over OrderedPool storage, every consumer moves to them, and collections/queue.rs goes with every facade over it (the queue wrappers and re-exports in abi/ and the queue-returning accessors in mainloop/state_abi.rs). No view over QUEUE storage is ever written for these queues.

**Touches** (reserved): `src/engine/world/`, `src/engine/clock/`, `src/engine/battle/link_queue.rs`, `src/comm/dispatch.rs`, `src/planets/c_api.rs`, `src/planets/devices.rs`, `src/planets/roster.rs`, `src/planets/scan/live.rs`, `src/planets/solarsys/lifecycle.rs`, `src/planets/generate/chmmr.rs`, `src/planets/generate/home_contact.rs`, `src/planets/generate/myc.rs`, `src/planets/generate/pet.rs`, `src/planets/generate/special.rs`, `src/planets/generate/utw.rs`, `src/planets/generate/yehat.rs`, `src/starbase/cffi.rs`, `src/starbase/platform.rs`, `src/ui/shipyard.rs`, `src/front_end/gameopt.rs`, `src/front_end/uqmdebug.rs`, `src/mainloop/restart_menu/menu_abi.rs`, `src/battle/build_native.rs`, `src/battle/encount_native.rs`, `src/battle/battle_native.rs`, `src/navigation/globals.rs`, `src/navigation/groups.rs`, `src/navigation/hyper_encounters.rs`, `src/navigation/hyperspace.rs`, `src/navigation/interplanetary.rs`, `src/savegame/live/load.rs`, `src/savegame/runtime/`, `src/time/mod.rs`, `src/time/clock_abi.rs`, `src/time/clock_bridge.rs`, `src/ships/ffi.rs`, `src/ships/master_native.rs`, `src/ships/native_adapters.rs`, `src/ships/native.rs`, `src/c_bindings.rs`, `src/planets/pstarmap.rs`, `src/mainloop/state_abi.rs`, `src/abi/ship_abi.rs`, `src/abi/navigation_abi.rs`, `src/abi/netplay_abi.rs`, `src/abi/global_state_abi.rs`, `crates/engine/src/collections/queue.rs`.
**Shared:** `crates/engine/src/collections/mod.rs`.

**Change**
- World::{avail_races, built_ships, npc_built_ships, ip_groups, encounters}() and GameClock::events() return LinkQueue views over OrderedPool storage with the InitQueue capacities of navigation/globals.rs:101,151-170; event_q (time/clock_abi.rs:170) moves into GameClock in src/engine/clock/.
- Mechanical commit: every direct call into collections::queue (comm/dispatch.rs:1030-1055, planets/c_api.rs:392-417, starbase/cffi.rs:716-742, ui/shipyard.rs:248-260, front_end/gameopt.rs:780-797 and the rest of the touches list) becomes a view call; fragment writes go through the view's get_mut; savegame runtime readers on the pools; planets/pstarmap.rs's fleet_queue, lock_fleet and check_fleet_link (:461-485) read World::avail_races().
- The queue facades go with their callers: the wrappers in abi/ship_abi.rs:1198-1229 (QUEUE_HANDLE, HLINK, LINK, the QUEUE re-export, FreeLink, InitQueue, UninitQueue, ReinitQueue, RemoveQueue, CountLinks), which ships::native::abi includes, and every caller of them; the collections::queue and QUEUE re-exports and QUEUE_HANDLE in abi/navigation_abi.rs (:1015-1038), abi/netplay_abi.rs (:238-241) and abi/global_state_abi.rs (:29-32, included by c_bindings::global_state); the link-handle aliases over HLINK (HEVENT, HFLEETINFO, HIPGROUP, HENCOUNTER, HSHIPFRAG, HMASTERSHIP), whose users take the views' typed ids; and the five queue-returning accessors rust_get_{avail_race,npc_built_ship,encounter,built_ship,ip_group}_queue (mainloop/state_abi.rs:63-68), which have no caller in the workspace.

**Deletes** (gone when the PR merges; the acceptance checks prove it)
- crates/engine/src/collections/queue.rs (InitQueue, UninitQueue, ReinitQueue, AllocLink, FreeLink, PutQueue, RemoveQueue and the rest of the C-style API) and the QUEUE type
- the QUEUE fields of the GAME_STATE queues and the event_q static (time/clock_abi.rs:170)
- the queue ABI facades: QUEUE_HANDLE, HLINK, LINK, the QUEUE re-export and the FreeLink, InitQueue, UninitQueue, ReinitQueue, RemoveQueue and CountLinks wrappers in abi/ship_abi.rs; the collections::queue and QUEUE re-exports and QUEUE_HANDLE in abi/navigation_abi.rs, abi/netplay_abi.rs and abi/global_state_abi.rs; the HLINK aliases HEVENT, HFLEETINFO, HIPGROUP, HENCOUNTER, HSHIPFRAG and HMASTERSHIP
- the five rust_get_*_queue accessors in mainloop/state_abi.rs:63-68

**Out of scope.** mode conversions; the main loop (U4-04).

**Acceptance**
- S (check bundle)
- P (star-contact, new-game, hyperspace, comm)
- `cd rust && test ! -e crates/engine/src/collections/queue.rs`
- `cd rust && cargo xtask quality ident QUEUE QUEUE_HANDLE HLINK HEVENT HFLEETINFO HIPGROUP HENCOUNTER HSHIPFRAG HMASTERSHIP InitQueue UninitQueue ReinitQueue AllocLink FreeLink PutQueue InsertQueue RemoveQueue CountLinks --expect 0`
- `cd rust && rg -q 'collections::queue|rust_get_(avail_race|npc_built_ship|encounter|built_ship|ip_group)_queue' src crates tests; test $? -eq 1`
- `cd rust && cargo xtask quality report queue_calls --expect 0 && cargo xtask quality report static_mut:time --expect 0`
- V3 (check bundle)
- V2 (check bundle)
- V4 (smoke plus 5 scenarios); inspect both galleries with the image-comparer:

  ```sh
  pkill -x uqm || true
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native smoke --artifacts "$PWD/tmp/U4-02/v4-smoke" --content "$PWD/sc2/content/packages"
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native gallery "$PWD/tmp/U4-02/v4-smoke" --output "$PWD/tmp/U4-02/v4-smoke-gallery"
  pkill -x uqm || true
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native run --artifacts "$PWD/tmp/U4-02/v4-run" --content "$PWD/sc2/content/packages" \
    --scenarios "starbase-visit-v1,real-sol-starbase-alliance,load-connected-funded-fleet-v1,load-connected-umgah-fleet-v1,fly-to-alpha-centauri-v1"
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native gallery "$PWD/tmp/U4-02/v4-run" --output "$PWD/tmp/U4-02/v4-run-gallery"
  ```

**Exit numbers.** collections/queue.rs deleted; QUEUE users 0, abi/ queue wrappers and re-exports included; queue-returning accessors 5 -> 0; time static_mut 0.

**Risks**
- A full queue is a game rule (the shipyard limit); the view's alloc returns None, never grows.
- Save files store queue contents in link order; V3 is the oracle.

### U4-03 The Observer replaces the Coordinator hooks

Phase 4 · semantic · lane `world` · about 3,000 changed lines · depends on U4-01

**Replaces** P4-02 (#632).

**Scope.** Three hooks threaded through Io; automation implements them; the 22 mode files that name crate::automation stop doing so.

**Touches** (reserved): `crates/engine/src/observe.rs`, `src/automation/coordinator.rs`, `src/automation/campaign_state.rs`, `src/automation/scheduler.rs`, `src/automation/input_ffi.rs`, `src/battle/battle_native.rs`, `src/comm/dispatch.rs`, `src/comm/ffi.rs`, `src/comm/hail.rs`, `src/comm/talk_segue.rs`, `src/front_end/credits.rs`, `src/front_end/gameopt.rs`, `src/front_end/intro.rs`, `src/mainloop/game_loop.rs`, `src/mainloop/input_abi.rs`, `src/mainloop/restart_menu/do_restart.rs`, `src/mainloop/restart_menu/restart_ops.rs`, `src/planet_side/adapters.rs`, `src/planet_side/ffi.rs`, `src/planets/devices.rs`, `src/planets/orbit.rs`, `src/planets/scan/live.rs`, `src/starbase/cffi.rs`, `src/supermelee/native/battle_picker.rs`, `src/supermelee/native/menu.rs`, `src/ui/starbase.rs`, `src/ui/util.rs`.
**Shared:** `crates/engine/src/lib.rs`.

**Change**
- engine::observe::Observer with input_admitted, checkpoint, presented; Io<'a> { observer } (the platform field is added in U7-01).
- Coordinator statics become an AutomationObserver owned by app; campaign keys (automation/campaign_state.rs) read typed fields.
- Before the old hooks are deleted, cargo xtask parity counts check --all matches the Phase 0 counts golden for every rust/scripts/*.json.

**Deletes** (gone when the PR merges; the acceptance checks prove it)
- COORDINATOR (automation/coordinator.rs:259), ORBIT_EXIT_PHASE (:212) and FLAGSHIP_DESTROYED (campaign_state.rs:352)
- the old coordinator hooks and every crate::automation path from mode code

**Out of scope.** the present path itself (U7-01).

**Acceptance**
- S (check bundle)
- P (check bundle)
- `cd rust && cargo xtask parity counts check --all`
- `cd rust && rg -q 'crate::automation' src/comm src/planets src/planet_side src/front_end src/ui src/starbase src/supermelee src/battle; test $? -eq 1`
- `cd rust && cargo xtask quality ident COORDINATOR ORBIT_EXIT_PHASE FLAGSHIP_DESTROYED --expect 0`
- V2 (check bundle)
- V4 (smoke plus 4 scenarios); inspect both galleries with the image-comparer:

  ```sh
  pkill -x uqm || true
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native smoke --artifacts "$PWD/tmp/U4-03/v4-smoke" --content "$PWD/sc2/content/packages"
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native gallery "$PWD/tmp/U4-03/v4-smoke" --output "$PWD/tmp/U4-03/v4-smoke-gallery"
  pkill -x uqm || true
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native run --artifacts "$PWD/tmp/U4-03/v4-run" --content "$PWD/sc2/content/packages" \
    --scenarios "introduction-v1,real-sol-earth-orbit,starbase-visit-v1,setup-menu-v1"
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native gallery "$PWD/tmp/U4-03/v4-run" --output "$PWD/tmp/U4-03/v4-run-gallery"
  ```

**Exit numbers.** crate::automation paths from mode code: 0; automation static state: 0.

**Risks**
- Hook order within a frame (architecture.md 7) is part of the trace; P lines catch a moved present.

### U4-04 Main loop as app::run; snapshot and apply; move the battle core and world into uqm-engine

Phase 4 · semantic · lane `world` · about 2,500 semantic and 1,500 mechanical changed lines · depends on U4-02, U4-03 · **critical path**

**Replaces** P4-06 (#636), P4-07 (#637).

**Scope.** The loop and the live save connection in semantic commits, then the mechanical move of the staging module into uqm-engine as the last commit.

**Touches** (reserved): `src/mainloop/`, `src/app/`, `src/navigation/globals.rs`, `src/savegame/live/`, `src/savegame/runtime/`, `src/engine/`, `crates/save/`, `crates/engine/src/`, `tests/save_runtime_capture.rs`.
**Shared:** `src/lib.rs`, `xtask/src/quality/crate_deps.rs`.

**Change**
- app::run keeps state_machine::evaluate/check_break; GameLoopOps/CffiOps (mainloop/game_loop.rs:290) go; World::new_game replaces InitGameStructures in the order of navigation/globals.rs:102-112.
- engine::save::{snapshot, apply} with SessionView; apply returns the saved activity (savegame/runtime/global.rs:79-85,112); state-file byte layouts move to uqm-save; V3b captures through snapshot.
- Move commit: git mv src/engine/* into crates/engine/src/ (battle, world, clock, save, observe); the root replaces its staging module with pub use uqm_engine as engine; quality move --prefix; delete the planned_engine_core budget; mark element-loop accessors #[inline] (architecture.md 2.1).

**Deletes** (gone when the PR merges; the acceptance checks prove it)
- GameLoopOps and CffiOps (mainloop/game_loop.rs:290), InitGameStructures and the mainloop statics
- the root state-file byte layout code (moved to uqm-save)
- the staging module src/engine/ (moved) and the planned_engine_core budget

**Out of scope.** mode conversions.

**Acceptance**
- S (check bundle)
- P (identical across the move commit without re-recording)
- V3 (check bundle)
- `cd rust && cargo xtask quality ident GameLoopOps CffiOps InitGameStructures --expect 0`
- `cd rust && test ! -e src/engine && cargo xtask quality report static_mut:mainloop --expect 0`
- `cd rust && cargo test -p uqm-engine --locked`
- `cd rust && cargo build --release --locked --timings   # attach before and after reports`
- V2 (check bundle)
- R5 (check bundle)
- V4 (smoke plus 49 scenarios); inspect both galleries with the image-comparer:

  ```sh
  pkill -x uqm || true
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native smoke --artifacts "$PWD/tmp/U4-04/v4-smoke" --content "$PWD/sc2/content/packages"
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native gallery "$PWD/tmp/U4-04/v4-smoke" --output "$PWD/tmp/U4-04/v4-smoke-gallery"
  pkill -x uqm || true
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native run --artifacts "$PWD/tmp/U4-04/v4-run" --content "$PWD/sc2/content/packages" \
    --scenarios "load-alpha-centauri-v1,load-bomb-preguard-v1,load-bomb-refueled-v1,load-broken-ultron-v1,load-burvixese-caster-v1,load-caster-diversion-v1,load-connected-ambush-v1,load-connected-arilou-v1,load-connected-chmmr-bomb-v1,load-connected-combat-prep-v1,load-connected-final-fleet-v1,load-connected-full-drives-v1,load-connected-funded-fleet-v1,load-connected-jugger-prep-v1,load-connected-lure-prep-v1,load-connected-lure-v1,load-connected-rainbow-funded-v1,load-connected-samatra-distraction-v1,load-connected-samatra-remnants-v1,load-connected-shattered-v1,load-connected-sun-device-v1,load-connected-syreen-v1,load-connected-taalo-v1,load-connected-talking-pet-v1,load-connected-umgah-fleet-v1,load-connected-vault-resupply-v1,load-connected-vault-v1,load-funded-sol-v1,load-ilwrath-approach-v1,load-north-artifacts-v1,load-north-ready-v1,load-north-refueled-v1,load-portal-sol-v1,load-portal-staging-v1,load-repaired-ultron-v1,load-save-v1,load-south-refueled-v1,load-southeast-staging-v1,load-spindle-pkunk-v1,load-starbase-armed-v1,load-thraddash-diverted-v1,load-utwig-alliance-v1,load-utwig-bomb-v1,load-war-cleared-v1,load-west-bomb-refueled-v1,campaign-s01-funded-sol,campaign-s06c-utwig-bomb,campaign-s08i-connected-arilou,campaign-s09j-connected-shattered"
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native gallery "$PWD/tmp/U4-04/v4-run" --output "$PWD/tmp/U4-04/v4-run-gallery"
  ```

**Exit numbers.** mainloop static_mut 0; GameLoopOps 0; src/engine/ removed; planned_engine_core budget deleted.

**Risks**
- Load and save are not inverse; the V3b normalisation list is the contract, not byte equality with the file.
- A release frame-time regression from lost inlining shows in V2 pacing reports; add #[inline], do not change the profile.

## Phase 5: navigation and planets

Exit: planets, navigation and planet_side have no ffi or unsafe baseline entries; cfg_not_test_modules <= 11; GenerateFunctions, the *_exports.rs shims, pSolarSysState and abi/global_state_abi.rs are gone; planets_parity passes.

### U5-01 SystemGenerator and SiteHost for every generator; delete the generator vtable and the *_exports.rs shims

Phase 5 · semantic · lane `planets` · about 5,300 changed lines · depends on U4-04 · **critical path**

**Replaces** P5-01 (#638), P5-02 (#639).

**Scope.** All generators convert in one PR, so the dispatcher calls the trait only and never chooses between a trait impl and the vtable.

**Touches** (reserved): `src/planets/generate/`, `src/planets/solarsys/lifecycle.rs`, `src/battle/gendef.rs`, `tests/planets_parity.rs`, `tests/planets_fixtures.rs`.
**Shared:** `xtask/src/quality/macros.rs`.

**Change**
- trait SystemGenerator with defaults, SiteHost with Story as supertrait; impl Story for StoryState replaces LiveStory; modes Site { game, io } implements SiteHost.
- Default, home, late, special, story and Sol generators as impls; the logic functions drop their story parameter (orz.rs:65-72).
- tests/planets_parity.rs runs through the new API against its C-captured fixtures.

**Deletes** (gone when the PR merges; the acceptance checks prove it)
- GenerateFunctions and its 13 hook types (vtable.rs:29) and getGenerateFunctions (battle/gendef.rs:74)
- planets/generate/{exports,home_exports,late_exports,special_exports,story_exports,sol_exports}.rs
- LiveStory
- the macros energy_hook!, moons_hook!, story_energy!, story_pickup! and their walker handlers in xtask/src/quality/macros.rs

**Out of scope.** the solar system session (U5-02).

**Acceptance**
- S (check bundle)
- P (solar-flight, new-game, lander)
- `cd rust && cargo test --locked --test planets_parity --test planets_fixtures`
- `cd rust && for f in exports home_exports late_exports special_exports story_exports sol_exports; do test ! -e src/planets/generate/$f.rs || exit 1; done`
- `cd rust && cargo xtask quality ident GenerateFunctions getGenerateFunctions LiveStory energy_hook moons_hook story_energy story_pickup --expect 0`
- V2 (check bundle)
- V4 (smoke plus 9 scenarios); inspect both galleries with the image-comparer:

  ```sh
  pkill -x uqm || true
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native smoke --artifacts "$PWD/tmp/U5-01/v4-smoke" --content "$PWD/sc2/content/packages"
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native gallery "$PWD/tmp/U5-01/v4-smoke" --output "$PWD/tmp/U5-01/v4-smoke-gallery"
  pkill -x uqm || true
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native run --artifacts "$PWD/tmp/U5-01/v4-run" --content "$PWD/sc2/content/packages" \
    --scenarios "real-sol-earth-orbit,real-sol-pluto-orbit-scan,load-utwig-alliance-v1,load-spindle-pkunk-v1,real-sol-mercury-radioactives,explore-planet-v1,collect-moon-energy-v1,load-connected-talking-pet-v1,load-north-artifacts-v1"
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native gallery "$PWD/tmp/U5-01/v4-run" --output "$PWD/tmp/U5-01/v4-run-gallery"
  ```

**Exit numbers.** *_exports.rs 0; GenerateFunctions 0; four macro handlers deleted.

**Risks**
- Site::talk enters comm synchronously; the SolarSystem session must stay on the caller's stack.
- planets_parity fixtures come from the C original; never re-capture them.

### U5-02 SolarSystem session: interplanetary flight, orbit, scan, starmap and devices; delete pSolarSysState

Phase 5 · semantic · lane `planets` · about 5,800 changed lines · depends on U5-01 · **critical path**

**Replaces** P5-03 (#640), P5-04 (#641).

**Scope.** pSolarSysState becomes a session and every reader moves to it in the same PR, so no global pointer to the session survives.

**Touches** (reserved): `src/planets/solarsys.rs`, `src/planets/solarsys/`, `src/planets/c_api.rs`, `src/planets/c_api/`, `src/planets/orbit.rs`, `src/planets/scan.rs`, `src/planets/scan/`, `src/planets/pstarmap.rs`, `src/planets/pstarmap/`, `src/planets/starmap.rs`, `src/planets/devices.rs`, `src/planets/roster.rs`, `src/planets/calc.rs`, `src/planets/ffi.rs`, `src/navigation/interplanetary.rs`, `src/navigation/groups.rs`, `src/front_end/uqmdebug.rs`, `src/savegame/runtime/live.rs`, `src/savegame/runtime/location.rs`, `src/savegame/runtime/solar.rs`, `src/savegame/runtime/summary.rs`, `src/automation/input_ffi.rs`, `src/c_bindings.rs`, `src/abi/navigation_abi.rs`.
**Shared:** none.

**Change**
- SolarSystem session with sys: SolarSys and a required star: StarId, created by solar_system::explore and passed by &mut.
- Interplanetary flight (navigation/interplanetary.rs, planets/solarsys/flight.rs) and planets/c_api on the session; orbit, scan, pstarmap and starmap take the session and Game; devices and roster on Game.
- The remaining pSolarSysState readers (planets/calc.rs, planets/ffi.rs, navigation/groups.rs, front_end/uqmdebug.rs, savegame/runtime/{live,location,solar,summary}.rs, automation/input_ffi.rs) take the session or the World.

**Deletes** (gone when the PR merges; the acceptance checks prove it)
- pSolarSysState (c_bindings.rs and its declaration in abi/navigation_abi.rs) and every read of it
- the extern "C" entry points of solarsys, c_api, interplanetary, orbit, scan, pstarmap and starmap (67 for the last four at 35d33879b)
- the planets static_mut (46)

**Out of scope.** planet side (U5-03); hyperspace (U5-04).

**Acceptance**
- S (check bundle)
- P (solar-flight, star-contact, lander)
- `cd rust && cargo test --locked planets::`
- `cd rust && cargo xtask quality ident pSolarSysState --expect 0`
- `cd rust && for m in static_mut extern_c_fn; do cargo xtask quality report $m:planets --expect 0 || exit 1; done`
- V3 (check bundle)
- V2 (check bundle)
- V4 (smoke plus 8 scenarios); inspect both galleries with the image-comparer:

  ```sh
  pkill -x uqm || true
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native smoke --artifacts "$PWD/tmp/U5-02/v4-smoke" --content "$PWD/sc2/content/packages"
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native gallery "$PWD/tmp/U5-02/v4-smoke" --output "$PWD/tmp/U5-02/v4-smoke-gallery"
  pkill -x uqm || true
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native run --artifacts "$PWD/tmp/U5-02/v4-run" --content "$PWD/sc2/content/packages" \
    --scenarios "real-sol-starbase-to-mars,real-sol-autopilot,fly-to-alpha-centauri-v1,load-funded-sol-v1,real-sol-earth-orbit,real-sol-pluto-orbit-scan,mineral-sizes-v1,real-sol-mercury-radioactives"
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native gallery "$PWD/tmp/U5-02/v4-run" --output "$PWD/tmp/U5-02/v4-run-gallery"
  ```

**Exit numbers.** pSolarSysState 0; planets static_mut 46 -> 0, extern_c_fn 199 -> 0.

**Risks**
- IP group movement order is saved; V3b and the star-contact fixture are the oracles.
- Scan reveals minerals with RNG draws; their order is in the solar-flight entry.
- The unit is close to the size limit; if it passes 6,000 non-mechanical lines, stop and file.

### U5-03 Planet side on the session; delete abi/global_state_abi.rs

Phase 5 · semantic · lane `planets` · about 2,000 semantic and 500 mechanical changed lines · depends on U5-02 · **critical path**

**Replaces** P5-05 (#642).

**Scope.** The remaining planet_side FFI, then the global_state ABI with all of its importers (65 in planets, 28 elsewhere at 35d33879b).

**Touches** (reserved): `src/planet_side/`, `src/planets/`, `src/abi/global_state_abi.rs`, `src/c_bindings.rs`, `src/time/clock_bridge.rs`, `src/pickship/ffi.rs`, `src/automation/battle_fight.rs`, `src/automation/input_ffi.rs`, `src/battle/c_bridge.rs`, `src/state/c_api.rs`, `src/savegame/abi.rs`, `src/savegame/runtime/`, `src/comm/locdata.rs`, `src/comm/dispatch.rs`, `src/comm/commglue_abi.rs`, `src/comm/sis_graphics.rs`, `src/comm/races/melnorm/numbers.rs`, `src/comm/races/slyland/numbers.rs`, `src/starbase/cffi.rs`, `src/ships/native.rs`.
**Shared:** none.

**Change**
- Lander, hazards and surface on the session; the remaining planet_side extern "C" entry points go.
- Mechanical commit: every import of c_bindings::global_state and abi/global_state_abi.rs switches to the engine and session types.

**Deletes** (gone when the PR merges; the acceptance checks prove it)
- the remaining planet_side extern "C" entry points and statics
- src/abi/global_state_abi.rs and c_bindings::global_state

**Out of scope.** hyperspace modules (U5-04).

**Acceptance**
- S (check bundle)
- P (lander, solar-flight)
- `cd rust && test ! -e src/abi/global_state_abi.rs && { rg -q 'global_state_abi|c_bindings::global_state' src; test $? -eq 1; }`
- `cd rust && for m in extern_c_fn static_mut unsafe; do cargo xtask quality report $m:planet_side --expect 0 || exit 1; done`
- V3 (check bundle)
- V2 (check bundle)
- V4 (smoke plus 4 scenarios); inspect both galleries with the image-comparer:

  ```sh
  pkill -x uqm || true
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native smoke --artifacts "$PWD/tmp/U5-03/v4-smoke" --content "$PWD/sc2/content/packages"
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native gallery "$PWD/tmp/U5-03/v4-smoke" --output "$PWD/tmp/U5-03/v4-smoke-gallery"
  pkill -x uqm || true
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native run --artifacts "$PWD/tmp/U5-03/v4-run" --content "$PWD/sc2/content/packages" \
    --scenarios "real-sol-planetside-collision-proof,real-sol-rust-planetside-smoke,explore-planet-v1,collect-moon-energy-v1"
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native gallery "$PWD/tmp/U5-03/v4-run" --output "$PWD/tmp/U5-03/v4-run-gallery"
  ```

**Exit numbers.** planet_side no ffi or unsafe entries; global_state importers 0.

**Risks**
- The import swap is wide; do it as the first mechanical commit so review can separate it.

### U5-04 Hyperspace modes on Game; testable hyper_* modules

Phase 5 · semantic · lane `navigation` · about 2,500 changed lines · depends on U4-04

**Replaces** P5-06 (#643).

**Scope.** Hyperspace UI, radar, menu, transitions and encounters.

**Touches** (reserved): `src/navigation/hyperspace.rs`, `src/navigation/hyper_data.rs`, `src/navigation/hyper_encounters.rs`, `src/navigation/hyper_menu.rs`, `src/navigation/hyper_radar.rs`, `src/navigation/hyper_transition.rs`, `src/navigation/hyper_units.rs`.
**Shared:** `src/navigation/mod.rs`.

**Change**
- The hyper_* modules take &mut Game and Io.
- The five #![cfg(not(test))] hyper_* modules become testable, with unit tests.

**Deletes** (gone when the PR merges; the acceptance checks prove it)
- the extern "C" entry points of the seven hyper_* modules
- the #![cfg(not(test))] attribute of the five hyper_* modules

**Out of scope.** interplanetary.rs (U5-02); navigation_abi.rs (U6-04).

**Acceptance**
- S (check bundle)
- P (hyperspace, star-contact)
- `cd rust && cargo test --locked navigation::`
- `cd rust && for f in hyperspace hyper_data hyper_encounters hyper_menu hyper_radar hyper_transition hyper_units; do cargo xtask quality report extern_c_fn:navigation/$f.rs --expect 0 || exit 1; done`
- `cd rust && rg -q '#!\[cfg\(not\(test\)\)\]' src/navigation; test $? -eq 1`
- V2 (check bundle)
- V4 (smoke plus 4 scenarios); inspect both galleries with the image-comparer:

  ```sh
  pkill -x uqm || true
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native smoke --artifacts "$PWD/tmp/U5-04/v4-smoke" --content "$PWD/sc2/content/packages"
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native gallery "$PWD/tmp/U5-04/v4-smoke" --output "$PWD/tmp/U5-04/v4-smoke-gallery"
  pkill -x uqm || true
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native run --artifacts "$PWD/tmp/U5-04/v4-run" --content "$PWD/sc2/content/packages" \
    --scenarios "load-save-v1,fly-to-alpha-centauri-v1,load-alpha-centauri-v1,pkunk-refuel-v1"
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native gallery "$PWD/tmp/U5-04/v4-run" --output "$PWD/tmp/U5-04/v4-run-gallery"
  ```

**Exit numbers.** cfg_not_test_modules 16 -> 11; hyper_* extern_c_fn 0.

**Risks**
- Melnorme approach and fuel draw order are in the hyperspace corpus entry.

## Phase 6: comm, starbase and menus; uqm-content and uqm-modes

Exit: comm, starbase, ui and front_end have no ffi or unsafe baseline entries; cfg_not_test_modules = 0; planned.uqm-content = 0 and planned.uqm-modes = 0 in a semantic commit; bridge_calls = 0 and app::bridge deleted; both crates are members with forbid(unsafe_code).

### U6-01 Comm session: COMM_STATE, COMM_DATA, the oscilloscope and the comm runtime statics owned by one session; every reader and writer

Phase 6 · semantic · lane `comm` · about 3,900 semantic and 700 mechanical changed lines · depends on U5-01 · **critical path**

**Replaces** P6-01 (#644).

**Scope.** Comm runtime state owned by a session, with every reader and writer moved to it in the same PR: the comm runtime, hail, the ABI helpers, the locdata copier, credits, LiveScript and the race callbacks. The callbacks take the session as a parameter, so no comm static survives for the dialogue adapters; U6-02 then gives the adapters their content and modes form.

**Touches** (reserved): `src/comm/state.rs`, `src/comm/locdata.rs`, `src/comm/oscill_abi.rs`, `src/comm/oscilloscope.rs`, `src/comm/segue.rs`, `src/comm/talk_segue.rs`, `src/comm/animation.rs`, `src/comm/commanim_abi.rs`, `src/comm/encounter.rs`, `src/comm/hail.rs`, `src/comm/comm_abi.rs`, `src/comm/commglue_abi.rs`, `src/comm/sis_graphics.rs`, `src/comm/ffi.rs`, `src/comm/dispatch.rs`, `src/comm/mod.rs`, `src/comm/races/`, `src/planets/generate/system.rs`, `src/mainloop/state_abi.rs`, `src/front_end/credits.rs`, `src/abi/navigation_abi.rs`.
**Shared:** `src/lib.rs`.

**Change**
- COMM_STATE (comm/state.rs:23), COMM_DATA (comm/locdata.rs:15), the oscilloscope statics (comm/oscill_abi.rs:48-71) and the comm runtime statics (comm_abi.rs:30-63 phrase buffer, CommWndRect, subtitle, text-cache and input state; sis_graphics.rs:219-223 response-list state; commanim_abi.rs:215-230 animation state; talk_segue.rs:1388) become fields of the Comm session; comm::race_communication(game, io) builds it from the race's descriptor and is the entry; Site::talk (planets/generate/system.rs) calls it directly.
- Every reader and writer takes the session: ffi.rs, encounter.rs (:219-370), talk_segue.rs, commanim_abi.rs, segue.rs, animation.rs, oscilloscope.rs, oscill_abi.rs, sis_graphics.rs, dispatch.rs, hail.rs (:180-253, :490), comm_abi.rs (subtitle and talk-description reads, :92-287), commglue_abi.rs (number speech and phrases, :97-126), and LiveScript (comm/races/script.rs:65-148) with the race callbacks (talkpet/api.rs, zoqfot/api.rs and each live module), whose descriptor callback signature gains &mut Comm.
- The locdata copier rust_copy_locdata_to_comm_data (mainloop/state_abi.rs:72-77) is replaced by building the session from the descriptor.
- oscillDisabled, sliderDisabled and the credits' use of CommWndRect become CommOptions { oscilloscope, slider, window } given when a conversation starts: credits (front_end/credits.rs:44, :283, :510-525, :597-599) starts its conversations with both off and its centred window; InitCommunication and race_communication pass the defaults.

**Deletes** (gone when the PR merges; the acceptance checks prove it)
- COMM_STATE, COMM_DATA and the oscilloscope statics (SCOPE_FRAME, SCOPE_INIT, SCOPE_WORK, SCOPE_COLOR, SCOPE_WIDTH, SCOPE_HEIGHT, SLIDER, BUTTON, SLIDER_CHANGED, LAST_OFFSET, oscillDisabled, sliderDisabled, SLIDER_SPACE)
- the comm runtime statics in comm_abi.rs (shared_phrase_buf, CommWndRect, ANIM_CONTEXT, TEXT_CACHE_CONTEXT, TEXT_CACHE_FRAME, CLEAR_SUBTITLES, SUBTITLE, LAST_SUBTITLE, LAST_BASELINE, CURRENT_INPUT), sis_graphics.rs (LAST_TOP_RESPONSE, LAST_NUM_RESPONSES, LAST_CUR_RESPONSE), commanim_abi.rs (LAST_TIME, SEQUENCES, ACTIVE_MASK, TALK_DESC, TRANSIT_DESC, TOTAL) and talk_segue.rs (NEXT_TIME)
- the extern declarations of these statics in other files (CommWndRect in comm/hail.rs:75 and front_end/credits.rs:44; oscillDisabled and sliderDisabled in abi/navigation_abi.rs:3803,3806), the CommData re-export in comm/hail.rs:19 and the COMM_STATE and CommState re-export in lib.rs:47
- rust_copy_locdata_to_comm_data (mainloop/state_abi.rs:72-77)

**Out of scope.** ScriptApi, the descriptors, the dispatch constants and the race live modules in content and modes form (U6-02); script logic; moving files (U6-04).

**Acceptance**
- S (check bundle)
- P (comm, new-game)
- `cd rust && cargo test --locked comm::`
- `cd rust && cargo xtask quality report static_mut:comm --expect-only comm/races   # the race descriptor statics go in U6-02`
- `cd rust && cargo xtask quality ident COMM_STATE COMM_DATA SCOPE_FRAME oscillDisabled sliderDisabled CommWndRect SUBTITLE LAST_TOP_RESPONSE rust_copy_locdata_to_comm_data --expect 0`
- V2 (check bundle)
- V4 (smoke plus 5 scenarios); inspect both galleries with the image-comparer:

  ```sh
  pkill -x uqm || true
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native smoke --artifacts "$PWD/tmp/U6-01/v4-smoke" --content "$PWD/sc2/content/packages"
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native gallery "$PWD/tmp/U6-01/v4-smoke" --output "$PWD/tmp/U6-01/v4-smoke-gallery"
  pkill -x uqm || true
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native run --artifacts "$PWD/tmp/U6-01/v4-run" --content "$PWD/sc2/content/packages" \
    --scenarios "comm-encounter-v1,real-sol-probe-to-commander-responses,campaign-s06a-utwig-alliance,campaign-s08-talking-pet,probe-encounter-v1"
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native gallery "$PWD/tmp/U6-01/v4-run" --output "$PWD/tmp/U6-01/v4-run-gallery"
  ```

**Exit numbers.** comm static_mut outside comm/races 0; comm-global readers and writers outside the session 0 (hail, comm_abi, commglue_abi, encounter, the locdata copier and credits included).

**Risks**
- Speech completion timing is replay-controlled; real-time behaviour must match V4 screenshots.
- No corpus entry or V4 scenario reaches the credits; a comm:: test starts a conversation with CommOptions off and checks that neither the oscilloscope nor the slider draws and that the window is the one given.
- Subtitle and response-list state persists across frames of one conversation; it lives in the session for the conversation's lifetime, never reset per frame.

### U6-02 Dialogue host boundary: ScriptApi as the content trait DialogueHost, descriptors as content data, adapters in modes

Phase 6 · semantic · lane `comm` · about 3,300 semantic and 400 mechanical changed lines · depends on U6-01 · **critical path**

**Replaces** P6-02 (#645).

**Scope.** The dialogue adapters on the session of U6-01 change form: scripts and descriptors become content, the adapters become a modes-side host, and the dispatch constants that both name move to the engine. Script bodies (Dialogue<A: Api>) are unchanged.

**Touches** (reserved): `src/comm/races/`, `src/comm/dispatch.rs`, `src/comm/types.rs`, `src/comm/response.rs`, `src/comm/host.rs`, `src/comm/mod.rs`, `crates/engine/src/species.rs`, `src/planets/generate/`, `src/planets/devices.rs`, `src/planets/scan/`, `src/automation/scenario.rs`, `src/starbase/`.
**Shared:** `crates/engine/src/lib.rs`.

**Change**
- ScriptApi (comm/races/script.rs) becomes the content trait DialogueHost; CLocData/CommData descriptors (orz.rs:3-4,63) become content-owned CommDescriptor data.
- LiveScript (script.rs:65-89, :106, :148) becomes modes::comm::Host { game, comm } implementing DialogueHost; each race's live module (for example orz.rs:846-875) with its static mut DESC and SESSION is replaced by an entry in a modes-side table keyed by race that builds the descriptor and runs the script on Host.
- The dispatch constants comm::dispatch::{ship, conv} (dispatch.rs:296-371) move to the new engine module crates/engine/src/species.rs; a mechanical commit switches every importer: the comm/races scripts, planets/generate (shof.rs:11, shof/tests.rs:7 and any generator naming them), planets/devices.rs:12, planets/scan/live.rs:244, automation/scenario.rs:139-167 and starbase/cffi.rs:948.

**Deletes** (gone when the PR merges; the acceptance checks prove it)
- ScriptApi and LiveScript
- each race's live module (init_<race>_comm and its static mut DESC and SESSION)
- the ship and conv modules of comm::dispatch (moved to crates/engine/src/species.rs)

**Out of scope.** script logic; the comm session (U6-01); moving files (U6-04).

**Acceptance**
- S (check bundle)
- P (comm, new-game)
- `cd rust && cargo test --locked comm::`
- `cd rust && cargo xtask quality report static_mut:comm --expect 0`
- `cd rust && cargo xtask quality ident ScriptApi LiveScript --expect 0`
- `cd rust && rg -q 'dispatch::(ship|conv)\b|dispatch::\{[^}]*\b(ship|conv)\b' src crates tests; test $? -eq 1`
- `cd rust && cargo xtask quality report planned.uqm-content:comm/races --expect 0`
- V2 (check bundle)
- V4 (smoke plus 5 scenarios); inspect both galleries with the image-comparer:

  ```sh
  pkill -x uqm || true
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native smoke --artifacts "$PWD/tmp/U6-02/v4-smoke" --content "$PWD/sc2/content/packages"
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native gallery "$PWD/tmp/U6-02/v4-smoke" --output "$PWD/tmp/U6-02/v4-smoke-gallery"
  pkill -x uqm || true
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native run --artifacts "$PWD/tmp/U6-02/v4-run" --content "$PWD/sc2/content/packages" \
    --scenarios "comm-encounter-v1,real-sol-probe-to-commander-responses,campaign-s06a-utwig-alliance,campaign-s08-talking-pet,probe-encounter-v1"
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native gallery "$PWD/tmp/U6-02/v4-run" --output "$PWD/tmp/U6-02/v4-run-gallery"
  ```

**Exit numbers.** comm static_mut 0; planned.uqm-content from comm/races 0; race live modules 0.

**Risks**
- Story bits written during dialogue are read after talk returns; reread through the host.
- Each descriptor must keep its field values, callbacks and resource names; the comm corpus entries and V4 screenshots are the check.

### U6-03 Starbase and ui on Game and Io; testable ui modules

Phase 6 · semantic · lane `menus` · about 3,000 changed lines · depends on U4-04

**Replaces** P6-03 (#646).

**Scope.** Starbase visit and the ten ui modules.

**Touches** (reserved): `src/starbase/`, `src/ui/`.
**Shared:** none.

**Change**
- starbase::visit(game, io); ui modules take Game and Io.
- The ten #![cfg(not(test))] ui modules become testable.

**Deletes** (gone when the PR merges; the acceptance checks prove it)
- the ui extern "C" entry points (73) and statics (17), and the starbase extern "C" entry points
- the #![cfg(not(test))] attribute of the ten ui modules

**Out of scope.** front_end (U6-04).

**Acceptance**
- S (check bundle)
- P (check bundle)
- `cd rust && cargo test --locked ui::`
- `cd rust && cargo test --locked starbase::`
- `cd rust && for s in ui starbase; do for m in extern_c_fn static_mut unsafe; do cargo xtask quality report $m:$s --expect 0 || exit 1; done; done`
- `cd rust && rg -q '#!\[cfg\(not\(test\)\)\]' src/ui; test $? -eq 1`
- V2 (check bundle)
- V4 (smoke plus 3 scenarios); inspect both galleries with the image-comparer:

  ```sh
  pkill -x uqm || true
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native smoke --artifacts "$PWD/tmp/U6-03/v4-smoke" --content "$PWD/sc2/content/packages"
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native gallery "$PWD/tmp/U6-03/v4-smoke" --output "$PWD/tmp/U6-03/v4-smoke-gallery"
  pkill -x uqm || true
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native run --artifacts "$PWD/tmp/U6-03/v4-run" --content "$PWD/sc2/content/packages" \
    --scenarios "starbase-visit-v1,real-sol-starbase-alliance,load-starbase-armed-v1"
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native gallery "$PWD/tmp/U6-03/v4-run" --output "$PWD/tmp/U6-03/v4-run-gallery"
  ```

**Exit numbers.** ui extern_c_fn 73 -> 0, static_mut 17 -> 0; cfg_not_test_modules -10.

**Risks**
- ui draws through C-style graphics; add safe wrappers in uqm-engine (AGENTS.md Unsafe), never unsafe in ui.

### U6-04 Front end, restart menu and the last bridge calls; delete navigation_abi.rs and app::bridge; split uqm-content and uqm-modes

Phase 6 · semantic · lane `menus` · about 3,000 semantic and 2,000 mechanical changed lines · depends on U6-02, U6-03, U5-03, U5-04, U3-02 · **critical path**

**Replaces** P6-04 (#647), P6-05 (#648).

**Scope.** The remaining modes and the bridge in semantic commits that bring planned.uqm-content and planned.uqm-modes to 0, then the mechanical moves of the content and modes rows of the layer table as the last commit.

**Touches** (reserved): `Cargo.toml`, `crates/content/`, `crates/modes/`, `src/front_end/`, `src/mainloop/restart_menu/`, `src/mainloop/state_abi.rs`, `src/mainloop/input_abi.rs`, `src/abi/navigation_abi.rs`, `src/app/bridge.rs`, `src/state/schema.rs`, `src/ships/`, `src/planets/`, `src/comm/`, `src/battle/`, `src/supermelee/`, `src/pickship/`, `src/navigation/`, `src/planet_side/`, `src/starbase/`, `src/ui/`.
**Shared:** `src/lib.rs`, `xtask/src/quality/crate_deps.rs`.

**Change**
- front_end and restart menu on Game and Io; front_end/cnctdlg.rs testable.
- abi/navigation_abi.rs is deleted after its importers (7 front_end, 14 ui converted in U6-03, and 16 others: planets/pstarmap/search.rs, planets/devices.rs, planet_side/generation_adapter.rs, ships/native_adapters.rs, state/schema.rs, starbase/new_game_reference.rs, navigation/interplanetary.rs, navigation/groups.rs, supermelee/native/menu.rs and menu/) switch to engine types.
- app::bridge goes with the last counted bridge calls; the first commit lists every reference still counted by planned.uqm-content or planned.uqm-modes from cargo xtask quality report, and the semantic commits resolve them.
- Move commit: git mv content files (races, descriptors, generators, tables, comm/races) to crates/content and modes to crates/modes, module paths kept; root re-exports (until U8-01); #![forbid(unsafe_code)] in both; CRATE_DEPS edges; quality move --prefix.

**Deletes** (gone when the PR merges; the acceptance checks prove it)
- src/abi/navigation_abi.rs
- app::bridge (src/app/bridge.rs: the NonNull<Cx> slot, bridge::enter and the counted accessors) and the bridge_calls budget (adapter A4)
- the front_end and restart-menu extern "C" entry points and statics, and #![cfg(not(test))] in front_end/cnctdlg.rs
- the planned.uqm-content and planned.uqm-modes entries and the root copies of the moved modules (git mv)

**Out of scope.** code changes in the move commit.

**Acceptance**
- S (check bundle)
- P (new-game; identical across the move commit without re-recording)
- `cd rust && cargo xtask quality report bridge_calls --expect 0`
- `cd rust && test ! -e src/abi/navigation_abi.rs && test ! -e src/app/bridge.rs`
- `cd rust && rg -q '#!\[cfg\(not\(test\)\)\]' src crates; test $? -eq 1`
- `cd rust && cargo test -p uqm-content -p uqm-modes --locked`
- `mkdir -p tmp/U6-04 && cd rust && cargo tree -p uqm-modes -e normal --locked > ../tmp/U6-04/tree.txt && { grep -q 'uqm-rust ' ../tmp/U6-04/tree.txt; test $? -eq 1; }`
- `mkdir -p tmp/U6-04 && cd rust && cargo build --locked && touch crates/modes/src/lib.rs && cargo build --locked > ../tmp/U6-04/rebuild.log 2>&1 && { grep -qE 'Compiling uqm-(engine|content) ' ../tmp/U6-04/rebuild.log; test $? -eq 1; }`
- V2 (check bundle)
- R5 (check bundle)
- V4 (smoke plus 6 scenarios); inspect both galleries with the image-comparer:

  ```sh
  pkill -x uqm || true
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native smoke --artifacts "$PWD/tmp/U6-04/v4-smoke" --content "$PWD/sc2/content/packages"
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native gallery "$PWD/tmp/U6-04/v4-smoke" --output "$PWD/tmp/U6-04/v4-smoke-gallery"
  pkill -x uqm || true
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native run --artifacts "$PWD/tmp/U6-04/v4-run" --content "$PWD/sc2/content/packages" \
    --scenarios "main-menu-v1,setup-menu-v1,first-open-load-empty-v1,first-open-load-populated-v1,netplay-dialog-v1,introduction-v1"
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native gallery "$PWD/tmp/U6-04/v4-run" --output "$PWD/tmp/U6-04/v4-run-gallery"
  ```

**Exit numbers.** cfg_not_test_modules = 0; bridge_calls = 0 and budget deleted; planned.uqm-content = 0 and planned.uqm-modes = 0, entries deleted; workspace members 4 -> 6; both new crates unsafe 0 by forbid.

**Risks**
- If the planned counters show more than this PR can resolve within 6,000 non-mechanical lines, stop and file the remainder as a unit before the move.
- Do the move commit last, when no other unit is in progress; every open branch rebases right after the merge.

## Phase 7: platform and content loading; uqm-platform

Exit: uio_* = 0 with every caller on Files; unsafe only in UNSAFE_ALLOWED modules of uqm-platform; uqm-engine has forbid(unsafe_code) and no OS crate; canvas digests match for every corpus entry; real-window screenshots inspected for the smoke set and the intro.

### U7-01 Render pipeline: Platform present, pixel-buffer canvases and video presentation; DCQ, contexts and primitives behind safe types

Phase 7 · semantic · lane `graphics` · about 5,800 changed lines · depends on U6-04 · **critical path**

**Replaces** P7-01 (#649), P7-02 (#650).

**Scope.** Canvases become Rust pixel buffers, only the final present touches SDL, and the draw command queue and contexts that write those canvases convert in the same PR; video moves to the new canvas and present path with it.

**Touches** (reserved): `crates/engine/src/graphics/sdl/`, `crates/engine/src/graphics/native_canvas/`, `crates/engine/src/graphics/ffi.rs`, `crates/engine/src/graphics/software_scalers/`, `crates/engine/src/graphics/dcqueue.rs`, `crates/engine/src/graphics/dcqueue_abi.rs`, `crates/engine/src/graphics/context.rs`, `crates/engine/src/graphics/context_abi.rs`, `crates/engine/src/graphics/render_context.rs`, `crates/engine/src/graphics/tfb_draw.rs`, `crates/engine/src/graphics/tfb_draw_abi.rs`, `crates/engine/src/graphics/tfb_prim_abi.rs`, `crates/engine/src/graphics/gfx_common.rs`, `crates/engine/src/graphics/gfx_common_abi.rs`, `crates/engine/src/graphics/software_primitives.rs`, `crates/engine/src/graphics/wrappers.rs`, `crates/engine/src/video/ffi.rs`, `crates/engine/src/video/scaler.rs`, `crates/modes/src/`, `src/app/`, `src/automation/capture.rs`, `tests/video_native.rs`, `tests/canvas_native.rs`.
**Shared:** `crates/engine/src/host.rs`, `xtask/src/quality/macros.rs`.

**Change**
- engine::host::Platform with present; SdlPlatform implements it; Io gains its platform field; the caller of present calls Observer::presented and captures through Platform.
- SDL_Surface canvases become Vec<u32> RGBA8888 buffers that the scalers already take.
- video/ffi.rs stops importing SDL_Surface and SDL_Screen (:18, :42-45) and creating SDL textures through with_gfx_state (:846-852); frames are written to the canvas and presented through Platform.
- DCQ commands as a Rust enum over the canvas; contexts as owned values; primitives draw through them; the safe wrappers added since Phase 1 become their API.

**Deletes** (gone when the PR merges; the acceptance checks prove it)
- SDL_Surface canvases outside graphics/sdl, including video's
- the present_hook and readback_hook fields of the graphics backend state (adapter A5)
- graphics/{dcqueue_abi,context_abi,tfb_draw_abi,tfb_prim_abi,gfx_common_abi}.rs, with screen_global! and its walker handler
- graphics/sdl/{pure_abi,platform_abi}.rs, whose C-style rendering entry points are replaced by Platform present

**Out of scope.** drawables, frames, fonts, colour maps (U7-02); video decoding and audio (U7-04).

**Acceptance**
- S (check bundle)
- P (canvas digests for every corpus entry)
- `cd rust && cargo test --locked --test video_native --test canvas_native --test intro_presentation`
- `cd rust && cargo test -p uqm-engine --locked graphics::dcqueue`
- `cd rust && cargo test -p uqm-engine --locked graphics::context`
- `cd rust && for f in dcqueue_abi context_abi tfb_draw_abi tfb_prim_abi gfx_common_abi; do test ! -e crates/engine/src/graphics/$f.rs || exit 1; done`
- `cd rust && cargo xtask quality ident present_hook readback_hook screen_global --expect 0`
- `cd rust && cargo xtask quality ident SDL_Surface --expect-only graphics/sdl`
- V2 (check bundle)
- R5 (check bundle)
- V4 (smoke plus 5 scenarios); inspect both galleries with the image-comparer:

  ```sh
  pkill -x uqm || true
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native smoke --artifacts "$PWD/tmp/U7-01/v4-smoke" --content "$PWD/sc2/content/packages"
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native gallery "$PWD/tmp/U7-01/v4-smoke" --output "$PWD/tmp/U7-01/v4-smoke-gallery"
  pkill -x uqm || true
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native run --artifacts "$PWD/tmp/U7-01/v4-run" --content "$PWD/sc2/content/packages" \
    --scenarios "introduction-v1,battle-v1,real-sol-earth-orbit,setup-menu-v1,starbase-visit-v1"
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native gallery "$PWD/tmp/U7-01/v4-run" --output "$PWD/tmp/U7-01/v4-run-gallery"
  ```

**Exit numbers.** SDL_Surface uses outside graphics/sdl: 0, video included; the five *_abi.rs files deleted and their no_mangle and extern_c_fn counts 0.

**Risks**
- Capture generation must match the present it belongs to.
- DCQ ordering decides overdraw; canvas digests are the oracle.

### U7-02 Graphics assets and ResourceCache: drawables, frames, fonts, colour maps and string tables as typed handles; delete the graphics ABI and res_*

Phase 7 · semantic · lane `graphics` · about 5,500 changed lines · depends on U7-01, U7-03 · **critical path**

**Replaces** P7-03 (#651), P7-05 (#653).

**Scope.** The rest of the graphics C-style API and the resource handles that own its assets, including the screen-format call video initialisation uses. File reading is already on Files (U7-03).

**Touches** (reserved): `crates/engine/src/graphics/drawable.rs`, `crates/engine/src/graphics/drawable_abi.rs`, `crates/engine/src/graphics/frame.rs`, `crates/engine/src/graphics/frame_abi.rs`, `crates/engine/src/graphics/font.rs`, `crates/engine/src/graphics/font_abi.rs`, `crates/engine/src/graphics/cmap.rs`, `crates/engine/src/graphics/cmap_abi.rs`, `crates/engine/src/graphics/pixmap.rs`, `crates/engine/src/graphics/pixmap_abi.rs`, `crates/engine/src/graphics/widgets_abi.rs`, `crates/engine/src/graphics/native_abi.rs`, `crates/engine/src/graphics/geometry_ffi.rs`, `crates/engine/src/graphics/gfxload.rs`, `crates/engine/src/graphics/filegfx.rs`, `crates/engine/src/graphics/resgfx.rs`, `crates/engine/src/graphics/loaddisp.rs`, `crates/engine/src/video/control.rs`, `crates/engine/src/resource/`, `crates/engine/src/strings/`, `crates/engine/src/battle/`, `crates/content/src/`, `src/abi/graphics_abi.rs`.
**Shared:** none.

**Change**
- Drawables, frames, fonts, colour maps and pixmaps as owned types with safe methods.
- ResourceCache returns Handle<Frame>, Handle<StringTable>, Handle<Sound>; dropping the owner releases; FrameSetRef handles backed by the cache.
- video/control.rs (:30-38) reads the screen format through the safe canvas API instead of TFB_DrawCanvas_GetScreenFormat.

**Deletes** (gone when the PR merges; the acceptance checks prove it)
- graphics/{drawable_abi,frame_abi,font_abi,cmap_abi,pixmap_abi,widgets_abi,native_abi,geometry_ffi}.rs and src/abi/graphics_abi.rs
- the res_* resource functions and the CaptureDrawable/ReleaseDrawable/DestroyDrawable pairs
- the TFB_DrawCanvas_GetScreenFormat call in video/control.rs

**Out of scope.** decoders (U7-04).

**Acceptance**
- S (check bundle)
- P (frame-sets, canvas digests for every corpus entry)
- `cd rust && cargo test -p uqm-engine --locked graphics::`
- `cd rust && cargo test --locked --test graphics_loading --test string_hash_table --test video_native`
- `cd rust && for f in drawable_abi frame_abi font_abi cmap_abi pixmap_abi widgets_abi native_abi geometry_ffi; do test ! -e crates/engine/src/graphics/$f.rs || exit 1; done && test ! -e src/abi/graphics_abi.rs`
- `cd rust && for m in no_mangle extern_c_fn; do cargo xtask quality report $m:graphics --expect 0 || exit 1; done`
- `cd rust && cargo xtask quality ident TFB_DrawCanvas_GetScreenFormat CaptureDrawable ReleaseDrawable DestroyDrawable --expect 0`
- `cd rust && rg -q '\bres_[A-Za-z]+\(' src crates; test $? -eq 1`
- V2 (check bundle)
- V4 (smoke plus 5 scenarios); inspect both galleries with the image-comparer:

  ```sh
  pkill -x uqm || true
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native smoke --artifacts "$PWD/tmp/U7-02/v4-smoke" --content "$PWD/sc2/content/packages"
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native gallery "$PWD/tmp/U7-02/v4-smoke" --output "$PWD/tmp/U7-02/v4-smoke-gallery"
  pkill -x uqm || true
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native run --artifacts "$PWD/tmp/U7-02/v4-run" --content "$PWD/sc2/content/packages" \
    --scenarios "introduction-v1,comm-encounter-v1,real-sol-earth-orbit,starbase-visit-v1,real-sol-pluto-orbit-scan"
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native gallery "$PWD/tmp/U7-02/v4-run" --output "$PWD/tmp/U7-02/v4-run-gallery"
  ```

**Exit numbers.** graphics no_mangle 243 -> 0, extern_c_fn 295 -> 0; res_* functions 0.

**Risks**
- Colour-map fades run over several frames; the comm and intro screenshots are the check.
- A blast that copied a weapon's frame set must draw after its ship is torn down (frame-sets fixture).

### U7-03 Files port with one Vfs (read, write, remove, list, stat); every file caller on it; delete the uio_* API

Phase 7 · semantic · lane `content-loading` · about 5,500 semantic and 500 mechanical changed lines · depends on U7-01 · **critical path**

**Replaces** P7-04 (#652), P7-07 (#655).

**Scope.** One Vfs behind engine::host::Files and every caller of the uio_* API moved to it in the same PR, so uio_* is never a wrapper over Vfs. Depends on U7-01 because both edit video/ffi.rs and graphics/sdl/.

**Touches** (reserved): `crates/engine/src/io/`, `crates/engine/src/kernel/`, `src/mainloop/dir_prep.rs`, `src/mainloop/options_abi.rs`, `src/game_init/kernel_abi.rs`, `crates/engine/src/resource/direct.rs`, `crates/engine/src/resource/ffi_bridge.rs`, `crates/engine/src/strings/getstr.rs`, `crates/engine/src/graphics/gfxload.rs`, `crates/engine/src/graphics/sdl/sdluio.rs`, `crates/engine/src/sound/aiff_ffi.rs`, `crates/engine/src/sound/dukaud_ffi.rs`, `crates/engine/src/sound/ffi.rs`, `crates/engine/src/sound/heart_ffi.rs`, `crates/engine/src/sound/mod_ffi.rs`, `crates/engine/src/sound/wav_ffi.rs`, `crates/engine/src/video/ffi.rs`, `src/savegame/slots.rs`, `src/savegame/lzh/`, `src/savegame/legacy.rs`, `crates/save/src/lzh/`, `crates/modes/src/supermelee/native/teams.rs`, `crates/modes/src/supermelee/native/load_teams.rs`, `crates/modes/src/supermelee/native/menu.rs`, `crates/modes/src/supermelee/setup/persistence.rs`.
**Shared:** `crates/engine/src/host.rs`, `crates/save/src/lib.rs`.

**Change**
- uio_bridge.rs becomes Vfs, keeping mount order and precedence (uio_bridge.rs:20-27) and zip packages; mount setup (mainloop/dir_prep.rs, options_abi.rs, kernel init) and the content, config, save and melee directory handles live in Vfs; the replay build uses the same Vfs.
- engine::host::Files covers what callers do today: open for read, open for write, remove, list a directory, stat.
- Readers on Files: resource/direct.rs (:22-45), strings/getstr.rs (:52-65, :395, :423), graphics/gfxload.rs and sdl/sdluio.rs, the sound decoders' stream opens (for example sound/aiff_ffi.rs:41-62) and the video decoder (video/ffi.rs:112-151).
- Save slots write, settle and remove through Files (savegame/slots.rs:15-17, 61-87), keeping the remove-on-failed-write rule; the LZH streams read and write through Read/Write and move to uqm-save (git mv plus stream changes).
- Melee team files save, remove and list through Files: supermelee/setup/persistence.rs (:94-100, :269, std::fs today) and native/{teams,load_teams,menu}.rs (uio today).
- The uio tests (io/uio_*_tests.rs) are rewritten against Vfs with the same data.

**Deletes** (gone when the PR merges; the acceptance checks prove it)
- every uio_* function and the C-style io API (io/ffi.rs, the uio_bridge.rs C entry points)
- the LZH statics _LP_CUR_CODE_DESC, _STREAM_TYPE, _STREAM, _WORKBUF, _WORKBUFLEN (savegame/lzh/mod.rs:45-49) and the root savegame/lzh/ (moved)
- std::fs use in crates/modes (supermelee/setup/persistence.rs)

**Out of scope.** ResourceCache handles (U7-02); decoder vtables (U7-04).

**Acceptance**
- S (check bundle)
- P (check bundle)
- `cd rust && cargo test --locked --test zip_integration_test`
- `cd rust && cargo test -p uqm-engine --locked io::`
- `cd rust && cargo test -p uqm-save --locked lzh::`
- `cd rust && cargo test --locked savegame::slot`
- `cd rust && rg -q '\buio_[a-zA-Z]+\(' src crates; test $? -eq 1`
- `cd rust && test ! -e src/savegame/lzh && cargo xtask quality ident _LP_CUR_CODE_DESC _STREAM_TYPE _WORKBUF _WORKBUFLEN --expect 0`
- `cd rust && cargo xtask quality report banned_std:uqm-modes --expect 0 && cargo xtask quality report unsafe:io --expect 0`
- `cd rust && cargo build --release --locked --bin uqm && target/release/uqm --check-content --contentdir=../sc2/content`
- V3 (check bundle)
- V2 (check bundle)
- V4 (smoke plus 7 scenarios); inspect both galleries with the image-comparer:

  ```sh
  pkill -x uqm || true
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native smoke --artifacts "$PWD/tmp/U7-03/v4-smoke" --content "$PWD/sc2/content/packages"
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native gallery "$PWD/tmp/U7-03/v4-smoke" --output "$PWD/tmp/U7-03/v4-smoke-gallery"
  pkill -x uqm || true
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native run --artifacts "$PWD/tmp/U7-03/v4-run" --content "$PWD/sc2/content/packages" \
    --scenarios "first-open-load-populated-v1,load-save-v1,first-open-load-empty-v1,super-melee-mutual-loss-v1,introduction-v1,comm-encounter-v1,starbase-visit-v1"
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native gallery "$PWD/tmp/U7-03/v4-run" --output "$PWD/tmp/U7-03/v4-run-gallery"
  ```

**Exit numbers.** one Vfs implementation; uio_* functions 0; io unsafe 0; std::fs in uqm-modes 0.

**Risks**
- Addon and package precedence decide which file wins; --check-content and V4 screenshots are the check.
- A failed save write must leave no slot file, as today; slot tests cover it.
- The unit is at the size limit; if it passes 6,000 non-mechanical lines, stop and file.

### U7-04 Sound and video decoders as trait Decoder; the Platform audio port

Phase 7 · semantic · lane `content-loading` · about 2,500 changed lines · depends on U7-02 · **critical path**

**Replaces** P7-06 (#654).

**Scope.** Decoder vtables and the audio thread boundary; video decoding and video audio. Streams already open through Files (U7-03).

**Touches** (reserved): `crates/engine/src/sound/`, `crates/engine/src/video/decoder.rs`, `crates/engine/src/video/player.rs`, `crates/engine/src/video/legacy.rs`, `crates/engine/src/video/registry.rs`, `crates/engine/src/video/ffi.rs`, `src/lib.rs`.
**Shared:** `crates/engine/src/host.rs`.

**Change**
- Decoder vtables (rust_ova_DecoderVtbl, lib.rs:51, and the aiff, dukaud, heart, mod, wav and ogg vtables) become trait Decoder.
- The rodio backend implements the Platform audio methods; ReplayAudio implements the same trait.
- The video decoder plays its audio through the port.

**Deletes** (gone when the PR merges; the acceptance checks prove it)
- rust_ova_DecoderVtbl and every other decoder vtable with its extern "C" functions
- sound/ffi.rs, whose decoder entry points and vtable are replaced by Decoder
- sound and video unsafe outside the rodio backend

**Out of scope.** input (U7-05).

**Acceptance**
- S (check bundle)
- P (check bundle)
- `cd rust && cargo test --locked --test sound_integration --test video_native --test intro_presentation`
- `cd rust && cargo xtask quality ident rust_ova_DecoderVtbl --expect 0`
- `cd rust && cargo xtask quality report unsafe:sound --expect-only sound/rodio_audio.rs,sound/rodio_backend.rs && cargo xtask quality report unsafe:video --expect 0`
- V2 (check bundle)
- V4 (smoke plus 2 scenarios); inspect both galleries with the image-comparer:

  ```sh
  pkill -x uqm || true
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native smoke --artifacts "$PWD/tmp/U7-04/v4-smoke" --content "$PWD/sc2/content/packages"
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native gallery "$PWD/tmp/U7-04/v4-smoke" --output "$PWD/tmp/U7-04/v4-smoke-gallery"
  pkill -x uqm || true
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native run --artifacts "$PWD/tmp/U7-04/v4-run" --content "$PWD/sc2/content/packages" \
    --scenarios "introduction-v1,comm-encounter-v1"
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native gallery "$PWD/tmp/U7-04/v4-run" --output "$PWD/tmp/U7-04/v4-run-gallery"
  ```

**Exit numbers.** decoder vtables 0; sound and video unsafe outside the rodio backend: 0.

**Risks**
- Speech end times in replay come from decoded lengths; changing a decoder changes them.

### U7-05 Input, threading, callbacks and the OS clock behind Platform; delete time::source

Phase 7 · semantic · lane `graphics` · about 2,600 changed lines · depends on U7-01

**Replaces** P7-08 (#656).

**Scope.** The remaining OS-facing engine code, including the replay clock module that U0-05 created and U1-04 moved.

**Touches** (reserved): `crates/engine/src/input/`, `crates/engine/src/threading/`, `crates/engine/src/callback/`, `crates/engine/src/time/`, `src/time/mod.rs`, `src/main.rs`, `src/app/`.
**Shared:** `crates/engine/src/host.rs`.

**Change**
- VControl reads events from Platform::poll_input; sdl_input stays SDL-only.
- Pacing, GetTimeCounter and the callback alarm clock use the Platform clock. crates/engine/src/time/source.rs is replaced: its Real arm becomes SdlPlatform's clock and its Replay arm a Platform clock implementation that the app selects from --replay where it builds the Platform (src/main.rs, src/app/), replacing main.rs's TIME_SOURCE install; the module, its declaration in crates/engine/src/time/mod.rs and its re-export in the root src/time/mod.rs are deleted.
- The callback queue becomes a plain owned queue.

**Deletes** (gone when the PR merges; the acceptance checks prove it)
- threading/legacy_abi.rs (InitThreadSystem, CreateThread_Core, WaitThread, FinishThread, ProcessThreadLifecycles; CreateThread_Core has no caller)
- crates/engine/src/time/source.rs with TimeSource and the TIME_SOURCE set-once static (adapter A6), its module declaration and its root re-export in src/time/mod.rs
- the callback queue statics

**Out of scope.** the crate carve (U7-06).

**Acceptance**
- S (check bundle)
- P (check bundle)
- `cd rust && cargo test --locked --test input_integration_tests`
- `cd rust && cargo test --locked replay::`
- `cd rust && test ! -e crates/engine/src/threading/legacy_abi.rs && test ! -e crates/engine/src/time/source.rs && cargo xtask quality ident TIME_SOURCE TimeSource CreateThread_Core --expect 0`
- `cd rust && for s in input threading callback; do cargo xtask quality report unsafe:$s --expect-only input/sdl_input.rs,threading/qos.rs || exit 1; done`
- V2 (check bundle)
- V4 (smoke plus 3 scenarios); inspect both galleries with the image-comparer:

  ```sh
  pkill -x uqm || true
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native smoke --artifacts "$PWD/tmp/U7-05/v4-smoke" --content "$PWD/sc2/content/packages"
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native gallery "$PWD/tmp/U7-05/v4-smoke" --output "$PWD/tmp/U7-05/v4-smoke-gallery"
  pkill -x uqm || true
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native run --artifacts "$PWD/tmp/U7-05/v4-run" --content "$PWD/sc2/content/packages" \
    --scenarios "setup-menu-v1,main-menu-v1,battle-v1"
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native gallery "$PWD/tmp/U7-05/v4-run" --output "$PWD/tmp/U7-05/v4-run-gallery"
  ```

**Exit numbers.** input, threading, callback unsafe outside sdl_input and qos: 0; new globals of the plan: 0.

**Risks**
- Key repeat and joystick timing read the real clock in native mode; V2 and V4 check them.

### U7-06 Process, descriptor and file-system OS calls behind platform::os; carve uqm-platform out of uqm-engine

Phase 7 · semantic · lane `platform` · about 2,200 semantic and 2,000 mechanical changed lines · depends on U7-04, U7-05 · **critical path**

**Replaces** P7-09 (#657), P7-10 (#658).

**Scope.** Every unsafe in src/automation, plus platform.rs's descriptor and errno operations, moves behind one reviewed os module in a semantic commit; the move commit carves the platform row, with os and the CoreGraphics and sandbox parts of automation, into uqm-platform.

**Touches** (reserved): `Cargo.toml`, `crates/platform/`, `crates/engine/`, `src/automation/`, `examples/platform_smoke.rs`, `.github/workflows/ci.yaml`, `xtask/src/quality/`.
**Shared:** `src/lib.rs`.

**Change**
- engine::platform::os with safe functions for what automation does today: descriptor inheritance (child_session.rs:253), monotonic clock (:351-358), poll with deadline and read/write (:384-411), signals and locks (:1463, :1546), fd-relative openat/unlinkat/mkdirat/linkat/fchmod/fsync and mkfifo (native_runner.rs:461-543, 2523-2548, 2827-2844, 3885-3932, 5421), getuid/geteuid (:1306-1307); platform.rs's redirect_stderr (dup2, open_osfhandle; :12-36) and errno cell (:71-101) join it. Automation keeps its process model and calls the safe functions; the existing cfg(unix)/cfg(windows) splits are kept.
- Move commit: git mv graphics/sdl, ffi_video*, input/sdl_input, sound/rodio_*, network sockets, io backends, threading/qos, time/counter_abi and engine::platform to crates/platform/src/{sdl,macos,net,qos,vfs,clock,os}, with the CoreGraphics and sandbox parts of automation/native_window.rs and native_isolation.rs.
- #![deny(unsafe_code)] in uqm-platform with allows on UNSAFE_ALLOWED; #![forbid(unsafe_code)] in uqm-engine; build.rs and examples/platform_smoke.rs move; CI game-build example command updated; delete [crate_os."uqm-engine"]; CRATE_DEPS platform -> engine.

**Deletes** (gone when the PR merges; the acceptance checks prove it)
- every unsafe block in src/automation (moved behind os, then into uqm-platform)
- platform.rs in uqm-engine (moved) and the root examples/platform_smoke.rs (moved)
- uqm-engine's OS crate dependencies (cpal, gl, libc, rodio, sdl2, sdl2-sys, windows-sys) and the [crate_os."uqm-engine"] baseline entry

**Out of scope.** code changes in the move commit beyond visibility and module wiring.

**Acceptance**
- S (check bundle)
- P (identical across the move commit)
- `cd rust && cargo test --locked automation::`
- `cd rust && cargo xtask quality report unsafe:automation --expect 0`
- `cd rust && cargo xtask quality report crate_os:uqm-engine --expect 0`
- `cd rust && cargo run --release --locked -p uqm-platform --example platform_smoke --features bundled-sdl`
- V2 (check bundle)
- R5 (check bundle)
- V4 (smoke only); inspect both galleries with the image-comparer:

  ```sh
  pkill -x uqm || true
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native smoke --artifacts "$PWD/tmp/U7-06/v4-smoke" --content "$PWD/sc2/content/packages"
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native gallery "$PWD/tmp/U7-06/v4-smoke" --output "$PWD/tmp/U7-06/v4-smoke-gallery"
  ```

**Exit numbers.** automation unsafe 0; workspace members 6 -> 7; uqm-engine unsafe 0 by forbid and no OS crate.

**Risks**
- Child-session pipes carry the native harness protocol; V2 and V4 exercise it end to end.
- Release packaging finds sdl2-sys through cargo metadata; run the release-tools tests (R5).

## Phase 8: closeout

Exit: static_mut = 0; no_mangle = 0; foreign items, repr(C) and raw-pointer parameters only in uqm-platform; root forbid(unsafe_code); unwrap/expect in production = 0; coverage >= 70%; baseline holds LOC and complexity entries only.

### U8-01 Closeout: delete abi/, abi-check and the root re-exports; debt lints deny, root forbid, unwrap and coverage targets

Phase 8 · semantic · lane `closeout` · about 3,000 semantic and 6,000 mechanical changed lines · depends on U7-06 · **critical path**

**Replaces** P8-01 (#659), P8-02 (#660).

**Scope.** Mechanical deletion and path rewrite first (from then on S8 replaces S), then the Phase 8 exit criteria.

**Touches** (reserved): `Cargo.toml`, `src/`, `crates/`, `xtask/src/abi_check.rs`, `.github/workflows/ci.yaml`.
**Shared:** `xtask/src/lib.rs`.

**Change**
- Mechanical commits: delete rust/src/abi/ (12.1k lines today), xtask abi-check, its dispatcher arm and its CI step; replace root re-exports of crate modules with crate paths; delete non_snake_case, non_upper_case_globals and non_camel_case_types allows by renaming (renames recorded with quality move).
- Debt lints become deny; #![forbid(unsafe_code)] in the root.
- Remaining production unwrap/expect become Result or fail-fast panics with context; tests until coverage reaches 70%; final --tighten.

**Deletes** (gone when the PR merges; the acceptance checks prove it)
- rust/src/abi/, xtask/src/abi_check.rs, the abi-check dispatcher arm and CI step
- the root re-exports of crate modules (adapter A7)
- every non_snake_case, non_upper_case_globals and non_camel_case_types allow
- every production unwrap/expect and the debt-set baseline entries

**Out of scope.** new features.

**Acceptance**
- S8 (check bundle)
- P (check bundle)
- V3 (check bundle)
- `cd rust && test ! -e src/abi && test ! -e xtask/src/abi_check.rs && { grep -q 'abi-check' ../.github/workflows/ci.yaml; test $? -eq 1; }`
- `cd rust && cargo xtask quality report foreign_items --expect-only uqm-platform`
- `cd rust && rg -q 'non_snake_case|non_upper_case_globals|non_camel_case_types' src crates; test $? -eq 1`
- V2 (check bundle)
- R5 (check bundle)
- V4 (smoke plus 46 scenarios); inspect both galleries with the image-comparer:

  ```sh
  pkill -x uqm || true
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native smoke --artifacts "$PWD/tmp/U8-01/v4-smoke" --content "$PWD/sc2/content/packages"
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native gallery "$PWD/tmp/U8-01/v4-smoke" --output "$PWD/tmp/U8-01/v4-smoke-gallery"
  pkill -x uqm || true
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native run --artifacts "$PWD/tmp/U8-01/v4-run" --content "$PWD/sc2/content/packages" \
    --scenarios "load-alpha-centauri-v1,load-bomb-preguard-v1,load-bomb-refueled-v1,load-broken-ultron-v1,load-burvixese-caster-v1,load-caster-diversion-v1,load-connected-ambush-v1,load-connected-arilou-v1,load-connected-chmmr-bomb-v1,load-connected-combat-prep-v1,load-connected-final-fleet-v1,load-connected-full-drives-v1,load-connected-funded-fleet-v1,load-connected-jugger-prep-v1,load-connected-lure-prep-v1,load-connected-lure-v1,load-connected-rainbow-funded-v1,load-connected-samatra-distraction-v1,load-connected-samatra-remnants-v1,load-connected-shattered-v1,load-connected-sun-device-v1,load-connected-syreen-v1,load-connected-taalo-v1,load-connected-talking-pet-v1,load-connected-umgah-fleet-v1,load-connected-vault-resupply-v1,load-connected-vault-v1,load-funded-sol-v1,load-ilwrath-approach-v1,load-north-artifacts-v1,load-north-ready-v1,load-north-refueled-v1,load-portal-sol-v1,load-portal-staging-v1,load-repaired-ultron-v1,load-save-v1,load-south-refueled-v1,load-southeast-staging-v1,load-spindle-pkunk-v1,load-starbase-armed-v1,load-thraddash-diverted-v1,load-utwig-alliance-v1,load-utwig-bomb-v1,load-war-cleared-v1,load-west-bomb-refueled-v1,introduction-v1"
  cargo run --release --locked --manifest-path rust/xtask/Cargo.toml -- native gallery "$PWD/tmp/U8-01/v4-run" --output "$PWD/tmp/U8-01/v4-run-gallery"
  ```

**Exit numbers.** foreign items outside uqm-platform 0; naming allows 0; unwrap/expect in production 0; coverage >= 70%; baseline holds LOC and complexity only.

**Risks**
- Renames are debt transfers; record them with quality move.
- If leftover unwraps exceed 6,000 non-mechanical lines, stop and file a unit per crate before this one (AGENTS.md).
