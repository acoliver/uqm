# Migration plan: C-shaped Rust to idiomatic Rust

Nine phases (0-8), ordered by dependency and risk. Each phase is a series of PRs;
`main` stays playable and green after every one. Target design:
[architecture.md](architecture.md). Checks: [quality-gates.md](quality-gates.md).
The PRs are listed as 40 work units in [work-items.md](work-items.md).
Implementing agents follow [AGENTS.md](AGENTS.md).

## 1. Starting numbers

Production code at 0ba2845ad, measured with line regexes (method in architecture.md
section 1, which also gives crate totals for the other counters). The Phase 0
baseline replaces these with syn-based counts.

| subsystem | prod lines | unsafe % | static mut | extern "C" fn defs | no_mangle |
|---|---|---|---|---|---|
| comm | 32,597 | 8 | 70 | 68 | 65 |
| automation | 28,017 | 3 | 0 | 16 | 6 |
| graphics | 20,438 | 22 | 43 | 295 | 243 |
| ships | 18,914 | 34 | 32 | 259 | 47 |
| planets | 16,857 | 32 | 46 | 199 | 135 |
| sound | 11,482 | 16 | 6 | 66 | 34 |
| abi | 11,240 | 3 | 0 | 0 | 0 |
| battle | 10,657 | 40 | 22 | 160 | 120 |
| supermelee | 6,676 | 49 | 16 | 267 | 221 |
| planet_side | 6,234 | 13 | 1 | 9 | 2 |
| front_end | 5,924 | 74 | 48 | 46 | 13 |
| starbase | 5,781 | 3 | 0 | 1 | 0 |
| ui | 5,604 | 71 | 17 | 73 | 67 |
| savegame | 5,322 | 11 | 6 | 11 | 5 |
| mainloop | 4,482 | 25 | 20 | 75 | 56 |
| io | 4,016 | 27 | 0 | 18 | 18 |
| navigation | 3,061 | 79 | 8 | 43 | 32 |
| input | 2,992 | 29 | 1 | 24 | 13 |
| video | 2,759 | 28 | 6 | 59 | 3 |
| state | 2,269 | 12 | 1 | 11 | 11 |
| resource | 2,040 | 34 | 1 | 48 | 36 |
| threading | 1,360 | 30 | 0 | 7 | 5 |
| network | 1,103 | 59 | 0 | 39 | 33 |
| game_init | 958 | 50 | 30 | 41 | 32 |
| strings | 958 | 42 | 0 | 24 | 19 |
| collections | 684 | 64 | 0 | 9 | 9 |
| others (bin, root, pickship, time, callback, math) | 5,308 | n/a | 1 | 54 | 32 |
| **total** | **217,733** | **23** | **375** | **1,922** | **1,257** |

All of `ui/` and `navigation/hyper_*` are `#![cfg(not(test))]`.

## 2. Verification kit

Each phase states which of these it needs. V1, V2, V3 and V5 run on every PR in CI.

**V1. Deterministic replay** (`cargo xtask parity record|check`, built in Phase 0).
Fixed seeds (`automation/trace.rs:49`) and SDL dummy drivers leave real time
(`time/counter_abi.rs:27-29`, `comm/commanim_abi.rs:258-297`), speech completion
(`comm/talk_segue.rs:411-451`) and pacing (`threading/pacing.rs:122-146`) variable.
Replay mode (`uqm --replay <scenario>`) replaces exactly these inputs:

1. **Clock.** `time::source` (`src/time/source.rs`, created by U0-05) is chosen once
   at start: `Real` (SDL ticks) or `Replay`.
   `GetTimeCounter`, the replay `PacingEnv` (`pacing.rs:100`) and the callback alarm
   queue (`callback/ffi.rs:12-14`, which reads `SDL_GetTicks` today) read it. The
   module moves with the counter into `uqm-engine` (U1-04), and the choice moves into
   `Platform` in Phase 7 (U7-05), which deletes the module. Replay time
   moves only when the game sleeps: `sleep_until(wake)` runs one `process_async` and
   `pump_and_flush` pass, then sets now to `max(now, wake)`; no real sleep happens.
   `TaskSwitch` and `Async_process` leave time unchanged, so alarms fire at fixed
   replay times. A loop that reads the clock 10,000 times without sleeping panics
   with its caller instead of hanging.
2. **Audio and speech.** The rodio thread is not started. `ReplayAudio` takes the same
   command enum (`sound/rodio_audio.rs:20`) on the game thread and answers
   `PlayingTrack`, channel and position queries from replay time and each stream's
   decoded length, so speech ends on the same step every run. Scenarios can still
   skip speech with input.
3. **Input.** The scenario is read before start and there is no automation I/O
   thread. Injected input is consumed only at input admission, by count.

Native verification (V2, V4) keeps the real clock and the audio thread.

The trace has one line per boundary (architecture.md section 7, `Observer`):

```
I <input#> <activity> <keys>
S <step#> <activity> <crc32> <state-digest> [<event>...]
P <present#> <capture-generation> <canvas-digest>
```

- `crc32` is the netplay checksum, unchanged.
- `state-digest` is the migration trace. Per element in link order: slot, generation,
  pred and succ slots, flags, every scalar field, logical parent (ship side and slot,
  or IP group id), target slot, hook names, frame-set owner and frame index. Per ship:
  species, side, crew, energy, the weapon, special and energy counters
  (`abi/ship_abi.rs:1787-1801`), status flags, characteristics, race state. At mode
  checkpoints: story bits, game clock, event queue, activity. Hook names come from one
  registry listing each hook, legacy or converted, under a stable name. No addresses
  or padding are hashed. `record` keeps the field values in a side file, so `check`
  prints the differing fields at the first differing step.
- Events: `spawn <side> <species>`, `hook <species> <hook>`, `death <side> <species>`.
- `canvas-digest` is the first 8 bytes of SHA-256 over the internal RGBA frame.
- `TraceRecord.elapsed_ms` (`automation/trace.rs:166-173`) is replay time; pacing
  reports and other host-timing fields are not part of the trace.

Acceptance, before any golden is committed and for each new entry: three recordings
are byte-identical: one on macOS, one on ubuntu CI, and one with
`UQM_REPLAY_JITTER=<seed>`, which adds random real sleeps of 0-20 ms at each boundary
while a CPU-load process runs.

### 2.1 Corpus

| Name | Basis | Covers |
|---|---|---|
| `melee-cyborg-a`..`d` | `super-melee-v1` flow, both sides Cyborg, `SuperMeleeBattle` seed | four fleets of 7 cover the 25 melee ships; `check` asserts from events that each ship spawned and fought two different opponent species, and, once its race is converted, ran its weapon and special hooks |
| `melee-input` | `super-melee-v1` with its recorded player input | human-input path |
| `campaign-battle` | `lost-battle-v1` | SIS flagship, campaign battle setup and teardown |
| `samatra` | `dev-docs/testing/samatra.md` | Sa-Matra, final battle path |
| `hyperspace` | `out-of-fuel-v1` | hyperspace element loop, fuel, Melnorme approach |
| `solar-flight` | `real-sol-starbase-to-mars` | IP flight, orbit, landing, takeoff |
| `lander` | `lander-destroyed-v1` | planet side, hazards, lander loss |
| `comm` | `comm-encounter-v1` | dialogue phrase sequence, speech completion |
| `new-game` | `main-menu-v1` then New Game | kernel load, menus, `NewGame` seeding |

Focused fixtures use `automation/melee_fixture.rs` setups with scripted input and run
a few hundred steps:

| Fixture | Exercises |
|---|---|
| `androsynth-blazer` | collision hook saved, replaced and restored (`androsynth_native.rs:389-397`) |
| `pkunk-rebirth` | hook restore, reincarnation, the `hTarget` hack (`pkunk_native.rs:401-442`) |
| `chmmr-satellites` | delayed point defence walking `pred` links (`chmmr_native.rs:406-460,494-507`) |
| `death-spawn` | a collision kills a ship whose death hook spawns elements mid-traversal |
| `mixed-collision` | converted VUX limpet hits an unconverted ship; unconverted Orz howitzer hits converted VUX |
| `pool-full` | fighters until `AllocElement` fails at 150 |
| `frame-sets` | Arilou threat test against a weapon still flying after its ship died and a same-species ship entered; blast after ship teardown; trilinear resolution index |
| `star-contact` | hyperspace star contact, repeat suppression, system exit, save and reload |

Goldens are created once, by U0-06 with the owner's approval, together with a counts
golden (`input_seen`, `present_seen`, action-phase sequence) for every
`rust/scripts/*.json`. Later PRs may add a golden only with a new corpus entry and the
three-recording evidence, and may not modify or delete one. A deliberate behaviour
fix is a separate PR with the `parity-golden-update` label and the reason in the
commit.

**V2. Native failure scenarios.** `lander-destroyed-v1`, `out-of-fuel-v1`,
`lost-battle-v1`, `super-melee-v1` on a real macOS window (CI `failure-scenarios`).

**V3. Save compatibility** (`cargo test`, added in Phase 0). Loading and saving are
not inverse operations: load returns the saved activity as `NextActivity`
(`savegame/live/load.rs:33-35`), and save prepares location and group state, derives
the summary from live solar state and clears `in_orbit` (`live/save.rs:16-34`).
- **V3a, codec.** For the 73 saves in `rust/scripts/saves/` and
  `tests/fixtures/savegame/legacy-save.00`, decode then encode gives the input bytes.
  A separate writer fixture covers the element-amount fill (`live/save.rs:65-69`).
- **V3b, runtime.** Per save, under replay mode with the clock frozen: fresh game,
  load, main loop takes `NextActivity` and enters the saved mode, stop at its first
  input admission, capture a `SaveFile` through the live path. Phase 0 commits these
  as `rust/tests/golden/saves/<name>.capture`; later phases compare their capture,
  through `snapshot` with the live `SessionView`, at the same boundary. The test
  lists the normalisations of the current path as data (activity, `in_orbit`,
  summary fields, IP group queue), so a capture may differ from its input file.

**V4. Native scenarios** (macOS). `native smoke` (`rust/xtask/smoke.json`) plus a
list of scenario ids fixed per unit in work-items.md; units that touch world state
or saves run all 45 `load-*-v1` scenarios. None of those saves starts in hyperspace
(41 assert `IN_INTERPLANETARY`), so hyperspace work uses `fly-to-alpha-centauri-v1`,
`load-save-v1` and `load-alpha-centauri-v1`. OS screenshots are inspected with the
image-comparer, per standards section 7.

**V5. Gates.** `cargo xtask quality`, `clippy`, `cargo test`, `deny`, `coverage`, five
release builds.

## 3. How main stays green

1. **One work unit per PR: change, migrate and delete together.** A unit is one
   vertical slice: a queue, a module, a session, or a group of races sized by their
   source and test lines. It introduces the new representation or API, migrates every
   user of the thing it replaces, and deletes the replaced code; the PR is not done
   until the replaced code is gone. Each unit's Deletes list names the items and a
   zero-count check (`raw_records`, `queue_calls`, `planned.<crate>`,
   `quality ident`, `uio_*` callers) proves them gone. No code with zero callers and
   no dual implementation survives a merge, and no PR relies on a later one to be
   correct. Transitional adapters exist only while live, unconverted users remain in
   code a later unit owns; work-items.md lists each with the unit that deletes it.
2. **One transition pattern.** The staged authority of architecture.md section 4.1,
   with `Hook::Legacy` and `app::bridge`. The counters `legacy_hooks` and
   `bridge_calls` get finite budgets in U1-02, the PR that creates them (quality-gates.md
   section 4). Budgets only shrink; `legacy_hooks` reaches 0 at Phase 2 exit,
   `bridge_calls` at Phase 6 exit, before the `uqm-modes` split.
3. **Mechanical and semantic changes in separate commits of the same PR.** Moving
   `GlobData.x` reads to accessors, CamelCase renames and crate moves are mechanical;
   V1 is the oracle for every commit.
4. **Size.** Up to about 6,000 non-mechanical changed lines per PR; mechanical
   commits (git mv, renames, accessor swaps, generated golden data) do not count.
5. **Tests move with the code.** 34 of the 61 files in `rust/tests/` declare `extern
   "C"` symbols (for example `tests/arilou_native.rs:8-14`). A PR that removes a
   symbol rewrites its test against the Rust API and keeps the golden data (for
   example `tests/fixtures/planets/*`, captured from the C originals).
6. **No long-lived branches.** At most five worktrees under `tmp/wt/`, each removed
   when its PR merges. Units reserve their `touches` paths and may share only
   registries and dispatchers listed as `shared` (AGENTS.md).
7. **No incremental compilation.** `rust/.cargo/config.toml` keeps
   `incremental = false`; build speed comes from crate reuse after the splits.

### 3.1 Crate splits

Crates follow the schedule of architecture.md section 2.2. Each split is one unit with
two kinds of commit: semantic commits that bring the crate's `planned.<crate>`
counter (quality-gates.md section 3.5) to 0, then a mechanical move commit, the last
in the PR:

1. `git mv` the directories to `rust/crates/<name>/src/`, keeping module paths; add the
   member's `Cargo.toml` (`version.workspace`, `publish = false`, `[lints] workspace =
   true`, only the external dependencies the moved code uses) and its `lib.rs` with
   the `unsafe` attribute of architecture.md section 2.1.
2. The root re-exports the moved modules and forwards the features; widen `pub(crate)`
   items used across the new boundary to `pub` (18 for `uqm-engine` by name match).
3. `cargo xtask quality move --prefix <old> <new>`; add the edge to `CRATE_DEPS`.
4. Record `cargo build --release --timings` (clean) and the rebuild after `touch
   rust/src/comm/mod.rs`, before and after, in the PR description.

The move commit changes no code inside a moved file except visibility, so V1 must be
identical across it without re-recording.

## 4. Pilot choice: battle core

| Candidate | Size | Depended on by | Oracle | What it proves |
|---|---|---|---|---|
| **Battle core** (element queue, process loop, collisions, ship/weapon helpers) | 10.7k lines, 40% unsafe | ships, Super Melee, hyperspace, IP flight, netplay checksum, hyperspace save step | netplay CRC plus migration trace; melee runs from a menu with a fixed seed | staged authority, re-entrant safe hooks, session ownership, bridge |
| Super Melee (setup, teams, netplay) | 6.7k, 49% | nothing | `super-melee-v1`, packet fixtures | session ownership (already shown by `planet_side`), packet codec |
| Planet generators | 16.9k, 32% | solar system | `tests/planets_parity.rs` | little: the logic is already safe (`planets/generate/orz.rs`) |
| World state (`GlobData`) | 689 references in 111 files | everything | V3, V4 | an access pattern, before any pattern is settled |

Battle core is chosen. The hardest part of the design (exact C ordering with
re-entrant callbacks) lives there, ships and Super Melee depend on its interfaces,
and it has the strongest oracle. Piloting Super Melee first would mean converting its
battle calls twice. The risk is that unconverted races, hyperspace and IP flight read
the records directly. Stage A keeps one representation for all of them, and the pilot
proves a mixed collision and a navigation callback on it before it expands.

## 5. Phases

Sizes are changed lines (added plus deleted) and files touched.

### Phase 0: soundness, gates, replay harness

Scope:
- xtask: `quality` (ports of llxprt-code-rs `loc.rs` and `complexity.rs`, plus ffi,
  unsafe, allows, layers, native-files, deps), `clippy`, `coverage`, `deny`, `parity`;
  `abi-check` extended to foreign statics. The source walk has a handler for each of
  the 16 item-generating first-party macros compiled today (U0-02 lists them), and the
  debt lints arrive with the `xtask clippy` classifier that baselines them (U0-03).
  `[workspace.lints]`, `rust/clippy.toml`,
  `rust/deny.toml`, the `xtask` and `xtask-native` Cargo aliases, xtask's game
  dependency behind its `native` feature, `ci.yaml` (quality-gates.md section 5).
- The first crate, `uqm-save`, with the save codec (architecture.md section 2.2), so
  the workspace plumbing, the crate-graph check and release packaging are proven on a
  crate with no cycle to break.
- Replay mode, migration trace, corpus and fixtures (V1), fed by the existing
  coordinator's input and presentation callbacks plus a checkpoint call at
  `rust_battle_frame_advance` and each mode's loop end; V3a and V3b goldens.
- Soundness fixes, landed first (U0-01) so the first baseline already counts them:
  - the 20 `clippy::not_unsafe_ptr_arg_deref` allows in 8 files become `unsafe fn`,
    starting with safe `InitQueue` (`collections/queue.rs:84-117`);
  - each foreign static gets its defining type: `DisplayArray`
    (`planet_side/orbit_scan.rs:79` declares 150 entries, the definition has 330),
    `CurStarDescPtr` (six declarations), `GlobData` (`CGlobData` vs `GLOBDATA`, which
    is Stage A for `GlobData`); this removes the 7 `clashing_extern_declarations`
    allows;
  - the ignored `_shadow` parameter of `getGameState`/`setGameState`
    (`navigation/globals.rs:19-27`) and the dead `GameState[155]` field go, leaving
    one authority for story bits (`state/ffi.rs:15`);
  - `ShipState`'s `unsafe impl Send` over raw pointers (`ships/traits.rs:61`) goes
    with the dormant model.
- Remove dormant parallel models after confirming them dead with `pub(crate)` and
  `dead_code`: `ships/traits.rs`, the non-`_native` halves of `ships/races/*.rs` and
  their registry path (`ships/registry.rs:1012-1090`), `battle/process_loop.rs`,
  `battle/display_list.rs`, `battle/tactical.rs`, `battle/integration.rs`.

Removes: about 5.5k production lines; 20 + 7 allows; the shell `c-ratchet` job.

Exit criteria:
- baseline committed; `quality --base` and `clippy --base` green on a PR and on a
  push to `main`; `quality deps` passes on the current lockfile with `bundled-sdl`;
- `native-files` = 0; `not_unsafe_ptr_arg_deref` = 0; `clashing_extern_declarations` = 0;
- `abi-check` passes for functions and statics;
- V1 acceptance (three identical recordings) for every corpus entry and fixture;
- V3a and V3b pass for 74 files; coverage floor recorded; `cargo xtask deny` passes;
- `uqm-save` is a member with `#![forbid(unsafe_code)]`; `cargo xtask quality` runs
  without compiling the game; `planned.uqm-engine` is measured (20 by the scan).

Verification: V1, V2, V3, V5. Size: xtask +4k lines; src about -5k; about 60 files.
Units: U0-01 to U0-06 (soundness with the dormant-code removal; the xtask foundation; the debt lints and gates; `uqm-save`; replay mode; parity tooling with the corpus and goldens).

### Phase 1 (pilot): battle core, Stage A and first Stage B consumers; `uqm-engine`

Scope: `battle/{process_native, battle_native, world_elements, weapon_native,
ship_native, velocity, gravity, status_native, tactrans_native}.rs`, queue globals in
`game_init/kernel_abi.rs:36,43` and the hook writes in all race files. PRs in this
order:

1. Stage A (architecture.md section 4.1): records and queue statics move to
   `engine::battle::{record, legacy}`; the ABI copies become re-exports. Layouts and
   hook fields are unchanged (U1-01).
2. Stage B in one transition with its first consumer: `ElementTable`,
   `ElementId`/`ShipId`, `Cx`, `app::bridge`, the hook fields as
   `ElementHook`/`CollisionHook` with every call through `invoke_*` (the legacy
   writes are one mechanical commit), and the process loop and collisions on ids
   (U1-02). `LinkQueue<T>` arrives with its first users in Phase 2 (U2-06).
3. Proof (U1-03): the Native hook variant with its first users, VUX, Androsynth and
   the battle core's own hooks, and the hyperspace `star_collision` and
   `star_death` callbacks (`navigation/hyper_transition.rs:191-229`) moved to `Cx`;
   `mixed-collision`, `death-spawn`, `androsynth-blazer`, `star-contact` and
   `chmmr-satellites` (unconverted, walking raw `pred` links on the shared storage)
   pass. The `Race`, `ElementHooks` and `Cx` APIs are approved here, with the shared
   battle test harness `tests/support/battle.rs` that every race test moves onto; no
   further conversion merges before this PR.

A second lane, independent of the battle files, splits the first engine crate in one
unit (U1-04), a semantic commit then the move commit:

4. The 13 upward paths and 7 foreign statics of the former `libs/` layer
   (architecture.md section 2.2) are inverted: graphics no longer names automation,
   the dir handles and `input_templates` are defined in `io` and `input`, the game
   registers the ship code resource type, and the game clock stays in the root.
5. The mechanical move of that layer into `uqm-engine` (section 3.1).

Arilou and the readers `supermelee/netplay/checksum.rs:55`,
`savegame/runtime/hyper_save.rs:129` and `automation/battle_fight.rs:456` move to
Phase 2, where they join a race group and the reader units.

Removes: the `ELEMENT`/`STARSHIP`/`RACE_DESC` copies in `navigation_abi.rs`,
`netplay_abi.rs` and `global_state_abi.rs` with their layout assertions and
`link_name` aliases; statics `zoom_out`, `nth_frame`, `OPT_MAX_ZOOM_OUT` and
`battle_counter`, which move into `Battle` (readers in `supermelee`, `pickship` and
`automation` use counted bridge accessors). Kept until Stage C: the authority
statics (`disp_q`, `race_q`, `DisplayArray`, `DisplayLinks`, `DisplayFreeList`),
raw links, `pParent`, `hTarget`, unions, `farray`.

Exit criteria:
- one definition of each record and queue static; no alias for them in `abi-check`;
- battle `static_mut` holds only the Stage A authority statics; the other battle
  statics are in `Battle` or `Kernel`; `legacy_hooks` and `bridge_calls` within budget;
- VUX and Androsynth: 0 `unsafe`, 0 raw-pointer parameters; battle `extern "C"
  fn` definitions are only entry points called by unconverted code;
- stale-id lookups across the corpus = 0;
- `uqm-engine` is a member; `planned.uqm-engine` = 0; about 50k production lines have
  left the root package.

Verification: V1 (corpus and fixtures identical); V2; V3; V4 with the battle and
hyperspace scenarios listed per unit, plus `smoke.json`; `tests/netplay_packets.rs`.
Size: about 45 files, 8-10k lines, plus the mechanical move. Units: U1-01 to U1-04.

### Phase 2: ships, combat AI, remaining record readers, Stage C

Scope: the other 25 `ships/races/*_native.rs`, `ships/{code_resources, loader_native,
master_native, native, native_adapters, battle_bridge, ffi_contract, catalog,
lifecycle, writeback, queue, runtime, types}.rs`, `battle/cyborg_native.rs` (walks the
queue at `:170`), `battle/ai*.rs`; then the element callbacks and record readers in
`navigation/{hyperspace, hyper_radar, hyper_transition, hyper_encounters,
interplanetary}.rs`, `planet_side/collision_adapter.rs`, `front_end/uqmdebug.rs` and
`savegame/runtime/queues.rs`, and the three readers moved from Phase 1; the
`STARSHIP` and `RACE_DESC` readers in `pickship/ffi.rs`,
`supermelee/native/battle_picker.rs`, the encounter build and the battle sound
tracker (`battle/sounds.rs`, which keeps `*mut ELEMENT` and indexes `DisplayArray`).
The races go in six units sized by race plus test lines (at most about 5,400), with
Melnorme, Chmmr, Pkunk and Orz each the only hard race of its unit and each fixture in
the unit of its race. Orz goes last (U2-08), after every other race unit and both
reader units; it converts the last legacy hook, replaces the race templates by
`race(species)` and deletes `Hook::Legacy`, the `RACE_DESC` function-pointer
dispatch and the battle entry points of `app::bridge`, whose only callers were legacy
hooks. Stage C (architecture.md section 4.1, U2-09) lands `OrderedPool` with its
order tests and its first users, and runs only when `raw_records` outside
`engine::battle` is 0.

Removes: 28 `static mut <RACE>_DESC` templates and `INITIALIZERS`
(`ships/code_resources.rs:75`); `RaceDescPtr`; `data: *mut c_void`; `Hook::Legacy`;
the battle part of `app::bridge`; the Stage A authority statics;
the record types and `ElementProcessFunc`/`ElementCollisionFunc` in `ship_abi.rs`.

Exit criteria: ships and battle have no `ffi` or `unsafe` baseline entries;
`legacy_hooks` = 0 and its budget deleted; the corpus assertions of section 2.1 hold
for every race. Verification: V1, V2, V3, V4 subset (`campaign-s06c-utwig-bomb`,
`samatra`, `load-connected-final-fleet-v1`), V5.
Size: about 95 files, 35-40k lines including the rewritten race tests.
Units: U2-01 to U2-09.

### Phase 3: Super Melee and netplay

Scope: `supermelee/` (setup, team files, picker, netplay), `pickship/`, `network/` and
`network.rs`, `battle/netplay.rs`.

Removes: `packetTypeData` (`supermelee/netplay/packet.rs:29`) and `#[repr(C)]` packet
structs (become `enum Packet` with explicit codec); `netConnections`, `netplayOptions`
and the other netplay statics, whose readers in the melee menu, the connection dialog
(`front_end/cnctdlg.rs`) and the battle move to the `NetplaySession` in the same unit;
supermelee `extern_c_fn` 267 → 0, `no_mangle` 221 → 0, `static_mut` 16 → 0. Netplay
reaches sockets through the `engine::host::Net` port; the socket code (std::net, or
libc/WinSock kept as an audited module if non-blocking connect needs it) stays in
`uqm-engine::network` until U7-06 moves it to `uqm-platform::net`. The `.mle` byte
layout moves to `uqm-save`; team-file I/O moves to `Files` in U7-03. The pickers'
record access was already converted in U2-06.

Exit criteria: supermelee and pickship unsafe = 0; network unsafe only in the socket
module; packet byte fixtures unchanged; a two-peer loopback battle over the in-memory
`Net` agrees on checksums every interval and matches the `melee-input` golden; `.mle`
team-file fixtures round-trip through `uqm-save`. Verification: V1, V2
(`super-melee-v1`), V3, V5. Size: about 45 files, 6-8k lines. Units: U3-01 (packets, session and
the `Net` port) and U3-02 (the melee session); U3-01 depends only on Stage B (U1-02) and
the `uqm-engine` split and may run during Phase 2; U3-02 may run
during Phase 4.

### Phase 4: world state and main loop

Scope: `navigation/globals.rs`, `state/`, `time/` (game clock and `event_q`),
`mainloop/`, `game_init/`, `c_bindings.rs`, `comm/locdata.rs` (`CGlobData`),
`savegame/runtime/*` live readers, `automation/campaign_state.rs` and the
`Coordinator` statics (become the `Observer`), and every remaining `QUEUE` consumer.
`GlobData` goes through Stage B and Stage C in one unit (U4-01): a mechanical commit
moves its 689 references in 111 files to `World` accessors and a semantic commit
makes it the owned `World` in `Game`, so no accessor layer over the static survives;
`CurStarDescPtr` becomes `World::current_star` in the same unit. The GAME_STATE
queues and `event_q` then move to `OrderedPool` behind `LinkQueue` views together
with every consumer in every subsystem (comm, planets, starbase, ui, front_end,
battle, ships, navigation, savegame), and `collections/queue.rs` is deleted in the
same unit with its `abi/` wrappers and re-exports and the queue accessors of
`mainloop/state_abi.rs` (U4-02, `queue_calls` = 0).

Removes: `GlobData`; `CurStarDescPtr`; `GLOBAL_GAME_STATE` and both
`GLOBAL_STATE_FILES` (`state/ffi.rs:15-16`, `state/state_file.rs:321`); string-keyed
story access (`state/game_state_keys.rs:477`) and raw bit indices
(`planets/pstarmap.rs:202-213`); the `global!` kernel statics; `GameLoopOps`/`CffiOps`
(`mainloop/game_loop.rs:290`); `collections/queue.rs`. `abi/global_state_abi.rs`
moves to Phase 5: 65 `planets` files and 28 others import it as
`c_bindings::global_state`.

Exit criteria: `GlobData` references = 0; `static_mut` = 0 in state, time, mainloop,
game_init, navigation, collections; story lookups by string = 0; `bridge_calls`
limited to mode entry points not yet converted; before the old coordinator hooks
are deleted, every `rust/scripts/*.json` scenario gives the same `input_seen` and
`present_seen` counts and action-phase sequence as its trace from the old hooks.
The last unit moves the battle core, `World`, `Game`, `Kernel`, `StoryState`,
`GameClock`, `Observer`, the host ports and `engine::save` from `rust/src/engine/` and
the root modules into `uqm-engine`; the state-file byte layouts move to `uqm-save`.
Verification: V1 (with `star-contact`), V2, V3b through `snapshot`/`apply` against
the Phase 0 captures, full V4 (45 `load-*` scenarios and the connected campaign
producers), V5. Size: about 130 files, 10-14k lines, mostly mechanical.
Units: U4-01 to U4-04.

### Phase 5: navigation and planets

Scope: the rest of `navigation/hyper_*` (UI, radar, menu, transitions, encounters),
`navigation/interplanetary.rs`, `planets/{solarsys, orbit, scan, pstarmap, starmap}`,
`planets/generate` (vtable and `*_exports.rs`), remaining FFI in `planet_side/`.

Removes: `GenerateFunctions` and its 13 hook types (`planets/generate/vtable.rs:29`);
`exports.rs`, `home_exports.rs`, `late_exports.rs`, `special_exports.rs`,
`story_exports.rs`, `sol_exports.rs`; `getGenerateFunctions` (`battle/gendef.rs:74`);
`pSolarSysState`; `abi/global_state_abi.rs`; planets `static_mut` 46 → 0 and
`extern_c_fn` 199 → 0; navigation 43 → 0; the five `#![cfg(not(test))]`
`navigation/hyper_*` modules become testable.

Exit criteria: planets, navigation and planet_side have no `ffi`/`unsafe` baseline
entries; `cfg_not_test_modules` ≤ 11. Verification: V1 (`hyperspace`,
`solar-flight`, `lander`, `new-game`, `star-contact`), `tests/planets_parity.rs`
against its C-captured fixtures through the new API, V2, V3, V4, V5.
`abi/navigation_abi.rs` moves to Phase 6: 14 `ui` and 7 `front_end` files import it,
plus 16 others that U6-04 switches. All generators convert in one unit (U5-01), so
the vtable is never a second dispatch path. The solar system work is two units: the
session with IP flight, orbit, scan, starmap and every `pSolarSysState` reader
(U5-02), then planet side with the `global_state` import swap (93 importers) and the
ABI deletion (U5-03).
Size: about 110 files, 15-17k lines. Units: U5-01 to U5-04.

### Phase 6: comm, starbase and menus

Scope: `comm/` (dialogue `Api` implementations, segues, oscilloscope, ambient
animation), `starbase/`, `ui/`, `front_end/`, `mainloop/restart_menu/`.

Removes: `COMM_STATE` (`comm/state.rs:23`), `COMM_DATA` (`comm/locdata.rs:15`),
oscilloscope statics (`comm/oscill_abi.rs:48-51`); comm `static_mut` 70 → 0;
ui and front_end `extern_c_fn` 119 → 0, `static_mut` 65 → 0; the ten `ui` modules and
`front_end/cnctdlg.rs` stop being `#![cfg(not(test))]`; `abi/navigation_abi.rs`;
`app::bridge` and its budget. The last unit splits `uqm-content` and `uqm-modes`.

The comm session comes first (U6-01): `COMM_STATE`, `COMM_DATA`, the oscilloscope
and the other comm runtime statics become the `Comm` session, and every reader and
writer moves to it in the same PR, including hail, the comm ABI helpers, the locdata
copier, credits and the dialogue adapters, whose callbacks take the session. The
dialogue host boundary follows (U6-02): `ScriptApi` becomes the content trait
`DialogueHost`; the `CLocData`/`CommData` descriptors become content data; dispatch
constants move to the engine; the `LiveScript` adapter, which calls comm and ui
directly (`comm/races/script.rs:65-89`), becomes `modes::comm::Host { game, comm }`;
each race's `live` module (`comm/races/orz.rs:846-875`) becomes an entry in a
modes-side table. The last unit (U6-04) brings `planned.uqm-content` and
`planned.uqm-modes` (counted from U0-03) to 0 in its semantic commits, so its move
commit finds nothing.

Exit criteria: comm, starbase, ui and front_end have no `ffi`/`unsafe` baseline
entries; `cfg_not_test_modules` = 0; `bridge_calls` = 0; dialogue script bodies in
`comm/races/*` are unchanged; `uqm-content` and `uqm-modes` are
members with `#![forbid(unsafe_code)]`. Verification: V1 (`comm`, `new-game`),
dialogue unit tests, `starbase-visit-v1`, `main-menu-v1`, V2, V3, V4, V5.
Size: about 120 files, 13-16k lines, plus the mechanical moves. Units: U6-01 to U6-04;
U6-03 depends only on Phase 4 and U6-01 on U5-01, so both may run during Phase 5.

### Phase 7: platform and content

Scope: `graphics/` (drawables, contexts, frames, fonts, colour maps, DCQ, canvases,
scalers), `resource/`, `io/` (`uio_bridge.rs`, 2.9k lines), `strings/`, `sound/`
(decoder vtables such as `rust_ova_DecoderVtbl`, `lib.rs:51`), `video/`, `input/`,
`threading/`, `callback/`, the OS part of `time/`.

Removes: `abi/graphics_abi.rs` and the graphics `*_abi.rs` shims; `SDL_Surface`
canvases (become Rust pixel buffers, SDL used only to present); the `uio_*` C-style
API (becomes `uqm_platform::vfs::Vfs` behind the `Files` port, which covers read,
write, remove and list); `res_*` (becomes
`ResourceCache`); decoder vtables (become `trait Decoder`); `threading/legacy_abi.rs`;
the present hook of U1-04 (the caller calls `Observer::presented`). Graphics
`no_mangle` 243 → 0, `extern_c_fn` 295 → 0.

Order: the render pipeline (canvases, present, DCQ, contexts) moves with video's
canvas, surface and present use (U7-01). The Files port with one Vfs then arrives
with every one of its callers (resource, strings, graphics loading, decoder streams,
video, saves, LZH, melee team files) and deletes `uio_*` (U7-03). Drawables, frames,
fonts and colour maps move with `ResourceCache` and video's screen-format call
(U7-02), then decoders and the audio port (U7-04); input, threading and the clock
run beside them (U7-05). Automation's POSIX process, descriptor and fd-relative file
calls (`automation/child_session.rs`, `native_runner.rs`, `interrupt.rs`) and
`platform.rs`'s descriptor and errno operations move behind `engine::platform::os`
in the unit that then carves `uqm-platform` out of `uqm-engine`, with `os` and the
CoreGraphics and sandbox parts of automation (U7-06).

Exit criteria: `unsafe` only in `UNSAFE_ALLOWED` modules; `uqm-engine` carries
`#![forbid(unsafe_code)]` and no OS crate dependency; `FrameSetRef` handles backed by
`ResourceCache`; no `include!` of `abi/*` except under `uqm-platform`.
Verification: V1 with canvas digests for every corpus entry, V2, V3, V4 including the
intro presentation (`tests/intro_presentation.rs`) and real-window screenshots
inspected for every smoke scenario, five release builds, V5.
Size: about 170 files, 24-28k lines. Units: U7-01 to U7-06.

### Phase 8: closeout

Removes: `rust/src/abi/` (12.1k lines today), `xtask abi-check`, all
`non_snake_case`/`non_upper_case_globals`/`non_camel_case_types` allows, and the root
re-exports of crate modules (replaced by crate paths). Adds `#![forbid(unsafe_code)]`
to the root and moves the debt lints to `deny`.

Exit criteria: crate-wide `static_mut` = 0, `extern_c_fn` = 0 outside platform,
`no_mangle` = 0, foreign items, `repr(C)` and raw-pointer parameters only in platform,
`bridge_calls` = 0, `allow` attributes only the platform `unsafe_code` allows,
`unwrap`/`expect` in production = 0, coverage ≥ 70%. Remaining baseline entries are
LOC and complexity only. Verification: full V1-V5, with the `S8` bundle once
`abi-check` is gone. Size: about 40 files, net deletion. Units: U8-01.

## 6. Review response

Each finding of the design review was checked against the cited code at 0ba2845ad.
All are accepted; none is rejected.

| # | Verdict | Change |
|---|---|---|
| 1 | accepted | ARCH 4.1 staged authority and its rules; Phases 1-2 reordered, Stage C at Phase 2 exit |
| 2 | accepted | V1 replay clock, `ReplayAudio`, boundaries, trace projection, three-recording acceptance |
| 3 | accepted | QG 3.4: bans first-party native builds and vendoring, allows the external graph |
| 4 | accepted | ARCH 2, 5.1, 5.3: engine `Cx`, content host traits, copy-call-reread, two sequences |
| 5 | accepted | V3a and V3b; ARCH 7 `SessionView`, `apply` returns the activity |
| 6 | accepted | V1 state digest with field reports; ARCH 7 checksum description corrected |
| 7 | accepted | V1 event assertions, eight fixtures, demanding hooks in the Phase 1 proof PR |
| 8 | accepted | ARCH 7 three `Observer` hooks and their order; Phase 4 trace-count check |
| 9 | accepted | QG 4 budgets, debt transfers, reported headroom, per-file coverage |
| 10 | accepted | ARCH 3 `World::current_star`; `star-contact`; removal moved to Phase 4 |
| 11 | accepted | ARCH 4 `FrameSetRef`; `frame-sets` fixture |
| 12 | accepted | QG 2.3 `--force-warn`, 3.2 walk and identity, allow scopes |
| 13 | accepted | QG 5 base fetch, push handling, Cargo alias |

### 6.1 Corrections from the crate analysis

Checking the module graph for the crate split found three places where the phase
text disagreed with the code, and the work breakdown moved one item. The phases above
include all four:

| Item | Was | Now | Reason |
|---|---|---|---|
| `abi/global_state_abi.rs` removal | Phase 4 | Phase 5 | 65 `planets` files and 28 others import `c_bindings::global_state` |
| `abi/navigation_abi.rs` removal | Phase 5 | Phase 6 | 14 `ui`, 7 `front_end` and 16 other files import `navigation::abi` |
| `bridge_calls` reaches 0 | Phase 8 | Phase 6 | the bridge lives in the app; `uqm-modes` cannot call it once split |
| Arilou, three record readers | Phase 1 | Phase 2 | keeps the pilot to the API proof; Arilou joins a race unit with `frame-sets` |

## 7. Work units and agents

[work-items.md](work-items.md) splits the phases into 40 work units (P0 6, P1 4,
P2 9, P3 2, P4 4, P5 4, P6 4, P7 6, P8 1), each one PR with its dependencies,
reserved and shared files, scope, deletes, acceptance commands (V4 scenario ids
included), exit counters and risks, plus the transitional adapters, the waves (28),
the critical path (24 units), the mapping from the earlier 63 units and the label
set. The issues are the source of truth once they exist. Agents follow [AGENTS.md](AGENTS.md).

## 8. Review 2 response

Each finding of the design review (summarised in plan.md section 8) was checked against the cited code at 35d33879b. All 15
are accepted; none is rejected. The Finding and Verified columns use the review's
(old) unit ids; the Disposition column uses the new ones.

| # | Finding | Verified | Disposition |
|---|---|---|---|
| 1 | P0-01 cannot activate its gates green | 16 item-generating `macro_rules!` compile today (`state_abi.rs:14`, `hash_table.rs:396`, `home_exports.rs:268`, `setupmenu.rs:146` and 12 more); `encode.rs:44` uses `expect` | accepted: P0-03 handles all 16 with tests; debt lints move to P0-04 with the classifier that replaces raw clippy |
| 2 | P1-01 hook types need P1-02's API | hook fields are `Option<unsafe extern "C" fn>` (`ship_abi.rs:1302-1319`) | accepted: P1-01 is record and static unification only; P1-02 changes ids, `Cx`, bridge, hook enums and every call site in one transition |
| 3 | Stage C deletes records with live raw readers | `pickship/ffi.rs:164-180,408-424`, `battle_picker.rs:73-74,391,416`, `battle/sounds.rs:3-9,128-138,161-166` | accepted: P2-14 converts pickers, encounter build, sounds and the remaining battle files; `raw_records` (P0-03) covers ship, descriptor, `DisplayArray` and queue names; P2-15 and P2-16 depend on it |
| 4 | P4-03 is not the last queue user | about 40 non-test files call `collections::queue` (comm, planets, starbase, ui, front_end, ships, battle) | accepted: `LinkQueue` views from P1-02; P4-03 moves every consumer (`queue_calls` = 0); P4-04 swaps storage and deletes `queue.rs` |
| 5 | P6-02 does not prepare `comm/races` | `orz.rs:3-4,63,793-795,846-875`, `script.rs:65-89` name comm, ui and graphics | accepted: P6-02 is the dialogue host boundary (content trait, content descriptors, modes adapters); P0-04 counts `planned` for content and modes; P6-04 requires both 0 before P6-05 |
| 6 | UIO deleted before its consumers; `Files` read-only | `resource/direct.rs:22-45`, `getstr.rs:52-65`, `aiff_ffi.rs:41-62`, `video/ffi.rs:112-151`; saves write and remove (`slots.rs:61-87`), melee writes and lists (`persistence.rs:94-100,269`) | accepted: P7-04 Vfs and `Files` with read, write, remove, list; P7-05 to P7-07 convert callers; P7-07 deletes `uio_*` last and converts melee files |
| 7 | Graphics teardown before video moves | `video/ffi.rs:18,42-45,846-852`, `video/control.rs:30-38` | accepted: video presentation moves in P7-01, the screen-format call in P7-03; P7-06 keeps decoding and audio |
| 8 | POSIX process code not assigned | `child_session.rs:253,351-358,384-411`, `native_runner.rs` (openat, unlinkat, mkfifo, signal), `platform.rs:12-36,71-101` | accepted: P7-09 moves them behind `engine::platform::os` before the carve; `os` joins `UNSAFE_ALLOWED` |
| 9 | Alarm clock stays real in replay | `callback/ffi.rs:12-14` reads `SDL_GetTicks`; `ui/sis.rs:1059-1073` schedules from game time | accepted: P0-06 routes `now_ms` through `time::source` with an order test |
| 10 | Dependencies do not order tooling | P0-03 used the alias, P0-07 lacked P0-02, P2-09 wrote the engine crate without P1-06 | accepted: soundness (P0-01) uses the manifest-path command in S0; P0-07 depends on P0-04; OrderedPool (P2-01) depends on P1-06 |
| 11 | Oversized semantic units | Melnorme, Chmmr, Pkunk: 1,756 source and 3,939 test lines | accepted: ten race units sized by source plus test lines (at most about 2,800), one hard race each; OrderedPool own PR; P5 solar work in three units; graphics in three; P2-16 at 2,900 without the pool |
| 12 | Rules forbid required transition work | `AGENTS.md` banned all new unsafe; P0-03 adds `unsafe fn`; P0-08 creates goldens | accepted: AGENTS.md "Unsafe during the migration" (moves allowed at no workspace growth, allowances owner-approved) and "Goldens" (creation by P0-08 only); soundness lands before the baseline |
| 13 | Invalid or hollow acceptance commands | multi-filter `cargo test`, `grep -c` exit codes, `abi-check` after deletion, empty replay output, unassigned `parity counts` | accepted: one filter per invocation; `quality report --expect` and `test $? -eq 1` forms; `S8` from P8-01; replay summary test in P0-06; `parity counts` in P0-07 with goldens in P0-08 |
| 14 | V4 commands and scenarios not fixed | V4 rendered as a link; no `load-*-v1` save starts in hyperspace (41 assert interplanetary) | accepted: each unit lists its scenario ids; work-items.md gives the complete commands per unit; the docs are published at `dev-docs/design/idiomatic-rust/` before issues |
| 15 | Parallel-safety claim unsupported | `xtask/src/lib.rs:94-104` dispatcher and race registries missing from touches | accepted: `touches` and `shared` per unit, one shared-file rule in AGENTS.md and work-items.md, waves and critical path recomputed (36 waves, 34-unit path) |

Unit count: 51 before, 63 now (+13, -1). Added: the pickers and sounds reader unit,
the queue view unit, one Files caller unit, the OS boundary, five race units, the
pool, one graphics unit and two solar units. Merged: the corpus and the fixtures
(sequential, same files). Kept whole rather than split: the walker with its baseline,
and Stage B infrastructure with the hook change. P6-02 was rewritten in place as the
dialogue boundary. Merging further would join unrelated work or mix mechanical and
semantic changes, so 63 is the count against the target of about 60.

The unit ids in the table above are those of the 63-unit breakdown; the consolidation
below maps each of them to one current unit (work-items.md, "Old units").

### 8.1 Consolidation

The owner's rule: the change and the migration happen in the same unit, and removing
the dead code is part of its acceptance; 63 PRs were too many. Every unit is now a
vertical slice that introduces the new representation or API, migrates every user and
deletes the replaced code, with a Deletes list and a zero-count check per item
(`cargo xtask quality ident`, added to U0-02 for this). A semantic unit may reach about
6,000 changed lines; mechanical commits do not count.

Unit count: 63 before, 40 now (P0 6, P1 4, P2 9, P3 2, P4 4, P5 4, P6 4, P7 6, P8 1).
The merges join units that only separated adding from migrating or deleting: the pool
with Stage C; Stage B with the process loop; queue views with the queue storage;
`World` accessors with the owned `World`; the first generators with the vtable
deletion; the solar session with orbit and scan; the Files port with its last callers and the `uio_*` deletion; each crate's
semantic preparation with its move. Races pair into units of about 5,400 lines with at
most one hard race each; Orz joins the ships infrastructure so that the unit converting
the last legacy hook deletes `Hook::Legacy`. Seven transitional adapters remain, each
with its deleting unit (work-items.md, "Transitional adapters"); the three-stage
storage transition is kept only for the battle records, where unconverted races
dereference them. The comm session and the dialogue boundary were merged at first and
are two units again: once every comm-global reader (hail, the comm ABI helpers, the
locdata copier, credits) and the comm runtime statics joined the session unit, it
passed 6,000 lines. U6-01 moves every reader and writer, the dialogue adapters
included, and U6-02 changes only the adapters' form, so no adapter sits between them.
The critical path is 24 units over 28 waves.

Each fix of the table above still holds:

| # | Where it lives now |
|---|---|
| 1 | U0-02 handles all 16 macros with tests; debt lints arrive with their classifier in U0-03 |
| 2 | U1-01 unifies records and statics only; U1-02 changes ids, `Cx`, bridge, hook enums and every call site in one transition |
| 3 | U2-06 converts pickers, encounter build, sounds and the remaining battle files; U2-09 depends on it through U2-08 and asserts `raw_records` = 0 |
| 4 | U4-02 moves every queue consumer, the `abi/` wrappers and the `mainloop/state_abi.rs` queue accessors included, and swaps the storage in one PR; `queue_calls` = 0 and `queue.rs` deleted |
| 5 | U6-02 is the dialogue host boundary, after U6-01 moves every comm-global reader and writer into the session; U6-04 requires both planned counters at 0 before its move commit |
| 6 | U7-03: one Vfs, `Files` with read, write, remove, list and stat, every caller converted, `uio_*` deleted last in the same PR |
| 7 | video presentation moves in U7-01, the screen-format call in U7-02; U7-04 keeps decoding and audio |
| 8 | U7-06 moves the POSIX process, descriptor and errno code behind `os` before its carve commit |
| 9 | U0-05 routes `now_ms` through `time::source` with an order test |
| 10 | U0-01 uses S0; U0-06 depends on U0-03, U0-04 and U0-05; `OrderedPool` (U2-09) depends on U1-04 |
| 11 | race units sized by source plus test lines, one hard race each; semantic units at most about 6,000 lines |
| 12 | AGENTS.md "Unsafe during the migration" and "Goldens" (creation by U0-06 only) |
| 13 | one filter per invocation; `--expect` and `test $? -eq 1` forms; `S8` in U8-01; replay summary test in U0-05; `parity counts` and goldens in U0-06 |
| 14 | every unit lists its exact V4 scenario ids, unions of the merged units' lists |
| 15 | `touches` and `shared` per unit, the one shared-file rule, waves and critical path recomputed |
